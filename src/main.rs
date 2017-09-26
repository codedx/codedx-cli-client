extern crate clap;
extern crate futures;
extern crate hyper;
extern crate serde;
extern crate tokio_core;
extern crate url;

#[macro_use] extern crate maplit;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use clap::{Arg, ArgMatches, App};
use futures::{Future, Stream};
use hyper::{Client, Method, Request, Uri};
use hyper::header;
use hyper::header::Authorization;
use std::collections::HashMap;
use std::option::Option::*;
use std::str::FromStr;
use tokio_core::reactor::Core;
use url::{Url};

fn main() {
    match ClientConfig::from_cli_args() {
        Ok(config) => {
            println!("arguments are good: {:?}", config);
            let mut client = ApiClient::new(&config).unwrap();
            let request = client.get_project_list();
            match client.run(request) {
                Ok(projects) => {
                    println!("All projects:");
                    for project in projects {
                        println!("  {:?}", project);
                    }
                },
                Err(e) => {
                    println!("Error in request: {:?}", e);
                }
            }

            println!("");

            let query_projects = client.query_projects(&ApiProjectFilter {
                name: Some("scrape"),
                metadata: Some(hashmap!{
                    "Owner" => "dylan"
                })
            });
            match client.run(query_projects) {
                Ok(projects) => {
                    println!("Projects with a name matching 'scrape' and owned by 'dylan':");
                    for project in projects {
                        println!("  {:?}", project);
                    }
                },
                Err(e) => {
                    println!("Error in request: {:?}", e)
                }
            }
        },
        Err(ConfigError::MissingAuth) => println!("Authorization info missing or incomplete. Either an API Key or a Username + Password must be provided"),
        Err(ConfigError::MissingUrl) => println!("Missing the Base URL"),
        Err(ConfigError::InvalidUrl) => println!("Invalid Base URL"),
    };
}

#[derive(Serialize)]
struct ApiProjectFilter<'a> {
    name: Option<&'a str>,
    metadata: Option<HashMap<&'a str, &'a str>>
}

#[derive(Deserialize, Debug)]
struct ApiProject {
    id: u32,
    name: String,
    #[serde(rename = "parentId")]
    parent_id: Option<u32>,
}

type BoxedClientFuture<T> = Box<ClientFuture<T>>;
type ClientFuture<T> = Future<Item = T, Error = ClientError>;
type ClientResult<T> = Result<T, ClientError>;

struct ApiClient<'a> {
    config: &'a ClientConfig,
    core: Core,
    client: Client<hyper::client::HttpConnector, hyper::Body>
}

enum ReqBody {
    Json(serde_json::Value),
    // TODO: support multipart forms
}

impl From<ReqBody> for hyper::Body {
    fn from(body: ReqBody) -> hyper::Body {
        match body {
            ReqBody::Json(value) => {
                let raw_body = serde_json::to_vec(&value).unwrap();
                hyper::Body::from(raw_body)
            },
        }
    }
}

impl <'a> ApiClient<'a> {
    fn new(config: &'a ClientConfig) -> std::io::Result<ApiClient> {
        Core::new().map(|core| {
            let client = Client::new(&core.handle());
            ApiClient { config: &config, core, client }
        })
    }

    pub fn run<T>(&mut self, f: Box<ClientFuture<T>>) -> ClientResult<T> {
        self.core.run(f)
    }

    fn get_project_list(&self) -> BoxedClientFuture<Vec<ApiProject>> {
        self.request_json(Method::Get, &["x", "projects"])
    }

    fn query_projects<'f>(&self, filter: &'f ApiProjectFilter) -> BoxedClientFuture<Vec<ApiProject>> {
        let body = ReqBody::Json(json!({
            "filter": filter
        }));
        self.request_json_with_body(Method::Post, &["x", "projects", "query"], Some(body))
    }

    /// Underlying method for creating a request to be sent to Code Dx.
    ///
    /// Accepts a `method` (GET, POST, etc)
    /// and `path_segments` which form the API path, e.g. `&["x", "projects"]`.
    ///
    /// A type parameter `T` is given to indicate the concrete type the response
    /// should be deserialized to once the body is parsed as JSON.
    ///
    /// Returns a Boxed future that resolves with either the deserialized `T`
    /// or a `ClientError`.
    fn request_json<T>(&self, method: Method, path_segments: &[&str]) -> BoxedClientFuture<T>
        where T: serde::de::DeserializeOwned + std::fmt::Debug + 'static
    {
        self.request_json_with_body::<T, String>(method, path_segments, None)
    }

    /// Underlying method for creating a request to be sent to Code Dx.
    ///
    /// Accepts a `method` (GET, POST, etc),
    /// `path_segments` which form the API path, e.g. `&["x", "projects"]`
    /// and an optional `body` which will be attached to the request.
    ///
    /// A type parameter `T` is given to indicate the concrete type the response
    /// should be deserialized to once the body is parsed as JSON.
    ///
    /// Returns a Boxed future that resolves with either the deserialized `T`
    /// or a `ClientError`.
    fn request_json_with_body<T, B: Into<hyper::Body>>(&self, method: Method, path_segments: &[&str], body: Option<B>) -> BoxedClientFuture<T>
        where T: serde::de::DeserializeOwned + std::fmt::Debug + 'static
    {
        let mut req = Request::new(method, self.config.api_uri(path_segments));
        self.config.auth_info.apply_to(&mut req);
        for b in body {
            req.set_body(b);
        }

        println!("Request:\n{:?}", req);

        Box::new(
            self.client.request(req)
                .map_err(|err| ClientError::Protocol(err))
                .and_then(|res| {
                    match res.status() {
                        code if code.is_success() => Ok(res),
                        code => Err(ClientError::NonSuccess(code))
                    }
                })
                .and_then(|res| {
                    res.body()
                        .map_err(|err| ClientError::Protocol(err))
                        .fold(Vec::new(), |mut acc, chunk| {
                            acc.extend_from_slice(&*chunk);
                            futures::future::ok::<_, ClientError>(acc)
                        })
                        .and_then(|complete_body| {
                            serde_json::from_slice::<T>(&complete_body)
                                .map_err(|err| ClientError::Json(err))
                        })
                })
        )
    }
}



/// Things that might go wrong when making a request with the client.
#[derive(Debug)]
enum ClientError {
    /// Wrapper for errors in the underlying request, like invalid URI,
    /// request format, or IO issues while executing the request.
    Protocol(hyper::Error),

    /// Indicates that the request reached the server but the server
    /// responded with a non-success (non 2xx) response code.
    NonSuccess(hyper::StatusCode),

    /// Indicates that the response body was received but could not be
    /// parsed as JSON in the expected format.
    Json(serde_json::Error),
}

/// Connection information for Code Dx.
#[derive(Debug)]
struct ClientConfig {
    base_url: Url,
    auth_info: ClientAuthInfo
}

impl ClientConfig {
    fn api_uri(&self, segments: &[&str]) -> Uri {
        let mut url = self.base_url.clone();

        // open a scope to borrow the url mutably
        {
            let mut url_segments = url.path_segments_mut()
                .expect("Can't apply a path to base-url");
            for segment in segments {
                url_segments.push(segment);
            }
        }
        // now that the mutable borrow is done, we can use url again

        Uri::from_str(&url.into_string())
            .expect("Somehow failed to convert a valid URL to a URI")
    }
}

/// Authentication credentials for connecting to Code Dx.
/// Both "basic auth" (username + password) and "api key" are supported.
#[derive(Debug)]
enum ClientAuthInfo {
    Basic { username: String, password: String },
    ApiKey(String),
}

impl ClientAuthInfo {
    fn apply_to<'a>(&self, request: &'a mut Request) {
        match *self {
            ClientAuthInfo::Basic{ ref username, ref password } => {
                let mut headers = request.headers_mut();
                headers.set(Authorization(
                    header::Basic{
                        username: username.to_owned(),
                        password: Some(password.to_owned())
                    }
                ));
            },
            ClientAuthInfo::ApiKey(ref key) => {
                let mut headers = request.headers_mut();
                headers.set_raw("API-Key", key.to_owned());
            },
        }
    }
}

/// Things that can go wrong when parsing a `ClientConfig`
#[derive(Debug)]
enum ConfigError {
    MissingAuth,
    MissingUrl,
    InvalidUrl
}

impl ClientConfig {
    fn from_cli_args() -> Result<ClientConfig, ConfigError> {
        ClientConfig::from(|app| app.get_matches())
    }

    fn from<'a, F>(get_matches: F) -> Result<ClientConfig, ConfigError>
        where F: for<'b> FnOnce(App<'a, 'b>) -> ArgMatches<'a>
    {
        let app = App::new("codedx-client")
            .version("0.1")
            .author("Dylan H <DylanH@codedx.com>")
            .about("CLI client for the Code Dx REST API")
            .arg(Arg::with_name("base-url")
                .short("b")
                .long("base-url")
                .value_name("VALUE")
                .help("Code Dx base url (not including /index)")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("username")
                .short("u")
                .long("username")
                .value_name("VALUE")
                .help("for basic auth, the username")
                .takes_value(true)
            )
            .arg(Arg::with_name("password")
                .short("p")
                .long("password")
                .value_name("VALUE")
                .help("for basic auth, the password")
                .takes_value(true)
            )
            .arg(Arg::with_name("api-key")
                .short("k")
                .long("api-key")
                .value_name("VALUE")
                .help("for key-based auth, the API Key")
                .takes_value(true)
            );
        let matches = get_matches(app);

        let base_uri = match matches.value_of("base-url") {
            None => Err(ConfigError::MissingUrl),
            Some(raw) => Url::parse(raw).map_err(|_| ConfigError::InvalidUrl),
        };

        let client_auth_info = match matches.value_of("api-key") {
            Some(key) => Ok(ClientAuthInfo::ApiKey(String::from(key))),
            None => {
                let username = matches.value_of("username").map(String::from);
                let password = matches.value_of("password").map(String::from);
                let foo = username.and_then(|u| {
                    password.map(|p| {
                        ClientAuthInfo::Basic{ username: u, password: p }
                    })
                });
                foo.ok_or_else(|| ConfigError::MissingAuth)
            },
        };

        base_uri.and_then(|uri| {
            client_auth_info.map(|auth| {
                ClientConfig {
                    base_url: uri,
                    auth_info: auth
                }
            })
        })
    }
}

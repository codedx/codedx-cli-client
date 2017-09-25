extern crate clap;
extern crate hyper;
extern crate url;

use clap::{Arg, ArgMatches, App};
use hyper::{Method, Request, Uri};
use hyper::header::Authorization;
use hyper::header;
use std::str::FromStr;
use url::{Url};

fn main() {
    match ClientConfig::from_cli_args() {
        Ok(config) => {
            println!("arguments are good: {:?}", config);
            make_test_request(&config);
        },
        Err(ConfigError::MissingAuth) => println!("Authorization info missing or incomplete. Either an API Key or a Username + Password must be provided"),
        Err(ConfigError::MissingUrl) => println!("Missing the Base URL"),
        Err(ConfigError::InvalidUrl) => println!("Invalid Base URL"),
    };
}

fn make_test_request(config: &ClientConfig) -> () {
    let uri = config.api_uri(&["x", "projects"]);

    let mut req = Request::new(Method::Get, uri);
    config.auth_info.apply_to(&mut req);
    println!("Request:\n{:?}", req);
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

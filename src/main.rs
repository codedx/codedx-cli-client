extern crate clap;
extern crate futures;
extern crate reqwest;
extern crate rustls;
extern crate serde;
extern crate url;

#[macro_use] extern crate hyper;
#[macro_use] extern crate maplit;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use clap::{Arg, ArgMatches, App};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::option::Option::*;
use std::io::Read;
use std::path::Path;
use url::{Url};

fn main() {
    match ClientConfig::from_cli_args() {
        Ok(config) => {
            let client = ApiClient::new(Box::new(config));

            match client.get_projects() {
                Ok(projects) => {
                    println!("All projects:");
                    for project in projects {
                        println!("  {:?}", project);
                    }
                },
                Err(e) => println!("Error loading all projects: {:?}", e),
            }

            println!();
            let query_result = client.query_projects(&ApiProjectFilter {
                name: Some("scrape"),
                metadata: Some(hashmap!{
                    "Owner" => "dylan"
                })
            });
            match query_result {
                Ok(projects) => {
                    println!("Projects with a name matching 'scrape' and owned by 'dylan':");
                    for project in projects {
                        println!("  {:?}", project);
                    }
                },
                Err(e) => println!("Error querying projects: {:?}", e),
            }

            println!();
            let analysis_job_result = client.start_analysis(4, vec![
                Path::new("D:/CodeDx/data-sets/webgoat eval/bin-webgoat-r437.zip"),
                Path::new("D:/CodeDx/data-sets/webgoat eval/src-webgoat-r437.zip")
            ]);
            match analysis_job_result {
                Ok(response) => println!("Started analysis: {:?}", response),
                Err(e) => println!("Couldn't start analysis: {:?}", e),
            }

            println!();
            println!("Done.");
        },
        Err(ConfigError::MissingAuth) => println!("Authorization info missing or incomplete. Either an API Key or a Username + Password must be provided"),
        Err(ConfigError::MissingUrl) => println!("Missing the Base URL"),
        Err(ConfigError::InvalidUrl) => println!("Invalid Base URL"),
    };
}

header!{ (ApiKey, "API-Key") => [String] }

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

#[derive(Debug, Deserialize)]
struct ApiAnalysisJobResponse {
    #[serde(rename = "analysisId")]
    analysis_id: u32,
    #[serde(rename = "jobId")]
    job_id: String
}

enum ReqBody {
    Form(reqwest::multipart::Form),
    Json(serde_json::Value),
    None,
}

impl From<serde_json::Value> for ReqBody {
    fn from(json: serde_json::Value) -> ReqBody {
        ReqBody::Json(json)
    }
}
impl From<reqwest::multipart::Form> for ReqBody {
    fn from(form: reqwest::multipart::Form) -> ReqBody {
        ReqBody::Form(form)
    }
}

struct ApiClient {
    config: Box<ClientConfig>,
    client: reqwest::Client
}

#[derive(Debug)]
enum ApiError {
    Protocol(reqwest::Error),
    NonSuccess(hyper::StatusCode, ApiErrorMessage),
    IO(std::io::Error),
}
impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> ApiError {
        ApiError::IO(e)
    }
}

#[derive(Debug)]
enum ApiErrorMessage {
    Nice(String),
    Raw(String)
}
impl ApiErrorMessage {
    fn from_body(response: &mut reqwest::Response) -> Result<ApiErrorMessage, ApiError> {
        let mut body = String::new();
        response.read_to_string(&mut body).map_err(ApiError::from).and_then(|size|{
            serde_json::from_str::<ErrorMessageResponse>(&body)
                .map(|err_body| ApiErrorMessage::Nice(err_body.error))
                .or_else(|_| Ok(ApiErrorMessage::Raw(body)))
        })
    }
}

#[derive(Deserialize)]
struct ErrorMessageResponse {
    error: String
}

type ApiResult<T> = Result<T, ApiError>;
struct ApiResponse(ApiResult<reqwest::Response>);

impl ApiResponse {
    fn from(r: ApiResult<reqwest::Response>) -> ApiResponse {
        ApiResponse(r)
    }

    fn get(self) -> ApiResult<reqwest::Response> {
        self.0
    }

    fn expect_success(self) -> ApiResponse {
        ApiResponse(self.0.and_then(move |mut response| {
            if response.status().is_success() {
                Ok(response)
            } else {
                ApiErrorMessage::from_body(&mut response).and_then(|response_msg| {
                    Err(ApiError::NonSuccess(response.status(), response_msg))
                })
            }
        }))
    }

    fn expect_json<T: DeserializeOwned>(self) -> ApiResult<T> {
        self.0.and_then(|mut response| {
            response.json().map_err(ApiError::from)
        })
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> ApiError {
        ApiError::Protocol(err)
    }
}

impl ApiClient {
    fn new(config: Box<ClientConfig>) -> ApiClient {
        let client = reqwest::Client::builder()
            // various client config methods could go here
            .build()
            .unwrap();
        ApiClient { config, client }
    }

    fn get_projects(&self) -> ApiResult<Vec<ApiProject>> {
        self.api_get(&["x", "projects"])
            .expect_success()
            .expect_json()
    }

    fn query_projects<'a>(&self, filter: &'a ApiProjectFilter) -> ApiResult<Vec<ApiProject>> {
        self.api_post(&["x", "projects", "query"], json!({ "filter": filter }))
            .expect_success()
            .expect_json()
    }

    fn start_analysis(&self, project_id: u32, files: Vec<&Path>) -> ApiResult<ApiAnalysisJobResponse> {
        let form= files
            .iter()
            .enumerate()
            .fold(Ok(reqwest::multipart::Form::new()), |maybe_form, (index, file)| {
                maybe_form.and_then(|form| form.file(format!("file{}", index), file))
            })
            .map_err(ApiError::from);

        form.and_then(|form| {
            self.api_post(&["api", "projects", &project_id.to_string(), "analysis"], form)
                .expect_success()
                .expect_json::<ApiAnalysisJobResponse>()
        })
    }

    fn api_get(&self, path_segments: &[&str]) -> ApiResponse {
        self.api_request(hyper::Method::Get, path_segments, ReqBody::None)
    }

    fn api_post<B>(&self, path_segments: &[&str], body: B) -> ApiResponse
        where B: Into<ReqBody>
    {
        self.api_request(hyper::Method::Post, path_segments, body)
    }

    fn api_put<B>(&self, path_segments: &[&str], body: B) -> ApiResponse
        where B: Into<ReqBody>
    {
        self.api_request(hyper::Method::Put, path_segments, body)
    }

    fn api_request<B>(&self, method: hyper::Method, path_segments: &[&str], body: B) -> ApiResponse
        where B: Into<ReqBody>
    {
        let url = self.config.api_url(path_segments);
        let mut request_builder = self.client.request(method, url);
        self.config.auth_info.apply_to(&mut request_builder);
        match body.into() {
            ReqBody::Json(ref json) => {
                request_builder.json(json);
            },
            ReqBody::Form(form) => {
                request_builder.multipart(form);
            }
            ReqBody::None => (),
        };
        ApiResponse::from(request_builder.send().map_err(ApiError::from))
    }
}

/// Connection information for Code Dx.
#[derive(Debug)]
struct ClientConfig {
    base_url: Url,
    auth_info: ClientAuthInfo,
    insecure: bool
}

impl ClientConfig {
    fn api_url(&self, segments: &[&str]) -> Url {
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
        url
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
    fn apply_to(&self, request_builder: &mut reqwest::RequestBuilder) {
        match *self {
            ClientAuthInfo::Basic { ref username, ref password } => {
                let u: String = username.to_owned();
                let p: String = password.to_owned();
                request_builder.basic_auth(u, Some(p));
            },
            ClientAuthInfo::ApiKey(ref key) => {
                request_builder.header(ApiKey(key.to_string()));
            }
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
            )
            .arg(Arg::with_name("insecure")
                .long("insecure")
                .takes_value(false)
                .help("ignore https certificate validation")
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

        let insecure = matches.is_present("insecure");

        base_uri.and_then(|uri| {
            client_auth_info.map(|auth| {
                ClientConfig {
                    base_url: uri,
                    auth_info: auth,
                    insecure
                }
            })
        })
    }
}

extern crate clap;
extern crate reqwest;
extern crate url;

use clap::{Arg, ArgMatches, App};
use reqwest::{RequestBuilder};
use rpassword;
use url::Url;

/// Connection information for Code Dx.
#[derive(Debug)]
pub struct ClientConfig {
    pub base_url: Url,
    pub auth_info: ClientAuth,
    pub insecure: bool,
    pub no_prompt: bool
}

/// declares the `ApiKey` type which implements the Header trait
header!{ (ApiKey, "API-Key") => [String] }

/// Authentication credentials for connecting to Code Dx.
/// Both "basic auth" (username + password) and "api key" are supported.
#[derive(Debug)]
pub enum ClientAuth {
    Basic { username: String, password: String },
    ApiKey(String),
}

impl ClientAuth {
    fn apply_to(&self, request_builder: &mut RequestBuilder) {
        match *self {
            ClientAuth::Basic { ref username, ref password } => {
                let u: String = username.to_owned();
                let p: String = password.to_owned();
                request_builder.basic_auth(u, Some(p));
            },
            ClientAuth::ApiKey(ref key) => {
                request_builder.header(ApiKey(key.to_string()));
            }
        }
    }
}

/// Things that can go wrong when parsing a `ClientConfig`
#[derive(Debug)]
pub enum ConfigError {
    MissingAuth,
    MissingUrl,
    InvalidUrl
}

impl ClientConfig {
    pub fn from_cli_args() -> Result<ClientConfig, ConfigError> {
        ClientConfig::from(|app| app.get_matches())
    }

    pub fn from<'a, F>(get_matches: F) -> Result<ClientConfig, ConfigError>
        where F: for<'b> FnOnce(App<'a, 'b>) -> ArgMatches<'a>
    {
        let app = App::new("codedx-client")
            .version(crate_version!())
            .about("CLI client for the Code Dx REST API")
            .arg(Arg::with_name("base-url")
                .short("b")
                .long("base-url")
                .value_name("BASE URL")
                .help("Code Dx base url (e.g. 'https://localhost/codedx')")
                .takes_value(true)
                .required(true)
                .index(1)
            )
            .arg(Arg::with_name("username")
                .short("u")
                .long("username")
                .value_name("VALUE")
                .help("Username for basic auth")
                .takes_value(true)
            )
            .arg(Arg::with_name("password")
                .short("p")
                .long("password")
                .value_name("VALUE")
                .help("Password for basic auth")
                .takes_value(true)
            )
            .arg(Arg::with_name("api-key")
                .short("k")
                .long("api-key")
                .value_name("VALUE")
                .help("API Key for for key-based auth")
                .takes_value(true)
            )
            .arg(Arg::with_name("insecure")
                .long("insecure")
                .takes_value(false)
                .help("Ignore https certificate hostname validation")
            )
            .arg(Arg::with_name("no-prompt")
                .long("no-prompt")
                .takes_value(false)
                .help("Don't output REPL prompts to STDOUT")
            );
        let matches = get_matches(app);

        // parse the base-url as a URI, then attempt to access the `path_segments_mut` to
        // ensure that will work once we pass the base url to the api client code.
        let base_uri = match matches.value_of("base-url") {
            None => Err(ConfigError::MissingUrl),
            Some(raw) => Url::parse(raw).map_err(|_| ConfigError::InvalidUrl).and_then(|mut url| {
                let url_seems_ok = {
                    let url_segments = url.path_segments_mut();
                    match url_segments {
                        Ok(_) => Ok(()),
                        Err(_) => Err(ConfigError::InvalidUrl),
                    }
                };
                url_seems_ok.map(|_| url)
            }),
        };

        base_uri.and_then(|uri| {

            // interpret the authentication values
            let client_auth_info = match matches.value_of("api-key") {
                Some(key) => Ok(ClientAuth::ApiKey(String::from(key))),
                None => {
                    let username = matches.value_of("username").map(String::from);
                    let password = matches.value_of("password").map(String::from);
                    let foo = username.and_then(|u| {
                        password.or_else(|| {
                            // prompt for the password without actually showing what the user types
                            rpassword::prompt_password_stdout("password: ").ok()
                        }).map(|p| {
                            ClientAuth::Basic{ username: u, password: p }
                        })
                    });
                    foo.ok_or_else(|| ConfigError::MissingAuth)
                },
            };

            let insecure = matches.is_present("insecure");
            let no_prompt = matches.is_present("no-prompt");

            client_auth_info.map(|auth| {
                ClientConfig {
                    base_url: uri,
                    auth_info: auth,
                    insecure,
                    no_prompt,
                }
            })
        })
    }

    pub fn apply_auth(&self, request_builder: &mut RequestBuilder) {
        self.auth_info.apply_to(request_builder);
    }

    pub fn api_url(&self, segments: &[&str]) -> Url {
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

    pub fn allows_insecure(&self) -> bool {
        self.insecure
    }
}
extern crate reqwest;
extern crate rpassword;
extern crate serde;
extern crate url;

#[macro_use] extern crate clap;
#[macro_use] extern crate hyper;
#[macro_use] extern crate nom;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use clap::{Arg, ArgMatches, App, AppSettings, SubCommand};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

mod config;
use config::*;

mod client;
use client::*;

mod repl;
use repl::CmdArgs;

fn main(){
    match ClientConfig::from_cli_args() {
        Ok(config) => {
            let client = ApiClient::new(Box::new(config));
            run_repl(client);
        },
        Err(ConfigError::MissingAuth) => println!("Authorization info missing or incomplete. Either an API Key or a Username + Password must be provided"),
        Err(ConfigError::MissingUrl) => println!("Missing the Base URL"),
        Err(ConfigError::InvalidUrl) => println!("Invalid Base URL. Did you forget 'http://' or 'https://' ?"),
    }
}

/// Main program loop.
///
/// Prompts for a command from stdin, then attempts to interpret it as a `ReplCommand` and execute it.
///
/// Repeats until an EOF or the "exit" command are encountered.
fn run_repl(client: ApiClient) {
    loop {
        // friendly prompt
        if !client.get_config().no_prompt {
            print!("codedx> ");
            io::stdout().flush().unwrap();
        }

        // grab a line from STDIN
        let mut input = String::new();
        let num_read = io::stdin().read_line(&mut input)
            .expect("failed to read line");

        // EOF on stdin might happen if it's piped from a file, or if the user presses Ctrl+Z
        if num_read == 0 {
            break;
        }

        // ignore the trailing \n stuff
        let line = input.trim();

        if line.is_empty() {
            continue;
        } else {
            if let Ok(args) = line.parse::<CmdArgs>() {
                let mut args_vec = args.0;
                args_vec.insert(0, "<placeholder>".to_string());

                let matches = repl_app().get_matches_from_safe(args_vec);

                match matches {
                    Err(ref e) => {
                        match e.kind {
                            clap::ErrorKind::HelpDisplayed => {
                                // repl_app().print_help().unwrap();
                                e.write_to(&mut io::stdout()).unwrap();
                                println!("\n");
                            },
                            clap::ErrorKind::VersionDisplayed => {
                                //repl_app().write_version(&mut io::stdout()).unwrap();
                                println!("\n");
                            }
                            _ => {
                                e.write_to(&mut io::stdout()).unwrap();
                                println!("\n");
                            },
                        }
                    },
                    Ok(arg_matches) => {
                        match ReplCommand::from(&arg_matches) {
                            Ok(ReplCommand::Exit) => break,
                            Ok(ReplCommand::Analyze{ project_id, files, name }) => {

                                // no matter what, start the analysis
                                let mut analysis_response: ApiResult<ApiAnalysisJobResponse> = client
                                    .start_analysis(project_id, files)
                                    .map(|resp| {
                                        println!("# Started analysis {} with job id {}", resp.analysis_id, resp.job_id);
                                        resp
                                    });

                                // if a name was specified, tell the server to set the name
                                if let Some(name) = name {
                                    analysis_response = analysis_response.and_then(|analysis_job_response| {
                                        let analysis_id = analysis_job_response.analysis_id;

                                        client.set_analysis_name(project_id, analysis_id, name)
                                            .map(|_| {
                                                println!("# Set analysis {}'s name to \"{}\"", analysis_id, name);
                                                analysis_job_response
                                            })
                                    });
                                }

                                let analysis_result_status = analysis_response
                                    .and_then(|analysis_job_response| {
                                        let job_id = analysis_job_response.job_id;
                                        client.poll_job_completion(&job_id, Duration::from_secs(2))
                                    });

                                match analysis_result_status {
                                    Err(e) => eprintln!("Error during analysis: {:?}", e),
                                    Ok(status) => {
                                        println!("# Polling done");
                                        println!("{:?}", status);
                                    },
                                }
                            },

                            // PROJECTS
                            Ok(ReplCommand::Projects{ filter }) => {
                                let plist = match filter {
                                    Some(ref filter) => client.query_projects(filter),
                                    None => client.get_projects(),
                                };
                                match plist {
                                    Err(e) => eprintln!("Error loading projects: {:?}", e),
                                    Ok(projects) => {
                                        for project in projects {
                                            println!("{}", serde_json::to_string(&project).unwrap());
                                        }
                                    }
                                };
                            }

                            // ERROR
                            Err(msg) => {
                                println!("{}", msg);
                                println!("try again");
                            }
                        }

                    },
                };
            }
        }

    }

    if !client.get_config().no_prompt {
        println!("bye");
    }
}

/// Enumeration of REPL capabilities.
///
/// Members of this enum represent commands that can be input to the REPL,
/// which will be acted upon as part of the loop.
///
/// The reference lifetime corresponds to the lifetime of the `ArgMatches`
/// reference used to interpret the command.
#[derive(Debug)]
enum ReplCommand<'a> {
    Analyze {
        project_id: u32,
        files: Vec<&'a Path>,
        name: Option<&'a str>
    },
    Projects {
        filter: Option<ApiProjectFilter<'a>>
    },
    Exit,
}
impl <'a> ReplCommand<'a> {
    fn from(matches: &'a ArgMatches) -> Result<ReplCommand<'a>, &'a str> {
        // EXIT
        if let Some(_) = matches.subcommand_matches("exit"){
            Ok(ReplCommand::Exit)

        // ANALYZE
        } else if let Some(analyze_args) = matches.subcommand_matches("analyze") {
            // get the project id
            let project_id: u32 = analyze_args.value_of("project-id")
                .ok_or("project id missing")?
                .parse().map_err(|_| "project-id should be a number")?;
            // get the list of files
            let files = analyze_args.values_of("file")
                .ok_or("must specify at least one file to analyze")?
                .map(|file| Path::new(file))
                .collect();
            // optional name for the analysis
            let name = analyze_args.value_of("name");
            Ok(ReplCommand::Analyze { project_id, files, name })

        // PROJECTS
        } else if let Some(project_args) = matches.subcommand_matches("projects") {
            let mut metadatas = HashMap::new();
            for mut metadata_values in project_args.values_of("metadata") {
                while let Some(k) = metadata_values.next() {
                    let v = metadata_values.next().ok_or("metadata must be given as key value pairs")?;
                    metadatas.insert(k, v);
                }
            }
            let name = project_args.value_of("name");
            if metadatas.is_empty() && name.is_none() {
                Ok(ReplCommand::Projects { filter: None })
            } else {
                let metadatas_opt = if metadatas.is_empty() { None } else { Some(metadatas) };
                Ok(ReplCommand::Projects {
                    filter: Some(ApiProjectFilter{ name, metadata: metadatas_opt })
                })
            }

        // <anything else>
        } else {
            Err("unknown command")
        }
    }
}

/// Get a copy of the "App" for the internal REPL.
///
/// Note that since some of the methods we want to use on it will "consume" it,
/// we need to be able to easily get copies of the whole App for convenience.
fn repl_app() -> App<'static, 'static> {
    App::new("Code Dx API Client")
        .bin_name("")
        .version("2.6.1")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::ColoredHelp)
        .subcommand(SubCommand::with_name("exit")
            .alias("quit")
            .about("Exit this program ('quit' works too)")
        )
        .subcommand(SubCommand::with_name("analyze")
            .about("Analyze some files")
            .arg(Arg::with_name("project-id")
                .short("p")
                .long("project-id")
                .value_name("ID")
                .required(true)
                .takes_value(true)
            )
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("file")
                .short("f")
                .long("file")
                .value_name("FILE")
                .takes_value(true)
                .multiple(true)
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("projects")
            .about("Get a list of projects")
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("PART_OF_NAME")
                .help("Provide criteria by case-insensitive name matching")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("metadata")
                .short("m")
                .long("metadata")
                .number_of_values(2)
                .value_names(&["FIELD", "VALUE"])
                .help("Provide criteria by project metadata")
                .multiple(true)
                .required(false)
            )
        )

}
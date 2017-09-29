extern crate chrono;
extern crate clap;
extern crate reqwest;
extern crate serde;
extern crate url;

#[macro_use] extern crate hyper;
#[macro_use] extern crate maplit;
#[macro_use] extern crate nom;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use chrono::Utc;
use clap::{Arg, App, AppSettings, SubCommand};
use std::io;
use std::path::Path;
use std::time::Duration;

mod config;
use config::*;

mod client;
use client::*;

mod repl;
use repl::CmdArgs;

fn main(){
    let arg1 = ::std::env::args().nth(1);
    match arg1 {
        Some(ref flag) if flag == "--repl" => {
            println!("repl mode!");
            run_repl();
        },
        _ => {
            println!("arg was {:?}", arg1);
            main2();
        },
    }
}

fn run_repl() {
    loop {
        // grab a line from STDIN
        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .expect("failed to read line");

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
                                repl_app().print_help().unwrap();
                                println!();
                            },
                            clap::ErrorKind::VersionDisplayed => {
                                repl_app().write_version(&mut io::stdout()).unwrap();
                                println!();
                            }
                            _ => {
                                e.write_to(&mut io::stdout()).unwrap();
                                println!();
                            },
                        }
                    },
                    Ok(arg_matches) => {
                        if let Some(_) = arg_matches.subcommand_matches("exit") {
                            break;
                        }

                        println!("matches: {:?}", arg_matches);
                    },
                };
            }
        }

    }
    println!("bye");
}

/// Get a copy of the "App" for the internal REPL.
///
/// Note that since some of the methods we want to use on it will "consume" it,
/// we need to be able to easily get copies of the whole App for convenience.
fn repl_app() -> App<'static, 'static> {
    App::new("My Super Program")
        .bin_name("")
        .version("2.6.1")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::ColoredHelp)
        .subcommand(SubCommand::with_name("analyze")
            .about("Analyze some files")
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
        .subcommand(SubCommand::with_name("exit")
            .alias("quit")
            .about("Exit this program ('quit' works too)")
        )
}

fn main2() {
    match ClientConfig::from_cli_args() {
        Ok(config) => {
            let client = ApiClient::new(Box::new(config));

            // Request the project list, and println each of them
            match client.get_projects() {
                Ok(projects) => {
                    println!("All projects:");
                    for project in projects {
                        println!("  {:?}", project);
                    }
                },
                Err(e) => println!("Error loading all projects: {:?}", e),
            }

            // Query projects based on some example filter criteria, and println each of them
            println!();
            let query_result = client.query_projects(&ApiProjectFilter::new("scrape", hashmap!{ "Owner" => "dylan" }));
            match query_result {
                Ok(projects) => {
                    println!("Projects with a name matching 'scrape' and owned by 'dylan':");
                    for project in projects {
                        println!("  {:?}", project);
                    }
                },
                Err(e) => println!("Error querying projects: {:?}", e),
            }

            // Start an analysis (currently hardcoded project id and files), set its name, and poll until it finishes
            println!();
            let project_id = 4;
            let analysis_result = client
                .start_analysis(project_id, vec![
                    Path::new("D:/CodeDx/data-sets/webgoat eval/bin-webgoat-r437.zip"),
                    Path::new("D:/CodeDx/data-sets/webgoat eval/src-webgoat-r437.zip")
                ])
                .and_then(|analysis_job_response| {
                    let analysis_id = analysis_job_response.analysis_id;
                    println!("Started analysis: {}", analysis_id);
                    let name = format!("My CLI Analysis @ {}", format_current_datetime());

                    client.set_analysis_name(project_id, analysis_id, &name)
                        .map(|_| {
                            println!("Set analysis name to {}", name);
                            analysis_job_response
                        })
                })
                .and_then(|analysis_job_response| {
                    let job_id = analysis_job_response.job_id;
                    client.poll_job_completion(&job_id, Duration::from_secs(2))
                });
            match analysis_result {
                Ok(status) => println!("Analysis finished with status {:?}", status),
                Err(e) => println!("Couldn't start analysis: {:?}", e),
            }

            // Bye
            println!();
            println!("Done.");
        },
        Err(ConfigError::MissingAuth) => println!("Authorization info missing or incomplete. Either an API Key or a Username + Password must be provided"),
        Err(ConfigError::MissingUrl) => println!("Missing the Base URL"),
        Err(ConfigError::InvalidUrl) => println!("Invalid Base URL"),
    };
}

fn format_current_datetime() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
extern crate chrono;
extern crate clap;
extern crate reqwest;
extern crate serde;
extern crate url;

#[macro_use] extern crate hyper;
#[macro_use] extern crate maplit;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use chrono::Utc;
use std::path::Path;
use std::time::Duration;

mod config;
use config::*;

mod client;
use client::*;

fn main() {
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

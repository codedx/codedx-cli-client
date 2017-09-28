extern crate clap;
extern crate reqwest;
extern crate serde;
extern crate url;

#[macro_use] extern crate hyper;
#[macro_use] extern crate maplit;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use std::path::Path;

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

            // Start an analysis (currently hardcoded project id and files) and println the server's response
            println!();
            let analysis_job_result = client.start_analysis(107, vec![
                Path::new("D:/CodeDx/data-sets/webgoat eval/bin-webgoat-r437.zip"),
                Path::new("D:/CodeDx/data-sets/webgoat eval/src-webgoat-r437.zip")
            ]);
            match analysis_job_result {
                Ok(response) => println!("Started analysis: {:?}", response),
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



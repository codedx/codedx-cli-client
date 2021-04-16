/*
 * Copyright 2021 Code Dx, Inc
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

extern crate reqwest;
extern crate rpassword;
extern crate serde;
extern crate url;

#[macro_use] extern crate clap;
#[macro_use] extern crate hyper;
#[macro_use] extern crate nom;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

mod client;
mod commands;
mod config;
mod repl;

use clap::{ArgMatches, App, AppSettings};
use std::io;
use std::io::Write;

use config::*;
use client::*;
use repl::CmdArgs;

fn main(){
    let app = {
        let mut base_app = config::get_base_app();
        for command in commands::all() {
            base_app = base_app.subcommand(command.as_subcommand());
        }
        base_app
    };
    let matches = app.get_matches();

    match ClientConfig::from_matches(&matches) {
        Ok(config) => {
            let client = ApiClient::new(Box::new(config));
            if let (_, Some(_)) = matches.subcommand() {
                run_oneoff(client, &matches);
            } else {
                if !client.get_config().no_prompt {
                    println!("Welcome to the Code Dx CLI Client REPL.");
                    println!("In the REPL, you can enter commands without having to provide the Code Dx base url or credentials each time.");
                    println!("If this wasn't what you expected, make sure to include a command when running this program from the command line.");
                    println!("For a list of commands, type 'help'. To exit, type 'exit'");
                    println!();
                }
                run_repl(client);
            }
        },
        Err(ConfigError::MissingAuth) => eprintln!("Authorization info missing or incomplete. Either an API Key or a Username + Password must be provided"),
        Err(ConfigError::MissingUrl) => eprintln!("Missing the Base URL"),
        Err(ConfigError::InvalidUrl) => eprintln!("Invalid Base URL. Did you forget 'http://' or 'https://' ?"),
    }
}

/// Run a single command based on the `arg_matches`.
///
/// This function will be called when the main `App` matches a subcommand.
/// The subcommand will be run, and the program will exit immediately afterward.
fn run_oneoff<'a>(client: ApiClient, arg_matches: &ArgMatches<'a>) -> ! {
    let command_runner = CommandRunner(commands::all());

    let exit_code = match command_runner.maybe_run(&arg_matches, &client) {
        CommandRunnerResult::Done => 0,
        CommandRunnerResult::RequestedExit(code) => code,
        CommandRunnerResult::UnknownCommand => {
            eprintln!("Unknown command.");
            -1
        },
        CommandRunnerResult::InvalidArguments(msg) => {
            eprintln!("Invalid arguments for command: {}", msg);
            -2
        },
    };

    std::process::exit(exit_code);
}

/// Repeatedly prompt for- and execute- commands.
///
/// The loop ends when the "exit" command is run, or when STDIN reaches an EOF.
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
                                e.write_to(&mut io::stderr()).unwrap();
                                eprintln!("\n");
                            },
                            clap::ErrorKind::VersionDisplayed => {
                                eprintln!("\n");
                            }
                            _ => {
                                e.write_to(&mut io::stderr()).unwrap();
                                eprintln!("\n");
                            },
                        }
                    },
                    Ok(arg_matches) => {
                        let command_runner = CommandRunner(commands::all());

                        match command_runner.maybe_run(&arg_matches, &client) {
                            CommandRunnerResult::UnknownCommand => eprintln!("Unknown command; try again."),
                            CommandRunnerResult::InvalidArguments(msg) => eprintln!("Invalid arguments for command: {}\nTry again.", msg),
                            CommandRunnerResult::RequestedExit(code) => std::process::exit(code),
                            CommandRunnerResult::Done => (),
                        }
                    },
                };
            }
        }
    }
}

/// Get a copy of the "App" for the internal REPL.
///
/// Note that since some of the methods we want to use on it will "consume" it,
/// we need to be able to easily get copies of the whole App for convenience.
fn repl_app() -> App<'static, 'static> {
    let mut app = App::new("Code Dx API Client")
        .bin_name("")
        .version("2.6.1")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::ColoredHelp);

    for command in commands::all() {
        app = app.subcommand(command.as_subcommand());
    }
    app
}

/// Wrapper for a collection of `Command`s.
///
/// Its purpose is to run the first command that matches some given `arg_matches`, returning that command's result.
/// It exposes the result as a friendly enum, `CommandRunnerResult`.
struct CommandRunner<'a>(Vec<Box<commands::Command<'a>>>);
impl <'a> CommandRunner<'a> {
    fn maybe_run<'b>(&self, arg_matches: &'a ArgMatches, client: &'b ApiClient) -> CommandRunnerResult<'a> {
        let raw_result = self.0.iter().filter_map(|command_box| {
            let cmd = command_box.as_ref();
            cmd.maybe_run(arg_matches, client)
        }).next();
        raw_result.into()
    }
}

/// Result of attempting to run the first applicable command on some `ArgMatches`.
enum CommandRunnerResult<'a> {
    Done,
    UnknownCommand,
    InvalidArguments(&'a str),
    RequestedExit(i32),
}
impl <'a> From<Option<Result<commands::CommandResult, &'a str>>> for CommandRunnerResult<'a> {
    fn from(result: Option<Result<commands::CommandResult, &'a str>>) -> Self {
        match result {
            Some(Ok(Ok(()))) => CommandRunnerResult::Done,
            Some(Ok(Err(commands::Exit(code)))) => CommandRunnerResult::RequestedExit(code),
            Some(Err(msg)) => CommandRunnerResult::InvalidArguments(msg),
            None => CommandRunnerResult::UnknownCommand,
        }
    }
}
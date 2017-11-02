extern crate reqwest;
extern crate rpassword;
extern crate serde;
extern crate url;

#[macro_use] extern crate clap;
#[macro_use] extern crate hyper;
#[macro_use] extern crate nom;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use clap::{ArgMatches, App, AppSettings};
use std::io;
use std::io::Write;

mod config;
use config::*;

mod commands;

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
                        let command_runner = CommandRunner(commands::all());
                        let stuff = command_runner.maybe_run(&arg_matches, &client);
                        // TODO: interpret the result (and give it a better name)

                        match stuff {
                            None => {
                                println!("Invalid command.");
                                println!("Try again.");
                            },
                            Some(Err(msg)) => {
                                println!("Invalid arguments for command: {}", msg);
                                println!("Try again.");
                            },
                            Some(Ok(Err(commands::Exit(code)))) => {
                                std::process::exit(code);
                            },
                            Some(Ok(Ok(()))) => {
                                // command ran without issue
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

struct CommandRunner<'a>(Vec<Box<commands::Command<'a>>>);
impl <'a> CommandRunner<'a> {
    fn maybe_run<'b>(&self, arg_matches: &'a ArgMatches, client: &'b ApiClient) -> Option<Result<commands::CommandResult, &'a str>> {
        let foo = self.0.iter().filter_map(|command_box| {
            let cmd = command_box.as_ref();
            cmd.maybe_run(arg_matches, client)
        }).next();
        foo
    }
}
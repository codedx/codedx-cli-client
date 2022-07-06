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

use branching::*;
use clap::{ArgMatches, App, Arg, SubCommand};
use client::*;
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// A vector containing all of the runnable commands in this module.
pub fn all<'a>() -> Vec<Box<dyn Command<'a>>> {
    vec![
        Box::new(ExitCommand),
        Box::new(AnalyzeCommand),
        Box::new(ProjectsCommand),
        Box::new(BranchesCommand),
    ]
}

/// Represents the intention to exit the application with a specific exit code.
pub struct Exit(pub i32);

/// The result of running a command; either continue, or exit the application.
pub type CommandResult = Result<(), Exit>;

/// Everything about a command that can be run in this application.
///
/// The `Args` type is an internal representation of the arguments collected from
/// the `clap::App` described by `as_subcommand`. These simply need to be fed
/// back into the `run` method.
///
/// Because boxing a collection of Commands with different `Args` types isn't
/// possible, see the `Command` trait, which wraps this trait by combining
/// the `parse` and `run` methods into `maybe_run`, hiding the `Args` type.
pub trait CommandInner<'a> {

    /// Arguments parsed from the command line, to be passed into `run` at a later point.
    type Args;

    /// Get a description of this command as a `clap::App`.
    fn as_subcommand(&self) -> App<'static, 'static>;

    /// Extract an `Args` instance from the CLI argument matches.
    ///
    /// The `matches` passed to this method are expected to be the raw matches for
    /// the entire App, so implementations should typically start with
    /// `if let Some(inner_matches) = matches.subcommand_matches(name_of_this_command)`.
    ///
    /// If the subcommand was not matched, then the match applies to some other command,
    /// and this method should return `None`. Otherwise, the sub-matches should be
    /// parsed as `Args`, or an error message.
    fn parse(&self, matches: &'a ArgMatches) -> Option<Result<Self::Args, &'a str>>;

    /// Run the command using the given `args` and a reference to an `ApiClient`.
    ///
    /// This should perform any necessary HTTP activity to execute the command,
    /// and return a result signaling the REPL should continue, or exit with a
    /// particular exit code.
    fn run(&self, client: &ApiClient, args: Self::Args) -> CommandResult;
}

/// Wrapper trait for `CommandInner`.
///
/// This trait hides the `Args` type by combining the `parse` and `run` methods
/// into the `maybe_run` method.
pub trait Command<'a> {

    /// Same as Command::as_subcommand
    fn as_subcommand(&self) -> App<'static, 'static>;

    /// Attempt to extract relevent arguments from `matches`, then run the command with those arguments.
    ///
    /// If the arguments were not intended for this command, this method should return `None`.
    /// If the arguments were intended for this command, but were not formatted correctly, this
    /// method should return `Some(Err(explanation))` where `explanation` is a string describing
    /// what was wrong with the arguments.
    /// If the arguments are correctly formed, the command should run, and this method should
    /// return `Some(Ok(command_result))`.
    fn maybe_run<'b>(&self, matches: &'a ArgMatches, client: &'b ApiClient) -> Option<Result<CommandResult, &'a str>>;
}
impl <'a, T, A> Command<'a> for T where T: CommandInner<'a, Args = A> {
    fn as_subcommand(&self) -> App<'static, 'static> {
        CommandInner::as_subcommand(self)
    }

    fn maybe_run<'b>(&self, matches: &'a ArgMatches, client: &'b ApiClient) -> Option<Result<CommandResult, &'a str>> {
        let args_opt = self.parse(matches);
        args_opt.map(|parsed_args| {
            parsed_args.map(|ok_args| {
                self.run(client, ok_args)
            })
        })
    }
}

// -------------------------------------------------------------------------------------------------
// ABOVE THIS POINT: command traits and supporting structs
// -
// BELOW THIS POINT: commands and their implementations
// -------------------------------------------------------------------------------------------------


// -------------------------------------------------------------------------------------------------
// COMMAND: exit
// -------------------------------------------------------------------------------------------------
pub struct ExitCommand;
impl <'a> CommandInner<'a> for ExitCommand {
    type Args = ();

    fn as_subcommand(&self) -> App<'static, 'static> {
        SubCommand::with_name("exit")
            .alias("quit")
            .about("Exit this program ('quit' works too)")
    }

    fn parse(&self, matches: &'a ArgMatches) -> Option<Result<Self::Args, &'a str>> {
        if let Some(_) = matches.subcommand_matches("exit") {
            Some(Ok(()))
        } else {
            None
        }
    }

    fn run(&self, client: &ApiClient, _args: Self::Args) -> CommandResult {
        if !client.get_config().no_prompt {
            println!("goodbye")
        }
        Err(Exit(0))
    }
}


// -------------------------------------------------------------------------------------------------
// COMMAND: analyze
// -------------------------------------------------------------------------------------------------
pub struct AnalyzeCommand;
pub struct AnalyzeCommandArgs<'a> {
    project_context: ProjectContext,
    branch_name: Option<String>,
    include_git_source: bool,
    git_branch_name: Option<String>,
    files: Vec<&'a Path>,
    name: Option<&'a str>
}
impl <'a> AnalyzeCommand {
    // ANALYZE - helper for argument extraction
    fn inner_parse(&self, analyze_args: &'a ArgMatches) -> Result<AnalyzeCommandArgs<'a>, &'a str> {
        let project_context: ProjectContext = {
            let context_arg = analyze_args
                .value_of("project-context")
                .ok_or("project context missing")?;
            ProjectContext::parse(context_arg)?
        };
        // optional branch target name (Code Dx) for the analysis
        let branch_name = analyze_args.value_of("branch-name").map(|name| name.to_string());
        // optional flag for including git source
        let include_git_source = analyze_args.is_present("include-git-source");
        // optional branch target name (Git) for the analysis
        let git_branch_name = analyze_args.value_of("git-branch-name").map(|name| name.to_string());
        // get the list of files
        let files = analyze_args.values_of("file")
            .ok_or("must specify at least one file to analyze")?
            .map(|file| Path::new(file))
            .collect();
        // optional name for the analysis
        let name = analyze_args.value_of("name");
        Ok(AnalyzeCommandArgs { project_context, branch_name, include_git_source, git_branch_name, files, name })
    }
}
impl <'a> CommandInner<'a> for AnalyzeCommand {
    type Args = AnalyzeCommandArgs<'a>;

    // ANALYZE - argument specification
    fn as_subcommand(&self) -> App<'static, 'static> {
        SubCommand::with_name("analyze")
            .about("Analyze some files")
            .arg(Arg::with_name("project-context")
                .index(1)
                .value_name("CONTEXT")
                .takes_value(true)
                .required(true)
                .help("Project context for the analysis. Should be in the form of <project-id>, \
                <project-id>;branchId=<branch-id>, or <project-id>;branch=<branch-name>")
            )
            .arg(Arg::with_name("branch-name")
                .long("branch-name")
                .value_name("BRANCH-NAME")
                .takes_value(true)
                .required(false)
                .help("Code Dx target branch name. If the branch does not exist, a new one will be created \
                off of the given project context.")
            )
            .arg(Arg::with_name("include-git-source")
                .short("g")
                .long("include-git-source")
                .value_name("INCLUDE-GIT-SOURCE")
                .takes_value(false)
                .required(false)
                .help("Flag for including configured git source in the analysis.")
            )
            .arg(Arg::with_name("git-branch-name")
                .long("git-branch-name")
                .value_name("GIT-BRANCH-NAME")
                .takes_value(true)
                .required(false)
                .help("Git target branch name.")
            )
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .takes_value(true)
                .required(false)
                .help("Name of the analysis")
            )
            .arg(Arg::with_name("file")
                .index(2)
                .value_name("FILE(S)")
                .takes_value(true)
                .multiple(true)
                .required(true)
            )
    }

    // ANALYZE - argument extraction
    fn parse(&self, matches: &'a ArgMatches) -> Option<Result<Self::Args, &'a str>> {
        if let Some(analyze_args) = matches.subcommand_matches("analyze") {
            // get the project id
            let args = self.inner_parse(analyze_args);
            Some(args)
        } else {
            None
        }
    }

    // ANALYZE - execution
    fn run(&self, client: &ApiClient, args: AnalyzeCommandArgs<'a>) -> CommandResult {
        let AnalyzeCommandArgs { project_context, branch_name, include_git_source, git_branch_name, files, name } = args;

        let mut analysis_response: ApiResult<ApiAnalysisJobResponse> =
            if include_git_source || git_branch_name.is_some() {
                client.start_analysis_with_git(project_context.clone(), branch_name, include_git_source, git_branch_name, files)
                    .and_then(|resp| {
                        println!("Requesting new analysis with job id {} with included git source", resp.job_id);
                        client.poll_job_completion(&resp.job_id, Duration::from_secs(2))?;
                        client.get_job_result(&resp.job_id).map(|result| {
                            println!("# Started analysis {} with job id {} with included git source", result.analysis_id, resp.job_id);
                            result
                        })
                    })
            } else {
                client
                    .start_analysis(project_context.clone(), branch_name, files)
                    .map(|resp| {
                        println!("# Started analysis {} with job id {}", resp.analysis_id, resp.job_id);
                        resp
                    })
            };

        // if a name was specified, tell the server to set the name
        if let Some(name) = name {
            analysis_response = analysis_response.and_then(|analysis_job_response| {
                let analysis_id = analysis_job_response.analysis_id;

                client.set_analysis_name(project_context.clone(), analysis_id, name)
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
            Err(e) => {
                eprintln!("Error during analysis: {:?}", e);
                Err(Exit(1))
            },
            Ok(status) => {
                println!("# Polling done");
                println!("{:?}", status);
                Ok(())
            },
        }
    }
}

// -------------------------------------------------------------------------------------------------
// COMMAND: projects
// -------------------------------------------------------------------------------------------------
pub struct ProjectsCommand;
pub struct ProjectsCommandArgs<'a> {
    filter: Option<ApiProjectFilter<'a>>
}
impl <'a> ProjectsCommand {
    fn inner_parse(&self, project_args: &'a ArgMatches) -> Result<ProjectsCommandArgs<'a>, &'a str> {
        let mut metadatas = HashMap::new();
        for mut metadata_values in project_args.values_of("metadata") {
            while let Some(k) = metadata_values.next() {
                let v = metadata_values.next().ok_or("metadata must be given as key value pairs")?;
                metadatas.insert(k, v);
            }
        }
        let name = project_args.value_of("name");
        if metadatas.is_empty() && name.is_none() {
            Ok(ProjectsCommandArgs { filter: None })
        } else {
            let metadatas_opt = if metadatas.is_empty() { None } else { Some(metadatas) };
            Ok(ProjectsCommandArgs {
                filter: Some(ApiProjectFilter { name, metadata: metadatas_opt })
            })
        }
    }
}
impl <'a> CommandInner<'a> for ProjectsCommand {
    type Args = ProjectsCommandArgs<'a>;

    fn as_subcommand(&self) -> App<'static, 'static> {
        SubCommand::with_name("projects")
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
    }

    fn parse(&self, matches: &'a ArgMatches) -> Option<Result<Self::Args, &'a str>> {
        if let Some(project_args) = matches.subcommand_matches("projects") {
            Some(self.inner_parse(project_args))
        } else {
            None
        }
    }

    fn run(&self, client: &ApiClient, args: Self::Args) -> CommandResult {
        let ProjectsCommandArgs { filter } = args;

        let plist = match filter {
            Some(ref filter) => client.query_projects(filter),
            None => client.get_projects(),
        };
        match plist {
            Err(e) => {
                eprintln!("Error loading projects: {:?}", e);
                Err(Exit(1))
            },
            Ok(projects) => {
                for project in projects {
                    println!("{}", serde_json::to_string(&project).unwrap());
                }
                Ok(())
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// COMMAND: branches
// -------------------------------------------------------------------------------------------------
pub struct BranchesCommand;
pub struct BranchesCommandArgs<'a> {
    filter: ApiBranchFilter<'a>
}
impl <'a> BranchesCommand {
    fn inner_parse(&self, branch_args: &'a ArgMatches) -> Result<BranchesCommandArgs<'a>, &'a str> {
        let project_id = branch_args
            .value_of("project-id")
            .ok_or("must specify a numerical project-id")?
            .parse::<u32>()
            .map_err(|_| "project id should be a number")?;
        let name = branch_args.value_of("name");
        Ok(BranchesCommandArgs { filter: ApiBranchFilter { project_id, name } })
    }
}
impl <'a> CommandInner<'a> for BranchesCommand {
    type Args = BranchesCommandArgs<'a>;

    fn as_subcommand(&self) -> App<'static, 'static> {
        SubCommand::with_name("branches")
            .about("Get a list of branches for a project")
            .arg(Arg::with_name("project-id")
                .short("p")
                .long("project-id")
                .value_name("PROJECT_ID")
                .help("Provide project scope for branch ID lookup")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("PART_OF_NAME")
                .help("Provide criteria by case-insensitive name matching")
                .takes_value(true)
                .required(false)
            )
    }

    fn parse(&self, matches: &'a ArgMatches) -> Option<Result<Self::Args, &'a str>> {
        if let Some(branches_args) = matches.subcommand_matches("branches") {
            Some(self.inner_parse(branches_args))
        } else {
            None
        }
    }

    fn run(&self, client: &ApiClient, args: Self::Args) -> CommandResult {
        let BranchesCommandArgs { filter } = args;

        let blist = match filter.name {
            None => client.get_branches_for_project(filter.project_id),
            Some(branch_name) => client.query_branches_for_project(filter.project_id, branch_name)
        };

        match blist {
            Err(e) => {
                eprintln!("Error loading branches: {:?}", e);
                Err(Exit(1))
            },
            Ok(branches) => {
                for branch in branches {
                    println!("{}", serde_json::to_string(&branch).unwrap());
                }
                Ok(())
            }
        }
    }
}

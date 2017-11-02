[![Build status](https://ci.appveyor.com/api/projects/status/bvfw8fsuy2tt27tl?svg=true)](https://ci.appveyor.com/project/dylemma/codedx-cli-client)
[![Build status](https://api.travis-ci.org/codedx/codedx-cli-client.svg?branch=master)](https://travis-ci.org/codedx/codedx-cli-client)


This is a utility intended to make it easier to interact with
[Code Dx's REST API](https://codedx.com/Documentation/APIGuide.html) from the command line.

Currently only a couple API actions are supported, but more may come with demand (or with pull requests!)

 - [Usage](#usage)
   - [`analyze`](#command-analyze)
   - [`projects`](#command-projects)

# Usage

This program has two modes of operation; "one-shot" and "[REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop)".

With *one-shot mode*, you enter your Code Dx connection information followed by a command and its arguments, all a a single command in your terminal.
The program will run the command, then exit.

With *REPL mode*, you enter your Code Dx connection information the same way as with one-shot mode, but omit the command.
The program will enter a loop in which it prompts for commands.
You only have to enter the connection information once, up front; each command will not require the information a second time.
*REPL mode* is helpful if you want to explore the CLI's capabilities, or if you want to pipe in several commands from a file.

The required connection information includes the "base url" where you browse Code Dx,
and the information you use to log in (username+password, or an API Key).

```text
$> ./codedx-client https://localhost/codedx -u johndoe -p supersecret
Welcome to the Code Dx CLI Client REPL.
codedx>
```

```text
$> ./codedx-client https://localhost/codedx --api-key 8e218b38-fcdd-453d-8f78-185f7d1d9fa7
Welcome to the Code Dx CLI Client REPL.
codedx>
```

If you skip the password argument, the program will prompt for it afterwards.
This way, you enter your password without it appearing in your terminal.

```text
$> ./codedx-client https://localhost/codedx -u johndoe
password:
Welcome to the Code Dx CLI Client REPL.
codedx>
```

To run a command in *one-shot mode*, enter that command and its arguments as part of the same command you would use to start *REPL mode*.

```text
$> ./codedx-client https://localhost/codedx -u johndoe projects -n "WebGoat"
password:
{"id":5,"name":"WebGoat Java","parentId":null}
{"id":8,"name":"WebGoat.NET","parentId":null}
$>
```

If you wish to run several commands at once, it's easiest to write a shell script or batch file,
but you can also pipe the commands into the program's *REPL mode* from a file,
where each line of the file is interpreted as a command.
If you wish to do this, you may also want to use the `--no-prompt` flag to prevent the program from writing stuff like "codedx>" to `STDOUT`.

```text
$> ./codedx-client https://localhost/codedx --api-key 8e218b38-fcdd-453d-8f78-185f7d1d9fa7 --no-prompt < ./my-commands.txt
```

## About REPL Mode

In *REPL mode*, type `help` (and hit Enter) for a list of commands.
You can exit the program by typing `exit` or `quit`, or with <kbd>Ctrl+C</kbd> or sending an EOF signal.
You can learn more about a command by typing `help <command name>` e.g. `help analyze`.

For any command that takes arguments, each argument should be space-separated.
Arguments that contain spaces should be surrounded with quotes e.g. `'this is one argument'` or `"so is this"`.
Arguments surrounded with quotes treat backslash (`\`) as an escape character.
This means if you have a quoted argument that has a backslash or another quote, it needs a backslash in front of it, e.g. `"a \"quote\" inside"` or `"C:\\my data\\source\\files.zip"`.

Note that you can usually get around needing to escape anything by being clever:
 - For file paths with backslashes, just use forward slashes (`/`) instead, e.g. `"C:/my data/source/files.zip"`.
 - For arguments with quotes in them, surround them with the other type of quote, e.g. `'a "quoted" string'`

If you see an error message like "The filename, directory name, or volume label syntax is incorrect.",
you likely used backslashes (`\`) without escaping them (`\\`) inside a quoted argument.

## Arguments and Options

```text
$> ./codedx-client <BASE URL> [OPTIONS] [<command...>]
```

 - `BASE URL` The "base" URL where you can browser to Code Dx, e.g. `https://localhost/codedx`
 - `-u, --usename <USERNAME>`  Specify the username you want to use (basic auth). 
   With `-u`, you don't actually need the space, i.e. `-u johndoe` is the same as `-ujohndoe`.
 - `-p, --password <PASSWORD>` Specify the password you want to use (basic auth).
   With `-p`, you don't actually need the space, i.e. `-p supersecret` is the same as `-psupersecret`.
   A password is required if you choose to authenticate with basic auth, but you can omit it here
   to make the program prompt for your password later.
 - `-k, --api-key <KEY>` Specify an API Key to use for authentication, instead of username+password.
 - `--insecure` If provided, `https` requests will ignore certificate hostname validation.
   This option *does not* disable the certificate trust chain; your system still needs to trust 
   Code Dx's SSL certificate.
 - `--no-prompt` If provided, the program will avoid writing prompts like `codedx>` to `STDOUT`.
   This option is helpful if you want to parse the output of the application.

# Command: `analyze`

The `analyze` command sends one or more files to one of your Code Dx projects to be analyzed.
This is the most common way of adding findings to Code Dx.

The `analyze` command takes care of several actions for you:

 - Generate a request to upload the analysis input files
 - Get the `jobId` and `analysisId` from the response
 - Set the name of the analysis (optional)
 - Poll until the analysis "job" completes

## Arguments and Options

```text
analyze [OPTIONS] <PROJECT ID> <FILE(S)...>
```

 - `-n, --name <NAME>` Optionally specify a name for the analysis.
 - `<PROJECT ID>` Specify which Code Dx project you want to upload files to, by its ID.
   (Note: you can find a project's ID using the [`projects`](#command-projects) command,
   or finding the number in the URL when you visit that project in a browser)
 - `<FILE(S)>` Specify the path to one or more files that you wish to upload.
   Each file is a separate argument, separated by a space.

## Example

Suppose I want analyze my "WebGoat" project, which happens to have an ID of `5`:

```text
codedx> analyze -n "Hello Analysis" 5 "/path/to/workspace/webgoat-source.zip" "/path/to/workspace/webgoat-classes.zip"
# Started analysis 77 with job id f2f3b8c3-9a2c-4446-9765-e99a6d47e69e
# Set analysis 77's name to "Hello Analysis"
# Polling job completion, iteration 1: status = Running
...omitted for brevity...
# Polling job completion, iteration 13: status = Running
# Polling done
Completed
```

# Command: `projects`

The `projects` command helps you get a list of all Code Dx projects, or search for specific projects.

## Arguments and Options

```text
projects [OPTIONS]
```

 - *if no options* - Prints a list of all Code Dx projects that you have at least read access to.
 - `-n, --name <PART OF NAME>` - If this flag is given, it adds search criteria such that matching
   projects include `<PART OF NAME>` in their name (case insensitive).
 - `-m, --metadata <FIELD> <VALUE>` - If this flag is given, it should be followed a key-value
   pair related to the project's metadata. If given, it adds search criteria such that
   matching projects must have entries for the given metadata fields matching the respective
   given metadata values. To specify another key-value pair, use the `-m` flag again.

## Examples

```text
codedx> projects
{"id":1,"name":"My First Project","parentId":null}
{"id":2,"name":"Another Project","parentId":3}
{"id":3,"name":"Project Group","parentId":null}
{"id":4,"name":"Yet another","parentId":3}
...
```

```text
codedx> projects -n another
{"id":2,"name":"Another Project","parentId":3}
{"id":4,"name":"Yet another","parentId":3}
```

Here I search for projects with the metadata field `Owner` set to "johndoe" and the metadata field `Visibility` set to "high".
```text
codedx> projects -m Owner johndoe -m Visibility high
{"id":4,"name":"Yet another","parentId":3}
```

Note that for project metadata fields with the "Dropdown" type, you have to specify the full name of the dropdown option in order to get a match.
For regular (plain text entry) fields, you can just give part of the value for it to match.
```text
codedx> projects -m Owner jo -m Visibility high
{"id":4,"name":"Yet another","parentId":3}
```
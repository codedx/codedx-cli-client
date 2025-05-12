This is a utility intended to make it easier to interact with
[Code Dx's REST API](https://codedx.com/Documentation/APIGuide.html) from the command line.

Currently only a couple API actions are supported, but more may come with demand.
Please reach out to https://community.synopsys.com/s/ for support and feature requests.

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

Note: When connecting to Code Dx over https, Linux/Unix users will need to specify the path to the certificate trust store:

```sh
# note: this path may vary by machine - make sure you pick the right path for you!
export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
```

```text
$> ./codedx-client https://localhost/codedx -u johndoe -p supersecret
Welcome to the Code Dx CLI Client REPL.
codedx>
```

```text
$> ./codedx-client https://localhost/codedx --api-key api-key:XRLqjOCMbo1LTzBK6geIKW4GaPTAs87DIAtxkpGd
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
$> ./codedx-client https://localhost/codedx --api-key api-key:XRLqjOCMbo1LTzBK6geIKW4GaPTAs87DIAtxkpGd --no-prompt < ./my-commands.txt
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
 - `-g, --include-git-source` Flag for including configured git source in the analysis.
 - `--git-branch-name <GIT BRANCH NAME>` Git target branch name.
 - `--branch-name <BRANCH NAME>` Code Dx target branch name. If a branch of that name does not exist off of the given project context, a new one will
   be created.
 - `<PROJECT CONTEXT>` Specify which Code Dx project or project context you want to upload files to. Project context should be in the form of `<project-id>`, `<project-
   id>;branchId=<branch-id>`, or `<project-id>;branch=<branch-name>` (Note: you can find a project's ID using the [`projects`](#command-projects) command,
   or finding the number in the URL when you visit that project in a browser and branch names/IDs can be found using the [`branches`](#command-projects) command).
 - `<FILE(S)>` Specify the path to one or more files that you wish to upload.
   Each file is a separate argument, separated by a space.

## Examples

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

Now, suppose I want to run an analysis on this same project but not on its default branch. 
I use the `branches` command to view a list of branches and identify the name or ID of the desired branch. 
Let's say the desired branch has an ID of 7 and is named "feature". I then use this to construct the following project context: `5;branchId=7`
if building the context by branch ID, or `5;branch=feature` if building the context by name.

```text
codedx> analyze -n "Hello Analysis" 5;branchId=7 "/path/to/workspace/webgoat-source.zip" "/path/to/workspace/webgoat-classes.zip"
# Started analysis 78 with job id 414a68aa-3b86-4ec2-9118-677f34471a8f
...
```

The `-g` (or `--include-git-source`) flag can be used to include a git-source associated with the project. 
The `--git-branch-name <GIT BRANCH NAME>` option can be used to specify a target Git branch for the analysis. 
In this example, the `include-git-source` flag is set (via `-g`) and the target git branch is "bugfix".
```text
codedx> analyze -n "Hello Analysis" 5;branchId=7 -g --git-branch-name "bugfix"
# Requesting new analysis with job id 2fcc373d-f63f-46c5-8d9a-13eaccf0c70b with included git source
...
# Started analysis 79 with job id fa0bbad5-ec13-4213-a437-41520f1d6b9c
...
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
 - `-m, --metadata <FIELD> <VALUE>` - If this flag is given, it should be followed by a key-value
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

# Commands: `branches`
The `branches` command helps you get a list of all Code Dx branches for a specified project.

## Arguments and Options

```text
branches [OPTIONS]
```

- `-p, --project-id <PROJECT_ID>` - Specifies the project, by ID, to display a list of branches for.
- `-n, --name <PART_OF_NAME>` - Displays only branches with names containing the given name value.

## Examples

```text
codedx> branches -p 1
{"id":1,"name":"main","projectId":1,"isDefault":true}
{"id":2,"name":"another branch","projectId":1,"isDefault":false}
{"id":7,"name":"yet another branch","projectId":1,"isDefault":false}
...
```

```text
codedx> branches -p 1 -n another
{"id":2,"name":"another branch","projectId":1,"isDefault":false}
{"id":7,"name":"yet another branch","projectId":1,"isDefault":false}
```

# Troubleshooting

## Certificate verification errors

At the time or writing this, there are two cases where you might see a certificate verification error when running the CLI:

1. **The CLI doesn't know where to find your certificate trust store.**  
   
   A good litmus test for this case is pointing the CLI at `https://www.google.com` instead of your Code Dx server. 
   If you still get a certificate verification error with that address, the CLI won't trust anyone over HTTPS.
   To solve this, set the following environment variable:
   
   ```sh
   export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
   ```
   
   Note that the path and filename may vary depending on your system. 
   
2. **Your system doesn't trust Code Dx's SSL certificate.**
   Code Dx's installer sets up a self-signed certificate in order to run HTTPS, but it can't know the domain name you'll ultimately use it with.
   
   To get around this, you'll need to [set that certificate as trusted](https://help.ubuntu.com/community/OpenSSL#Importing_a_Certificate_into_the_System-Wide_Certificate_Authority_Database), and use the `--insecure` flag when running the CLI to disable hostname verification.  
   
   Alternatively, you could replace the auto-generated certificate with one of your own which is already trusted.

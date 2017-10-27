[![Build status](https://ci.appveyor.com/api/projects/status/bvfw8fsuy2tt27tl?svg=true)](https://ci.appveyor.com/project/dylemma/codedx-cli-client)
[![Build status](https://api.travis-ci.org/codedx/codedx-cli-client.svg?branch=master)](https://travis-ci.org/codedx/codedx-cli-client)


This is a utility intended to make it easier to interact with 
[Code Dx's REST API](https://codedx.com/Documentation/APIGuide.html) from the command line.

Currently only a couple API actions are supported, but more may come with demand (or with pull requests!)

# Usage

The program runs as a [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop).
Start it by providing the connection information (Code Dx "base url", and username+password or API Key):

```text
$> ./codedx-client -b https://localhost/codedx -u johndoe -p supersecret
codedx>
```

```text
$> ./codedx-client -b https://localhost/codedx --api-key 8e218b38-fcdd-453d-8f78-185f7d1d9fa7
codedx>
```

The program reads input from `STDIN`, so you can pipe the contents of a file to it, to run several commands in sequence.
Each line of the file will be interpreted as a command. 
When using this mode, you may wish to provide the `--no-prompt` flag to prevent the program from writing stuff like "codedx>" to `STDOUT`.

```text
$> ./codedx-client -b https://localhost/codedx --api-key 8e218b38-fcdd-453d-8f78-185f7d1d9fa7 --no-prompt < ./my-commands.txt
```

Once in the REPL, type `help` (and hit Enter) for a list of commands.
You can exit the REPL by typing `exit` or `quit`, or with <kbd>Ctrl+C</kbd> or sending an EOF signal.
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

# Example

You're probably here because you're trying to configure your CI environment to send files to Code Dx for analysis.
For this, you'll want the `analyze` command.
You can find more details about the `analyze` command by entering `help analyze` in the REPL, but here's an example:

The `analyze` command has two required flags: a project ID and at least one file. 
These are given with the `--project-id` (`-p`) and `--file` (`-f`) flags respectively.
Suppose I want analyze a file in my "WebGoat" project, which happens to have an ID of `5`:

```text
codedx> analyze -p 5 -f "/path/to/workspace/webgoat-source.zip"
# Started analysis 77 with job id f2f3b8c3-9a2c-4446-9765-e99a6d47e69e    
# Polling job completion, iteration 1: status = Running                   
...omitted for brevity...              
# Polling job completion, iteration 13: status = Running                  
# Polling done                                                            
Completed                                                                 
codedx>                                                                   
``` 

You can optionally set the name of the analysis with the `--name` flag.
The `analyze` command saves the effort of putting together a complex `curl` request for the initial file upload,
setting up a separate request to set an analysis name,
and setting up a polling loop to wait for the analysis "job" to complete.

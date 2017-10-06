This is a utility intended to make it easier to interact with 
[Code Dx's REST API](https://codedx.com/Documentation/APIGuide.html) from the command line.

Currently only a couple API actions are supported, but more may come with demand (or with pull requests!)

# Usage

The program runs as a [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop).
Start it by providing the connection information (Code Dx "base url", and username+password or API Key):

```text
$> ./codedx-client https://localhost/codedx --api-key 8e218b38-fcdd-453d-8f78-185f7d1d9fa7
codedx>
```

Once in the REPL, type `help` (and hit Enter) for a list of commands.
You can exit the REPL by typing `exit` or `quit`, or with <kbd>Ctrl+C</kbd> or sending an EOF signal.
You can learn more about a command by typing `help <command name>` e.g. `help analyze`.

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

> A note about command arguments inside the REPL:
> 
> Each argument is separated by a space. If the argument itself needs to have a space in it (e.g. for file paths),
> you must surround it with quotes (single `'` or double `"`). Within a quoted argument, the backslash character (`\`) 
> is used as the "escape", e.g. so that if you have another quote or a backslash in the argument (common with windows
> paths), you'll need to escape it e.g. `"C:\\path\\to\\some\\files.zip"` or just use forward slashes
> e.g. `"C:/path/to/some/files.zip"`.
>
> If you see a message like "The filename, directory name, or volume label syntax is incorrect.", you likely used
> backslashes (`\`) without escaping them (`\\`) inside a quoted argument.
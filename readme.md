# prism

The goal of `prism` came is to take output from a process and split it up into buckets based on a regular expression, and then letting you view the contents of each bucket
individually using a nice TUI.

`prism` was created primarily to manage the output from [turborepo](https://turborepo.org/), but it will split output based on a custom regex -- defaulting to one that supports turborepo output -- so it can be used for anything; it can even display its own log output as it is running.

This may all sound a bit vague, so it's probably better explained with some demos:

Running it with a dummy turborepo project
[![asciicast](https://asciinema.org/a/ln4BtwWKPMUqAaDycyswu2lXz.svg)](https://asciinema.org/a/ln4BtwWKPMUqAaDycyswu2lXz)

Running it on its own log output:
[![asciicast](https://asciinema.org/a/X22mGKcchw8BVeyShtwscZrLL.svg)](https://asciinema.org/a/X22mGKcchw8BVeyShtwscZrLL)

# Features

- Custom regular expression
- Color support

# Usage

**NOTE**: This is alpha level software. Use at your own risk.

```shell
$ prism -p <prefix_regex> <command>
```

where `prefix_regex` is a regex with at least two capture groups. The first capture group will be the prefix, and the second the message.

In the TUI, use `j`/`k` to navigate prefixes, and `tab` to cycle between messages, stderr and unparsable messages.

Examples:

Run `yarn dev` with the default regex

```shell
$ prism yarn dev
```

Run `cat file` with a regex that parses lines like `DEBUG This is a message`

```shell
$ prism -p '^([A-Z]*?) (.*)' cat file
```

Run a command with command line flags:

```shell
$ prism "tail -f file"
```

Use it to split its own log output(NB: if you want to try this, the log files grows very quickly due to the "recursive" nature of doing this)

```shell
$ RUST_LOG=debug prism -p '\[.* ([A-Z]+ .*?)\] (.*)' "tail -f log" 2>log
```

## Known issues

- When used with `turborepo`, child processes are not terminated reliably
- The default regular expression is probably not very good

## TODO

- Scrolling messages
- tests
- Configurable scrollback limit
- Show an indicator for when the process has exited

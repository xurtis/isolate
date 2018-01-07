`isolate` is a command line tool for using linux's namespace isolation
facilities.

* [Documentation](https://docs.rs/isolate) ![badge](https://docs.rs/isolate/badge.svg)

# Installation

```bash
cargo install isolate
```

# Basic Usage

```bash
isolate [--config-file <path>] <command>
```

# Configuration File

The format of the configuration file can be seen in the default
[`isolate.toml`](src/isolate.toml).

The configuration file can be specified at the command line using the `-f` or `--config-file`
flag. Alternatively, the following locations are searched in order:

1. `./isolate.toml`
1. `./.isolate.toml`
1. `~/.config/isolate.toml`
1. `~/.isolate.toml`
1. `/etc/isolate.toml`

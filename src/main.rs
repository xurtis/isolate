//! `isolate` is a command line tool that encapsulates the behaviour provided by the
//! [`unshare`](https://docs.rs/unshare) library.
//!
//! `isolate` uses a configuration file to construct what is essentially a lightweight container
//! for the command that it then executes.
//!
//! # Configuration file
//!
//! The configuration file can be specified at the command line using the `-f` or `--config-file`
//! flag. Alternatively, the following locations are searched in order:
//!
//! 1. `./isolate.toml`
//! 1. `./.isolate.toml`
//! 1. `~/.config/isolate.toml`
//! 1. `~/.isolate.toml`
//! 1. `/etc/isolate.toml`
//!
//! # Usage
//!
//! `isolate [--config-file <path>] <command>`

extern crate docopt;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate unshare;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::exit;

use docopt::Docopt;
use toml::de::from_str;

fn main() {
    let args = Arguments::load();

    if args.flag_default_config {
        print!("{}", DEFAULT_CONFIG);
        exit(0);
    }

    args.into_command().exec();
}

const USAGE: &'static str = "
Usage:
    isolate [--config-file <file>] <program> [<args>...]
    isolate [-v | -h | -d]

Options:
    -f <file>, --config-file <file>  Location of configuration file to use.
    -h, --help                       Show this help.
    -v, --version                    Show the version.
    -d, --default-config             Dumpt the default configuration to stdout.
";

#[derive(Deserialize)]
struct Arguments {
    flag_config_file: Option<String>,
    flag_default_config: bool,
    arg_program: String,
    arg_args: Vec<String>
}

impl Arguments {
    /// Load arguments from the command line.
    fn load() -> Arguments {
        Docopt::new(USAGE)
            .unwrap_or_else(|e| e.exit())
            .help(true)
            .version(Some(version()))
            .deserialize()
            .unwrap_or_else(|e| e.exit())
    }

    /// Construct the command to execute.
    fn into_command(self) -> Command {
        let config = self.config();
        Command::new(self.arg_program, self.arg_args, config)
    }

    /// Determine the path to configuration file.
    fn config(&self) -> Configuration {
        let text = if let Some(ref path) = self.find_config_path() {
            let mut file = File::open(path).expect("could not open configuration file");
            let mut text = String::new();
            file.read_to_string(&mut text).expect("could not read configuration file");
            text
        } else {
            DEFAULT_CONFIG.to_string()
        };

        from_str(&text).expect("could not parse configuration")
    }

    /// Determine the path of the configuration file.
    fn find_config_path(&self) -> Option<String> {
        if let Some(ref path) = self.flag_config_file {
            Some(path.clone())
        } else {
            let paths = Arguments::default_config_paths();

            for path in paths {
                if Path::new(&path).exists() {
                    return Some(path)
                }
            }

            None
        }
    }

    /// Default configuration path list.
    fn default_config_paths() -> Vec<String> {
        let mut paths = vec![
            "isolate.toml".to_string(),
            ".isolate.toml".to_string()
        ];

        if let Ok(path) = env::var("HOME") {
            paths.push(format!("{}/.config/isolate.toml", path));
            paths.push(format!("{}/.isolate.toml", path));
        }

        paths.push("/etc/isolate.toml".to_string());

        paths
    }
}

const DEFAULT_CONFIG: &'static str = include_str!("isolate.toml");

#[derive(Deserialize)]
struct Configuration {
}

struct Command {
    program: String,
    arguments: Vec<String>,
    config: Configuration,
}

impl Command {
    /// COnstruct a new command
    fn new(program: String, args: Vec<String>, config: Configuration) -> Command {
        Command {
            program: program,
            arguments: args,
            config: config,
        }
    }

    /// Execute the given command.
    fn exec(&self) {
        unshare::Command::new(&self.program)
            .args(&self.arguments)
            .spawn()
            .expect("unable to spawn process")
            .wait()
            .expect("error in child process");
    }
}

/// Construct the version string for the program.
fn version() -> String {
    format!(
        "{} - {}\n{}\n\n{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_DESCRIPTION"),
    )
}

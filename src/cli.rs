use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "dpilot",
    version,
    about = "Interactive CLI pilot to deploy services and set up software environments",
    long_about = "dpilot is an interactive deployment pilot designed to make spinning up services, databases, and system tooling seamless through beautiful interactive prompts and declarative recipes."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a service recipe (interactive prompt + step execution)
    Run {
        /// Name of the service, embedded recipe, or remote URL
        #[arg(value_name = "SERVICE")]
        service: Option<String>,

        /// Path to a custom local YAML recipe file
        #[arg(short, long, value_name = "PATH")]
        file: Option<PathBuf>,

        /// Print rendered commands without actually running them
        #[arg(long)]
        dry_run: bool,
    },

    /// List all available recipes (built-in, local, and system)
    List {
        /// Filter recipes by category (e.g. service, system)
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Validate syntax and integrity of a recipe YAML file
    Validate {
        /// Path to recipe file to validate
        #[arg(short, long, value_name = "PATH")]
        file: PathBuf,
    },
}

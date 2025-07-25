use clap::{Parser, Subcommand};
use crate::commands::add;

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Add a new task
    Add(add::AddArgs),
    Run {
        /// the task name to run
        name: String,
    },
    /// Print the summary of the configuration to see what it should do
    Summary,
}

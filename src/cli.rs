use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Run {
        /// the task name to run
        name: String,
    },
    /// Print the summary of the configuration to see what it should do
    Summary,
}

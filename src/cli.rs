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
}

pub fn run_task(task: &str) -> anyhow::Result<()> {
    println!("will run task: {}", task);

    Ok(())
}

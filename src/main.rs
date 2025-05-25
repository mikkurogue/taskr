mod cli;
mod config;

use clap::Parser;
use cli::{Cli, Commands, run_task};
use config::{Config, ConfigError};
use std::process;

fn main() {
    let cli = Cli::parse();

    let config_path = match Config::find_config_file() {
        Some(path) => {
            println!("Found config file: {}", path.display());
            path
        }
        None => {
            eprintln!(
                "No config file found.. Looking for: taskr.toml, .tasks.toml, tasks.toml or task_runner.toml"
            );
            process::exit(1);
        }
    };

    let config = match Config::load_from_file(&config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load cofig: {}", e);
            process::exit(1);
        }
    };

    match &cli.command {
        Commands::Run { name } => {
            if let Err(err) = run_task_with_deps(&config, &name) {
                eprintln!("{err}");
                process::exit(1);
            }
        }
    }

    // print_summary(&config);
}

fn run_task_with_deps(config: &Config, task_name: &str) -> anyhow::Result<()> {
    // Check that the task exists
    if !config.has_task(task_name) {
        return Err(anyhow::anyhow!(
            "Task '{}' not found in project configuration",
            task_name
        ));
    }

    let exec_order = config.get_exec_order(task_name)?;

    println!(
        "Executing commands in following order::: {}",
        exec_order.join(" ==> ")
    );

    // For now we simulate the task runner, as I don't trust myself yet
    for task in exec_order {
        let task_config = config.get_task(&task).unwrap();

        println!("Running task '{}':: '{}'", task, task_config.command);

        if let Some(desc) = &task_config.description {
            println!("    {}", desc);
        }

        // TODO: actually run the command here with like a
        // run_command(&task_config.command)?;
    }

    Ok(())
}

fn find_root_tasks(config: &Config, task: &str) -> anyhow::Result<()> {
    let root_tasks = config.get_root_tasks();

    if root_tasks.iter().any(|t| *t == task) {
        run_task(task)?;
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Task '{}' not found or defined in configuration",
            task
        ))
    }
}

fn find_tasks(config: &Config, task: &str) -> anyhow::Result<()> {
    let root_tasks = config.get_dependent_tasks(task);

    if root_tasks.iter().any(|t| *t == task) {
        run_task(task)?;
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Task '{}' not found or defined in configuration",
            task
        ))
    }
}

fn print_summary(config: &Config) {
    let global = config.get_global_config();

    println!("\n=== Task Runner Configuration ===");
    println!(
        "Log Level: {}",
        global.log_level.unwrap_or_else(|| "info".to_string())
    );
    println!("Max Parallel: {}", global.max_parallel.unwrap_or(4));
    println!(
        "Output Dir: {}",
        global
            .output_dir
            .unwrap_or_else(|| ".task-logs".to_string())
    );

    println!("\n=== Tasks ({}) ===", config.tasks.len());
    for (name, task) in &config.tasks {
        println!("  {}", name);
        println!("     Command: {}", task.command);

        if let Some(desc) = &task.description {
            println!("     Description: {}", desc);
        }

        if let Some(deps) = &task.depends_on {
            println!("     Dependencies: {}", deps.join(", "));
        }

        if let Some(parsers) = &task.parsers {
            println!("     Parsers: {}", parsers.join(", "));
        }

        if let Some(watch) = &task.watch_files {
            println!("     Watching: {}", watch.join(", "));
        }

        if task.auto_restart == Some(true) {
            println!("     Auto-restart: enabled");
        }

        if let Some(port) = task.port_check {
            println!("     Port check: {}", port);
        }

        println!();
    }

    if let Some(parsers) = &config.parsers {
        println!("=== Parsers ({}) ===", parsers.len());
        for (name, parser) in parsers {
            println!("  {}", name);
            println!("     Patterns: {}", parser.patterns.len());
            for pattern in &parser.patterns {
                println!("       - {} ({})", pattern.regex, pattern.level);
            }
            println!();
        }
    }

    // Show task dependency tree
    let root_tasks = config.get_root_tasks();
    if !root_tasks.is_empty() {
        println!("=== Root Tasks (no dependencies) ===");
        for task in root_tasks {
            println!("  {}", task);
        }
        println!();
    }
}

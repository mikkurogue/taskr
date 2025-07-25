use clap::Args;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::collections::HashMap;
use std::fs;
use anyhow::Result;
use toml;

use crate::config::{Config, Parser, Task};

#[derive(Args, Debug)]
pub struct AddArgs {}

pub fn add_task() -> Result<(), anyhow::Error> {
    let tasks = get_tasks_from_config()?;
    let task_name = Input::<String>::new()
        .with_prompt("Task name")
        .interact_text()?;

    let command = Input::<String>::new()
        .with_prompt("Command")
        .interact_text()?;

    let description: String = Input::<String>::new()
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact_text()?;

    let depends_on_selection = MultiSelect::new()
        .with_prompt("Select tasks this task depends on (optional)")
        .items(&tasks)
        .interact()?;

    let depends_on: Option<Vec<String>> = if depends_on_selection.is_empty() {
        None
    } else {
        Some(
            depends_on_selection
                .into_iter()
                .map(|i| tasks[i].clone())
                .collect(),
        )
    };

    let watch_files_input: String = Input::<String>::new()
        .with_prompt("Watch files (optional, comma-separated)")
        .allow_empty(true)
        .interact_text()?;

    let watch_files: Vec<String> = watch_files_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let auto_restart = Confirm::new()
        .with_prompt("Auto-restart on file change?")
        .default(false)
        .interact()?;

    let working_dir: String = Input::<String>::new()
        .with_prompt("Working directory (optional)")
        .allow_empty(true)
        .interact_text()?;

    let port_check_input: String = Input::<String>::new()
        .with_prompt("Port to check (optional)")
        .allow_empty(true)
        .interact_text()?;

    let port_check = if port_check_input.is_empty() {
        None
    } else {
        port_check_input.parse::<u16>().ok()
    };

    let configured_parsers = configure_parsers()?;

    let mut config = read_config()?;
    let new_task = Task {
        command,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        depends_on,
        watch_files: if watch_files.is_empty() {
            None
        } else {
            Some(watch_files)
        },
        auto_restart: if auto_restart { Some(true) } else { None },
        working_dir: if working_dir.is_empty() {
            None
        } else {
            Some(working_dir)
        },
        port_check,
        env: None, // Added missing field
        parsers: if configured_parsers.is_empty() {
            None
        } else {
            Some(configured_parsers.keys().cloned().collect())
        },
    };

    config.tasks.insert(task_name, new_task);

    // Correctly handle Option<HashMap> for parsers
    let parsers_map = config.parsers.get_or_insert_with(HashMap::new);
    for (name, parser) in configured_parsers {
        parsers_map.insert(name, parser);
    }

    write_config(&config)?;

    println!("Task added successfully!");

    Ok(())
}

fn get_tasks_from_config() -> Result<Vec<String>, anyhow::Error> {
    let config = read_config()?;
    Ok(config.tasks.keys().cloned().collect())
}

fn read_config() -> Result<Config, anyhow::Error> {
    let config_str = fs::read_to_string("taskr.toml").unwrap_or_default();
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

fn write_config(config: &Config) -> Result<(), anyhow::Error> {
    let config_str = toml::to_string_pretty(config)?;
    fs::write("taskr.toml", config_str)?;
    Ok(())
}

fn configure_parsers() -> Result<HashMap<String, Parser>, anyhow::Error> {
    let mut parsers = HashMap::new();
    if Confirm::new()
        .with_prompt("Do you want to add a parser for this task's output?")
        .interact()? // Corrected missing block
    {
        let parser_options = vec!["pre-defined", "custom"];
        let selection = Select::new()
            .with_prompt("Choose a parser type")
            .items(&parser_options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let predefined_parsers = vec!["yarn-install", "nextjs", "webpack-dev", "nx-serve", "typescript", "jest"];
                let selections = MultiSelect::new()
                    .with_prompt("Select pre-defined parsers")
                    .items(&predefined_parsers)
                    .interact()?;

                for selection in selections {
                    let parser_name = predefined_parsers[selection].to_string();
                    let parser = get_predefined_parser(&parser_name);
                    parsers.insert(parser_name, parser);
                }
            }
            1 => {
                let name = Input::<String>::new()
                    .with_prompt("Custom parser name")
                    .interact_text()?;
                let pattern = Input::<String>::new()
                    .with_prompt("Regex pattern")
                    .interact_text()?;
                let level = Input::<String>::new()
                    .with_prompt("Log level (e.g., info, warn, error, success)")
                    .interact_text()?;
                let extract = Input::<String>::new()
                    .with_prompt("Extract value (optional)")
                    .interact_text()?;

                let mut custom_parser = Parser {
                    patterns: Vec::new(),
                };
                custom_parser.patterns.push(crate::config::Pattern {
                    regex: pattern,
                    level,
                    extract: if extract.is_empty() { None } else { Some(extract) },
                    action: None, // Added missing field
                });
                parsers.insert(name, custom_parser);
            }
            _ => {}
        }
    }
    Ok(parsers)
}

fn get_predefined_parser(name: &str) -> Parser {
    let mut parser = Parser {
        patterns: Vec::new(),
    };
    match name {
        "yarn-install" => {
            parser.patterns.push(crate::config::Pattern {
                regex: r"warning (.+)".to_string(),
                level: "warn".to_string(),
                extract: Some("message".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"error (.+)".to_string(),
                level: "error".to_string(),
                extract: Some("message".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"✨  Done in (.+)s".to_string(),
                level: "success".to_string(),
                extract: Some("duration".to_string()),
                action: None,
            });
        }
        "nextjs" => {
            parser.patterns.push(crate::config::Pattern {
                regex: r"ready - started server on.+".to_string(),
                level: "success".to_string(),
                extract: None,
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"event - compiled (.+)".to_string(),
                level: "info".to_string(),
                extract: Some("status".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"wait  - compiling".to_string(),
                level: "info".to_string(),
                extract: None,
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"Error: (.+)".to_string(),
                level: "error".to_string(),
                extract: Some("message".to_string()),
                action: None,
            });
        }
        "webpack-dev" => {
            parser.patterns.push(crate::config::Pattern {
                regex: r"webpack compiled with (\d+) warning".to_string(),
                level: "warn".to_string(),
                extract: Some("count".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"webpack compiled successfully".to_string(),
                level: "success".to_string(),
                extract: None,
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"Module not found: (.+)".to_string(),
                level: "error".to_string(),
                extract: Some("message".to_string()),
                action: None,
            });
        }
        "nx-serve" => {
            parser.patterns.push(crate::config::Pattern {
                regex: r"Web Development Server is listening".to_string(),
                level: "success".to_string(),
                extract: None,
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"Application bundle generation complete".to_string(),
                level: "info".to_string(),
                extract: None,
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"ERROR in (.+)".to_string(),
                level: "error".to_string(),
                extract: Some("message".to_string()),
                action: None,
            });
        }
        "typescript" => {
            parser.patterns.push(crate::config::Pattern {
                regex: r"Found (\d+) error".to_string(),
                level: "error".to_string(),
                extract: Some("count".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"Compiled successfully".to_string(),
                level: "success".to_string(),
                extract: None,
                action: None,
            });
        }
        "jest" => {
            parser.patterns.push(crate::config::Pattern {
                regex: r"PASS (.+)".to_string(),
                level: "success".to_string(),
                extract: Some("file".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"FAIL (.+)".to_string(),
                level: "error".to_string(),
                extract: Some("file".to_string()),
                action: None,
            });
            parser.patterns.push(crate::config::Pattern {
                regex: r"Tests:\s+(\d+) passed".to_string(),
                level: "info".to_string(),
                extract: Some("passed".to_string()),
                action: None,
            });
        }
        _ => {}
    }
    parser
}
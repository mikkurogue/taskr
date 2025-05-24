use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub global: Option<GlobalConfig>,
    pub tasks: HashMap<String, Task>,
    pub parsers: Option<HashMap<String, Parser>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalConfig {
    pub log_level: Option<String>,
    pub max_parallel: Option<u32>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Task {
    pub command: String,
    pub description: Option<String>,
    pub parsers: Option<Vec<String>>,
    pub watch_files: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub auto_restart: Option<bool>,
    pub port_check: Option<u16>,
    pub env: Option<HashMap<String, String>>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Parser {
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pattern {
    pub regex: String,
    pub level: String,
    pub extract: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {0}: {1}")]
    FileRead(std::path::PathBuf, std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Task '{task}' references unknown dependency '{dependency}'")]
    InvalidDependency { task: String, dependency: String },

    #[error("Task '{task}' references unknown parser '{parser}'")]
    InvalidParser { task: String, parser: String },

    #[error("Circular dependency detected involving task '{0}'")]
    CircularDependency(String),
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self, ConfigError> {
        let content = fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.as_ref().to_path_buf(), e))?;

        Self::load_from_string(&content)
    }

    pub fn load_from_string(content: &str) -> anyhow::Result<Self, ConfigError> {
        let config: Config = toml::from_str(content).map_err(ConfigError::ParseError)?;

        config.validate()?;

        Ok(config)
    }

    /// validate that the configuration is valid
    fn validate(&self) -> anyhow::Result<(), ConfigError> {
        // check that deps exist
        for (task_name, task) in &self.tasks {
            if let Some(deps) = &task.depends_on {
                for dep in deps {
                    if !self.tasks.contains_key(dep) {
                        return Err(ConfigError::InvalidDependency {
                            task: task_name.clone(),
                            dependency: dep.clone(),
                        });
                    }
                }
            }

            // check that parsers exist
            if let Some(parsers) = &task.parsers {
                for parser in parsers {
                    if let Some(parser_configs) = &self.parsers {
                        if !parser_configs.contains_key(parser) {
                            return Err(ConfigError::InvalidParser {
                                task: task_name.clone(),
                                parser: parser.clone(),
                            });
                        }
                    } else {
                        return Err(ConfigError::InvalidParser {
                            task: task_name.clone(),
                            parser: parser.clone(),
                        });
                    }
                }
            }

            // TODO: check for circular deps
        }

        Ok(())
    }

    /// find config file in current dir or in parents
    pub fn find_config_file() -> Option<PathBuf> {
        let valid_names = [
            "taskr.toml",
            ".taskr.toml",
            "tasks.toml",
            "task_runner.toml",
        ];

        let mut current_dir = current_dir().ok()?;

        loop {
            for name in &valid_names {
                let config_path = current_dir.join(name);
                if config_path.exists() {
                    return Some(config_path);
                }
            }

            if !current_dir.pop() {
                break;
            }
        }

        None
    }

    /// get the global configuration, if none is found then default to binary configuration
    pub fn get_global_config(&self) -> GlobalConfig {
        self.global.clone().unwrap_or_else(|| GlobalConfig {
            log_level: Some("info".to_string()),
            max_parallel: Some(4),
            output_dir: Some(".task-logs".to_string()),
        })
    }

    /// get all the tasks that dont depend on others, and run these first as these could be what
    /// others depend on
    pub fn get_root_tasks(&self) -> Vec<&String> {
        self.tasks
            .iter()
            .filter(|(_, task)| {
                task.depends_on.is_none() || task.depends_on.as_ref().unwrap().is_empty()
            })
            .map(|(name, _)| name)
            .collect()
    }

    pub fn get_dependent_tasks(&self, task_name: &str) -> Vec<&String> {
        self.tasks
            .iter()
            .filter(|(_, task)| {
                task.depends_on
                    .as_ref()
                    .map_or(false, |deps| deps.contains(&task_name.to_string()))
            })
            .map(|(name, _)| name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let toml_content = r#"
[global]
log_level = "debug"
max_parallel = 2

[tasks.install]
command = "yarn install"
description = "Install dependencies"

[tasks.dev]
command = "yarn dev"
depends_on = ["install"]
auto_restart = true

[parsers.yarn-install]
patterns = [
    { regex = "warning (.+)", level = "warn" },
    { regex = "error (.+)", level = "error" }
]
        "#;

        let config = Config::load_from_string(toml_content).unwrap();

        assert_eq!(
            config.global.as_ref().unwrap().log_level.as_ref().unwrap(),
            "debug"
        );
        assert_eq!(config.tasks.len(), 2);
        assert!(config.tasks.contains_key("install"));
        assert!(config.tasks.contains_key("dev"));

        let dev_task = &config.tasks["dev"];
        assert_eq!(dev_task.command, "yarn dev");
        assert_eq!(dev_task.depends_on.as_ref().unwrap(), &vec!["install"]);
    }

    #[test]
    fn test_invalid_dependency() {
        let toml_content = r#"
[tasks.dev]
command = "yarn dev"
depends_on = ["nonexistent"]
        "#;

        let result = Config::load_from_string(toml_content);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidDependency { .. }
        ));
    }

    #[test]
    fn test_get_root_tasks() {
        let toml_content = r#"
[tasks.install]
command = "yarn install"

[tasks.dev]
command = "yarn dev"
depends_on = ["install"]

[tasks.test]
command = "yarn test"
        "#;

        let config = Config::load_from_string(toml_content).unwrap();
        let mut root_tasks = config.get_root_tasks();
        root_tasks.sort();

        assert_eq!(root_tasks, vec!["install", "test"]);
    }
}

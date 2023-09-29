use serde::{Serialize, Deserialize};

use crate::errors::TranError;

const CYAN: &str = "#6EE2FF";

#[derive(Serialize, Deserialize, Clone)]
pub struct IncompleteConfig {
    pub target_files: Option<Vec<String>>,
    pub current_color: Option<String>,
    pub colors: Option<Vec<String>>,
}

impl Default for IncompleteConfig {
    fn default() -> Self {
        IncompleteConfig {
            target_files: Some(Vec::new()),
            colors: Some(vec![CYAN.to_string()]),
            current_color: Some(CYAN.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub target_files: Vec<String>,
    pub current_color: String,
    pub colors: Vec<String>,
}

impl TryFrom<IncompleteConfig> for Config {
    type Error = TranError;

    fn try_from(value: IncompleteConfig) -> Result<Self, Self::Error> {
        Ok(Config {
            target_files: match value.target_files {
                Some(v) => v,
                None => Vec::new(),
            },
            colors: match value.colors {
                Some(v) => v,
                None => Vec::new(),
            },
            current_color: match value.current_color {
                Some(v) => v,
                None => return Err(TranError::ConfigError("Current color not set".to_string())),
            },
        })
    }
}

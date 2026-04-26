use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{CompletionMode, Difficulty, LearningMode, PlayMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_learning_mode: LearningMode,
    pub default_play_mode: PlayMode,
    pub default_difficulty: Difficulty,
    pub completion: CompletionMode,
    pub show_examples: bool,
    pub show_synonyms: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_learning_mode: LearningMode::Linux,
            default_play_mode: PlayMode::Quiz,
            default_difficulty: Difficulty::Easy,
            completion: CompletionMode::On,
            show_examples: true,
            show_synonyms: true,
        }
    }
}

impl AppConfig {
    pub fn load_or_default(path: Option<PathBuf>) -> Result<(Self, Option<PathBuf>)> {
        let resolved = match path {
            Some(path) => Some(path),
            None => default_config_path(),
        };

        let Some(path) = resolved else {
            return Ok((Self::default(), None));
        };

        if !path.exists() {
            return Ok((Self::default(), Some(path)));
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config = toml::from_str(&raw)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;

        Ok((config, Some(path)))
    }
}

fn default_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("cmd-mock-cli");
    path.push("config.toml");
    Some(path)
}

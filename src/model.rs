use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LearningMode {
    Linux,
    Macos,
    Docker,
}

impl LearningMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::Docker => "docker",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayMode {
    Quiz,
    Challenge,
}

impl PlayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quiz => "quiz",
            Self::Challenge => "challenge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionMode {
    On,
    Off,
}

impl CompletionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressStats {
    pub answered: u32,
    pub correct: u32,
    pub mistakes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub learning_mode: LearningMode,
    pub play_mode: PlayMode,
    pub difficulty: Difficulty,
    pub completion: CompletionMode,
    pub quiz_index: usize,
    pub challenge_index: usize,
    pub current_prompt: Option<String>,
    pub command_history: Vec<String>,
    pub stats: ProgressStats,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub learning_mode: LearningMode,
    pub play_mode: PlayMode,
    pub difficulty: Difficulty,
    pub completion: CompletionMode,
    pub quiz_index: usize,
    pub challenge_index: usize,
    pub current_prompt: Option<String>,
    pub command_history: Vec<String>,
    pub stats: ProgressStats,
}

impl From<SessionState> for SessionSnapshot {
    fn from(value: SessionState) -> Self {
        Self {
            learning_mode: value.learning_mode,
            play_mode: value.play_mode,
            difficulty: value.difficulty,
            completion: value.completion,
            quiz_index: value.quiz_index,
            challenge_index: value.challenge_index,
            current_prompt: value.current_prompt,
            command_history: value.command_history,
            stats: value.stats,
        }
    }
}

impl From<SessionSnapshot> for SessionState {
    fn from(value: SessionSnapshot) -> Self {
        Self {
            learning_mode: value.learning_mode,
            play_mode: value.play_mode,
            difficulty: value.difficulty,
            completion: value.completion,
            quiz_index: value.quiz_index,
            challenge_index: value.challenge_index,
            current_prompt: value.current_prompt,
            command_history: value.command_history,
            stats: value.stats,
        }
    }
}

use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{Difficulty, LearningMode, PlayMode};

#[derive(Debug, Clone, Deserialize)]
pub struct Question {
    pub prompt: String,
    pub required_tokens: Vec<String>,
    pub synonyms: Vec<String>,
    pub explanation: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Challenge {
    pub prompt: String,
    pub required_steps: Vec<Vec<String>>,
    pub forbidden_tokens: Vec<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct QuestionSet {
    questions: Vec<QuestionSeed>,
    challenges: Vec<ChallengeSeed>,
}

#[derive(Debug, Clone, Deserialize)]
struct QuestionSeed {
    prompt: String,
    required_tokens: Vec<String>,
    synonyms: Vec<String>,
    explanation: String,
    hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChallengeSeed {
    prompt: String,
    required_steps: Vec<Vec<String>>,
    forbidden_tokens: Vec<String>,
    hint: Option<String>,
}

pub fn question_for(mode: LearningMode, difficulty: Difficulty, index: usize) -> Question {
    let set = question_set(mode);
    let seed = &set.questions[index % set.questions.len()];
    Question {
        prompt: seed.prompt.clone(),
        required_tokens: seed.required_tokens.clone(),
        synonyms: seed.synonyms.clone(),
        explanation: seed.explanation.clone(),
        hint: hint_for(difficulty, seed.hint.clone()),
    }
}

pub fn challenge_for(mode: LearningMode, difficulty: Difficulty, index: usize) -> Challenge {
    let set = question_set(mode);
    let seed = &set.challenges[index % set.challenges.len()];
    Challenge {
        prompt: seed.prompt.clone(),
        required_steps: seed.required_steps.clone(),
        forbidden_tokens: seed.forbidden_tokens.clone(),
        hint: hint_for(difficulty, seed.hint.clone()),
    }
}

pub fn opening_prompt(
    play_mode: PlayMode,
    mode: LearningMode,
    difficulty: Difficulty,
    quiz_index: usize,
    challenge_index: usize,
) -> String {
    match play_mode {
        PlayMode::Quiz => {
            let question = question_for(mode, difficulty, quiz_index);
            render_prompt(&question.prompt, question.hint.as_deref())
        }
        PlayMode::Challenge => {
            let challenge = challenge_for(mode, difficulty, challenge_index);
            render_prompt(&challenge.prompt, challenge.hint.as_deref())
        }
    }
}

pub fn validate_quiz_answer(question: &Question, line: &str) -> bool {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    ordered_token_match(&question.required_tokens, &tokens)
}

pub fn validate_challenge(challenge: &Challenge, history: &[String]) -> bool {
    if history.iter().any(|line| {
        challenge
            .forbidden_tokens
            .iter()
            .any(|bad| line.contains(bad))
    }) {
        return false;
    }

    let tokenized: Vec<Vec<&str>> = history
        .iter()
        .map(|line| line.split_whitespace().collect())
        .collect();

    let mut current_index = 0usize;
    for step in &challenge.required_steps {
        let mut found = false;
        while current_index < tokenized.len() {
            if ordered_token_match(step, &tokenized[current_index]) {
                found = true;
                current_index += 1;
                break;
            }
            current_index += 1;
        }
        if !found {
            return false;
        }
    }

    true
}

fn question_set(mode: LearningMode) -> &'static QuestionSet {
    match mode {
        LearningMode::Linux | LearningMode::Macos => shell_question_set(),
        LearningMode::Docker => docker_question_set(),
    }
}

fn shell_question_set() -> &'static QuestionSet {
    static SET: OnceLock<QuestionSet> = OnceLock::new();
    SET.get_or_init(|| load_question_set(include_str!("../data/questions/shell.json"), "shell"))
}

fn docker_question_set() -> &'static QuestionSet {
    static SET: OnceLock<QuestionSet> = OnceLock::new();
    SET.get_or_init(|| load_question_set(include_str!("../data/questions/docker.json"), "docker"))
}

fn load_question_set(raw: &str, label: &str) -> QuestionSet {
    parse_question_set(raw)
        .unwrap_or_else(|err| panic!("failed to load {} question set: {}", label, err))
}

fn parse_question_set(raw: &str) -> Result<QuestionSet> {
    let parsed: QuestionSet =
        serde_json::from_str(raw).context("failed to parse question set json")?;
    if parsed.questions.is_empty() {
        anyhow::bail!("question set must include at least one question");
    }
    if parsed.challenges.is_empty() {
        anyhow::bail!("question set must include at least one challenge");
    }
    Ok(parsed)
}

fn ordered_token_match(required: &[String], actual: &[&str]) -> bool {
    let mut cursor = 0usize;
    for token in required {
        let mut matched = false;
        while cursor < actual.len() {
            if actual[cursor] == token {
                matched = true;
                cursor += 1;
                break;
            }
            cursor += 1;
        }
        if !matched {
            return false;
        }
    }
    true
}

fn render_prompt(prompt: &str, hint: Option<&str>) -> String {
    match hint {
        Some(hint) => format!("{}\n{}", prompt, hint),
        None => prompt.to_string(),
    }
}

fn hint_for(difficulty: Difficulty, hint: Option<String>) -> Option<String> {
    match difficulty {
        Difficulty::Easy => hint,
        Difficulty::Normal => {
            Some("ヒントは最小限です。コマンドの骨格を思い出してください。".to_string())
        }
        Difficulty::Hard => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Question, parse_question_set, question_for, validate_quiz_answer};
    use crate::model::{Difficulty, LearningMode};

    #[test]
    fn ordered_match_rejects_wrong_order() {
        let question = Question {
            prompt: String::new(),
            required_tokens: vec!["docker".to_string(), "run".to_string(), "nginx".to_string()],
            synonyms: vec![],
            explanation: String::new(),
            hint: None,
        };
        assert!(validate_quiz_answer(&question, "docker run nginx"));
        assert!(!validate_quiz_answer(&question, "nginx docker run"));
    }

    #[test]
    fn question_bank_rotates() {
        let first = question_for(LearningMode::Docker, Difficulty::Easy, 0);
        let fourth = question_for(LearningMode::Docker, Difficulty::Easy, 3);
        assert_eq!(first.prompt, fourth.prompt);
    }

    #[test]
    fn question_set_requires_content() {
        let invalid = r#"{"questions":[],"challenges":[]}"#;
        assert!(parse_question_set(invalid).is_err());
    }
}

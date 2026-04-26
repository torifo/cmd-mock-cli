use crate::model::{Difficulty, LearningMode, PlayMode};

#[derive(Debug, Clone)]
pub struct Question {
    pub prompt: String,
    pub required_tokens: Vec<String>,
    pub synonyms: Vec<String>,
    pub explanation: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Challenge {
    pub prompt: String,
    pub required_steps: Vec<Vec<String>>,
    pub forbidden_tokens: Vec<String>,
    pub hint: Option<String>,
}

pub fn question_for(mode: LearningMode, difficulty: Difficulty, index: usize) -> Question {
    let bank = question_bank(mode, difficulty);
    bank[index % bank.len()].clone()
}

pub fn challenge_for(mode: LearningMode, difficulty: Difficulty, index: usize) -> Challenge {
    let bank = challenge_bank(mode, difficulty);
    bank[index % bank.len()].clone()
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

fn question_bank(mode: LearningMode, difficulty: Difficulty) -> Vec<Question> {
    match mode {
        LearningMode::Linux | LearningMode::Macos => vec![
            Question {
                prompt: "現在のディレクトリ配下から `app.log` を探すコマンドを打ってください。"
                    .to_string(),
                required_tokens: vec!["find".to_string(), ".".to_string(), "app.log".to_string()],
                synonyms: vec!["find . -name app.log".to_string()],
                explanation: "`find` は条件に合うパスを再帰的に探します。".to_string(),
                hint: hint_for(
                    difficulty,
                    "ヒント: `find <開始位置> -name <名前>` の形を思い出してください。",
                ),
            },
            Question {
                prompt: "`readme.txt` の内容を表示するコマンドを打ってください。".to_string(),
                required_tokens: vec!["cat".to_string(), "readme.txt".to_string()],
                synonyms: vec!["cat ./readme.txt".to_string()],
                explanation: "`cat` はファイル内容の表示に使います。".to_string(),
                hint: hint_for(
                    difficulty,
                    "ヒント: 内容をそのまま標準出力へ出す基本コマンドです。",
                ),
            },
            Question {
                prompt: "`backup` ディレクトリを作るコマンドを打ってください。".to_string(),
                required_tokens: vec!["mkdir".to_string(), "backup".to_string()],
                synonyms: vec!["mkdir ./backup".to_string()],
                explanation: "`mkdir` はディレクトリを新規作成します。".to_string(),
                hint: hint_for(
                    difficulty,
                    "ヒント: make directory の略です。",
                ),
            },
        ],
        LearningMode::Docker => vec![
            Question {
                prompt: "`nginx` イメージから `web2` という名前でコンテナを起動するコマンドを打ってください。".to_string(),
                required_tokens: vec![
                    "docker".to_string(),
                    "run".to_string(),
                    "--name".to_string(),
                    "web2".to_string(),
                    "nginx".to_string(),
                ],
                synonyms: vec!["docker run --name web2 nginx:latest".to_string()],
                explanation: "`docker run` はイメージからコンテナを作成して起動します。".to_string(),
                hint: hint_for(
                    difficulty,
                    "ヒント: `docker run --name <名前> <image>` の順序で考えてください。",
                ),
            },
            Question {
                prompt: "ローカルのイメージ一覧を表示するコマンドを打ってください。".to_string(),
                required_tokens: vec!["docker".to_string(), "images".to_string()],
                synonyms: vec!["docker image ls".to_string()],
                explanation: "`docker images` は取得済みイメージ一覧を表示します。".to_string(),
                hint: hint_for(difficulty, "ヒント: subcommand は複数形です。"),
            },
            Question {
                prompt: "`web` コンテナのログを表示するコマンドを打ってください。".to_string(),
                required_tokens: vec![
                    "docker".to_string(),
                    "logs".to_string(),
                    "web".to_string(),
                ],
                synonyms: vec!["docker logs web".to_string()],
                explanation: "`docker logs` はコンテナの標準出力ログを表示します。".to_string(),
                hint: hint_for(difficulty, "ヒント: `docker <subcommand> <container>` の形です。"),
            },
        ],
    }
}

fn challenge_bank(mode: LearningMode, difficulty: Difficulty) -> Vec<Challenge> {
    match mode {
        LearningMode::Linux | LearningMode::Macos => vec![
            Challenge {
                prompt: "課題: `logs` ディレクトリを作成し、`readme.txt` を `logs/readme.bak` にコピーしてから `submit` してください。".to_string(),
                required_steps: vec![
                    vec!["mkdir".to_string(), "logs".to_string()],
                    vec![
                        "cp".to_string(),
                        "readme.txt".to_string(),
                        "logs/readme.bak".to_string(),
                    ],
                ],
                forbidden_tokens: vec!["rm".to_string()],
                hint: hint_for(
                    difficulty,
                    "ヒント: 2手必要です。ディレクトリ作成とコピーを順に行ってください。",
                ),
            },
            Challenge {
                prompt: "課題: `notes.txt` を作成し、その後 `archive/notes.txt` に移動してから `submit` してください。".to_string(),
                required_steps: vec![
                    vec!["touch".to_string(), "notes.txt".to_string()],
                    vec!["mkdir".to_string(), "archive".to_string()],
                    vec![
                        "mv".to_string(),
                        "notes.txt".to_string(),
                        "archive/notes.txt".to_string(),
                    ],
                ],
                forbidden_tokens: vec!["rm".to_string()],
                hint: hint_for(
                    difficulty,
                    "ヒント: ファイル作成、保存先作成、移動の3手です。",
                ),
            },
        ],
        LearningMode::Docker => vec![
            Challenge {
                prompt: "課題: `alpine` イメージを取得し、`lab` という名前で起動してから `submit` してください。".to_string(),
                required_steps: vec![
                    vec!["docker".to_string(), "pull".to_string(), "alpine".to_string()],
                    vec![
                        "docker".to_string(),
                        "run".to_string(),
                        "--name".to_string(),
                        "lab".to_string(),
                        "alpine".to_string(),
                    ],
                ],
                forbidden_tokens: vec!["docker rm".to_string()],
                hint: hint_for(
                    difficulty,
                    "ヒント: まずイメージ取得、その後コンテナ起動です。",
                ),
            },
            Challenge {
                prompt: "課題: `web` コンテナを停止し、その後ログを表示してから `submit` してください。".to_string(),
                required_steps: vec![
                    vec!["docker".to_string(), "stop".to_string(), "web".to_string()],
                    vec!["docker".to_string(), "logs".to_string(), "web".to_string()],
                ],
                forbidden_tokens: vec!["docker rm".to_string()],
                hint: hint_for(
                    difficulty,
                    "ヒント: 停止とログ確認の2手です。",
                ),
            },
        ],
    }
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

fn hint_for(difficulty: Difficulty, text: &str) -> Option<String> {
    match difficulty {
        Difficulty::Easy => Some(text.to_string()),
        Difficulty::Normal => {
            Some("ヒントは最小限です。コマンドの骨格を思い出してください。".to_string())
        }
        Difficulty::Hard => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Question, question_for, validate_quiz_answer};
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
}

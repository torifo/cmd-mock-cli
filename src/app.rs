use std::{io, path::PathBuf, time::Duration};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    command::execute as execute_command,
    config::AppConfig,
    docker::DockerSim,
    model::{CompletionMode, Difficulty, LearningMode, PlayMode, SessionSnapshot, SessionState},
    persistence::{PersistedData, SessionStore, StoredSession},
    quiz::{challenge_for, opening_prompt, question_for, validate_challenge, validate_quiz_answer},
    ui::{self, UiModel, UiState},
    vfs::VirtualFs,
};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "cmdock — CLI game for Linux and Docker command practice"
)]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        help = "Target environment to learn (linux, macos, docker)"
    )]
    pub learning_mode: Option<CliLearningMode>,
    #[arg(long, value_enum, help = "Game mode (quiz, challenge)")]
    pub play_mode: Option<CliPlayMode>,
    #[arg(long, value_enum, help = "Hint level (easy, normal, hard)")]
    pub difficulty: Option<CliDifficulty>,
    #[arg(long, help = "Disable tab completion")]
    pub no_completion: bool,
    #[arg(long, help = "List all available modes and options, then exit")]
    pub list: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CliLearningMode {
    Linux,
    Macos,
    Docker,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CliPlayMode {
    Quiz,
    Challenge,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CliDifficulty {
    Easy,
    Normal,
    Hard,
}

impl From<CliLearningMode> for LearningMode {
    fn from(value: CliLearningMode) -> Self {
        match value {
            CliLearningMode::Linux => Self::Linux,
            CliLearningMode::Macos => Self::Macos,
            CliLearningMode::Docker => Self::Docker,
        }
    }
}

impl From<CliPlayMode> for PlayMode {
    fn from(value: CliPlayMode) -> Self {
        match value {
            CliPlayMode::Quiz => Self::Quiz,
            CliPlayMode::Challenge => Self::Challenge,
        }
    }
}

impl From<CliDifficulty> for Difficulty {
    fn from(value: CliDifficulty) -> Self {
        match value {
            CliDifficulty::Easy => Self::Easy,
            CliDifficulty::Normal => Self::Normal,
            CliDifficulty::Hard => Self::Hard,
        }
    }
}

pub struct App {
    config: AppConfig,
    store: SessionStore,
    persisted: PersistedData,
    state: SessionState,
    vfs: VirtualFs,
    docker: DockerSim,
    log_lines: Vec<String>,
}

enum InputAction {
    Continue,
    Consumed,
    Exit,
}

struct HandlerOutput {
    action: InputAction,
    lines: Vec<String>,
}

impl HandlerOutput {
    fn new(action: InputAction) -> Self {
        Self {
            action,
            lines: Vec::new(),
        }
    }

    fn exit() -> Self {
        Self::new(InputAction::Exit)
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

impl App {
    pub fn bootstrap(cli: Cli) -> Result<Self> {
        let (config, _) = AppConfig::load_or_default(cli.config.clone())?;
        let store = SessionStore::new()?;
        let persisted = store.load()?;

        let (mut state, vfs, docker) = if let Some(stored) = persisted.session.clone() {
            (
                SessionState::from(stored.snapshot),
                stored.vfs,
                stored.docker,
            )
        } else {
            (
                SessionState {
                    learning_mode: config.default_learning_mode,
                    play_mode: config.default_play_mode,
                    difficulty: config.default_difficulty,
                    completion: config.completion,
                    quiz_index: 0,
                    challenge_index: 0,
                    current_prompt: None,
                    command_history: Vec::new(),
                    stats: Default::default(),
                },
                VirtualFs::default(),
                DockerSim::default(),
            )
        };

        apply_cli_overrides(&mut state, &cli);

        let mut app = Self {
            config,
            store,
            persisted,
            state,
            vfs,
            docker,
            log_lines: Vec::new(),
        };
        app.ensure_prompt_loaded();
        app.log_lines = app.render_startup_lines();
        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        let _guard = TerminalGuard::enter()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        let mut ui_state = UiState::default();

        loop {
            let suggestions = self.completion_suggestions(ui_state.input(), ui_state.cursor());
            ui_state.sync_completion_index(suggestions.len());
            let model = self.ui_model(&ui_state, &suggestions);
            terminal.draw(|frame| ui::render(frame, &model))?;

            if !event::poll(Duration::from_millis(200))? {
                continue;
            }

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.persist_session()?;
                    break;
                }
                KeyCode::Char('d')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && ui_state.input().is_empty() =>
                {
                    self.persist_session()?;
                    break;
                }
                KeyCode::Enter => {
                    let line = ui_state.input().trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let should_exit = self.process_line(&line)?;
                    ui_state.clear_input();
                    if should_exit {
                        break;
                    }
                }
                KeyCode::Char(ch)
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    ui_state.insert_char(ch);
                }
                KeyCode::Backspace => ui_state.backspace(),
                KeyCode::Delete => ui_state.delete(),
                KeyCode::Left => ui_state.move_left(),
                KeyCode::Right => ui_state.move_right(),
                KeyCode::Home => ui_state.move_home(),
                KeyCode::End => ui_state.move_end(),
                KeyCode::Tab => {
                    if let Some(suggestion) = suggestions.get(ui_state.completion_index()) {
                        ui_state.apply_completion(suggestion);
                    }
                }
                KeyCode::BackTab => ui_state.select_prev_completion(suggestions.len()),
                KeyCode::Up => ui_state.select_prev_completion(suggestions.len()),
                KeyCode::Down => ui_state.select_next_completion(suggestions.len()),
                KeyCode::Esc => {
                    if ui_state.input().is_empty() {
                        self.persist_session()?;
                        break;
                    }
                    ui_state.clear_input();
                }
                _ => {}
            }
        }

        terminal.show_cursor()?;
        Ok(())
    }

    fn process_line(&mut self, line: &str) -> Result<bool> {
        self.push_log_line(format!("> {}", line));

        let meta_output = self.handle_meta_command(line)?;
        self.push_log_lines(meta_output.lines);
        match meta_output.action {
            InputAction::Exit => {
                self.persist_session()?;
                return Ok(true);
            }
            InputAction::Consumed => {
                self.persist_session()?;
                return Ok(false);
            }
            InputAction::Continue => {}
        }

        let output = self.handle_learning_command(line)?;
        self.push_log_lines(output);
        self.persist_session()?;
        Ok(false)
    }

    fn handle_meta_command(&mut self, line: &str) -> Result<HandlerOutput> {
        match line {
            "quit" | "exit" => return Ok(HandlerOutput::exit()),
            "help" => {
                return Ok(HandlerOutput {
                    action: InputAction::Consumed,
                    lines: vec![self.help_text()],
                });
            }
            "resume" => {
                if let Some(stored) = self.persisted.session.clone() {
                    self.restore(stored);
                    let mut lines = vec!["resumed saved session".to_string(), self.render_status()];
                    lines.extend(self.current_prompt_lines());
                    return Ok(HandlerOutput {
                        action: InputAction::Consumed,
                        lines,
                    });
                }

                return Ok(HandlerOutput {
                    action: InputAction::Consumed,
                    lines: vec!["no saved session".to_string()],
                });
            }
            "result" => {
                return Ok(HandlerOutput {
                    action: InputAction::Consumed,
                    lines: vec![self.result_text()],
                });
            }
            "submit" => {
                if self.state.play_mode != PlayMode::Challenge {
                    return Ok(HandlerOutput {
                        action: InputAction::Consumed,
                        lines: vec!["submit is available only in challenge mode".to_string()],
                    });
                }
                let challenge = challenge_for(
                    self.state.learning_mode,
                    self.state.difficulty,
                    self.state.challenge_index,
                );
                let success = validate_challenge(&challenge, &self.state.command_history);
                self.record_result(success, Some(challenge.prompt.clone()));
                let mut lines = Vec::new();
                if success {
                    lines.push("challenge clear".to_string());
                    self.state.challenge_index += 1;
                    self.state.command_history.clear();
                    self.ensure_prompt_loaded();
                    lines.extend(self.current_prompt_lines());
                } else {
                    lines.push("challenge failed".to_string());
                    if let Some(hint) = challenge.hint {
                        lines.push(hint.to_string());
                    }
                }
                return Ok(HandlerOutput {
                    action: InputAction::Consumed,
                    lines,
                });
            }
            _ => {}
        }

        Ok(HandlerOutput::new(InputAction::Continue))
    }

    fn handle_learning_command(&mut self, line: &str) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let result = execute_command(
            self.state.learning_mode,
            line,
            &mut self.vfs,
            &mut self.docker,
        );

        match result {
            Ok(output) => {
                lines.extend(output.stdout.into_iter().filter(|line| !line.is_empty()));
            }
            Err(err) => {
                self.record_command_error(line);
                lines.push(format!("error: {}", err));
            }
        }

        self.state.command_history.push(line.to_string());

        match self.state.play_mode {
            PlayMode::Quiz => {
                let question = question_for(
                    self.state.learning_mode,
                    self.state.difficulty,
                    self.state.quiz_index,
                );
                let success = validate_quiz_answer(&question, line);
                self.record_result(success, Some(question.prompt.clone()));
                if success {
                    lines.push("correct".to_string());
                    lines.push(format!("explanation: {}", question.explanation));
                    if self.config.show_synonyms && !question.synonyms.is_empty() {
                        lines.push(format!("also valid: {}", question.synonyms.join(" | ")));
                    }
                    self.state.quiz_index += 1;
                    self.ensure_prompt_loaded();
                    lines.extend(self.current_prompt_lines());
                } else {
                    lines.push("not quite".to_string());
                    if let Some(hint) = question.hint {
                        lines.push(hint.to_string());
                    }
                }
            }
            PlayMode::Challenge => {
                lines.push("step recorded; use submit when you are done".to_string());
            }
        }

        Ok(lines)
    }

    fn help_text(&self) -> String {
        [
            "meta commands:",
            "  help",
            "  result",
            "  resume",
            "  submit",
            "  quit",
            "",
            "supported shell commands:",
            "  pwd ls cd mkdir touch cat cp mv rm find grep echo",
            "",
            "supported docker commands:",
            "  docker images|pull|run|ps|stop|rm|logs|exec",
            "",
            "to change modes, restart with CLI flags:",
            "  cmdock --list",
        ]
        .join("\n")
    }

    fn result_text(&self) -> String {
        let answered = self.state.stats.answered.max(1);
        let ratio = (self.state.stats.correct as f32 / answered as f32) * 100.0;
        let mistake_sample = if self.state.stats.mistakes.is_empty() {
            "none".to_string()
        } else {
            self.state
                .stats
                .mistakes
                .iter()
                .rev()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "answered: {}\ncorrect: {}\naccuracy: {:.1}%\nmode: {}\ndifficulty: {}\ncompletion: {}\nquiz index: {}\nchallenge index: {}\nrecent mistakes: {}",
            self.state.stats.answered,
            self.state.stats.correct,
            ratio,
            self.state.play_mode.as_str(),
            self.state.difficulty.as_str(),
            self.state.completion.as_str(),
            self.state.quiz_index,
            self.state.challenge_index,
            mistake_sample
        )
    }

    fn render_status(&self) -> String {
        format!(
            "[target:{}] [play:{}] [difficulty:{}] [completion:{}]",
            self.state.learning_mode.as_str(),
            self.state.play_mode.as_str(),
            self.state.difficulty.as_str(),
            self.state.completion.as_str()
        )
    }

    fn record_result(&mut self, success: bool, prompt: Option<String>) {
        self.state.stats.answered += 1;
        let play_mode_stats = self
            .state
            .stats
            .by_play_mode
            .entry(self.state.play_mode.as_str().to_string())
            .or_default();
        play_mode_stats.answered += 1;
        let difficulty_stats = self
            .state
            .stats
            .by_difficulty
            .entry(self.state.difficulty.as_str().to_string())
            .or_default();
        difficulty_stats.answered += 1;
        if success {
            self.state.stats.correct += 1;
            play_mode_stats.correct += 1;
            difficulty_stats.correct += 1;
        } else if let Some(ref prompt_text) = prompt {
            self.state.stats.mistakes.push(prompt_text.clone());
        }
        self.state.current_prompt = prompt.clone();
        self.push_recent_result(success, prompt);
    }

    fn ensure_prompt_loaded(&mut self) {
        self.state.current_prompt = Some(opening_prompt(
            self.state.play_mode,
            self.state.learning_mode,
            self.state.difficulty,
            self.state.quiz_index,
            self.state.challenge_index,
        ));
    }

    fn render_startup_lines(&self) -> Vec<String> {
        let mut lines = vec!["cmdock".to_string(), self.render_status()];
        lines.extend(self.current_prompt_lines());
        lines
    }

    fn current_prompt_lines(&self) -> Vec<String> {
        self.state
            .current_prompt
            .iter()
            .flat_map(|prompt| prompt.lines().map(ToString::to_string))
            .collect()
    }

    fn persist_session(&mut self) -> Result<()> {
        self.persisted.session = Some(StoredSession {
            snapshot: SessionSnapshot::from(self.state.clone()),
            vfs: self.vfs.clone(),
            docker: self.docker.clone(),
        });
        self.store.save(&self.persisted)
    }

    fn restore(&mut self, stored: StoredSession) {
        self.state = SessionState::from(stored.snapshot);
        self.vfs = stored.vfs;
        self.docker = stored.docker;
        self.ensure_prompt_loaded();
    }

    fn push_recent_result(&mut self, success: bool, prompt: Option<String>) {
        let summary = format!(
            "{} | {} | {}",
            if success { "ok" } else { "ng" },
            self.state.learning_mode.as_str(),
            prompt.unwrap_or_else(|| "no prompt".to_string())
        );
        self.persisted.recent_results.push(summary);
        if self.persisted.recent_results.len() > 20 {
            let overflow = self.persisted.recent_results.len() - 20;
            self.persisted.recent_results.drain(0..overflow);
        }
    }

    fn record_command_error(&mut self, line: &str) {
        let command_name = line
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string();
        *self
            .state
            .stats
            .command_errors
            .entry(command_name)
            .or_insert(0) += 1;
    }

    fn completion_suggestions(&self, input: &str, cursor: usize) -> Vec<String> {
        if self.state.completion == CompletionMode::Off {
            return Vec::new();
        }

        let safe_cursor = cursor.min(input.len());
        let prefix = &input[..safe_cursor];
        let token_start = prefix
            .rfind(char::is_whitespace)
            .map(|index| index + 1)
            .unwrap_or(0);
        let current = &prefix[token_start..];

        let mut candidates = base_completions()
            .into_iter()
            .filter(|item| item.starts_with(prefix) || item.starts_with(current))
            .collect::<Vec<_>>();
        candidates.extend(self.vfs.find_paths_with_prefix(current));
        candidates.extend(self.docker.completions(current));
        candidates.sort();
        candidates.dedup();
        candidates.truncate(self.completion_limit());
        candidates
    }

    fn completion_limit(&self) -> usize {
        match self.state.difficulty {
            Difficulty::Easy => 12,
            Difficulty::Normal => 6,
            Difficulty::Hard => 3,
        }
    }

    fn ui_model(&self, ui_state: &UiState, suggestions: &[String]) -> UiModel {
        UiModel {
            summary_lines: self.summary_lines(),
            prompt_lines: self.current_prompt_lines(),
            log_lines: self.log_lines.clone(),
            history_lines: self.history_lines(),
            input: ui_state.input().to_string(),
            cursor: ui_state.cursor(),
            selected_suggestion: ui_state.completion_index(),
            suggestions: suggestions.to_vec(),
            completion_on: self.state.completion == CompletionMode::On,
        }
    }

    fn summary_lines(&self) -> Vec<String> {
        let answered = self.state.stats.answered.max(1);
        let accuracy = (self.state.stats.correct as f32 / answered as f32) * 100.0;
        let mut lines = vec![
            self.render_status(),
            format!(
                "answered:{} correct:{} accuracy:{:.1}%",
                self.state.stats.answered, self.state.stats.correct, accuracy
            ),
            format!(
                "quiz:{} challenge:{} errors:{}",
                self.state.quiz_index,
                self.state.challenge_index,
                self.state.stats.command_errors.values().sum::<u32>()
            ),
            "keys: Enter run | Tab complete | Up/Down select | Esc clear".to_string(),
        ];
        if let Some(last) = self.persisted.recent_results.last() {
            lines.push(format!("last result: {}", last));
        }
        lines
    }

    fn history_lines(&self) -> Vec<String> {
        let mut lines = vec!["command history".to_string()];
        if self.state.command_history.is_empty() {
            lines.push("(empty)".to_string());
        } else {
            lines.extend(
                self.state
                    .command_history
                    .iter()
                    .rev()
                    .take(10)
                    .map(|entry| format!("> {}", entry)),
            );
        }

        lines.push(String::new());
        lines.push("recent results".to_string());
        if self.persisted.recent_results.is_empty() {
            lines.push("(empty)".to_string());
        } else {
            lines.extend(self.persisted.recent_results.iter().rev().take(6).cloned());
        }
        lines
    }

    fn push_log_lines(&mut self, lines: Vec<String>) {
        for line in lines {
            self.push_log_line(line);
        }
    }

    fn push_log_line(&mut self, line: String) {
        self.log_lines.push(line);
        if self.log_lines.len() > 400 {
            let overflow = self.log_lines.len() - 400;
            self.log_lines.drain(0..overflow);
        }
    }
}

fn apply_cli_overrides(state: &mut SessionState, cli: &Cli) {
    if let Some(mode) = cli.learning_mode.clone() {
        state.learning_mode = mode.into();
    }
    if let Some(mode) = cli.play_mode.clone() {
        state.play_mode = mode.into();
        state.command_history.clear();
    }
    if let Some(difficulty) = cli.difficulty.clone() {
        state.difficulty = difficulty.into();
    }
    if cli.no_completion {
        state.completion = CompletionMode::Off;
    }
}

fn base_completions() -> Vec<String> {
    [
        "pwd",
        "ls",
        "cd",
        "mkdir",
        "touch",
        "cat",
        "cp",
        "mv",
        "rm",
        "find",
        "grep",
        "echo",
        "docker",
        "docker images",
        "docker pull",
        "docker run",
        "docker ps",
        "docker stop",
        "docker rm",
        "docker logs",
        "docker exec",
        "help",
        "result",
        "resume",
        "submit",
        "quit",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

pub fn list_modes() -> String {
    [
        "Available options for cmdock:",
        "",
        "  --learning-mode <MODE>   Target environment to learn",
        "    linux    Linux shell commands (default)",
        "    macos    macOS shell commands",
        "    docker   Docker CLI commands",
        "",
        "  --play-mode <MODE>       Game mode",
        "    quiz       Answer prompts with the correct command (default)",
        "    challenge  Complete multi-step tasks then type submit",
        "",
        "  --difficulty <LEVEL>     Hint and range control",
        "    easy    Detailed hints, basic commands (default)",
        "    normal  Minimal hints, wider range",
        "    hard    No hints, broadest range",
        "",
        "  --no-completion          Disable tab completion",
        "",
        "Examples:",
        "  cmdock",
        "  cmdock --learning-mode docker --difficulty hard",
        "  cmdock --play-mode challenge --no-completion",
        "  cmdock --list",
        "",
    ]
    .join("\n")
}

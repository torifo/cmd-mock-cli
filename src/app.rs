use std::{cell::RefCell, path::PathBuf, rc::Rc};

use anyhow::{Result, anyhow};
use clap::Parser;
use rustyline::{
    Context, Editor, Helper,
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
};

use crate::{
    command::execute,
    config::AppConfig,
    docker::DockerSim,
    model::{CompletionMode, Difficulty, LearningMode, PlayMode, SessionSnapshot, SessionState},
    persistence::{PersistedData, SessionStore, StoredSession},
    quiz::{challenge_for, opening_prompt, question_for, validate_challenge, validate_quiz_answer},
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
    #[arg(long, value_enum, help = "Target environment to learn (linux, macos, docker)")]
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
    vfs: Rc<RefCell<VirtualFs>>,
    docker: Rc<RefCell<DockerSim>>,
    helper_state: Rc<RefCell<HelperRuntime>>,
}

enum InputAction {
    Continue,
    Consumed,
    Exit,
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

        let helper_state = Rc::new(RefCell::new(HelperRuntime {
            completion: state.completion,
            difficulty: state.difficulty,
        }));

        let mut app = Self {
            config,
            store,
            persisted,
            state,
            vfs: Rc::new(RefCell::new(vfs)),
            docker: Rc::new(RefCell::new(docker)),
            helper_state,
        };
        app.ensure_prompt_loaded();
        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        let helper = ShellHelper::new(
            Rc::clone(&self.helper_state),
            Rc::clone(&self.vfs),
            Rc::clone(&self.docker),
        );
        let mut editor = Editor::new()?;
        editor.set_helper(Some(helper));

        println!("cmdock");
        println!("{}", self.render_status());
        self.print_current_prompt();

        loop {
            let prompt = format!("{}> ", self.state.learning_mode.as_str());
            match editor.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    editor.add_history_entry(line)?;
                    match self.handle_meta_command(line)? {
                        InputAction::Exit => break,
                        InputAction::Consumed => {
                            self.persist_session()?;
                            continue;
                        }
                        InputAction::Continue => {}
                    }
                    self.handle_learning_command(line)?;
                    self.persist_session()?;
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    self.persist_session()?;
                    break;
                }
                Err(err) => return Err(anyhow!(err)),
            }
        }

        Ok(())
    }

    fn handle_meta_command(&mut self, line: &str) -> Result<InputAction> {
        match line {
            "quit" | "exit" => return Ok(InputAction::Exit),
            "help" => {
                println!("{}", self.help_text());
                return Ok(InputAction::Consumed);
            }
            "resume" => {
                if let Some(stored) = self.persisted.session.clone() {
                    self.restore(stored);
                    println!("resumed saved session");
                    println!("{}", self.render_status());
                    self.print_current_prompt();
                } else {
                    println!("no saved session");
                }
                return Ok(InputAction::Consumed);
            }
            "result" => {
                println!("{}", self.result_text());
                return Ok(InputAction::Consumed);
            }
            "submit" => {
                if self.state.play_mode != PlayMode::Challenge {
                    println!("submit is available only in challenge mode");
                    return Ok(InputAction::Consumed);
                }
                let challenge = challenge_for(
                    self.state.learning_mode,
                    self.state.difficulty,
                    self.state.challenge_index,
                );
                let success = validate_challenge(&challenge, &self.state.command_history);
                self.record_result(success, Some(challenge.prompt.clone()));
                if success {
                    println!("challenge clear");
                    self.state.challenge_index += 1;
                    self.state.command_history.clear();
                    self.ensure_prompt_loaded();
                    self.print_current_prompt();
                } else {
                    println!("challenge failed");
                    if let Some(hint) = challenge.hint {
                        println!("{}", hint);
                    }
                }
                self.persist_session()?;
                return Ok(InputAction::Consumed);
            }
            _ => {}
        }

        Ok(InputAction::Continue)
    }

    fn handle_learning_command(&mut self, line: &str) -> Result<()> {
        let result = {
            let mut vfs = self.vfs.borrow_mut();
            let mut docker = self.docker.borrow_mut();
            execute(self.state.learning_mode, line, &mut vfs, &mut docker)
        };

        match result {
            Ok(output) => {
                for line in output.stdout {
                    if !line.is_empty() {
                        println!("{}", line);
                    }
                }
            }
            Err(err) => println!("error: {}", err),
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
                    println!("correct");
                    println!("explanation: {}", question.explanation);
                    if self.config.show_synonyms && !question.synonyms.is_empty() {
                        println!("also valid: {}", question.synonyms.join(" | "));
                    }
                    self.state.quiz_index += 1;
                    self.ensure_prompt_loaded();
                    self.print_current_prompt();
                } else {
                    println!("not quite");
                    if let Some(hint) = question.hint {
                        println!("{}", hint);
                    }
                }
            }
            PlayMode::Challenge => {
                println!("step recorded; use submit when you are done");
            }
        }

        Ok(())
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
        if success {
            self.state.stats.correct += 1;
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

    fn print_current_prompt(&self) {
        if let Some(prompt) = &self.state.current_prompt {
            println!("{}", prompt);
        }
    }

    fn persist_session(&mut self) -> Result<()> {
        self.persisted.session = Some(StoredSession {
            snapshot: SessionSnapshot::from(self.state.clone()),
            vfs: self.vfs.borrow().clone(),
            docker: self.docker.borrow().clone(),
        });
        self.store.save(&self.persisted)
    }

    fn sync_helper_state(&self) {
        let mut helper_state = self.helper_state.borrow_mut();
        helper_state.completion = self.state.completion;
        helper_state.difficulty = self.state.difficulty;
    }

    fn restore(&mut self, stored: StoredSession) {
        self.state = SessionState::from(stored.snapshot);
        *self.vfs.borrow_mut() = stored.vfs;
        *self.docker.borrow_mut() = stored.docker;
        self.ensure_prompt_loaded();
        self.sync_helper_state();
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

#[derive(Clone, Copy)]
struct HelperRuntime {
    completion: CompletionMode,
    difficulty: Difficulty,
}

struct ShellHelper {
    state: Rc<RefCell<HelperRuntime>>,
    vfs: Rc<RefCell<VirtualFs>>,
    docker: Rc<RefCell<DockerSim>>,
}

impl ShellHelper {
    fn new(
        state: Rc<RefCell<HelperRuntime>>,
        vfs: Rc<RefCell<VirtualFs>>,
        docker: Rc<RefCell<DockerSim>>,
    ) -> Self {
        Self { state, vfs, docker }
    }

    fn limit(&self) -> usize {
        match self.state.borrow().difficulty {
            Difficulty::Easy => 12,
            Difficulty::Normal => 6,
            Difficulty::Hard => 3,
        }
    }
}

impl Helper for ShellHelper {}
impl Hinter for ShellHelper {
    type Hint = String;
}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if self.state.borrow().completion == CompletionMode::Off {
            return Ok((0, Vec::new()));
        }

        let safe_pos = pos.min(line.len());
        let start = line[..safe_pos]
            .rfind(char::is_whitespace)
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let current = &line[start..safe_pos];

        let mut candidates = base_completions()
            .into_iter()
            .filter(|item| item.starts_with(current))
            .collect::<Vec<_>>();
        candidates.extend(self.vfs.borrow().find_paths_with_prefix(current));
        candidates.extend(self.docker.borrow().completions(current));
        candidates.sort();
        candidates.dedup();

        let pairs = candidates
            .into_iter()
            .take(self.limit())
            .map(|candidate| Pair {
                display: candidate.clone(),
                replacement: candidate,
            })
            .collect();
        Ok((start, pairs))
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

# Install Script & Interactive Onboarding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a curl-based install script and an interactive first-launch onboarding flow that guides users through a demo command, then play-mode and difficulty selection.

**Architecture:** `install.sh` is a self-contained Bash script in the repo root that fetches the correct binary from GitHub Releases. The onboarding flow is a new `AppPhase` state machine inside `App`; when no `--play-mode` flag is given and no saved session exists, `App` starts in `Onboarding(Demo)` and advances through `SelectMode` → `SelectDifficulty` before entering `Playing`. All enum changes stay in `src/app.rs` (not serialised, so not in `model.rs`).

**Tech Stack:** Rust / ratatui 0.28 / crossterm — Bash (install.sh)

---

## File Map

| File | Change |
|------|--------|
| `install.sh` | Create — curl installer for GitHub Releases |
| `src/ui.rs` | Modify — fix multi-byte Unicode cursor ops in `UiState` |
| `src/app.rs` | Modify — add `AppPhase`/`OnboardingStep`, `phase` field, onboarding logic |
| `README.md` | Modify — install.sh section, onboarding Quick Start |

---

### Task 1: install.sh

**Files:**
- Create: `install.sh`

- [ ] **Step 1: Write install.sh**

Create `/Users/akito-shoji/dev/cli/cmd-mock/install.sh`:

```bash
#!/bin/bash
set -euo pipefail

REPO="torifo/cmd-mock-cli"
BIN_NAME="cmdock"
INSTALL_DIR="${HOME}/.local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "${OS}" in
  darwin) OS_TAG="macos" ;;
  linux)  OS_TAG="linux" ;;
  *)
    echo "Unsupported OS: ${OS}" >&2
    exit 1
    ;;
esac

case "${ARCH}" in
  x86_64)        ARCH_TAG="x86_64" ;;
  aarch64|arm64) ARCH_TAG="aarch64" ;;
  *)
    echo "Unsupported architecture: ${ARCH}" >&2
    exit 1
    ;;
esac

ARCHIVE="${BIN_NAME}-${OS_TAG}-${ARCH_TAG}.tar.gz"

LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "${LATEST}" ]; then
  echo "Failed to fetch latest release tag" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARCHIVE}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Downloading cmdock ${LATEST} (${OS_TAG}-${ARCH_TAG})..."
curl -fsSL "${URL}" -o "${TMP_DIR}/${ARCHIVE}"
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"

mkdir -p "${INSTALL_DIR}"
install -m 755 "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

echo ""
echo "Installed: ${INSTALL_DIR}/${BIN_NAME}"
echo ""

if ! printf '%s\n' "${PATH//:/$'\n'}" | grep -qx "${INSTALL_DIR}"; then
  echo "NOTE: Add to your shell profile (~/.zshrc or ~/.bashrc):"
  echo "  export PATH=\"${INSTALL_DIR}:\${PATH}\""
  echo ""
fi

echo "Get started:"
echo "  cmdock"
```

- [ ] **Step 2: Make executable and verify syntax**

```bash
chmod +x install.sh
bash -n install.sh
echo "syntax OK"
```

Expected: `syntax OK`

- [ ] **Step 3: Commit**

```bash
git add install.sh
git commit -m "Add curl-based install script / curlインストールスクリプトを追加"
```

---

### Task 2: Fix Unicode cursor handling in UiState

**Files:**
- Modify: `src/ui.rs`

The current `backspace()` decrements `self.cursor` by 1 byte; for multi-byte UTF-8 characters (3+ bytes, e.g. Japanese) this lands on a non-char-boundary and `String::remove` panics. `move_left()` has the same issue. `move_right()` needs char-aware increment too.

- [ ] **Step 1: Write failing tests**

Append at the end of `src/ui.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::UiState;

    #[test]
    fn backspace_handles_multibyte_char() {
        let mut s = UiState::default();
        // 'あ' is 3 UTF-8 bytes
        s.insert_char('あ');
        assert_eq!(s.cursor(), 3);
        s.backspace();
        assert_eq!(s.input(), "");
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn move_left_right_handles_multibyte_chars() {
        let mut s = UiState::default();
        s.insert_char('あ');
        s.insert_char('い');
        // cursor == 6 (2 × 3 bytes)
        s.move_left();
        assert_eq!(s.cursor(), 3);
        s.move_left();
        assert_eq!(s.cursor(), 0);
        s.move_right();
        assert_eq!(s.cursor(), 3);
    }
}
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cargo test ui::tests --lib 2>&1 | tail -15
```

Expected: FAIL — `backspace` panics or cursor is wrong

- [ ] **Step 3: Fix backspace(), move_left(), move_right()**

In `src/ui.rs`, replace the three method bodies:

```rust
pub fn backspace(&mut self) {
    if self.cursor == 0 {
        return;
    }
    let prev = self.input[..self.cursor]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0);
    self.input.remove(prev);
    self.cursor = prev;
    self.completion_index = 0;
}

pub fn move_left(&mut self) {
    if self.cursor > 0 {
        self.cursor = self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }
}

pub fn move_right(&mut self) {
    if self.cursor < self.input.len() {
        let ch = self.input[self.cursor..].chars().next().unwrap();
        self.cursor += ch.len_utf8();
    }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test ui::tests --lib 2>&1 | tail -5
```

Expected: `test result: ok. 2 passed`

- [ ] **Step 5: Run full suite**

```bash
cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 22 passed` (20 existing + 2 new)

- [ ] **Step 6: Commit**

```bash
git add src/ui.rs
git commit -m "Fix Unicode cursor ops in UiState / UiStateのUnicodeカーソル操作を修正"
```

---

### Task 3: AppPhase state machine and onboarding logic

**Files:**
- Modify: `src/app.rs`

This task adds `OnboardingStep` + `AppPhase` enums, a `phase` field on `App`, the `process_onboarding()` method, and wires everything together. The enums live in `src/app.rs` — they are runtime-only and are never serialised.

**Triggering onboarding:**
- No saved session AND no `--play-mode` → `Onboarding(Demo)` → `SelectMode` → `SelectDifficulty`
- No saved session AND `--play-mode` given but no `--difficulty` → `Onboarding(SelectDifficulty)`
- Saved session OR both flags given → `Playing` directly

- [ ] **Step 1: Write failing tests**

Add to the bottom of `src/app.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{App, AppPhase, Cli, CliDifficulty, CliPlayMode, OnboardingStep};

    fn bare_cli() -> Cli {
        Cli {
            config: None,
            learning_mode: None,
            play_mode: None,
            difficulty: None,
            no_completion: false,
            list: false,
        }
    }

    #[test]
    fn no_flags_starts_onboarding_demo() {
        let app = App::bootstrap(bare_cli()).unwrap();
        assert_eq!(app.phase, AppPhase::Onboarding(OnboardingStep::Demo));
    }

    #[test]
    fn play_mode_and_difficulty_flags_skip_onboarding() {
        let mut cli = bare_cli();
        cli.play_mode = Some(CliPlayMode::Quiz);
        cli.difficulty = Some(CliDifficulty::Easy);
        let app = App::bootstrap(cli).unwrap();
        assert_eq!(app.phase, AppPhase::Playing);
    }

    #[test]
    fn play_mode_without_difficulty_starts_at_select_difficulty() {
        let mut cli = bare_cli();
        cli.play_mode = Some(CliPlayMode::Quiz);
        let app = App::bootstrap(cli).unwrap();
        assert_eq!(
            app.phase,
            AppPhase::Onboarding(OnboardingStep::SelectDifficulty)
        );
    }
}
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cargo test app::tests --lib 2>&1 | tail -10
```

Expected: FAIL — `AppPhase`, `OnboardingStep`, `app.phase` do not exist yet

- [ ] **Step 3: Add enums after `enum InputAction` in src/app.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardingStep {
    Demo,
    SelectMode,
    SelectDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPhase {
    Onboarding(OnboardingStep),
    Playing,
}
```

- [ ] **Step 4: Add `phase` field to `App` struct**

```rust
pub struct App {
    config: AppConfig,
    store: SessionStore,
    persisted: PersistedData,
    state: SessionState,
    vfs: VirtualFs,
    docker: DockerSim,
    log_lines: Vec<String>,
    pub phase: AppPhase,
}
```

- [ ] **Step 5: Add `initial_phase` free function near `apply_cli_overrides`**

```rust
fn initial_phase(cli: &Cli, has_saved_session: bool) -> AppPhase {
    if has_saved_session {
        return AppPhase::Playing;
    }
    if cli.play_mode.is_none() {
        return AppPhase::Onboarding(OnboardingStep::Demo);
    }
    if cli.difficulty.is_none() {
        return AppPhase::Onboarding(OnboardingStep::SelectDifficulty);
    }
    AppPhase::Playing
}
```

- [ ] **Step 6: Update `bootstrap()` to use the new field**

Replace the `let mut app = Self { ... }; app.ensure_prompt_loaded(); app.log_lines = ...; Ok(app)` block with:

```rust
let has_saved_session = persisted.session.is_some();
let phase = initial_phase(&cli, has_saved_session);

let mut app = Self {
    config,
    store,
    persisted,
    state,
    vfs,
    docker,
    log_lines: Vec::new(),
    phase,
};
if matches!(phase, AppPhase::Playing) {
    app.ensure_prompt_loaded();
}
app.log_lines = app.render_startup_lines();
Ok(app)
```

- [ ] **Step 7: Update `render_startup_lines()` to skip quiz prompt during onboarding**

Replace the method body:

```rust
fn render_startup_lines(&self) -> Vec<String> {
    if matches!(self.phase, AppPhase::Onboarding(_)) {
        vec!["cmdock".to_string()]
    } else {
        let mut lines = vec!["cmdock".to_string(), self.render_status()];
        lines.extend(self.current_prompt_lines());
        lines
    }
}
```

- [ ] **Step 8: Add `process_onboarding()` method to `impl App`**

```rust
fn process_onboarding(&mut self, step: OnboardingStep, line: &str) -> Result<bool> {
    self.push_log_line(format!("> {}", line));
    match step {
        OnboardingStep::Demo => {
            let result = execute_command(
                LearningMode::Linux,
                line,
                &mut self.vfs,
                &mut self.docker,
            );
            match result {
                Ok(output) => {
                    let lines: Vec<String> =
                        output.stdout.into_iter().filter(|l| !l.is_empty()).collect();
                    self.push_log_lines(lines);
                }
                Err(err) => self.push_log_line(format!("error: {}", err)),
            }
            self.push_log_line("--- Virtual environment OK ---".to_string());
            self.phase = AppPhase::Onboarding(OnboardingStep::SelectMode);
        }
        OnboardingStep::SelectMode => match line.trim() {
            "1" => {
                self.state.play_mode = PlayMode::Quiz;
                self.push_log_line("play mode: quiz".to_string());
                self.phase = AppPhase::Onboarding(OnboardingStep::SelectDifficulty);
            }
            "2" => {
                self.state.play_mode = PlayMode::Challenge;
                self.push_log_line("play mode: challenge".to_string());
                self.phase = AppPhase::Onboarding(OnboardingStep::SelectDifficulty);
            }
            _ => {
                self.push_log_line("Enter 1 (quiz) or 2 (challenge).".to_string());
            }
        },
        OnboardingStep::SelectDifficulty => {
            let chosen = match line.trim() {
                "1" => Some(Difficulty::Easy),
                "2" => Some(Difficulty::Normal),
                "3" => Some(Difficulty::Hard),
                _ => None,
            };
            match chosen {
                Some(difficulty) => {
                    self.state.difficulty = difficulty;
                    self.phase = AppPhase::Playing;
                    self.ensure_prompt_loaded();
                    self.push_log_line(format!("difficulty: {}", difficulty.as_str()));
                    self.push_log_line("--- Game Start! ---".to_string());
                    let prompt_lines = self.current_prompt_lines();
                    self.push_log_lines(prompt_lines);
                }
                None => {
                    self.push_log_line("Enter 1 (easy), 2 (normal), or 3 (hard).".to_string());
                }
            }
        }
    }
    self.persist_session()?;
    Ok(false)
}
```

- [ ] **Step 9: Update `process_line()` to dispatch to onboarding**

At the top of `process_line`, before the existing `self.push_log_line(format!("> {}", line))`:

```rust
fn process_line(&mut self, line: &str) -> Result<bool> {
    if let AppPhase::Onboarding(step) = self.phase {
        return self.process_onboarding(step, line);
    }
    self.push_log_line(format!("> {}", line));
    // ... rest of existing code unchanged
```

- [ ] **Step 10: Add `onboarding_prompt_lines()` method**

```rust
fn onboarding_prompt_lines(&self, step: OnboardingStep) -> Vec<String> {
    match step {
        OnboardingStep::Demo => vec![
            "Welcome to cmdock!".to_string(),
            String::new(),
            "Try typing this command to confirm the virtual environment is working:".to_string(),
            "  ls".to_string(),
            String::new(),
            "Press Enter to run it.".to_string(),
        ],
        OnboardingStep::SelectMode => vec![
            "Select play mode:".to_string(),
            "  1) quiz       Answer prompts with the correct command".to_string(),
            "  2) challenge  Complete multi-step tasks then type submit".to_string(),
        ],
        OnboardingStep::SelectDifficulty => vec![
            "Select difficulty:".to_string(),
            "  1) easy    Detailed hints, basic commands".to_string(),
            "  2) normal  Minimal hints, wider range".to_string(),
            "  3) hard    No hints, broadest range".to_string(),
        ],
    }
}
```

- [ ] **Step 11: Update `ui_model()` to show onboarding content**

Replace the `ui_model` method:

```rust
fn ui_model(&self, ui_state: &UiState, suggestions: &[String]) -> UiModel {
    let (summary_lines, prompt_lines, completion_on) =
        if let AppPhase::Onboarding(step) = self.phase {
            (
                vec!["cmdock — CLI command practice".to_string()],
                self.onboarding_prompt_lines(step),
                false, // no completions during onboarding
            )
        } else {
            (
                self.summary_lines(),
                self.current_prompt_lines(),
                self.state.completion == CompletionMode::On,
            )
        };

    UiModel {
        summary_lines,
        prompt_lines,
        log_lines: self.log_lines.clone(),
        history_lines: self.history_lines(),
        input: ui_state.input().to_string(),
        cursor: ui_state.cursor(),
        selected_suggestion: ui_state.completion_index(),
        suggestions: suggestions.to_vec(),
        completion_on,
    }
}
```

- [ ] **Step 12: Run tests**

```bash
cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 25 passed` (22 + 3 new app tests)

- [ ] **Step 13: Commit**

```bash
git add src/app.rs
git commit -m "Add interactive onboarding flow / 対話的なオンボーディングフローを追加"
```

---

### Task 4: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace `## Install` section**

The new Install section should read:

```markdown
## Install

### Script (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/torifo/cmd-mock-cli/main/install.sh | bash
```

バイナリは `~/.local/bin/cmdock` に配置されます。`~/.local/bin` が PATH に含まれていない場合は、スクリプトが案内します。

将来的には Homebrew formula の提供を予定しています。

### From Source

前提: Rust toolchain / Cargo

```bash
git clone git@github.com:torifo/cmd-mock-cli.git
cd cmd-mock-cli
cargo install --path .
cmdock --help
```
```

- [ ] **Step 2: Replace `## Quick Start` section**

```markdown
## Quick Start

```bash
cmdock
```

引数なしで起動するとオンボーディングが始まります。

```
Welcome to cmdock!

Try typing this command to confirm the virtual environment is working:
  ls

Press Enter to run it.
> ls
readme.txt
--- Virtual environment OK ---

Select play mode:
  1) quiz       Answer prompts with the correct command
  2) challenge  Complete multi-step tasks then type submit
> 1

Select difficulty:
  1) easy    Detailed hints, basic commands
  2) normal  Minimal hints, wider range
  3) hard    No hints, broadest range
> 2

--- Game Start! ---
```

フラグで直接起動することもできます。

```bash
cmdock --learning-mode docker --difficulty hard
cmdock --play-mode challenge --no-completion
cmdock --list   # 全オプションを確認
```
```

- [ ] **Step 3: Remove "Homebrew / install script は未提供です" from Current Limitations**

The `## Current Limitations` section should read:

```markdown
## Current Limitations

- macOS 専用の問題セットはまだ Linux と共通です
- 問題セットはまだ最小限です（shell 8問、docker 6問）
```

- [ ] **Step 4: Run tests**

```bash
cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 25 passed`

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "Update README: install script and onboarding / READMEを更新: インストールとオンボーディング"
```

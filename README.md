# cmdock

Linux / Docker コマンド学習用のモック CLI ゲームです。  
実環境には触れず、仮想ファイルシステムと仮想 Docker 状態だけを更新します。

## What It Does

- Linux / macOS / Docker の学習モードを選べます
- `quiz` と `challenge` の 2 モードでコマンド練習できます
- Tab キーで補完候補を表示・選択できます（難易度で候補数が変わります）
- 同義コマンドも正解として判定します
- セッション保存、再開、成績確認ができます
- 実際のマシンには変更を加えません

## Install

### From GitHub Release

[Releases](https://github.com/torifo/cmd-mock-cli/releases) から自分の OS 向けアーカイブをダウンロードして実行してください。

```bash
# macOS / Linux
tar -xzf cmdock-macos-aarch64.tar.gz
./cmdock --help
```

将来的には以下を提供予定です。

- Homebrew install
- install script

### From Source

前提: Rust toolchain / Cargo

```bash
git clone git@github.com:torifo/cmd-mock-cli.git
cd cmd-mock-cli
cargo install --path .
cmdock --help
```

## Quick Start

```bash
cmdock
```

起動すると対話型 TUI が開きます。問題文が上部に表示されるので、コマンドを入力してください。

モードを指定して起動することもできます。

```bash
cmdock --learning-mode docker --difficulty hard
cmdock --play-mode challenge --no-completion
cmdock --list   # 全オプションを確認
```

## Modes

すべてのモードは起動時の CLI フラグで指定します。

```bash
cmdock --list
```

```text
Available options for cmdock:

  --learning-mode <MODE>   Target environment to learn
    linux    Linux shell commands (default)
    macos    macOS shell commands
    docker   Docker CLI commands

  --play-mode <MODE>       Game mode
    quiz       Answer prompts with the correct command (default)
    challenge  Complete multi-step tasks then type submit

  --difficulty <LEVEL>     Hint and range control
    easy    Detailed hints, basic commands (default)
    normal  Minimal hints, wider range
    hard    No hints, broadest range

  --no-completion          Disable tab completion
```

## TUI Layout

```
┌─────────────────────────────────────────────────────────┐
│ [linux] [quiz] [easy] [completion:on]                   │  ← ステータスバー
├─────────────────────────────────────────────────────────┤
│ Question                                                │  ← 問題文
│ 現在のディレクトリ配下から `app.log` を探してください。  │
├─────────────────────────────────────────────────────────┤
│ Input  (Tab: complete  Shift+Tab: prev  Ctrl+C: quit)   │  ← 入力欄
│ > find . -name app.log█                                 │
├─────────────────────────────────────────────────────────┤
│ Completions                                             │  ← 補完候補（Tab で表示）
│ ▶ find . -name app.log                                  │
├─────────────────────────────────────────────────────────┤
│ History                                                 │  ← フィードバック履歴
│ correct                                                 │
│ explanation: find . -name で再帰検索します               │
└─────────────────────────────────────────────────────────┘
```

## In-Game Commands

ゲーム中に使えるメタコマンドです。

```text
help     このヘルプを表示
result   正答率・モード別・難易度別の成績を表示
resume   保存済みセッションを再開
submit   課題の採点 (challenge モードのみ)
quit     終了
```

## Supported Commands

### Shell

| コマンド | 対応フラグ |
|---|---|
| `pwd` | — |
| `ls` | `-a`, `-l`, `-la` |
| `cd` | — |
| `mkdir` | `-p` (親ディレクトリ自動作成) |
| `touch` | — |
| `cat` | — |
| `cp` | — |
| `mv` | — |
| `rm` | `-r`, `-rf` (再帰削除) |
| `find` | `-name`, `-type` |
| `grep` | ファイル引数指定 |
| `echo` | — |

### Docker

| コマンド | 対応フラグ |
|---|---|
| `docker images` | — |
| `docker pull <image>` | — |
| `docker run <image>` | `--name`, `-d`, `-p`, `-e`, `-v` |
| `docker ps` | `-a` |
| `docker stop <name>` | — |
| `docker rm <name>` | — |
| `docker logs <name>` | — |
| `docker exec <name> <cmd>` | — |

## Result Display

`result` で以下を確認できます。

```text
answered: 12  correct: 10  accuracy: 83.3%

by play mode:
  quiz: 8/10 (80.0%)
  challenge: 2/2 (100.0%)

by difficulty:
  easy: 6/7 (85.7%)
  hard: 4/5 (80.0%)

most missed commands:
  find: 2 times
  grep: 1 times
```

## Example Session

```text
$ cmdock --learning-mode docker --play-mode quiz
# TUI が起動し、問題文と入力欄が表示されます

docker run --name web2 nginx   → correct
docker image ls                → correct (synonym accepted)
```

## Development

```bash
cargo run                                          # 起動
cargo test                                         # テスト (20件)
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
```

## CI / Release

- CI: `.github/workflows/ci.yml` (fmt / clippy / test)
- Release: `.github/workflows/release.yml` (タグ `v*.*.*` で Linux/macOS/Windows バイナリを自動生成)

## Current Limitations

- Homebrew / install script は未提供です
- macOS 専用の問題セットはまだ Linux と共通です
- 問題セットはまだ最小限です（shell 8問、docker 6問）

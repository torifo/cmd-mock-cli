# cmd-mock-cli

Linux / Docker コマンド学習用のモック CLI ゲームです。  
実環境には触れず、仮想ファイルシステムと仮想 Docker 状態だけを更新します。

## What It Does

- Linux / macOS / Docker の学習モードを切り替えられます
- `quiz` と `challenge` の 2 モードでコマンド練習できます
- 補完あり / なしを切り替えられます
- 実際のマシンには変更を加えません
- セッション保存、再開、結果確認ができます

## Install

現在の安定した導入方法はソースからの起動です。

### From Source

前提:

- Rust toolchain
- Cargo

```bash
git clone git@github.com:torifo/cmd-mock-cli.git
cd cmd-mock-cli
cargo run -- --help
```

### From GitHub Release

`v0.1.0` 以降は GitHub Release に各 OS 向けバイナリを添付します。  
利用者は Release ページから自分の環境向けアーカイブを取得して実行できます。

将来的には以下を提供予定です。

- Homebrew install
- install script

## Quick Start

```bash
cargo run
```

起動後は問題文が表示されるので、コマンドを入力してください。

例:

```text
linux> cat readme.txt
docker> docker images
```

## Modes

### Learning Targets

- `linux`
- `macos`
- `docker`

### Play Modes

- `quiz`: 問題文に対して適切なコマンドを打つ
- `challenge`: 複数手順の課題を進めて `submit` で判定する

### Difficulty

- `easy`
- `normal`
- `hard`

### Completion

- `completion:on`
- `completion:off`

## Current Features

- 学習対象モード: `linux`, `macos`, `docker`
- プレイモード: `quiz`, `challenge`
- 難易度: `easy`, `normal`, `hard`
- 補完切替: `completion:on`, `completion:off`
- セッション継続: `resume`
- 成績表示: `result`
- 課題提出: `submit`

## Meta Commands

```text
help
result
resume
submit
mode quiz|challenge|linux|macos|docker|easy|normal|hard|completion:on|completion:off
quit
```

## Supported Commands

### Shell

```text
pwd ls cd mkdir touch cat cp mv rm find grep echo
```

### Docker

```text
docker images
docker pull
docker run
docker ps
docker stop
docker rm
docker logs
docker exec
```

## Example Session

```text
$ cargo run -- --learning-mode docker --play-mode quiz
cmd-mock-cli
[target:docker] [play:quiz] [difficulty:easy] [completion:on]
`nginx` イメージから `web2` という名前でコンテナを起動するコマンドを打ってください。

docker> docker run --name web2 nginx
started web2
correct
```

## Development

### Run

```bash
cargo run
```

### Tests

```bash
cargo test
```

### Lint

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Format

```bash
cargo fmt
```

## CI / Release

- CI: `.github/workflows/ci.yml`
- Release: `.github/workflows/release.yml`
- Roadmap: `release-plan.md`

## Current Limitations

- UI は現時点では `rustyline` ベースの対話 CLI です
- `ratatui` ベースの本格 TUI は未実装です
- 問題セットとコマンド定義はまだコード内にあります
- Homebrew / install script は未提供です

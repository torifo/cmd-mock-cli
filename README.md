# cmdock

Linux / Docker コマンド学習用のモック CLI ゲームです。  
実環境には触れず、仮想ファイルシステムと仮想 Docker 状態だけを更新します。

## What It Does

- Linux / macOS / Docker の学習モードを選べます
- `quiz` と `challenge` の 2 モードでコマンド練習できます
- 補完あり / なしを切り替えられます
- 実際のマシンには変更を加えません
- セッション保存、再開、結果確認ができます

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

起動すると問題文が表示されるので、コマンドを入力してください。

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

## In-Game Commands

ゲーム中に使えるメタコマンドです。

```text
help     このヘルプを表示
result   正答率と成績を表示
resume   保存済みセッションを再開
submit   課題の採点 (challenge モードのみ)
quit     終了
```

## Supported Commands

### Shell

```text
pwd  ls  cd  mkdir  touch  cat  cp  mv  rm  find  grep  echo
```

### Docker

```text
docker images
docker pull <image>
docker run [--name <name>] <image>
docker ps [-a]
docker stop <name>
docker rm <name>
docker logs <name>
docker exec <name> <cmd>
```

## Example Session

```text
$ cmdock --learning-mode docker --play-mode quiz
cmdock
[target:docker] [play:quiz] [difficulty:easy] [completion:on]
`nginx` イメージから `web2` という名前でコンテナを起動するコマンドを打ってください。

docker> docker run --name web2 nginx
started web2
correct
explanation: `docker run` はイメージからコンテナを作成して起動します。
also valid: docker run --name web2 nginx:latest
```

## Development

```bash
cargo run                                          # 起動
cargo test                                         # テスト
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
```

## CI / Release

- CI: `.github/workflows/ci.yml` (fmt / clippy / test)
- Release: `.github/workflows/release.yml` (タグ `v*.*.*` で Linux/macOS/Windows バイナリを自動生成)

## Current Limitations

- UI は現時点では `rustyline` ベースの対話 CLI です。`ratatui` ベースの TUI は未実装です
- 問題セットはまだ最小限です
- Homebrew / install script は未提供です

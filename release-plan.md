# Release Plan

## Goal

`cmd-mock-cli` を GitHub Release で配布し、最終的にコマンドでインストールできる状態まで持っていく。

到達イメージ:

- タグ作成で各 OS 向けバイナリを自動ビルド
- GitHub Release に成果物を自動添付
- 将来的に Homebrew または install script から導入可能

## Current State

- Rust の単一バイナリとして起動可能
- `cargo test` が通る
- ローカル Git remote 設定済み
- GitHub Actions は未整備
- 配布用パッケージ、インストール導線は未整備

## Milestones

### M1. Quality Gate

目的:

- main ブランチへの変更を最低限の品質チェックで守る

作業:

- GitHub Actions で `cargo fmt --check`
- GitHub Actions で `cargo clippy -- -D warnings`
- GitHub Actions で `cargo test`

完了条件:

- Pull Request / push のたびにチェックが自動実行される

### M2. Release Automation

目的:

- タグ作成だけで配布物を生成する

作業:

- `v*.*.*` タグで release workflow を起動
- macOS / Linux / Windows の3種類をビルド
- 実行ファイルをアーカイブ化
- GitHub Release に自動アップロード

完了条件:

- タグ push 後に Release 画面から各 OS 用バイナリを取得できる

### M3. Install Strategy

目的:

- ユーザーがコマンドで導入できるようにする

推奨順:

1. Homebrew tap
2. install script
3. `cargo install --git`

補足:

- macOS ユーザー向けには Homebrew が最優先
- Linux では `curl -fsSL ... | sh` 形式の install script も有効
- Rust 利用者向けには `cargo install --git` を補助導線として残す

完了条件:

- README にインストール手順が載る
- Release 資産を使ってバージョン指定インストールできる

### M4. Distribution UX

目的:

- 配布後の初回体験を安定させる

作業:

- `--version` と `--help` の品質改善
- 設定ファイル生成導線
- 保存先、設定先の OS 別安定化
- エラーメッセージ整理

完了条件:

- 新規ユーザーが README の手順だけで起動できる

## Open Tasks

### Product

- コマンド対応範囲の拡張
- 問題セットのデータ外出し
- `ratatui` への移行
- 同義コマンド判定の強化

### Release Engineering

- CI workflow の安定化
- Release workflow の安定化
- バージョニングルールの定義
- CHANGELOG 運用の定義
- チェックサム生成

### Packaging

- Homebrew tap リポジトリ準備
- Formula 生成方針の決定
- install script 作成
- README の配布手順整備

## Suggested Next Order

1. CI を有効化
2. Release workflow を有効化
3. タグベースの配布を確認
4. Homebrew 配布を追加
5. install script を追加
6. README を配布中心に更新

## Tagging Policy

推奨:

- `v0.1.0`
- `v0.1.1`
- `v0.2.0`

ルール:

- 破壊的変更: minor 以上で明示
- 学習コンテンツ追加のみでも release は切ってよい
- 配布導線が変わる変更は changelog に必ず記載


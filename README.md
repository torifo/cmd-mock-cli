# cmd-mock-cli

Linux / Docker コマンド学習用のモック CLI ゲームです。  
実環境には触れず、仮想ファイルシステムと仮想 Docker 状態だけを更新します。

## 起動

```bash
rtk cargo run -- --help
rtk cargo run
```

## 対応済み MVP

- 学習対象モード: `linux`, `macos`, `docker`
- プレイモード: `quiz`, `challenge`
- 難易度: `easy`, `normal`, `hard`
- 補完切替: `completion:on`, `completion:off`
- セッション継続: `resume`
- 成績表示: `result`
- 課題提出: `submit`

## メタコマンド

```text
help
result
resume
submit
mode quiz|challenge|linux|macos|docker|easy|normal|hard|completion:on|completion:off
quit
```

## 対応コマンド

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

## テスト

```bash
rtk cargo test
```

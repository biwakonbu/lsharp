# Rust 非必須 Fresh Clone 仕様

Rust ツールチェーンを必要としない L# の native selfhost 開発導線を定義する。GitHub Actions の自動 build は使わない。検証と release は Mac Apple Silicon と Lima Linux x86_64 VM の手動 local gate で行う。

## 目的

通常の L# 開発、テスト、WASI コンパイルは native stage0 package だけで開始できるようにする。Rust workspace は削除しないが、stage0 producer、oracle differential、緊急 rollback、Rust host integration の保守境界に限定する。

Supported product/release targets は `aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` のみである。

## Native stage0 を取得する

```bash
git clone https://github.com/biwakonbu/lsharp.git
cd lsharp

# 手動 GitHub Release から target-native stage0 を取得する
STAGE0_VERSION=v<version> ./scripts/fetch-stage0.sh
```

- asset 名は `lsharp-stage0-<version>-<target>.tar.gz` とする。
- `fetch-stage0.sh` は release-level `checksums.txt`、package 内 `checksums.txt`、`lsharp-native-selfhost-stage0` manifest、target triple、実行可能な compiler/transport/materializer を検証する。
- 成功時は `stage0/manifest.json` と `stage0/bin/{compiler,transport-driver,materializer}` を配置する。App.Cli の native-only archive は stage0 package として受け入れない。
- `STAGE0_TARGET=<triple>` と `STAGE0_RELEASE_BASE_URL=<url>` は手動 mirror または local release set の検証時だけ上書きする。

## Rust なしで開発する

`scripts/native-selfhost-dev.sh` は既定で `./stage0` を使う。環境変数や `--stage0-dir` は別の stage0 を試す場合だけ必要である。

```bash
./scripts/native-selfhost-dev.sh check examples/fib.ls
./scripts/native-selfhost-dev.sh test examples/fib.ls
./scripts/native-selfhost-dev.sh --bootstrap compile examples/fib.ls -o fib.wasm
```

初回または `--bootstrap` 指定時だけ source tree から `program.native` を再生成する。source fingerprint が変わらなければ生成済み native compiler を再利用する。対応 command と Rust-only boundary は `rust-boundary-reduction.md` を正本とする。

## 手動 release gate

両 target の current fixed-point stage3 から作った stage0 directory と App.Cli artifact を渡して、release set を手元で作る。

```bash
VERSION=v<version> \
MACOS_APP_CLI_ARTIFACT_DIR=<mac-app-cli-dir> \
LINUX_APP_CLI_ARTIFACT_DIR=<linux-app-cli-dir> \
MACOS_STAGE0_DIR=<mac-stage0-dir> \
LINUX_STAGE0_DIR=<linux-stage0-dir> \
MACOS_ROLLBACK_ARCHIVE=<mac-rollback-archive> \
LINUX_ROLLBACK_ARCHIVE=<linux-rollback-archive> \
  bash scripts/ci/native-official-release-local.sh
```

この gate は App.Cli archive と target 別 stage0 archive を package 化し、`dist/native-official/checksums.txt` を生成する。続けて同じ local release set から `fetch-stage0.sh` を再実行し、manifest と checksum の取得経路まで確認する。Mac archive は host 上、Linux App.Cli archive は Lima VM 上で smoke する。GitHub Release にはこれらの archive と `checksums.txt` を手動で添付する。

## 現在の到達点と残件

- Mac Apple Silicon は current fixed-point stage3 の stage0 package を使い、`cargo`、`rustc`、host `lsharp` を block した source-file smoke を完走している。
- Linux x86_64 は commit `4bd9ee9` から生成した stage0 package で stage1 -> stage3 fixed-point と source-file smoke を完了している。以後の GADT / record pattern selfhost checkpoint はその gate 後の変更なので、checkpoint commit 後に同じ source-file smoke と手動 release gate を再実行する。
- Rust host integration が必要なのは `mcp-server`、`--emit-ir`、native/web target、および emergency rollback である。通常の native selfhost 開発成功を Rust fallback で代替してはならない。

## Legacy compatibility reference

`scripts/bootstrap.sh`、`scripts/release-bundle.sh`、`scripts/ci/test-fresh-clone.sh`、`release-smoke`、`smoke_test_readme.sh` は rollback compatibility や過去の downloaded artifact 調査用に残す。通常の開発・release では実行しない。`NATIVE_ONLY_RELEASE=0` の rollback compatibility archive は native stage0 の代替配布物ではない。

## Legacy CI ジョブ

`test-fresh-clone`、`fresh-clone-smoke`、`release-smoke` と関連 workflow は historical reference と rollback compatibility の再現用途に残す。通常の開発、検証、公開では実行・dispatch しない。`NATIVE_ONLY_RELEASE=0` は host launcher + guest component の rollback compatibility archive を意味し、native stage0 release asset を意味しない。

## 証跡

- `scripts/fetch-stage0.sh`
- `scripts/native-selfhost-dev.sh`
- `scripts/ci/package-native-stage0-release.sh`
- `scripts/ci/native-official-release-local.sh`
- `scripts/ci/native-selfhost-dev-source-file-smoke.sh`

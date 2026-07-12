# リリースプレイブック

L# の **手元実行手順** を定義する。配布チャネル、supported product/release targets、署名、package manager 方針の正本は [`release-distribution-signing.md`](./release-distribution-signing.md)。このページは自動化スクリプト `scripts/release-playbook.sh` と並走するオペレーター向け runbook に絞る。stable の配布モデルは **native-only archive** を正本とし、host launcher + guest component は rollback compatibility asset として扱う。

## 概要

```
バージョンバンプ → 手元検証 → target-native input 事前生成 → local manual release gate → タグ作成 → 手動 GitHub Release
```

- channel / target matrix は `release-distribution-signing.md`
- artifact retention は `artifact-policy.md`
- CI 自動 build は停止中。手元の manual release gate が正本

> **Temporary policy (2026-07-12): CI を起動せず**、Mac Apple Silicon と Lima Linux x86_64 VM 上で release artifact を生成・検証する。`.github/workflows/release.yml` は legacy Actions fallback であり、通常の release では dispatch しない。

## 手順

### 1. バージョンバンプ

```bash
# Cargo.toml のバージョンを更新
# workspace 全体で統一バージョンを使用
vim Cargo.toml   # version = "0.x.y"
```

- `Cargo.toml` の `[workspace.package]` セクションで一元管理
- セマンティックバージョニングに従う

### 2. 手元検証

```bash
./scripts/release-playbook.sh <version>
```

スクリプトは以下を順に実行する:

| Step | コマンド | 説明 |
|------|----------|------|
| 1 | `cargo build --release` | リリースビルド |
| 2 | `cargo test` | 全テスト実行 |
| 3 | `cargo clippy -- -D warnings` | リント |
| 4 | `cargo fmt --check` | フォーマット検証 |
| 5 | `LSHARP_BIN=target/release/lsharp bash scripts/ci/compile-phase11-inputs.sh` | local release binary で selfhost / stdlib / examples の固定入力セットを検証 |
| 6 | `LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh` + `scripts/smoke_test_readme.sh` | local release binary smoke + README smoke |
| 7 | `bash scripts/ci/release-smoke.sh dist/lsharp-<version>-<target>.<ext>` | 生成済み release archive を展開し、checksum 検証と packaged binary smoke を行う |
| 8 | チェックサム生成 | `scripts/checksum.sh` |

Mac + Lima VM 上の Linux x86_64 actual self-regeneration は local operator evidence であり、この required CI 検証や `ci-gate-v2` の job graphには含めない。

### 3. アーティファクト生成

リリースビルド成果物:

| アーティファクト | 説明 |
|---|---|
| immutable App.Cli input bundle | archive root の `program.native` + `manifest.json`。target ごとに bundle 自体の SHA-256 を固定 |
| native-only archive alias | archive 内 `lsharp`（`program.native` と同一 bytes） |
| rollback compatibility asset | target ごとの実在 host launcher archive。archive 自体の SHA-256 を固定 |
| release playbook 検証成果物 | `target/release-playbook/` 以下の bootstrap / smoke 出力 |
| チェックサム | SHA-256 チェックサムファイル |

配布対象の supported product/release target 切り分けと命名規則は `release-distribution-signing.md` と `artifact-policy.md` を参照。

release workflow は事前生成済み bundle を URL + SHA-256 で受け取り、archive root の `program.native` / `manifest.json` をそれぞれ `NATIVE_ONLY_PROGRAM` / `NATIVE_ONLY_PROGRAM_MANIFEST` として `scripts/release.sh` に渡す。同時に実在する rollback archive を `ROLLBACK_COMPATIBILITY_ASSET_PATH` へ渡し、`scripts/ci/release-smoke.sh <stable-archive> <rollback-archive>` で照合する。展開済み stable archive 上では `program.native` / `manifest.json` / `native-program-manifest.json` / `README.md` / `LICENSE` / `checksums.txt`、manifest の `rollback_anchor`、packaged `lsharp` alias の `--version` / `check` / `fmt` / `compile` / `test` / `doc` smoke を通す。README / fresh-clone 側でも `scripts/smoke_test_readme.sh` が inline Quick Start fixture を使って checksum / compile / test / doc の導線を再確認する。

### 4. チェックサム生成

```bash
# scripts/checksum.sh が利用可能な場合
bash scripts/checksum.sh
```

全リリースアーティファクトに SHA-256 チェックサムを付与する。native-only archive と rollback compatibility asset を同じ release に添付する場合は、両方に個別チェックサムを付ける。

### 5. タグ作成と stable input の固定

```bash
git tag v<version>
git push origin v<version>
```

- タグ名は `v` プレフィックス付き（例: `v0.2.0`）
- タグはリリースコミットに対して作成する
- tag push だけでは stable workflow を起動しない。通常の release では `workflow_dispatch` も実行せず、入力 URL / SHA-256 は手元の manual release gate と手動 GitHub Release 公開のために固定する。legacy Actions fallback を使う例外時だけ次節の `workflow_dispatch` を明示実行する
- `aarch64-apple-darwin` は Mac Apple Silicon、`x86_64-unknown-linux-gnu` は Mac + Lima x86_64 VM で実 `App.Cli` を事前生成・実行検証する
- 各 target の input bundle は archive root に `program.native` と `manifest.json` を置く。manifest は `target` / `entry_module: App.Cli` / `source: src/App/Cli.ls` / `source_commit` / `program_sha256` を持つ。producer は clean worktree で実行し、未コミット bytes を `HEAD` provenance として公開しない
- `ROLLBACK_VERSION=v<version> bash scripts/ci/native-rollback-compat-local.sh` を Mac + Lima で実行し、両 target の実在する `lsharp-v<version>-<target>-host-launcher.tar.gz` を rollback input にする
- input bundle と rollback archive は runner から HTTPS download できる場所へ置き、それぞれの SHA-256 を publish 前に固定する

### 6. Legacy Actions fallback (`.github/workflows/release.yml`)

通常の release ではこの workflow を dispatch しない。Actions fallback を使う例外時だけ `enable_legacy_actions_release=true` を明示し、対象 tag の commit から `release_tag` と 2 target 分の `*_url` / `*_sha256` を入力する。workflow は以下の順で実行される:

| ジョブ | 内容 |
|------|------|
| `verify` | `cargo test` + `cargo clippy` + `cargo fmt --check` |
| `build` | 両 target の input bundle / rollback archive を download して入力 SHA-256 を検証し、`scripts/release.sh` と `scripts/ci/release-smoke.sh` へ渡す |
| `release-smoke` | macOS arm64 / Ubuntu x86_64 runner で各 target の stable archive と rollback archive を download し、Rust toolchain 無しで再実行 |
| `release` | `softprops/action-gh-release` で指定済み tag の GitHub Release を作成し、2 native-only archive、2 rollback archive、`dist/checksums.txt` だけを添付 |

- `release-smoke` job は各 archive を実行できる target-native runner に分ける
- 外部 input は download 中から 512 MiB の hard limit を適用し、失敗時に一時領域を削除する。展開前にも entry 数、圧縮/展開サイズ、path traversal、symlink/hardlink、regular-file whitelist を検証する
- `build` job の workflow-local artifact は native-only archive と対応する rollback compatibility archive のみを同梱する
- Linux x86_64 の heavy actual self-regeneration は Mac + Lima の事前生成 gate に閉じ、GitHub required CI / release job の `needs` には入れない
- representative build artifact と experimental RC/evidence artifact は stable inputにも GitHub Release asset にも使わない
- `release` job は build 済み archive を download した後、`bash scripts/checksum.sh dist > dist/checksums.txt` で release-level checksum asset を生成してから公開する
- バージョン文字列にハイフンが含まれる場合 (例: `v0.2.0-rc1`) はプレリリースとして公開
- `release_notes` は GitHub の自動生成を使用

### 7. 手動公開（現行の正本）

local manual release gate を通した後、GitHub Release を手動で作成する:

1. GitHub Releases ページで新規リリースを作成
2. タグ `v<version>` を選択
3. リリースノートを記載（変更点、破壊的変更、移行手順）
4. native-only archive をアップロード
5. rollback compatibility を同時公開する場合だけ host launcher archive / `lsharp-<version>-<target>.component.wasm` を添付
6. `dist/checksums.txt` を checksum asset として添付
7. `Rollback anchor` セクションに tag / asset 名 / checksum 名を記録

stable / nightly の扱い、署名順序、package manager 更新順は `release-distribution-signing.md` を参照。

### 8. Rollback anchor の記録

stable release を publish したら、同じ GitHub Release notes に以下の `Rollback anchor` セクションを追記する。

```text
Rollback anchor
- last-known-good release tag: v<version>
- native-only archive assets: <attached asset names>
- rollback compatibility assets: <attached asset names>
- checksum: <attached checksum file>
```

- asset 名は **実際に添付したファイル名** をそのまま書く。
- package manager package は二次配布なので anchor には含めない。
- rollback 手順はこの anchor を起点に `rollback-procedure.md` の B/C フローへ入る。

### 9. immutable App.Cli input の準備

Mac Apple Silicon producer と Mac + Lima producer は、最終的に target ごとの staging directoryへ `program.native` と検証済み `manifest.json` を出力する。stable workflow へ渡す bundle は次の形に固定する。

```bash
tar -C <target-staging-dir> -czf <target>-native-input-bundle.tar.gz program.native manifest.json
shasum -a 256 <target>-native-input-bundle.tar.gz
shasum -a 256 lsharp-v<version>-<target>-host-launcher.tar.gz
```

Mac producer は `scripts/ci/native-macos-aarch64-selfhost-release.sh`、Linux producer は `scripts/ci/native-linux-x86-hostgen-vm-exec.sh` の target-only `src/App/Cli.ls` export を使う。Linux の full self-regeneration は green な stage2/stage3 fixed-point evidence を先に得る operator gateであり、stable workflow はその heavy replay を再実行しない。

両 target の program / manifest / rollback archive が揃ったら、workflow input を公開する前に同じ入力を local gateへ渡す。

```bash
VERSION=v<version> \
MACOS_APP_CLI_ARTIFACT_DIR=<mac-staging-dir> \
LINUX_APP_CLI_ARTIFACT_DIR=<linux-staging-dir> \
MACOS_ROLLBACK_ARCHIVE=<mac-rollback-archive> \
LINUX_ROLLBACK_ARCHIVE=<linux-rollback-archive> \
  bash scripts/ci/native-official-release-local.sh
```

この gate は macOS archive を host上、Linux x86_64 archive を Lima VM 上で `scripts/release.sh` / `scripts/ci/release-smoke.sh` に通す。stable / rollback manifest の `target` / `version` / `source_commit` は recursive smoke で一致を確認する。GitHub workflow は同じ immutable inputs を再 package / smoke するが、heavy self-regeneration 自体は繰り返さない。

macOS payload を署名する場合は bundle 固定前に `program.native` を署名・verify し、署名後 bytes の `program_sha256` を manifest に記録する。workflow は immutable manifest/hash を壊す再署名をせず、secret がある場合に署名 verify と notarization submit を行う。

### 10. Native-only official replacement track

native-only を公式配布へ完全置換する作業のうち、V2-13 target matrix、V2-14 native-only official archive layout、V2-15 native-only release smoke / rollback anchor の基盤は完了済みである。native-only archive を stable / nightly の正本にし、host launcher + embedded guest component は rollback compatibility 用の互換成果物へ降格する。

Supported product/release targets は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) の 2 つに限る。macOS Intel (`x86_64-apple-darwin`)、Windows (`x86_64-pc-windows-msvc`)、Linux ARM (`aarch64-unknown-linux-gnu`) は out of support scope であり、Rosetta / Mach-O smoke、archived Authenticode design、Linux ARM cross-build の未完了を公式配布 blocker にしない。

V2-14 の native-only official archive layout は `program.native`、`manifest.json`、`checksums.txt`、README/LICENSE、target metadata を必須 payload とする。`program.native` は target native executable、`manifest.json` は target triple / archive schema version / source commit / native backend evidence / rollback compatibility asset 参照を持つ。現行の host launcher + embedded guest component archive と `lsharp.component.wasm` companion sidecar は stable 既定 payload ではなく rollback compatibility asset へ降格する。

#### Experimental native-only RC evidence

`scripts/ci/native-only-rc-smoke.sh` は experimental native-only RC の layout と evidence を確認する手動診断用 smoke である。stable release の local manual gate を置き換えず、RC artifact を通常の GitHub Release asset として公開しない。

current stable contract は以下である。

1. actual native self-regeneration evidence は `aarch64-apple-darwin` と Linux x86_64 server priority track で保持する。Linux x86_64 の full replay は Mac + Lima VM の local operator gate であり、required GitHub Actions job にはしない。
2. 両 supported target の stable input は representative artifact ではなく、実 `App.Cli` の `program.native` + manifest bundle と実在 rollback compatibility archive に固定する。
3. `.github/workflows/release.yml` は input hashを検証した後に `scripts/release.sh` / `scripts/ci/release-smoke.sh` へ渡し、2 target の stable assetだけを publish する。

この track を再開する場合は、V2-13 target matrix の正本である `docs/language/native-backend-spec.md` を確認し、target-specific blocker を解消したうえで native-only official archive layout / release smoke / rollback anchor を `release-distribution-signing.md` と workflow に同期する。

## ロールバック

リリース後に致命的問題が発見された場合:

1. 該当リリースを GitHub Releases で `pre-release` に変更
2. 修正版を緊急リリース（パッチバージョン）
3. 必要に応じて `docs/development/operations/rollback-procedure.md` に従い、直前の正常な host launcher / guest component 組へ巻き戻す

詳細は `docs/development/operations/rollback-procedure.md` を参照。

## 証跡

- `scripts/release-playbook.sh`
- `scripts/release.sh`
- `scripts/checksum.sh`
- `scripts/smoke_test_readme.sh`
- `.github/workflows/release.yml`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops06_release_playbook`)

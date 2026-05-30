# リリースプレイブック

L# の **手元実行手順** を定義する。配布チャネル、tier1/tier2、署名、package manager 方針の正本は [`release-distribution-signing.md`](./release-distribution-signing.md)。このページは自動化スクリプト `scripts/release-playbook.sh` と並走するオペレーター向け runbook に絞る。配布モデルは **Wasmtime embedding + guest Wasm component + host launcher single binary** を前提とする。

## 概要

```
バージョンバンプ → CI 検証 → host launcher / component package 生成 → チェックサム → タグ作成 → GitHub Release
```

- channel / target matrix は `release-distribution-signing.md`
- artifact retention は `artifact-policy.md`
- CI gate は `ci-gate-v2-job-graph.md`

## 手順

### 1. バージョンバンプ

```bash
# Cargo.toml のバージョンを更新
# workspace 全体で統一バージョンを使用
vim Cargo.toml   # version = "0.x.y"
```

- `Cargo.toml` の `[workspace.package]` セクションで一元管理
- セマンティックバージョニングに従う

### 2. CI 検証

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
| 5 | `LSHARP_BIN=target/release/lsharp bash scripts/ci/compile-phase11-inputs.sh` | release host launcher で selfhost / stdlib / examples の固定入力セットを検証 |
| 6 | `LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh` + `scripts/smoke_test_readme.sh` | release host launcher + guest component の smoke + README smoke |
| 7 | `bash scripts/ci/release-smoke.sh dist/lsharp-<version>-<target>.<ext>` | 生成済み release archive を展開し、checksum 検証と packaged binary smoke を行う |
| 8 | チェックサム生成 | `scripts/checksum.sh` |

### 3. アーティファクト生成

リリースビルド成果物:

| アーティファクト | 説明 |
|---|---|
| `lsharp` host launcher | `target/release/lsharp` |
| `lsharp-lsp` language server | `target/release/lsharp-lsp` |
| guest component sidecar | `dist/lsharp-<version>-<target>.component.wasm`（archive 内には `lsharp.component.wasm` として同梱） |
| release playbook 検証成果物 | `target/release-playbook/` 以下の bootstrap / smoke 出力 |
| チェックサム | SHA-256 チェックサムファイル |

配布対象の tier1 / tier2 切り分けと命名規則は `release-distribution-signing.md` と `artifact-policy.md` を参照。

release workflow では `scripts/release.sh` の直後に `scripts/ci/release-smoke.sh dist/lsharp-<version>-<target>.<ext>` を実行し、展開済み archive 上で `README.md` / `LICENSE` / `checksums.txt` / `lsharp-lsp` / `lsharp.component.wasm` の存在確認、`checksums.txt` 検証、packaged `lsharp` binary の `--version` / `check` / `fmt` / `compile` / `test` / `doc` smoke を通す。README / fresh-clone 側でも `scripts/smoke_test_readme.sh` が inline Quick Start fixture を使って checksum / compile / test / doc の導線を再確認し、host-backed `doc` distribution ownership を二重化して確認する。

### 4. チェックサム生成

```bash
# scripts/checksum.sh が利用可能な場合
bash scripts/checksum.sh
```

全リリースアーティファクトに SHA-256 チェックサムを付与する。single-binary host launcher と sidecar component を併売する場合は、両方に個別チェックサムを付ける。

### 5. タグ作成と自動リリース

```bash
git tag v<version>
git push origin v<version>
```

- タグ名は `v` プレフィックス付き（例: `v0.2.0`）
- タグはリリースコミットに対して作成する
- `v*` タグの push により `.github/workflows/release.yml` が自動起動する

### 6. 自動リリース workflow (`.github/workflows/release.yml`)

`v*` タグを push すると以下の順で自動実行される:

| ジョブ | 内容 |
|------|------|
| `verify` | `cargo test` + `cargo clippy` + `cargo fmt --check` |
| `build` | Tier1 の 4 プラットフォームで `cargo build --release` + `scripts/release.sh` で host launcher archive / guest component sidecar を作成 |
| `release-smoke` | Ubuntu 上で Linux x86_64 archive (`lsharp-{version}-x86_64-unknown-linux-gnu.tar.gz`) を download し、`scripts/ci/release-smoke.sh` を Rust toolchain 無しで再実行 |
| `release` | `softprops/action-gh-release` で GitHub Release を作成し、全 archive / sidecar component / `dist/checksums.txt` を添付 |

- `release-smoke` job は Ubuntu 上で実行可能な downloaded artifact に絞るため、Linux x86_64 archive を 1 本だけ再検証する
- `build` job の workflow-local artifact には host launcher archive と companion sidecar `lsharp-{version}-{target}.component.wasm` を同梱する
- `release` job は build 済み archive を download した後、`bash scripts/checksum.sh dist > dist/checksums.txt` で release-level checksum asset を生成してから公開する
- バージョン文字列にハイフンが含まれる場合 (例: `v0.2.0-rc1`) はプレリリースとして公開
- `release_notes` は GitHub の自動生成を使用

### 7. Rollback anchor の記録

stable release を publish したら、同じ GitHub Release notes に以下の `Rollback anchor` セクションを追記する。

```text
Rollback anchor
- last-known-good release tag: v<version>
- host launcher assets: <attached asset names>
- guest component assets: lsharp-<version>-<target>.component.wasm
- checksum: <attached checksum file>
```

- asset 名は **実際に添付したファイル名** をそのまま書く。
- package manager package は二次配布なので anchor には含めない。
- rollback 手順はこの anchor を起点に `rollback-procedure.md` の B/C フローへ入る。

#### 手動公開が必要な場合のみ

自動 workflow を使わず手動で GitHub Release を作成する場合:

1. GitHub Releases ページで新規リリースを作成
2. タグ `v<version>` を選択
3. リリースノートを記載（変更点、破壊的変更、移行手順）
4. アーティファクトをアップロード
5. `lsharp-<version>-<target>.component.wasm` を guest component asset として添付
6. `dist/checksums.txt` を checksum asset として添付
7. `Rollback anchor` セクションに tag / asset 名 / checksum 名を記録

stable / nightly の扱い、署名順序、package manager 更新順は `release-distribution-signing.md` を参照。

### 8. experimental native-only RC（公式配布外）

native-only RC は stable / nightly の host launcher + embedded guest component 配布を置き換えない。Darwin arm64 の actual native self-regeneration artifact を調査用に固める experimental channel としてのみ扱う。

```bash
NATIVE_PROXY_ARTIFACT_ID=<version>-aarch64-apple-darwin bash scripts/ci/build-native.sh
bash scripts/ci/native-only-rc-smoke.sh ci-artifacts/native-proxy/<version>-aarch64-apple-darwin
tar -C ci-artifacts/native-proxy -czf dist/experimental-native-rc-<version>-aarch64-apple-darwin.tar.gz <version>-aarch64-apple-darwin
bash scripts/checksum.sh dist > dist/checksums.txt
```

experimental archive には top-level `manifest.json` / `actual-stage23-gap.json` と、`stage1-native` / `stage2-native` / `stage3-native` の `program.o`, `runtime.o`, `linker-response.txt`, `program.native`, `stdout.txt`, `stderr.txt`, `summary.json` を含める。release notes には `experimental native-only RC`、`host launcher + embedded guest component distribution を置き換えない`、`scripts/ci/native-only-rc-smoke.sh` の結果を明記する。

### 9. Native-only official replacement track

native-only を公式配布へ完全置換する作業のうち、V2-13 target matrix、V2-14 native-only official archive layout、V2-15 native-only release smoke / rollback anchor は完了済みである。native-only archive を stable / nightly の正本にし、host launcher + embedded guest component は rollback compatibility 用の互換成果物へ降格する。

V2-14 の native-only official archive layout は `program.native`、`manifest.json`、`checksums.txt`、README/LICENSE、target metadata を必須 payload とする。`program.native` は target native executable、`manifest.json` は target triple / archive schema version / source commit / native backend evidence / rollback compatibility asset 参照を持つ。現行の host launcher + embedded guest component archive と `lsharp.component.wasm` companion sidecar は stable 既定 payload ではなく rollback compatibility asset へ降格する。

現時点で残る target-specific blocker は以下である。

1. actual native self-regeneration は `aarch64-apple-darwin` と Linux x86_64 server priority track で完了している。Linux x86_64 は tag release 前に Mac + Lima VM 上で `NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=<release-id> scripts/ci/native-linux-x86-selfregen.sh` を実行し、stage2/stage3 compare 証跡を local artifact として残す。`x86_64-apple-darwin` / `x86_64-pc-windows-msvc` の Tier1 official gate は未完了。
2. `x86_64-pc-windows-msvc` は native backend spec 上 BLOCKED で、COFF/PE runtime/link/smoke と Authenticode gate が必要。
3. `scripts/release.sh` / `scripts/ci/release-smoke.sh` / `.github/workflows/release.yml` は native-only official archive layout を stable release path として扱う。host launcher + embedded guest component は rollback compatibility asset としてのみ扱う。

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

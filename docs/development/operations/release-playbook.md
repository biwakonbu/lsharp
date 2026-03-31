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
| guest component package | `target/release-playbook/` 以下の component / bundle 出力（埋め込み用または検証用 sidecar） |
| release playbook 検証成果物 | `target/release-playbook/` 以下の bootstrap / smoke 出力 |
| チェックサム | SHA-256 チェックサムファイル |

配布対象の tier1 / tier2 切り分けと命名規則は `release-distribution-signing.md` と `artifact-policy.md` を参照。

release workflow では `scripts/release.sh` の直後に `scripts/ci/release-smoke.sh dist/lsharp-<version>-<target>.<ext>` を実行し、展開済み archive 上で `checksums.txt` 検証と packaged `lsharp` binary の `--version` / `check` / `fmt` / `compile` smoke を通す。

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
| `build` | Tier1 の 4 プラットフォームで `cargo build --release` + `scripts/release.sh` で host launcher archive / component package を作成 |
| `release` | `softprops/action-gh-release` で GitHub Release を作成し、全アーティファクトを添付 |

- バージョン文字列にハイフンが含まれる場合 (例: `v0.2.0-rc1`) はプレリリースとして公開
- `release_notes` は GitHub の自動生成を使用

### 7. Rollback anchor の記録

stable release を publish したら、同じ GitHub Release notes に以下の `Rollback anchor` セクションを追記する。

```text
Rollback anchor
- last-known-good release tag: v<version>
- host launcher assets: <attached asset names>
- guest component assets: <attached asset names>
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
5. チェックサムファイルを添付
6. `Rollback anchor` セクションに tag / asset 名 / checksum 名を記録

stable / nightly の扱い、署名順序、package manager 更新順は `release-distribution-signing.md` を参照。

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

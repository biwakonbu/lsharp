# CI / ブランチ保護設定

## ワークフロー概要

`.github/workflows/ci.yml` で以下のジョブが PR および main push 時に実行される:

| ジョブ | 内容 |
|--------|------|
| Test | `cargo test` -- 全テスト実行 |
| Lint (clippy) | `cargo clippy -- -D warnings` |
| Format (rustfmt) | `cargo fmt --check` |
| Bootstrap (selfhost) | `bash scripts/ci/compile-phase11-inputs.sh` -- selfhost / stdlib / examples の固定入力セットを `lsharp` バイナリ経路で検証 |
| Default path (lsharp binary) | `bash scripts/ci/default-path-smoke.sh` -- `cargo run` ではなくビルド済み `lsharp` で `check` / `compile` を検証 |
| Fresh clone smoke | `bash scripts/ci/test-fresh-clone.sh` -- clean checkout 相当のコピーで `lsharp` を再ビルドし、代表的な selfhost / stdlib compile を検証 |
| Audit Docs | `bash scripts/audit_docs.sh` |
| Shadow Oracle (differential test) | selfhost と Rust 経路の差分比較（非ブロッキング） |
| CI Gate | 必須 7 ジョブの成功を集約 |
| CI Gate v2 | `CI Gate` 相当 + `shadow-oracle` を集約 |

## ブランチ保護ルールの設定手順

GitHub リポジトリの Settings から以下を設定する:

1. **Settings** > **Branches** > **Add branch protection rule**
2. **Branch name pattern**: `main`
3. 以下を有効化:
   - **Require a pull request before merging**
   - **Require status checks to pass before merging**
     - **Status checks that are required**: `CI Gate` を検索して追加
   - **Require branches to be up to date before merging** (推奨)

`CI Gate` は test / lint / format / bootstrap / audit-docs / default-path-smoke / fresh-clone-smoke の成功を要求するため、これ1つを required にすれば必須チェックがカバーされる。

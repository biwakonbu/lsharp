# CI / ブランチ保護設定

## ワークフロー概要

`.github/workflows/ci.yml` で以下のジョブが PR および main push 時に実行される:

| ジョブ | 内容 |
|--------|------|
| Test | `cargo test` -- 全テスト実行 |
| Lint (clippy) | `cargo clippy -- -D warnings` |
| Format (rustfmt) | `cargo fmt --check` |
| CI Gate | 上記3ジョブの成功を集約 |

## ブランチ保護ルールの設定手順

GitHub リポジトリの Settings から以下を設定する:

1. **Settings** > **Branches** > **Add branch protection rule**
2. **Branch name pattern**: `main`
3. 以下を有効化:
   - **Require a pull request before merging**
   - **Require status checks to pass before merging**
     - **Status checks that are required**: `CI Gate` を検索して追加
   - **Require branches to be up to date before merging** (推奨)

`CI Gate` は test / lint / format 全ジョブの成功を要求するため、これ1つを required にすれば全チェックがカバーされる。

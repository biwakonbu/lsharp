# CI / ブランチ保護設定

CI の **ジョブ構成と依存関係の正本** は [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md)。このページは、branch protection に必要な最小限の運用メモだけを残す。

## 参照先

- ジョブ一覧 / `needs` / required checks: [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md)
- release / distribution / signing / tier1/tier2: [`release-distribution-signing.md`](./release-distribution-signing.md)
- artifact retention: [`artifact-policy.md`](./artifact-policy.md)

## ブランチ保護ルールの設定手順

GitHub リポジトリの Settings から以下を設定する:

1. **Settings** > **Branches** > **Add branch protection rule**
2. **Branch name pattern**: `main`
3. 以下を有効化:
   - **Require a pull request before merging**
   - **Require status checks to pass before merging**
      - **Status checks that are required**: Actions 表示名 `CI Gate v2` を検索して追加（workflow job id は `ci-gate-v2`）
    - **Require branches to be up to date before merging** (推奨)

required check の UI 表示名 (`CI Gate v2`) と workflow 上の job id (`ci-gate-v2`) の対応、および blocking ジョブ集合は `ci-gate-v2-job-graph.md` に合わせる。job 名を変更した場合、このページではなく正本側を先に更新する。

# CI / ブランチ保護設定

> **Temporary policy (2026-07-12): CI 自動実行は停止**。`.github/workflows/ci.yml` は `workflow_dispatch` だけを許可し、`push` / `pull_request` では build、test、artifact upload を実行しない。通常の release は Mac + Lima VM の手元 gate と手動 GitHub Release 公開で行う。

CI の **ジョブ構成と依存関係の正本** は [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md)。ジョブ graph は明示的な診断 dispatch と将来の CI 再開用に保持する。

## 参照先

- ジョブ一覧 / `needs` / required checks: [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md)
- release / distribution / signing / tier1/tier2: [`release-distribution-signing.md`](./release-distribution-signing.md)
- artifact retention: [`artifact-policy.md`](./artifact-policy.md)

## 停止中のブランチ保護

GitHub リポジトリの Settings では、既存の PR review / linear history / force push 禁止などの保護方針は維持しつつ、CI に依存する設定だけを外す:

1. **Settings** > **Branches** で `main` の rule を開く。
2. **Require status checks to pass before merging** を無効にするか、required status checks から `CI Gate v2` を外す。
3. CI 停止中は CI Gate v2 を required 設定しない。自動 workflow が起動しないため、required のままだと PR が不必要に block される。
4. release 前に [`release-playbook.md`](./release-playbook.md) の `scripts/ci/native-official-release-local.sh` を Mac + Lima VM で通し、成功した immutable input だけを手動公開する。

CI を再開する場合は、`push` / `pull_request` trigger を戻し、green run を確認してから `CI Gate v2` を required check に再登録する。required check の UI 表示名 (`CI Gate v2`) と workflow 上の job id (`ci-gate-v2`) の対応、および blocking ジョブ集合は `ci-gate-v2-job-graph.md` に合わせる。

# Branch protection / required checks（人手作業チェックリスト）

Phase 11 の `CP-06`（CI / release cutover）で、GitHub 上の **Branch protection** と **required status checks** を更新するときの確認用です。リポジトリ設定は API/UI の両方があり得るため、PR マージ前にここを踏んでください。

> **Temporary policy (2026-07-12): CI 自動実行は停止**。通常 release の正本は手元の manual release gate であり、GitHub Actions の `CI Gate v2` は required check にしない。

## CI 停止中の確認

- [ ] `main`（およびリリース対象ブランチ）で **Require status checks to pass** が無効、または required status checks から `CI Gate v2` を削除済み
- [ ] CI Gate v2 を required にしない。`.github/workflows/ci.yml` は `workflow_dispatch` だけを許可するため、required のままでは PR を block する
- [ ] 手元の manual release gate (`scripts/ci/native-official-release-local.sh`) が Mac Apple Silicon と Lima Linux x86_64 VM で green
- [ ] 追加した E2E や `lsharp-wasm` のフィルタテスト名が、仕様書・`TODO.md` の証跡と一致している
- [ ] リリース artifact 名が `docs/development/operations/artifact-policy.md` と一致
- [ ] **Require branches to be up to date** を方針に合わせて有効/無効のいずれかで統一

> 2026-04-02 の `CI Gate v2` required check 設定は、2026-07-12 からの停止方針では解除対象である。linear history / conversation resolution / no force pushes / no deletions などの CI 非依存の保護を変更する必要はない。

## CI 再開時の確認

- [ ] `cargo test` / `cargo clippy` がローカルと対象 PR で green
- [ ] `scripts/ci/test-fresh-clone.sh`（または同等の no-Rust / fresh-clone ジョブ）がワークフロー定義と手順書で同じパスを指している
- [ ] required check の対応表が `.github/workflows/ci.yml` / `CI.md` / `ci-gate-v2-job-graph.md` / このチェックリストで一致している（job id: `ci-gate-v2`, Actions 表示名: `CI Gate v2`）
- [ ] **Status checks that are required** に Actions 表示名 `CI Gate v2` を追加するのは、`push` / `pull_request` trigger の復帰と green run の確認後だけにする

## 署名・タグ

- [ ] リリースタグ向けの署名方針（例: signed tags）が `release.yml` / playbook と一致

## 参照

- `docs/development/planning/completion-criteria.md`（ゲート条件）
- `docs/development/operations/ci-gate-v2-job-graph.md`
- `docs/development/operations/artifact-policy.md`

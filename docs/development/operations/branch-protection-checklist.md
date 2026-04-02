# Branch protection / required checks（人手作業チェックリスト）

Phase 11 の `CP-06`（CI / release cutover）で、GitHub 上の **Branch protection** と **required status checks** を更新するときの確認用です。リポジトリ設定は API だけでは完結しないため、PR マージ前にここを踏んでください。

## Required checks に載せる前

- [ ] `cargo test` / `cargo clippy` がローカルまたは該当 PR で green
- [ ] 追加した E2E や `lsharp-wasm` のフィルタテスト名が、仕様書・`TODO.md` の証跡と一致している
- [ ] `scripts/ci/test-fresh-clone.sh`（または同等の no-Rust / fresh-clone ジョブ）がワークフロー定義と手順書で同じパスを指している
- [ ] リリース artifact 名が `docs/development/operations/artifact-policy.md` と一致
- [ ] required check の対応表が `.github/workflows/ci.yml` / `CI.md` / `ci-gate-v2-job-graph.md` / このチェックリストで一致している（job id: `ci-gate-v2`, Actions 表示名: `CI Gate v2`）

## Branch protection（GitHub）

- [ ] `main`（およびリリース対象ブランチ）で **Require status checks to pass** が有効
- [ ] **Status checks that are required** に Actions 表示名 `CI Gate v2` が含まれる（workflow job id は `ci-gate-v2`）
- [ ] required check の表示名 / job id の対応が `docs/development/operations/CI.md` と一致している
- [ ] **Require branches to be up to date** を方針に合わせて有効/無効のいずれかで統一

## 署名・タグ

- [ ] リリースタグ向けの署名方針（例: signed tags）が `release.yml` / playbook と一致

## 参照

- `docs/development/planning/completion-criteria.md`（ゲート条件）
- `docs/development/operations/ci-gate-v2-job-graph.md`
- `docs/development/operations/artifact-policy.md`

# ADR: v0.2 native validation evidence-edge registry precedence

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source/source_edges.rs`, `crates/lsharp-types/tests/validation_source/edges.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`, `scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-review-edge-evidence-registry.md`

## Context

`supports` と `contradicts` は evidence registry に登録された observation だけを参照できる。
selfhost `IntentSource` は registry closure を wire ID の stable-ID 検証より先に判定し、未登録
なら code `6` を返す。一方、Rust source adapter は `EvidenceId::parse` を先に実行していたため、
`evidence:checkout` のように `/` を欠く未登録 ID では code `2` の edge-ID error になっていた。
同じ入力で Rust と selfhost/native の failure boundary が異なり、registry の fail-closed policy が
wire shape によって変質していた。

## Decision

- `supports` / `contradicts` は raw wire value を evidence registry と照合し、未登録なら
  `EvidenceRegistryRequired` (code `6`) を返す。
- registry に登録されている場合だけ typed `EvidenceId` を parse し、既存の endpoint/node
  validation と edge construction を継続する。
- 未登録 evidence の `evidence:checkout` fixtureを両 relation で Rust oracle、selfhost actual
  Wasm、native source-file smoke に固定する。exit `1`、stdout reportなし、manifestなしを維持する。

## Evidence

- RED: `cargo test -p lsharp-types --test validation_source source_adapter_reports_unregistered_evidence_before_invalid_edge_id -- --nocapture` は Rust が `EdgeIdAt` を返して失敗した。
- Rust GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture` は 50 tests passed。
- Rust-host selfhost GREEN:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_intent_source_adapter::test_e2e_selfhost_source_adapter_reports_evidence_registry_before_invalid_edge_id -- --exact --nocapture` が passed。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` が fake
  Lima/provenance harness で passedし、`supports` / `contradicts` の code `6`、no-report/no-manifest
  を検査した。
- `bash scripts/audit_docs.sh`、`bash -n`、`git diff --check` は task-owned worktree で passed。
  `cargo fmt --all -- --check` も実行したが、origin/main から引き継いだ unrelated files の既存整形差分で
  red のままであり、共有される別タスクの変更を巻き戻さないため全体整形は行っていない。

## Boundary and follow-up

この slice は evidence registry precedence と両 edge relation の入力拒否境界だけを閉じる。
current source-commit に一致する packaged stage0 artifact/runtime、Mac/Linux runtime parity、
native fallback exclusion、EC-M2-02 全体の完了は証明しない。`EC-M2-01`〜`EC-M2-03` は `[~]` を維持する。

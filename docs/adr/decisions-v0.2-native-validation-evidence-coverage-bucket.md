# ADR: v0.2 native validation evidence coverage bucket

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source/source_evidence.rs`, `crates/lsharp-types/tests/validation_source/evidence.rs`, `selfhost/src/Tools/Validation/Evidence.ls`, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-selfhost-evidence-registry.md`

## Context

source `:evidence` の `:coverage` は canonical `SamplingPlan` へ投影される optional field だが、
Rust source adapter は bucket 名の空文字を検査せず、`BTreeMap` の `{"": count}` として graph に
登録していた。selfhost Evidence consumer は empty bucket を empty-field code `4` で拒否しており、
同じ source record が入口によって成功／失敗に分かれていた。また selfhost の coverage validation
error は malformed／empty／negative／duplicate の場合に form span を失っていた。

## Decision

- Rust source adapter は `SamplingPlan::new` へ投影する前に coverage bucket を検査し、空文字を
  `SourceGraphError::InvalidEvidenceField { field: "coverage", value, span }` として evidence
  directive span 付きで fail-closed にする。
- selfhost `source-evidence-coverage-valid-loop` は form の start/end を受け取り、coverage の
  malformed、empty、negative、duplicate error へ同じ directive span を設定する。
- parser が検出する duplicate `:coverage` field／bucket の syntax boundary はこの adapter
  validation と分離し、今回の sliceでは変更しない。

## Evidence

- RED: `source_adapter_rejects_empty_sampling_coverage_bucket_with_directive_span` は、修正前に
  empty bucket を含む graph が成功し `coverage: {"": 1}` を保持した。
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`（54 passed）。
- Selfhost actual Wasm: `selfhost_evidence_registry`（39 passed）と
  `selfhost_intent_source_adapter`（30 passed）。source parser 経路は empty bucket の code `4` と
  source span `80..124`、registry の malformed coverage は span `10..20` を返す。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` が
  通過し、既存 Linux x86_64 source-file fail-closed／provenance gate を維持した。
- touched Rust の `rustfmt --edition 2021 --check`、`git diff --check`、docs audit が対象 gate として残る。

## Boundary and follow-up

これは source adapter と selfhost consumer の empty coverage bucket／diagnostic span parity に
限定した verified partial sliceである。whitespace-only bucket の policy、coverage count の
canonical policy、parser／manifest／`validate` CLI 全体、current-source stage0 artifact/runtime、
Mac Apple Silicon と Linux x86_64 の supported matrix、EC-M2-02 aggregate は未完了であり、TODO の
`[~]` を維持する。

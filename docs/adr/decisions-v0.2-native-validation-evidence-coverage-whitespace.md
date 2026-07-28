# ADR: v0.2 native validation evidence coverage whitespace policy

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/evidence.rs`, `crates/lsharp-types/src/validation_source/source_evidence.rs`, `selfhost/src/Tools/Validation/Evidence.ls`, coverage validation tests, native source-file smoke
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-bucket.md`、`docs/adr/decisions-v0.2-native-validation-evidence-canonical-sampling.md`

## Context

空文字の coverage bucket は前の slice で拒否していたが、`"  "` のような whitespace-only
bucket は canonical `SamplingPlan`、Rust source adapter、selfhost Evidence consumer の入口で
扱いが揃っていなかった。これを許すと同じ evidence record が入口ごとに graph 登録の成否と
診断内容を変える。

## Decision

- canonical `SamplingPlan::validate_required_fields` は `trim().is_empty()` の coverage bucket を
  `EvidenceValidationError::EmptyField { field: "coverage" }` として登録前に拒否する。
- Rust source adapter は同じ non-blank policy を `InvalidEvidenceField { field: "coverage" }` に
  適用し、元の bucket value と evidence directive span を保持する。
- selfhost `source-evidence-coverage-valid-loop` は同じ non-blank policy を使い、code `4`、field、
  元の bucket value、form span を返す。空文字 bucket の既存出力は変更しない。
- native source-file smoke は whitespace-only coverage fixtureで exit `1`、stderr の
  `source validation error:4`、report/manifestなしを要求する。

## Evidence

- RED: canonical sampling、canonical graph、Rust source adapter、selfhost Evidence registry、
  selfhost source adapter の各 whitespace-only fixtureは修正前に bucket を成功扱いした。
- GREEN: `cargo test -p lsharp-types --test evidence_required_fields -- --nocapture`（8 passed）、
  `cargo test -p lsharp-types --test evidence_graph -- --nocapture`（5 passed）、
  `cargo test -p lsharp-types --test validation_source -- --nocapture`（55 passed）。
- Selfhost actual Wasm: `selfhost_evidence_registry`（40 passed）と
  `selfhost_intent_source_adapter`（31 passed）。code `4`、field `coverage`、raw value、span を確認した。
- Native: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` が
  Linux x86_64 source-file smoke と provenance gate を通過した。

## Boundary and follow-up

この判断は coverage bucket の blank policy に限定した verified partial sliceである。Unicode whitespace
の target 間差異、coverage count/cases の意味論、duplicate bucket の parser/manifest policy、
`validate` 全体、current-source artifact/runtime、Mac/Linux supported matrix、EC-M2-02 aggregate は
未完了であり、TODO の `[~]` を維持する。

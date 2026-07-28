# ADR: v0.2 native validation evidence required-field span

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_evidence.rs`, `crates/lsharp-types/tests/validation_source/evidence.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-review-error-span.md`

## Context

source evidence の required field が空または whitespace-only の場合、Rust adapter は canonical
`GraphError::InvalidEvidence` に直接落としていた。そのため selfhost の stable code `4` と
directive span に対応する source-local diagnostic がなく、required-field の入力拒否と
invalid enum/subject field（code `8` 相当）を source error で区別できなかった。

## Decision

- `SourceGraphError::InvalidEvidenceRequiredField` を追加し、field、value、evidence directive span
  を保持する。
- `runner`、`target`、`source_commit`、`artifact_digest`、`generator`、`producer`、
  `tool_version`、`timestamp` の required-field 検査をこの variant へ投影する。
- required-field の precedence は evidence ID parse より先に保ち、既存の
  `InvalidEvidenceField` は method/outcome/independence/subject の invalid field 用に残す。

## Evidence

- RED: source test が `InvalidEvidenceRequiredField` を要求し、variant 未実装で compile error になった。
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`（52 tests）。
- Selfhost actual Wasm: empty runner の stable code `4`、field、directive span E2E が通過した。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  が通過し、required evidence fixture の code `4`、exit `1`、no-report/no-manifest boundary を維持した。
- Touched Rust files の `rustfmt --edition 2021 --check` が通過した。

## Boundary and follow-up

これは Rust source adapter の required-field diagnostic と既存 selfhost/native contract の verified
partial sliceである。driver の EmbeddedCli build は current `origin/main` の
`selfhost/src/Tools/Validation/Stale.ls` にある既存 `vector-push-single-rooted-v3` undefined error
で停止するため未検証とした。実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion、
provider authentication、evidence lifecycle、M2 aggregate completion は未完了であり、TODO の `[~]`
を維持する。

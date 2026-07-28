# ADR: v0.2 native validation evidence subject ID span

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_evidence.rs`, `crates/lsharp-types/tests/validation_source/evidence.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-evidence-id-span.md`

## Context

source evidence の `subject` ID が wire format／segment 規則に違反した場合、Rust adapter は
`SourceGraphError::EdgeId(StableIdError::...)` に直接落としていた。そのため selfhost Evidence
consumer の stable code `2`、field/value、directive span に対応する source-local diagnostic がなく、
subject ID failure を evidence directive の位置へ結び付けられなかった。

## Decision

- `SourceGraphError::EvidenceSubjectIdAt` を追加し、`StableIdError` と evidence directive span を保持する。
- `StableId`、`IntentId`、`ClaimId`、`ContractId` の subject parse failure をこの variant へ投影する。
- unsupported subject kind は既存 `InvalidEvidenceField`、未登録 node は `MissingNodeReference` として残し、
  malformed ID、kind policy、registry closure を別の診断責務として扱う。

## Evidence

- RED: source evidence test が `EvidenceSubjectIdAt` と directive span を要求し、variant 未実装で compile error になった。
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`（53 tests）。
- Selfhost actual Wasm: malformed/whitespace subject ID の stable code `2`、field/value、directive span E2E が通過した。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  が通過し、invalid subject ID の code `2`、exit `1`、no-report/no-manifest boundary を維持した。
- Touched Rust files の `rustfmt --edition 2021 --check` と `git diff --check` が通過した。

## Boundary and follow-up

これは Rust source adapter の evidence subject ID diagnostic と既存 selfhost/native contract の verified
partial sliceである。driver の EmbeddedCli build は current `origin/main` の
`selfhost/src/Tools/Validation/Stale.ls` にある既存 `vector-push-single-rooted-v3` undefined error
で停止するため未検証とした。実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion、
M2 aggregate completion は未完了であり、TODO の `[~]` を維持する。

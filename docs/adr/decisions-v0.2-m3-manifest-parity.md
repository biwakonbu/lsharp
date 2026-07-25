# ADR: v0.2 M3 canonical manifest parity slice

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry.rs`
- Related: `v0.2-milestone-03.md`, `decisions-v0.2-source-manifest-emission.md`

## Context

M2 の Rust serializer と selfhost serializer は、schema version、node、evidence、edge の
個別フィールドをそれぞれ検査していた。しかし、フィールド単位の assertion だけでは、
順序、span、sampling、provenance、edge endpoint を含む canonical wire value の差分を
検出できない。

## Decision

- 同じ source fixture を Rust `source_program_to_intent_graph` と selfhost
  `source-evidence-graph-from-program` に入力する。
- Rust `IntentGraph::to_manifest_json_value()` を oracle とし、selfhost の
  `validation-source-manifest-json` を JSON value として比較する。
- fixture は intent / claim / assumption / open-question、motivates / constrained-by /
  tested-by / supports、sampling、provenance を含める。
- このテストは Rust-host actual Wasm の selfhost execution を検証するが、native stage0、
  Linux x86_64、release artifact、runtime matrix の完了を意味しない。

## Evidence

- `test_e2e_selfhost_evidence_manifest_matches_rust_canonical_value` が全 wire value の一致を
  `serde_json::Value` equality で固定した。
- focused gate: `cargo test -p lsharp-wasm --test e2e selfhost_evidence_manifest_matches_rust_canonical_value -- --nocapture`
  （1 passed）。
- `rustfmt --edition 2024` と `git diff --check` を通過した。

## Boundary and follow-up

これは EC-M3-01 の Rust-host verified slice であり、M3 aggregate は未完了である。
次は `App.Cli` / `EmbeddedCli` の native stage0 parity、Mac/Linux artifact/runtime、
source fingerprint と fallback negative gate を別の RED として閉じる。

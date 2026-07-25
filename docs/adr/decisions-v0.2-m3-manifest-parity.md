# ADR: v0.2 M3 canonical manifest parity slice

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry.rs`, `crates/lsharp-driver/tests/validate_cli.rs`
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
  `validation-source-manifest-json` を JSON value として比較する。さらに
  `IntentGraph::to_manifest_json_string()` と selfhost の canonical bytes を比較し、
  field order の drift も検出する。
- fixture は intent / claim / assumption / open-question、motivates / constrained-by /
  tested-by / supports、sampling、provenance を含める。
- source input から出力した canonical manifest を manifest input として再検証し、validation
  report JSON と exit code (`pass=0`, `fail=1`, `unknown=2`) が source input と一致することを
  Rust CLI contract で固定する。
- このテストは Rust-host actual Wasm の selfhost execution を検証するが、native stage0、
  Linux x86_64、release artifact、runtime matrix の完了を意味しない。

## Evidence

- `test_e2e_selfhost_evidence_manifest_matches_rust_canonical_value` が全 wire value の一致を
  `serde_json::Value` equality と canonical JSON string equality で固定した。
- `validate_source_and_emitted_manifest_have_same_report_and_exit_code` が、同一 source fixture
  の source 入力と生成済み manifest 入力について report JSON equality と exit code equality を
  固定した。
- focused gates:
  - `cargo test -p lsharp-wasm --test e2e selfhost_evidence_manifest_matches_rust_canonical_value -- --nocapture`
    （1 passed）。
  - `cargo test -p lsharp-driver --test validate_cli validate_source_and_emitted_manifest_have_same_report_and_exit_code -- --nocapture`
    （1 passed）。
- 変更した Rust file は `rustfmt --edition 2024 --check crates/lsharp-driver/tests/validate_cli.rs`
  と `git diff --check` を通過した。workspace 全体の `cargo fmt --all -- --check` は、今回の
  差分外にある既存の formatting drift のため未通過である。

## Boundary and follow-up

これは EC-M3-01 の Rust-host verified slice であり、M3 aggregate は未完了である。source と
manifest の parity は Rust CLI の report boundary までで、native stage0 の同一 fixture実行を
まだ含まない。
次は `App.Cli` / `EmbeddedCli` の native stage0 parity、Mac/Linux artifact/runtime、
source fingerprint と fallback negative gate を別の RED として閉じる。

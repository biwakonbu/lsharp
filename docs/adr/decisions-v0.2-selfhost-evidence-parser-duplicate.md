# ADR: selfhost evidence parser の duplicate field fail-closed

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `selfhost/src/Syntax/Parser.ls` の `:evidence` metadata parser
- Related: `EC-M2-02`, `EC-M2-03`, `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

Rust の `:evidence` parser は同じ named field の重複を `LS0101` として拒否する。一方、
selfhost parser は field を payload の同じ slotへ上書きしており、重複した source が
valid evidence として registry に登録されていた。これは source producer parity の
failure boundary を曖昧にし、後続の manifest/validation へ誤った evidence を渡す。

## Decision

- selfhost parser は `:evidence` の 17 field slot に対応する presence mask を追跡する。
- 同じ field が二度現れた場合は、値を上書きした payload に内部 marker を追加して
  18 要素にし、既存 `Evidence.ls` の payload arity check で malformed (`error code 1`) として
  registry 前に fail-closed にする。
- 正常な payload は従来どおり 17 要素で、field value、optional sampling、directive span の
  wire shape は変更しない。
- Rust parser の duplicate contract を `lsharp-syntax` unit test と selfhost runtime E2E の
  同一 source shape で固定する。

## Evidence

- RED: selfhost E2E を先に追加し、duplicate `:subject` が `1\nvalid`（registry 登録成功）に
  なった。
- GREEN: presence mask と malformed marker 後、同じ E2E が `0\n1`（registry 前の拒否）になった。
- `cargo test -p lsharp-syntax --quiet` — 171 unit tests と integration suites が pass。
- `cargo test -p lsharp-wasm --test e2e selfhost_evidence_parser_contract -- --nocapture` — pass。
- `cargo test -p lsharp-wasm --test e2e selfhost_evidence_registry -- --nocapture` — 16 passed。
- `test_e2e_selfhost_cli_validate_source_json_reports_contradicting_evidence` — pass（293.49s）。
- `test_e2e_selfhost_parser_preserves_source_intent_metadata_forms` — pass。
- e2e 全体 clippy は今回触れていない `selfhost_native_stage_chain.rs` の 2件と
  `support.rs` の 1件で失敗したため、別作業の lint として修正しない。
- native stage0 / Linux VM gate はこの parser-only sliceでは実行していない。

## Consequences

duplicate named field が Rust/selfhost で同じ fail-closed 境界になり、valid evidence の
17-field wire shape は維持される。source `:evidence` の native stage0 parity、manifest
producer の両 supported target、generator policy の実行証跡は未完了であり、`EC-M2-02` /
`EC-M2-03` は `[~]` のまま継続する。

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

### 訂正 (2026-08-24、`--ignored` lane 全量 sweep)

直上の `test_e2e_selfhost_cli_validate_source_json_reports_contradicting_evidence` —
「pass（293.49s）」は**現在の実測と食い違う**。当時の観測なので原文は残す。

2026-08-24 の実測は FAILED (`crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs:15673`)。
落ちているのは `value["independent_reviews"]` の比較で `left: Number(0)` / `right: 1`。
**手前の 3 つ (exit code `1` / `status=fail` / `trace_gaps` 0 件) は通っており、
`contradicting_observations` は assert 到達前なので未検証**である。

すなわち「evidence report が出ない」のではなく、**`independent-review` record が
1 件も数えられていない**という限定的な食い違いである。本 ADR の Decision
(duplicate `:subject` の fail-closed 化) が覆ったことを意味しない。

**原因は未診断。** `ISSUES.md` の `I-75` と `TODO.md` の `SWEEP-UNCLASSIFIED-01` が
引き取っている。診断が付くまで、この Evidence 行を「正しい値」へ書き換えることはできない。
書き換えれば、まだ分かっていないことを分かったことにしてしまう。

#### 追記 (2026-08-28): 原因が付いた

上の段落が置いた条件 (「診断が付くまで書き換えない」) が満たされたので追記する。
**履歴として上の記述は残す。**

原因は本 ADR の Decision とは無関係だった。`App/Cli.ls:238-250` が独立 review gate に
`outcome=pass` の連言を持っており (`e37b9cd6` 2026-07-31)、fixture の `contradicted` は
そこで落ちる。**この gate は `docs/adr/decisions-v0.2-validation-independent-review-outcome.md`
(2026-07-29) が定めた契約どおりであり、陳腐化しているのは test の期待値の方である。**

したがって `left: Number(0)` は**正しい値**である。詳細と是正手順は `ISSUES.md` の `I-96` /
`TODO.md` の `VALIDATION-REVIEW-GATE-PARITY-01` が引き取った。

**この Evidence 行はまだ書き換えない。** `contradicting_observations` の assert は
依然として到達しておらず、`independent_reviews` を 0 へ直した後に何が起きるかは未実測である。
実測してから 1 度で直す。
- `test_e2e_selfhost_parser_preserves_source_intent_metadata_forms` — pass。
- e2e 全体 clippy は今回触れていない `selfhost_native_stage_chain.rs` の 2件と
  `support.rs` の 1件で失敗したため、別作業の lint として修正しない。
- native stage0 / Linux VM gate はこの parser-only sliceでは実行していない。

## Consequences

duplicate named field が Rust/selfhost で同じ fail-closed 境界になり、valid evidence の
17-field wire shape は維持される。source `:evidence` の native stage0 parity、manifest
producer の両 supported target、generator policy の実行証跡は未完了であり、`EC-M2-02` /
`EC-M2-03` は `[~]` のまま継続する。

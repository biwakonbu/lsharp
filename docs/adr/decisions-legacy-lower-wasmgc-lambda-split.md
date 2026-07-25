# ADR: lower WasmGC lambda helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-expr-helpers-split.md`

## Context

`lower/expr.rs` は巨大な `lower_expr` match と共に、WasmGC lambda の free-variable
filtering、captured environment の型・値構築、lifted function body の lowering、environment
call-ref type lookup を一つの module に抱えていた。これらは expression production 本体と
異なる補助責務で、親 module の変更衝突と review 単位を増やしていた。

## Decision

- `wasmgc_lambda_free_vars`、`lower_wasmgc_captured_lambda_value`、
  `wasmgc_env_call_ref_type` を `lower/expr/wasmgc_lambda.rs`（207 行）へ移動する。
- 親 `expr.rs` から呼ぶ helper だけを `pub(super)` とし、crate/public API は拡張せず、
  expression module 内の親子境界に限定する。
- GC environment field layout、function index arithmetic、lifted function body、free-variable
  filtering、typed call-ref lookup、既存の error/span semantics は変更しない。
- WasmGC lambda module seam test で free-variable filtering contract を固定する。

## Evidence

- RED: `wasmgc_lambda` module 未作成時は
  `wasmgc_lambda_module_preserves_free_variable_filtering` が `file not found for module
  wasmgc_lambda` で失敗。
- GREEN: seam test が captured free variable を保持することを確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（152 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは WasmGC lambda helper の責務分離だけを扱う。`lower_expr` 本体の全 production 分割、
lower 全体の semantics parity、native/runtime artifact、selfhost parity、I-01 / I-08 aggregate
の完了を意味しない。`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

# ADR: lower expression helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-type-helpers-split.md`

## Context

`lower/expr.rs` は巨大な `lower_expr` match と共に、二項演算子 emission、map key の
hash/root helper、user-call の root lifetime helper、WasmGC substring range guard を
一つの module に抱えていた。これらは expression production 本体とは異なる補助責務で、
parent module の変更衝突と review 単位を増やしていた。

## Decision

- `emit_binop`、map key/hash helper、root push/pop/temporary helper、root 判定、WasmGC
  range guard、static substring validation を `lower/expr/helpers.rs`（239 行）へ移す。
- `expr.rs` の parent `impl Lower` から呼ぶ helper だけを `pub(super)` とし、crate/public
  API を拡張せず expression module 内の親子境界に限定する。
- `helpers::validate_wasmgc_substring_static_range` は parent から明示 import し、既存の
  validation call site と error/span semantics を維持する。
- operator IR、root slot ordering、map string-key hashing、WasmGC invalid-range trap の
  semantics は変更しない。
- expr helper module seam test で binary operator emission contract を固定する。

## Evidence

- RED: `helpers` module 未作成時は
  `expr_helper_module_preserves_binop_emission_contract` が `file not found for module helpers`
  で失敗。
- GREEN: seam test が `+` の `I64Add` emission を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（151 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは expression lowering helper の責務分離だけを扱う。`lower_expr` 本体の全 production
分割、lower 全体の semantics parity、native/runtime artifact、selfhost parity、I-01 / I-08
aggregate の完了を意味しない。`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

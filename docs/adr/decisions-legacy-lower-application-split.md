# ADR: lower application production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-wasmgc-lambda-split.md`

## Context

`lower/expr.rs` の `Expr::App` production は、WasmGC lambda call、scalar/string builtins、
ref/vector、Map、root/runtime helper、trait/user/closure call を一つの巨大な match arm に
抱えていた。expression production 本体と異なる backend・runtime 責務が同居し、変更衝突と
review 単位を増やしていた。

## Decision

- `Expr::App` の WasmGC lambda special case と dispatch を
  `lower/expr/application.rs`（80 行）へ移動する。
- scalar/string builtins を `application_scalar.rs`（606 行）、ref/vector builtins を
  `application_ref_vector.rs`（414 行）、Map builtins を `application_map.rs`（451 行）、
  root/runtime/trait/user/closure calls を `application_calls.rs`（228 行）へ分割する。
- 各 helper は expression module 内だけで使う `pub(super)` method とし、既存の AST、IR、
  error/span、root lifetime、WasmGC call-ref semantics を保持する。
- application module seam test で binary operator application の IR emission を固定する。

## Evidence

- RED: `application` module 未作成時は
  `application_module_preserves_binary_operator_lowering` が `file not found for module
  application` と `lower_app` 未定義で失敗。
- GREEN: seam test が `(+ 1 2)` を `I64Add` へ lowering することを確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（153 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは `Expr::App` の責務分離だけを扱う。`lower_expr` 全 production の意味論 parity、
native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

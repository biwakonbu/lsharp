# ADR: lower lambda production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-application-split.md`

## Context

`lower/expr.rs` の `Expr::Lambda` production は、自由変数検出、WasmGC typed funcref
lifting、linear-memory closure layout、lifted body の lowering を一つの match arm に抱えて
いた。WasmGC と linear backend の異なる責務が expression production 本体に混在し、変更
衝突と review 単位を増やしていた。

## Decision

- `Expr::Lambda` の lowering を `lower/expr/lambda.rs`（228 行）の `lower_lambda` へ移動する。
- 親 `expr.rs` は expression span と lambda parameters/body を module helper へ渡すだけにし、
  AST/IR、function index、closure layout、error/span、WasmGC rejection semantics を維持する。
- lambda module seam test で non-capturing WasmGC lambda の `RefFunc` lifting contract を固定する。

## Evidence

- RED: `lambda` module 未作成時は `lambda_module_preserves_wasmgc_funcref_lifting` が
  `file not found for module lambda` と `lower_lambda` 未定義で失敗。
- GREEN: seam test が WasmGC lambda を lifted function と `RefFunc` へ変換することを確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（154 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは lambda production の責務分離だけを扱う。`lower_expr` 全 production の意味論 parity、
native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

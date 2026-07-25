# ADR: lower let production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-computation-split.md`

## Context

`lower/expr.rs` の `Expr::Let` production は、binding expression の lowering、WasmGC captured
lambda の special case、lambda function/environment type の選択、pattern binding、scoped
local の復元を一つの `lower_expr` match arm に抱えていた。binding lifetime と backend-specific
closure handling が他の expression production に混在し、変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Let` の lowering を `lower/expr/let_expr.rs`（106 行）の `lower_let` へ移動する。
- 親 `expr.rs` は bindings と body を helper へ渡すだけにし、captured lambda handling、
  `IrType` 選択、wildcard/unsupported pattern behavior、scoped local restore、error semantics を維持する。
- let module seam test で、binding value → `LocalSet` → body read の順序と scope 終了時の binding restore を固定する。

## Evidence

- RED: seam test は `lower_let` 未定義で `no method named lower_let` として失敗。
- GREEN: seam test が binding value、local set、body read の順序と scope 復元を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（158 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは let production の責務分離だけを扱う。全 pattern/backend parity、native/runtime artifact、
selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

# ADR: lower literal production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-var-split.md`

## Context

`lower/expr.rs` の `Expr::Lit` production は、scalar literal、WasmGC の String array、
linear-memory の String allocation を一つの match arm に抱えていた。backend-specific な
値表現と通常の expression dispatch が混在し、変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Lit` の lowering を `lower/expr/literal_expr.rs`（102 行）の `lower_lit` へ移動する。
- 親 `expr.rs` は source span と literal を helper へ渡すだけにし、Int/Float/Bool/Unit の命令、
  WasmGC `array.new_fixed`、linear-memory `__alloc`、String data/handle、error semantics を維持する。
- literal module seam tests で scalar、WasmGC String、linear-memory String の observable contract を固定する。

## Evidence

- RED: seam tests は `lower_lit` 未定義で `no method named lower_lit` として失敗。
- GREEN: seam tests が scalar、WasmGC String array、linear-memory String allocation boundary を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（167 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは literal production の責務分離だけを扱う。全 backend/native/runtime parity、selfhost parity、
I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

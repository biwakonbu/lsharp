# ADR: lower type helpers split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/mod.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-heap-helpers-split.md`

## Context

`lower/mod.rs` は `Lower` の orchestration と共に、型から IR 型への変換、型名抽出、
heap-like 判定、`TypeExpr` 変換を保持していた。state/context/heap helper の分離後も
型変換の責務が親 module に残り、型表現の変換規則を独立して確認しにくかった。

## Decision

- `type_to_ir`、`type_to_name`、`type_expr_to_name`、`is_heap_like_type_name`、
  `type_expr_to_ir` を `lower/type_helpers.rs`（59 行）へ移す。
- `type_to_ir` は親 `lower` module から public re-export し、既存の公開 API を維持する。
- 残りの helper は crate 内 re-export し、`Lower` の WasmGC-aware method と
  `decl` / `expr` / `state` の既存 call site を変更しない。
- `Int` / `Float` / `Bool` / `Unit` / `String` の IR 型、型名抽出、heap-like 判定の
  semantics は変更しない。
- type helper module seam test で代表的な変換・抽出・判定契約を固定する。

## Evidence

- RED: `type_helpers` module 未作成時は
  `type_helper_module_preserves_conversion_and_name_contracts` が `file not found for module type_helpers`
  で失敗。
- GREEN: seam test が型変換、型名抽出、`TypeExpr` 変換、heap-like 判定を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（150 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは lower の type helper 責務分離だけを扱う。型推論 semantics、lower 全体の parity、
native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

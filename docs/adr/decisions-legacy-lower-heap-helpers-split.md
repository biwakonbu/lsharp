# ADR: lower heap pointer helpers split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/mod.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-context-split.md`

## Context

`lower/mod.rs` は `Lower` の orchestration と共に、ヒープオブジェクトタグ定数、
タグ付きポインタの encode/decode、ヒープヘッダ書き込みの IR helper を保持していた。
state/context の分離後も値表現の helper が親 module に残り、runtime representation の
責務境界が曖昧だった。

## Decision

- ヒープタグ定数と pointer/header helper を `lower/heap_helpers.rs`（51 行）へ移す。
- `HEAP_TAG_*` 定数は親 `lower` module から public re-export し、既存の公開パスを維持する。
- `emit_tag_pointer` / `emit_untag_pointer` は crate 内 re-export、未使用の
  `emit_write_heap_header` は test-only re-export とし、production の import 契約と
  dead-code lint を両立する。
- タグ値、pointer encode/decode 命令列、header store の offset/順序 semantics は変更しない。
- heap helper module seam test で全タグ値と代表的な命令列を固定する。

## Evidence

- RED: `heap_helpers` module 未作成時は
  `heap_helper_module_preserves_pointer_and_tag_contract` が `file not found for module heap_helpers`
  で失敗。
- GREEN: seam test が tag/untag/header の 8 命令と全タグ値を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（149 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは lower の heap representation helper 責務分離だけを扱う。lower 全体の semantics
parity、native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。

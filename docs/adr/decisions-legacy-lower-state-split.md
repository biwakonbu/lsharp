# ADR: lower program-state preparation split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/mod.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-test-module-split.md`

## Context

`lower/mod.rs` は `Lower` の state reset、型推論結果の保存、record/ADT の GC 型登録、
import/user/trait/constructor の関数 index 登録、lower orchestration、`FuncCtx`、共通 IR
helper を一つの module に抱えていた。既存の expression/pattern/declaration split に続き、
program-state preparation を独立した review 単位にする。

## Decision

- `reset_state` と `prepare_program_state` を `lower/state.rs`（335 行）へ移す。
- state module から `Lower` の fields と既存 helper (`unwrap_private`、type conversion、
  WasmGC field resolution) を利用し、`lower_program*` の公開 API と内部 state semantics は
 変更しない。
- import index、user/field/ADT/trait/computation-builder registration、GC type ordering、
  reset/reuse semantics、WasmGC unsupported diagnostics を維持する。
- seam test で空 program の builtin import registration を固定し、lower focused suite で
  lowering behavior parity を確認する。

## Evidence

- RED: `state` module 未作成時は `state_module_exposes_program_state_preparation` が
  `file not found for module state` で失敗。
- GREEN: seam test が `print` index `0` と import count `17` を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（147 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`
- package 全体は 260 passed / 1 failed。失敗は既存 selfhost fixture の
  `vector-push-pair-rooted-v3` 未定義による
  `test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` であり、
  この state-only 差分の変更対象外として記録する。

## Boundary

これは lower の program-state preparation 責務分離だけを扱う。lower 全体の semantics
parity、native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。

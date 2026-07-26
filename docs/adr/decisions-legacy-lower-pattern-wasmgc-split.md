# ADR: legacy lower の WasmGC pattern lowering 分割

- Status: Accepted (verified partial decomposition)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/pattern.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [I-01](../../ISSUES.md#i-01), [I-08](../../ISSUES.md#i-08)

## Context

`pattern.rs` は linear-memory の ADT/record pattern lowering と WasmGC backend の
constructor/record/nested pattern sequence を同じ `impl Lower` に抱え、698 行になっていた。
WasmGC 側は `StructGet`、GC type/field name 解決、nested mismatch の次 arm への fail-through
という独立した backend 責務を持つため、linear-memory pattern と分けてレビュー可能にする。

## Decision

WasmGC-specific な次の lowering を `crates/lsharp-ir/src/lower/pattern_wasmgc.rs` へ移動する。

- constructor / record arm lowering
- record field checks と GC field type/name 解決
- nested constructor/literal/record pattern sequence

`pattern.rs` は `lower_match_arms`、linear-memory binding、guard body を担当し、child の
entry points (`lower_wasmgc_constructor_arm` / `lower_wasmgc_record_pattern_arm`) と
`lower_arm_body_with_guard` だけを `pub(super)` にして既存内部 dispatch を維持する。

移動時に次の observable contract を変更しない。

- WasmGC `StructGet` / `If` / `Else` / `Unreachable` の emission と nested fail-through
- ADT variant、record field、field type name の解決エラーと span
- linear-memory pattern の binding、guard 評価、既存 `Lower` 内部 API

## Evidence

RED は child module を宣言した直後に既存の ADT pattern test を実行し、
`pattern_wasmgc.rs` が存在しない E0583 を確認した。GREEN と regression gate は次の通り。

- `cargo test -q -p lsharp-ir lower::tests::heap_and_adt:: --lib`: 11 passed
- `cargo test -q -p lsharp-ir lower:: --lib`: 167 passed
- `cargo test -q -p lsharp-ir lower::tests::rooting_calls:: --lib`: 28 passed
- `cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`: pass
- `cargo check --workspace --quiet`: pass
- 対象 pattern files の Rust 2024 `rustfmt --check` と `git diff --check`: pass
- large-stack `cargo test -q -p lsharp-ir --lib`: 282 passed、既存の
  `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  が `IntentSource.ls` の `vector-push-pair-rooted-v3` 未定義で 1 failure

親は 698 行から 411 行、WasmGC child は 297 行となった。テスト失敗は今回の移動で導入した
ものではなく、直前の `origin/main` でも記録されていた baseline failure として扱う。

## Consequences

- linear-memory と WasmGC の pattern lowering の責務境界と focused review が明確になる。
- pattern 全体の parity、他の `lower` production/test 分割、I-01 / I-08 aggregate、
  Rust/native parity、Mac/Linux native stage0 の証跡は未完了である。
- `TODO.md` では partial slice として `[~]` 相当の残リスクを維持し、全要件完了まで完了項目へ
  移さない。

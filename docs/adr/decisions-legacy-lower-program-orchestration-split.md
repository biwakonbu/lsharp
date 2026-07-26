# ADR: legacy lower の program orchestration 分割

- Status: Accepted (verified partial decomposition)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/mod.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [I-01](../../ISSUES.md#i-01), [I-08](../../ISSUES.md#i-08)

## Context

`lower/mod.rs` は `Lower` の状態定義・型表現 helper と、program 全体の orchestration
（宣言/trait/constraint/ADT function collection、GC data collection、`lower_program*`）を
同じ file に抱えていた。program 側は function collection の順序、lambda lifting、module
assembly、root-lifetime validation を調整する独立した責務である。

## Decision

次の program-level method 群を `crates/lsharp-ir/src/lower/program.rs` へ移動する。

- `lower_defn_functions` / `lower_field_accessors`
- `lower_trait_impl_functions` / `lower_constraint_functions`
- `lower_adt_constructors`
- `clone_string_data_from` / `gc_types_for_program`
- public `lower_program` / `lower_program_with_expr_types`

既存の `pub(crate)` helper と public API の可視性、`prepare_program_state` の呼び出し、function
collection の順序、lambda lifting の追加、`Module` assembly、root-lifetime validation は変更
しない。状態フィールドと型表現 helper は parent に残す。

## Evidence

RED は child module を宣言した直後に既存の heap/ADT pattern test を実行し、
`program.rs` が存在しない E0583 を確認した。GREEN と regression gate は次の通り。

- `cargo test -q -p lsharp-ir lower::tests::heap_and_adt:: --lib`: 11 passed
- `cargo test -q -p lsharp-ir lower:: --lib`: 167 passed
- `cargo test -q -p lsharp-ir lower::tests::rooting_calls:: --lib`: 28 passed
- `cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`: pass
- `cargo check --workspace --quiet`: pass
- `program.rs` の Rust 2024 `rustfmt --check` と `git diff --check`: pass
- large-stack `cargo test -q -p lsharp-ir --lib`: 282 passed、既存の
  `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  が `IntentSource.ls` の `vector-push-pair-rooted-v3` 未定義で 1 failure

親は 536 行から 324 行、program child は 227 行となった。テスト失敗は今回の移動で導入した
ものではなく、直前の `origin/main` でも記録されていた baseline failure として扱う。

## Consequences

- 状態/型表現と program orchestration の責務境界が明確になり、lower pipeline の review が
  分割しやすくなる。
- Lower state/type representation、他の production/test 分割、I-01 / I-08 aggregate、
  Rust/native parity、Mac/Linux native stage0 の証跡は未完了である。
- `TODO.md` では partial slice として `[~]` 相当の残リスクを維持し、全要件完了まで完了項目へ
  移さない。

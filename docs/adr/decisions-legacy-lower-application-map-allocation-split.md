# ADR: legacy lower の HashMap allocation/size lowering 分割

- Status: Accepted (verified partial decomposition)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_map.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [I-01](../../ISSUES.md#i-01), [I-08](../../ISSUES.md#i-08)

## Context

lookup と mutation を別 module へ移動した後、`application_map.rs` には HashMap の生成
(`map-new`) と size 読み出し (`map-size`) だけが残った。生成は `__alloc`、heap header、entry
zero-fill を扱い、size は tagged pointer の unwrap と header load を扱うため、mutation/lookup
とは異なる allocation/metadata 責務である。

この ADR は HashMap の runtime parity や全 lower 分割を完了するものではなく、最後の
allocation/size 責務を移動した検証済み slice を記録する。

## Decision

`map-new` と `map-size` の lowering を
`crates/lsharp-ir/src/lower/expr/application_map_allocation.rs` の
`Lower::lower_app_map_allocation` に移動する。`application_map.rs` は lookup、mutation、
allocation の3 child を順に dispatch し、未対応関数には `Ok(false)` を返す wrapper とする。

移動時に次の observable contract を変更しない。

- `map-new` の `__alloc` 呼び出し、`HEAP_TAG_HASHMAP` header、capacity/size、entry zero-fill
- `__alloc` 未登録時の `LowerError::UndefinedFunction` と expression span
- `map-size` の tagged pointer unwrap、header offset 8 の size load、i64 への拡張
- 生成する Wasm opcode の順序と既存 `Lower` 内部 API

## Evidence

RED は child module を宣言した直後に map tests の listing を行い、
`application_map_allocation.rs` が存在しない E0583 を確認した。GREEN と regression gate は次の
通り。

- `cargo test -q -p lsharp-ir lower:: --lib`: 167 passed
- `cargo test -q -p lsharp-ir lower::tests::rooting_calls:: --lib`: 28 passed
- `cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`: pass
- `cargo check --workspace --quiet`: pass
- 対象 3 files の Rust 2024 `rustfmt --check` と `git diff --check`: pass
- large-stack `cargo test -q -p lsharp-ir --lib`: 282 passed、既存の
  `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  が `IntentSource.ls` の `vector-push-pair-rooted-v3` 未定義で 1 failure

親は 84 行から 26 行、allocation child は 79 行となった。テスト失敗は今回の移動で導入した
ものではなく、直前の `origin/main` でも記録されていた baseline failure として扱う。

## Consequences

- HashMap の lookup/mutation/allocation の責務境界と focused review が明確になる。
- `lower/expr` の他責務分割、I-01 / I-08 aggregate、Rust/native parity、Mac/Linux native
  stage0 の証跡は未完了である。
- `TODO.md` では partial slice として `[~]` 相当の残リスクを維持し、全要件完了まで完了項目へ
  移さない。

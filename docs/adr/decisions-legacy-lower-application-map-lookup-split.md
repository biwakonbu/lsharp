# ADR: legacy lower の HashMap lookup lowering 分割

- Status: Accepted (verified partial decomposition)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_map.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [I-01](../../ISSUES.md#i-01), [I-08](../../ISSUES.md#i-08)

## Context

`application_map.rs` は HashMap の allocation (`map-new`)、size/read-only lookup、mutation
(`map-insert` / `map-remove`) を一つの application match に抱えていた。read-only の
`map-get` / `map-contains?` は、mutation と異なる線形探索・結果復元・key rooting の責務を
持つため、同じ親ファイルに残すと責務境界と focused review が不明瞭になる。

この ADR は実装全体の HashMap parity を完了するものではなく、既存 lowering の移動だけを
検証済み slice として記録する。

## Decision

`map-get` と `map-contains?` の lowering を
`crates/lsharp-ir/src/lower/expr/application_map_lookup.rs` の
`Lower::lower_app_map_lookup` に移動する。`application_map.rs` はこの child を先行 dispatch
し、`map-new`、`map-size`、`map-insert`、`map-remove` の allocation/storage/mutation を
引き続き担当する。

移動時に次の observable contract を変更しない。

- `lower_map_key_to_local` と root push/pop による key/map の rooting 順序
- empty slot / tombstone をまたぐ線形探索と capacity boundary
- 未存在時の `map-get = 0`、`map-contains? = false`、一致時の結果値
- 生成する Wasm opcode の順序、既存 `Lower` 内部 API、diagnostic span の受け渡し

## Evidence

RED は child module を宣言した直後に既存の map-get rooting test を実行し、
`application_map_lookup.rs` が存在しない E0583 を確認した。GREEN と regression gate は次の
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

親は 451 行から 295 行、lookup child は 181 行となった。テスト失敗は今回の移動で導入した
ものではなく、直前の `origin/main` でも記録されていた baseline failure として扱う。

## Consequences

- read-only lookup の review / focused test の責務境界が明確になる。
- HashMap storage と mutation の追加分割、`lower/expr` 全体の 800 行未満化、I-01 / I-08
  aggregate、Rust/native parity、Mac/Linux native stage0 の証跡は未完了である。
- `TODO.md` では partial slice として `[~]` 相当の残リスクを維持し、全要件完了まで完了項目へ
  移さない。

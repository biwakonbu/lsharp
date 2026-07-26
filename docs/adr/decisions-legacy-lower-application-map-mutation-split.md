# ADR: legacy lower の HashMap mutation lowering 分割

- Status: Accepted (verified partial decomposition)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_map.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [I-01](../../ISSUES.md#i-01), [I-08](../../ISSUES.md#i-08)

## Context

前段で read-only lookup (`map-get` / `map-contains?`) を分離した後も、
`application_map.rs` は allocation/size と mutation (`map-insert` / `map-remove`) を
同じ match に抱えていた。mutation は key/value rooting、slot insertion、上書き、tombstone、
size 更新を持つ独立した責務であり、map object の生成・size 読み出しと同じ親に残すと
focused review の境界が不明瞭になる。

この ADR は HashMap 実装全体や Rust/native parity の完了を宣言せず、既存 lowering の
責務移動だけを検証済み slice として記録する。

## Decision

`map-insert` と `map-remove` の lowering を
`crates/lsharp-ir/src/lower/expr/application_map_mutation.rs` の
`Lower::lower_app_map_mutation` に移動する。`application_map.rs` は lookup dispatch に続いて
mutation child を dispatch し、`map-new` と `map-size` の allocation/size 責務を担当する。

移動時に次の observable contract を変更しない。

- map/key/value の root push/pop と `lower_map_key_to_local` の評価順序
- empty slot への insertion、既存 key の value 上書き、size の増減
- remove の tombstone (`-1`) と probing 継続/終了条件
- tagged pointer の返却、生成する Wasm opcode の順序、既存 `Lower` 内部 API

## Evidence

RED は child module を宣言した直後に既存の map-insert rooting test を実行し、
`application_map_mutation.rs` が存在しない E0583 を確認した。GREEN と regression gate は次の
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

親は 295 行から 84 行、mutation child は 236 行となった。テスト失敗は今回の移動で導入した
ものではなく、直前の `origin/main` でも記録されていた baseline failure として扱う。

## Consequences

- HashMap mutation の review / focused test の責務境界が明確になる。
- map-new/map-size の追加分割、`lower/expr` 全体の 800 行未満化、I-01 / I-08 aggregate、
  Rust/native parity、Mac/Linux native stage0 の証跡は未完了である。
- `TODO.md` では partial slice として `[~]` 相当の残リスクを維持し、全要件完了まで完了項目へ
  移さない。

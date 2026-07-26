# ADR: validation source adapter test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-types/tests/validation_source.rs`
- Related: `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

`validation_source.rs` は source node、typed edge、evidence record の contract tests を
一つの integration-test target に集約しており、956 行まで肥大化していた。M2 の入力
contract を変更せず、失敗境界ごとにレビュー・再実行できる責務単位へ分ける必要がある。

## Decision

- `validation_source.rs` は Cargo integration-test target と module tree だけを保持する。
- source node / declaration metadata のテストを `validation_source/nodes.rs`、node/evidence
  edge closure のテストを `validation_source/edges.rs`、evidence record・sampling・registry
  closure のテストを `validation_source/evidence.rs` に分離する。
- `#[path]` module seam を使い、test function、fixture、assertion、公開 adapter API は変更しない。
- root test target が 500 行を超えないことを `validation_source_file_size.rs` で固定する。

## Evidence

- RED: `validation_source_file_size` を先に追加し、`cargo test -p lsharp-types --test validation_source_file_size -- --nocapture` が `actual=956` で失敗。
- GREEN: node/edge/evidence module へ移動後、root は 11 行になり、既存 24 tests とサイズ契約が pass。
- `cargo test -p lsharp-types --quiet`（unit 221件を含む全 lsharp-types tests）
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`
- 変更ファイル単位の Rust 2024 rustfmt、`git diff --check`

## Consequences

test ownership と failure report の責務が分かれ、root の変更競合を小さくできる。source
adapter の graph semantics、sampling/provenance、diagnostic span、manifest roundtrip は
変更しない。selfhost/native parity、manifest/runtime target gate、EC-M2 aggregate の完了を
意味せず、`LEGACY-MAINT-01` と `EC-M2-02` は `[~]` のまま継続する。

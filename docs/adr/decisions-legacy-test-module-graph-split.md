# ADR: `module_graph.rs` のインラインテスト分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-ir/src/module_graph.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`module_graph.rs` はグラフ構築・探索・SCC・キャッシュ連携の実装に加えて、43 件の unit test を同じファイルへ保持していた。
実装とテストの責務が混在すると、module graph の変更差分が読みづらくなり、別レイヤの失敗を単体で切り分けにくい。

## Decision

- production の公開 API とロジックは変更せず、`#[cfg(test)]` のテストだけを `src/module_graph/` へ移動する。
- 既存の test module 境界を維持し、`tests.rs`、`nested_module_tests.rs`、`resolve_tests.rs`、`hierarchy_tests.rs` を親から宣言する。
- test module の名前空間は `module_graph::tests::*` など従来の構造を保ち、利用側の API や fixture は変更しない。

## Evidence

- 移動前の module graph focused gate: 43 tests passed。
- 移動後の同じ gate: 43 tests passed。
- `module_graph.rs` は 1,836 行から 1,011 行へ縮小し、テストは 417 / 301 / 54 / 49 行へ分離した。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 257 passed。
- default stack の全 crate gate は既知の `test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` stack overflow で abortした。これは C-1n に記録済みの既存 failure boundary であり、本移動による失敗ではない。
- changed files の rustfmt check と `git diff --check` は passした。

## Consequences

module graph の unit test を実装変更と独立してレビュー・再実行できるようになった。production file はまだ 800 行を超えるため、グラフ構築 / 解決 / SCC の production 分割と `I-01` / `I-08` aggregate 完了は後続タスクとして残す。

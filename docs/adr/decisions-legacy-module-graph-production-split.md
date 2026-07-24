# ADR: `module_graph.rs` の production path-resolution 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-ir/src/module_graph.rs`, `crates/lsharp-ir/src/module_graph/resolve.rs`
- Related: `I-01`, `I-05`, `imp-04-module-system-strengthening.md`, `imp-06-large-file-decomposition.md`

## Context

inline test 分離後も `module_graph.rs` は graph/SCC 操作と、探索パス・ファイル解決・entry graph 構築を同じ production file に保持していた。SCC/cache の変更と path resolution の変更を同じ差分でレビューすると、module graph の failure boundary を切り分けにくい。

## Decision

- `ModuleSearchPaths` と entry/source/import の path-resolution API を `module_graph/resolve.rs` へ移動する。
- 親 `module_graph.rs` は `mod resolve; pub use resolve::ModuleSearchPaths;` で従来の公開 path を維持する。
- `ModuleGraph` の private fields へアクセスする resolver impl は子 module に置くが、公開 API、エラー型、SCC/graph algorithm、戻り値の順序は変更しない。
- `to_snake_case` / `to_pascal_case` は内部 test が継続利用できる `pub(super)` とし、外部公開 API は増やさない。

## Evidence

- `module_graph::` focused tests: 43 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 257 passed。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check` が pass。
- 親 `module_graph.rs` は 1,011 行から 552 行、resolver は 473 行となり、両方を 800 行未満へ収めた。

## Consequences

graph/SCC と path resolution の責務が独立してレビュー・再実行できるようになった。`wasi.rs`、`main.rs`、`infer.rs`、`ir/lib.rs` の分割、`I-01` / `I-05` aggregate、native/selfhost parity は後続であり、この移動だけで完了扱いにはしない。

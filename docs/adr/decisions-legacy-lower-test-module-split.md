# ADR: `lower/tests.rs` の回帰テスト分離
- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-ir/src/lower/tests.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`lower/tests.rs` は lowering の helper・定数と 143 件の回帰テストを同じファイルに保持し、3,913 行まで肥大していた。production lowering の差分と、GC root・closure・record/ADT など異なる fixture の差分が混在するため、レビューと failure isolation の境界が不明瞭だった。

## Decision

- production lowering の API、実装、テスト body の意味論は変更せず、test-only のファイル移動に限定する。
- `tests.rs` には `lower` / `assert_ir` / root assertion helper と共有定数だけを残し、親を 133 行にする。
- 既存の `lower::tests` 配下で、次の9 moduleを宣言する。
  - `wasm_gc_and_roots` — WasmGC と root lifetime
  - `core_lowering` — 基本 lowering
  - `rooting_calls` — allocating call site の root
  - `rooting_loops` — self-TCO と spill completeness
  - `language_and_traits` — 言語構造・計算式・trait
  - `records_and_adt` — record と ADT
  - `module_and_lambdas` — module・Ref・lambda
  - `closure_calls` — closure call と引数 root
  - `heap_and_adt` — closure heap object と ADT memory
- 既存の test 名と `lower::tests` の filter を維持し、production lower の責務分割は後続タスクとして残す。

## Evidence

- 分離前後の `CARGO_TARGET_DIR=... cargo test -p lsharp-ir lower::tests -- --nocapture`: 143 passed。
- 移動前後の test name set は 143 件で完全一致し、欠落・重複はない。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 257 passed。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-ir --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`: pass。
- 最終の親/child 行数は親 133 行、child 129 / 692 / 414 / 531 / 531 / 228 / 598 / 290 / 390 行。全 child が 800 行未満となった。
- default stack の `cargo test -p lsharp-ir` は既知の `incremental_compile_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` stack overflow で abortした。large-stack gate で全 257 tests が通る既存 failure boundary であり、本移動による失敗ではない。

## Consequences

lower の回帰テストは機能領域ごとに独立して review・再実行でき、共有 helper と production の境界も明確になった。一方、`lower/expr.rs` など production 側の大規模ファイル分割、I-01 / I-08 aggregate、selfhost/native parity は未完了であり、TODO の partial slice を維持する。

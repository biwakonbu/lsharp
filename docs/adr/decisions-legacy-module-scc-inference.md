# ADR: SCC 単位の compile/infer 接続

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `compile_multi_file_with_mode` の相互再帰モジュール推論
- Related: `decisions-legacy-module-scc-detection.md`

## Context

`ModuleGraph::scc_groups()` で相互再帰群を検出できても、従来の compile 入口は
`topological_sort()` の循環エラーで停止し、モジュールごとの `Infer` は同じ SCC の
関数を参照できなかった。Formatter 3 モジュールにはこの制約を避ける merged 特例が
既にあるが、一般の相互再帰を扱う契約ではなかった。

## Decision

- strict な `build_from_entry()` / `build_from_entry_with_overrides()` は保持し、compile
  専用の `build_from_entry_with_scc()` だけが cyclic SCC を許容する。
- `compile_multi_file_with_mode` は全 module source を parse した後、依存先が先に来る
  `scc_groups()` ごとに宣言を連結した prepass `Infer::infer_program` を実行する。
- SCC 内の型は同じ merged environment で前登録し、SCC 外の依存だけ既存の
  `ModuleTypeSurface` と import visibility に従って注入する。確定した provisional surface を
  用いて各 module を元の import visibility でも再検証し、SCC 内の `:only` / private 境界を保つ。
- 型結果と `ExprTypeKey` は declaration origin / scope で元 module ごとに分割し、既存の
  modular/merged lowering と public single-file path は維持する。
- Formatter 3 の既存 batch surface はこの slice でも互換のため残す。C-1 の後続で一般 SCC
  経路へ置き換える。

## Evidence

- RED: `test_compile_multi_file_infers_mutual_recursive_scc` は実装前、
  `CyclicDependency: A -> B -> A` で失敗した。
- GREEN: 同テストは `A` ↔ `B` の相互再帰と `Main` の依存を temp source から compile し、
  `a-step` / `b-step` / `main` の IR 関数を確認する。
- Visibility: `test_compile_multi_file_scc_preserves_import_only_visibility` は SCC 内でも
  `:only` 外の `secret` を拒否することを確認する。
- Runtime: `test_e2e_multi_file_mutual_recursive_scc` は同じ source graph を Wasm/WASI で実行し、
  `a-step 4 == 1` を確認する。
- Regression: lsharp-ir lib 238 tests、multi-file/module graph focused tests、Wasm focused runtime、
  clippy、rustfmt を通過した。

## Residual risk

これは Rust host の IR compile と Mac host の Wasm/WASI runtime に限定された verified partial slice である。
`compile_multi_file_incremental` と source override の SCC/cache 統合、Formatter batch 特例の撤去、
selfhost compiler、Mac Apple Silicon / Linux x86_64 native stage0 parity は未完了で、
`LEGACY-MODULE-01` の aggregate 完了条件には到達していない。

Formatter 特例撤去の識別 probe は、canonical `FormatterExpr.ls` の `format-expr` 未定義
(`E0001`, span `3962..3973`) で失敗した。これは C-1 の generic SCC 実装ではなく、Formatter
source/API 側の先行 blocker として残す。

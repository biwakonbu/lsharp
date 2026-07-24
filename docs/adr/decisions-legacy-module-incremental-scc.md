# ADR: incremental compile の SCC fallback

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::compile_multi_file_incremental`
- Related: `decisions-legacy-module-scc-inference.md`

## Context

`compile_multi_file_incremental` は従来 `build_from_entry` の strict topological graph を使っていたため、
相互再帰 module を `CyclicDependency` で拒否していた。通常の `compile_multi_file` には SCC 一括推論が
既にあるが、cache 付き CLI/tooling 経路だけが別の failure boundary を持っていた。

## Decision

- incremental compile の graph 構築を `build_from_entry_with_scc` に切り替える。
- サイズ 2 以上の SCC がある場合は、全 module を parse して `infer_scc_type_surfaces` を依存 SCC 順に
  適用し、既存の modular lowering へ渡す。
- SCC 経路はまず correctness を優先し、`ModuleIrSegments` の segment reuse は行わない。linked IR、AST、
  型 surface、依存 surface key は `CompilationCache` に保存し、clean rebuild は cache 済み linked IR を返す。
- acyclic 経路の既存の module 単位型推論・segment reuse・link patch は変更しない。

## Evidence

- RED: `test_compile_multi_file_incremental_infers_mutual_recursive_scc` は A ↔ B の fixture で
  `モジュールグラフ構築エラー: 循環依存` を確認した。
- GREEN: 同テストは相互再帰 SCC を incremental compile し、2 回目の clean rebuild と linked IR の
  `dump()` が一致することを確認する。
- lsharp-ir regression 242 tests（既知の canonical Formatter probe 1 件を skip）、clippy、rustfmt、diff check を通過。

## Residual risk

これは SCC の correctness bridge に限定した verified partial slice である。source override の SCC-aware
inference、dirty SCC の局所再推論、segment reuse、Formatter 3 module の明示 import と batch special-case
除去、Wasm/runtime と native stage0 の両 target evidence は未完了であり、`LEGACY-MODULE-01` aggregate 完了
とは扱わない。

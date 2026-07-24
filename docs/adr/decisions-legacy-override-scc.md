# ADR: source override analysis の SCC inference

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `ModuleGraph::build_from_entry_with_overrides_scc`, `analyze_multi_file_incremental_with_overrides`
- Related: `decisions-legacy-override-cache-scope.md`, `decisions-legacy-module-incremental-scc.md`

## Context

Formatter の explicit import で source graph が循環すると、LSP の未保存 source override 解析だけが
`build_from_entry_with_overrides` の strict graph に残り、compile 側と同じ module でも
`CyclicDependency` を返していた。cache scope isolation は済んでいたが、循環 module の解析境界が閉じていなかった。

## Decision

- `ModuleGraph::build_from_entry_with_overrides_scc` を追加し、source override を import 抽出へ渡したまま
  SCC を許容する graph 入口を提供する。
- `analyze_multi_file_incremental_with_overrides` はサイズ 2 以上の SCC を
  `infer_scc_type_surfaces` で一括推論し、各 module の AST、型 surface、dependency key を cache する。
- この入口は analysis-only とし、IR lowering や SCC segment reuse は行わない。missing import と
  `:only` / private visibility の境界は既存 graph/inference 契約を維持する。

## Evidence

- RED: `test_analyze_multi_file_incremental_with_overrides_infers_mutual_recursive_scc` は A↔B fixture と
  A の未保存 override で `CyclicDependency` を確認した。
- GREEN: 同テストは override source を反映した SCC を解析し、A/B/Main の 3 cache entry を確認する。
- 既存 missing-import、entry-root scope、lsharp-ir regression 244 tests（canonical Formatter probe 1 件を skip）、
  clippy、rustfmt、docs audit を通過。

## Residual risk

SCC source override の correctness bridge は閉じたが、dirty SCC の局所再推論、segment reuse、disk persistence、
canonical Formatter の clean-cache compile/runtime parity、Mac/Linux native stage0 evidence は未完了である。
`LEGACY-MODULE-01` aggregate 完了とは扱わない。

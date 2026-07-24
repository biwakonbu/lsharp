# ADR: ModuleGraph の deterministic SCC 検出

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::ModuleGraph` の強連結成分検出 API
- Related: `../development/planning/improvement-designs/imp-04-module-system-strengthening.md`

## Context

`LEGACY-MODULE-01` / imp-04 Phase C-1 は、相互再帰モジュールをモジュール単位の
型推論から SCC 単位へ移すために、まずグラフの強連結成分を安定して列挙する必要がある。
従来の `detect_cycles` は循環をエラーとして返し、`topological_sort` は単一モジュールの
順序付けを担っていたため、相互再帰群を後続の一括推論へ渡す API が存在しなかった。

## Decision

- `ModuleGraph::scc_groups() -> Vec<Vec<String>>` を公開し、Tarjan 法で SCC を検出する。
- import edge は `module -> dependency` と解釈し、結果は依存先が先に来る順序で返す。
- DFS の開始順、import の走査順、SCC 内のメンバー順はモジュール名で安定化する。
- グラフに存在しない import は SCC へ暗黙に追加しない。未解決 import の診断は既存の
  `check_imports` の責務として保持する。
- 既存の `detect_cycles` / `topological_sort` と compile/infer 経路はこの slice では変更しない。

## Evidence

- RED: `test_scc_groups_are_stable_and_dependency_first` は API 未実装時にコンパイル失敗した。
- GREEN: 同テストは `Base`、`CycleA`/`CycleB`、`Consumer` の dependency-first 順序と、
  同一グラフを複数回処理した結果の安定性を確認する。
- focused `lsharp-ir` test、module graph test、clippy、rustfmt、docs audit を通過した。

## Residual risk

これは SCC 検出 API だけの verified partial slice である。`compile_multi_file_with_mode` の
SCC 単位 `infer_program`、Formatter 3 モジュールの特別扱い撤去、CLI `CompilationCache` と
依存 key、selfhost compiler 移植、Mac Apple Silicon / Linux x86_64 native stage0 の実行証跡は
未完了で、`LEGACY-MODULE-01` の aggregate 完了条件には到達していない。

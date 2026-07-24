# ADR: clean SCC の type surface を incremental compile で再利用する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::compile_multi_file_incremental_scc`

## Context

SCC 対応後の incremental compile は、linked IR の clean hit でない限り、source が変わっていない SCC も毎回
merged 推論と visibility revalidation を実行していた。dirty module が cycle の一部にある場合、外部の clean SCC
まで同じ型推論コストを負担していた。

各 `ModuleCacheEntry` には source fingerprint と direct dependency の type surface key が既に保存されている。
SCC の順序は dependency-first なので、外部依存の current surface を使って同じ key を再計算できる。

## Decision

`graph.scc_groups()` の各 group について、全 module の fingerprint と dependency surface key が cache entry と一致する
場合は、group 内の保存済み `ModuleTypeSurface` を `per_module_type_results` に戻し、`infer_scc_type_surfaces` を呼ばない。
一致しない group だけを従来の merged inference / visibility revalidation に通す。

cycle 内の import は current surface がまだ作られていないため既存 cache surface を key 計算に使う。group 内の source
fingerprint が一致している条件と組み合わせることで、cycle の型結果を stale にしない。外部依存の公開 surface が変化
した場合は key 不一致で downstream SCC を再推論する。

source override analysis、disk persistence、native stage0 parity はこの ADR の対象外である。

## Evidence

- RED: `test_compile_multi_file_incremental_scc_reuses_clean_type_surfaces_after_impl_change` は実装前、A の実装変更で
  Base / A↔B / Main の3 SCCを再推論し、期待値1に対して3となった。
- GREEN: 同テストは A↔B の1 SCCだけを再推論し、full compile と linked IR が一致。
- Regression: SCC/multi-file focused 10 tests、既知 Formatter probe を除く lsharp-ir regression 246 passed / 0 failed、
  clippy (`-D warnings`)、rustfmt、`git diff --check` が成功。

## Consequences

実装だけが変わり公開型 surface が不変な場合、外部 clean SCC の型推論を省略できる。公開型が変わる変更では
dependency key により downstream SCC を再推論する。override 経路の同等 cache、process 間 persistence、Formatter
canonical 初回 inference、両 native target の実行証跡は後続タスクとして残る。

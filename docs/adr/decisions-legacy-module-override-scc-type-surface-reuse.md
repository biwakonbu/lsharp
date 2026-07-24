# ADR: source override SCC analysis で clean type surface を再利用する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::analyze_multi_file_incremental_scc_with_overrides`

## Context

LSP の未保存 source を解析する SCC 経路は、override fingerprint を cache に保存していたが、同じ override を再解析する
場合や一つの SCC だけを変更した場合も、全 SCC の merged inference と visibility revalidation を繰り返していた。

compile 経路では fingerprint と dependency type surface key による group cache hit を導入済みである。override analysis も
同じ `CompilationCache` scope と `ModuleTypeSurface` 契約を使うため、解析結果の lowering を持ち込まずにこの判定を共有できる。

## Decision

override source から作った fingerprint と、dependency-first に再計算した dependency surface key が全 module で一致する SCC
は cache の type surface を復元し、推論を省略する。一つでも不一致なら group 全体を従来の SCC inference に通す。実際に推論した
group だけを tracker/evidence に記録する。

override 経路は解析専用で、IR segment reuse、linked module、process 間 disk persistence、native stage0 parity は対象外とする。

## Evidence

- RED: `test_analyze_multi_file_incremental_with_overrides_reuses_clean_scc_type_surfaces` は実装前、初回 override 分析の SCC
  inference count が 0 で、期待した 3 group evidence を満たさなかった。
- GREEN: 初回は Base / A↔B / Main の3 group、A override の実装変更後は A↔B の1 groupだけが推論される。
- Regression: override/incremental focused 24 tests、既知 Formatter probe を除く lsharp-ir regression 247 passed / 0 failed、
  clippy (`-D warnings`)、rustfmt、`git diff --check` が成功。

## Consequences

同一 LSP session 内で未保存 source の変更範囲に応じて型解析コストを SCC 単位へ限定できる。override source が公開 surface を
変えた場合は dependency key により downstream SCC を再解析する。override の IR segment cache、disk persistence、Formatter
canonical 初回 inference、両 native target の実行証跡は後続タスクとして残る。

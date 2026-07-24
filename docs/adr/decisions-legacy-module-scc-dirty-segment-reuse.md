# ADR: dirty SCC compile で clean module の IR segment を再利用する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::compile_multi_file_incremental_scc`

## Context

SCC を含む incremental compile は、SCC ごとの型推論後に全 module を毎回 lowering し、cache には linked
IR と type surface だけを保存していた。そのため、相互再帰群の一部またはその依存だけが変更された場合も、
変更されていない module の IR を再生成していた。

既存の acyclic 経路には `ModuleIrSegments`、segment layout 判定、linked IR の range patch がある。SCC 経路も
同じ関数 index と segment 順序の契約を使えるため、独自の linker 仕様を増やさずにこの仕組みを共有できる。

## Decision

SCC compile の各 cache entry に lowering 済み `ModuleIrSegments` を保存する。再利用候補は次の全条件を満たす
module に限定する。

1. source fingerprint が一致する。
2. direct dependency の type surface key が一致する。
3. direct dependency に今回 export surface が変わった module がない。

候補 segment は `lower_multi_file_modular_with_segments` に渡し、prefix/state/layout の既存安全判定に従って
部分再利用する。旧 linked module の module order と segment layout が一致するときは `patch_linked_module`
で range patch し、形状が変わったときだけ full relink に戻す。

型推論は引き続き SCC 単位で行う。したがって本 ADR は lowering/link の局所化であり、dirty SCC の局所型推論、
source override 経路の segment cache、process 間 disk persistence、native stage0 parity を完了扱いにしない。

## Evidence

- RED: `test_compile_multi_file_incremental_scc_reuses_clean_ir_segments_after_dirty_module` は実装前に、warm
  SCC compile 後の `Base` segment が空であることを検出。
- GREEN: A↔B cycle + Base + Main fixtureで、A の同型実装変更に対して fresh defn lower 1件、linked range patch 1件、
  full relink 0件を確認。
- Differential: incremental linked IR の `dump` と `string_data` が `compile_multi_file` の fresh compile と一致。
- Regression: SCC/multi-file focused 9 tests、lsharp-ir regression 245 passed / 0 failed（canonical Formatter probe
  の長時間テスト 1件は既知 blocker のため skip）、clippy (`-D warnings`)、rustfmt、`git diff --check` が成功。

## Consequences

dirty SCC compile で変更されていない module の lowering と full relink を避けられる。segment layout が変化した場合は
既存の full relink fallback が働くため、関数 index の安全性を犠牲にしない。Formatter canonical の初回 inference、
SCC 型推論の局所化、両 native target の実行証跡は後続タスクとして残る。

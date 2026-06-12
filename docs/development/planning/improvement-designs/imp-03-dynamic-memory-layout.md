# imp-03: GC メモリレイアウト動的化とアロケータ改善

> 対象 issue: [I-03](../../../../ISSUES.md#i-03) (固定スロット上限)、[I-04](../../../../ISSUES.md#i-04) (フリーリスト線形探索)、
> [D-10](../../../../ISSUES.md#d-10) (G1 sentinel edge case、documented limitation)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase A-3 / B-2 / B-5
>
> **正本**: GC 実装の詳細は `docs/development/planning/memory-management-roadmap.md`、
> runtime 安定性の判定は `docs/development/planning/runtime-stability-spec.md` を正本とする。
> 本書は容量・性能の改善設計のみを扱い、仕様固定済みの項目 (ADR-158 系) を変更しない。

## 概要

リニアメモリ GC ランタイムの容量がコンパイル時定数で固定されている:

```
crates/lsharp-wasm/src/wasi.rs:21  const ROOT_STACK_SLOT_CAPACITY: i32 = 32768;
crates/lsharp-wasm/src/wasi.rs:23  const GC_OBJECT_SLOT_CAPACITY: i32 = 4096;
crates/lsharp-wasm/src/wasi.rs:26  const GC_FREE_LIST_SLOT_CAPACITY: i32 = 4096;
```

同時生存オブジェクトが 4096 を超える、または root の深さが 32768 を超えるワークロードで
容量が枯渇し、上限はユーザーから調整できない。またフリーリストが単一リストの線形探索のため、
割り当て頻度が高いとアロケーションが O(n) に劣化する (I-04)。

## 設計

### 1. 容量の grow 戦略 (Phase A-3)

レイアウトの「位置」を固定したまま「容量」を可変にする:

- **テーブルの末尾配置**: GC オブジェクトテーブル / フリーリスト / root stack を
  ヒープ末尾側へ移し、`memory.grow` で線形メモリを拡張したとき各テーブルが
  倍々で伸長できる配置にする (ヒープ本体とテーブルが互いに衝突しない方向に伸ばす)
- **間接化**: テーブル基底アドレスと容量をグローバル (または固定アドレスのヘッダ) に持たせ、
  生成コードは定数ではなくヘッダ経由で参照する。grow 時はテーブルを新領域へコピーして
  ヘッダを差し替える
- **失敗時の挙動**: `memory.grow` が失敗した場合は trap ではなく、診断コード
  `LS4002` (GC 容量超過、imp-02 の体系) を持つランタイムエラーとして報告する
- **初期値**: 既存定数を初期容量として維持し、既存ワークロードの挙動 (GC メトリクス) を
  変えない。grow が起きない限り現行とバイト互換の動きにする

### 2. フリーリストのサイズクラス化 (Phase B-2)

- 単一フリーリストを、サイズクラス別 (例: 16/32/64/128/256/512/1024/それ以上) の
  複数リストへ分割し、割り当ては該当クラスの先頭 pop (O(1)) にする
- クラス外の大きな割り当てのみ従来の探索へフォールバック
- 効果測定: 既存の GC メトリクス artifact (`ci-artifacts/gc-metrics/`) に
  割り当て探索ステップ数のカウンタを追加し、改善前後で比較する

### 3. G1 precise discrimination の再評価 (Phase B-5、任意)

G1 (`i64::MIN + N` を意図的に計算した値が false-mark される理論的 edge case、
`runtime-stability-spec.md:278-282`) は documented limitation の整理を維持する。
本設計の間接化 (テーブルヘッダ導入) が完了すると、tagged handle の判別ビット拡張など
runtime-stability-spec.md に列挙された将来選択肢の実装コストが下がるため、
その時点で「documented limitation 継続」か「精密判別の実装」かを再評価する。
判定の正本は runtime-stability-spec.md のまま動かさない。

### 4. テスト戦略 (TDD)

1. RED: 同時生存オブジェクト 4096 超 / root 深度 32768 超を作る E2E を追加し、
   現行実装での失敗 (またはその一歩手前の挙動) を固定する
2. GREEN: grow 実装後、同テストが green になることを確認
3. 既存の GC 契約テスト (`gc_metrics_contract` 系、collector telemetry 系) の全件 green を維持
4. grow 発生時の GC メトリクス (collection count / freed count) が単調性を保つことを
   メトリクス契約テストへ追加
5. 限界値テスト (I-06 の一部): grow 上限到達時に LS4002 診断が返ることを E2E で固定

## 影響範囲

- 生成コードのテーブル参照が定数 → ヘッダ間接参照になるため、コードサイズと
  実行コストがわずかに増える (ベンチで定量化する)
- bootstrap 固定点 (stage chain) はコード生成が変わるため再生成・再検証が必要
- native backend (V2-13 系) のメモリレイアウトは別系統であり本書のスコープ外

## ステータス

設計のみ (2026-06-12 起草)。着手時は TODO.md に Phase A-3 / B-2 として項目を作成する。

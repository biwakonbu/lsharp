# imp-03: GC メモリレイアウト動的化とアロケータ改善

> 対象 issue: [I-03](../../../../ISSUES.md#i-03) (固定スロット上限)、[I-04](../../../../ISSUES.md#i-04) (フリーリスト線形探索)、
> [D-10](../../../../ISSUES.md#d-10) (G1 sentinel edge case、documented limitation)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase A-3 / B-2 / B-5
>
> **正本**: GC 実装の詳細は `docs/development/planning/memory-management-roadmap.md`、
> runtime 安定性の判定は `docs/development/planning/runtime-stability-spec.md` を正本とする。
> 本書は容量・性能の改善設計のみを扱い、仕様固定済みの項目 (ADR-158 系) を変更しない。

## 現状の正確な把握 (2026-06-12 コード検証済み)

ランタイムは `crates/lsharp-wasm/src/wasi.rs` の `emit_wasm_wasi()` (`:102`、component 版は `:742`)
が生成コードに直接埋め込む。容量はコンパイル時定数:

```
wasi.rs:21  const ROOT_STACK_SLOT_CAPACITY: i32 = 32768;
wasi.rs:23  const GC_OBJECT_SLOT_CAPACITY: i32 = 4096;
wasi.rs:26  const GC_FREE_LIST_SLOT_CAPACITY: i32 = 4096;
```

ランタイム関数 (生成モジュール内の内部関数として emit される):

- rooting: `root_push` / `root_pop` / `root_set` (関数インデックス WASI_IMPORT_COUNT+13〜15、
  `wasi.rs:154-156` のインデックスマップと `:184-186` の定義)
- 回収: `__gc_collect` (WASI_IMPORT_COUNT+16)、export 名は
  `__lsharp_gc_collect` / `__lsharp_gc_collection_count` (`wasi.rs:53-55`)
- 割り当て: `__alloc` 系がフリーリストを **first-fit 線形走査**して再利用、なければ bump
- メトリクス: `gc_live_alloc_count` / `gc_freed_count` / `gc_collection_count` カウンタが
  契約テスト (`crates/lsharp-wasm/tests/e2e/gc_metrics_contract.rs` ほか) と
  CI artifact (`ci-artifacts/gc-metrics/`) で観測される

mark-sweep は 3 状態マーク (UNMARKED → PENDING → SCANNED)。
tagged handle 判別は上位 1 bit (`TAGGED_POINTER_MASK = 1<<63`、
`runtime-stability-spec.md:246` 周辺) で、G1 edge case の正本整理は同 spec :278-282。

## 設計

### 1. 容量の grow 戦略 (Phase A-3)

方針: **テーブル基底アドレスと容量を「定数」から「ランタイムヘッダ経由の間接参照」に変える。**

1. 線形メモリ先頭の予約領域 (既存の I/O バッファ群の後ろ) に runtime header を新設:
   `{root_stack_base, root_stack_capacity, obj_table_base, obj_table_capacity, free_list_base, free_list_capacity}` (i32 x 6)
2. 生成コードの該当箇所 (root_push/pop/set、__alloc、mark/sweep のテーブル走査) は
   定数 `*_CAPACITY` の直接埋め込みをやめ、ヘッダの load に置き換える。
   変更点は wasi.rs 内で上記ランタイム関数を emit している関数群に閉じる
3. grow 手順 (容量到達時):
   - `memory.grow` で新領域を確保 → 旧テーブルを新領域へコピー (`memory.copy`) →
     ヘッダの base/capacity を更新。テーブルは相互に独立なので個別に倍々で grow できる
   - root stack は「アドレスを保持される」性質がない (インデックス参照のみ) ため
     コピー移動して安全。オブジェクトテーブルも同様にスロットインデックスで参照されるため
     移動可能 (ヒープ本体のオブジェクトは動かさない — 既存の非ムーブ GC を維持)
4. 失敗時: `memory.grow` が -1 を返したら trap ではなく診断付きエラー
   (imp-02 の `LS4002`) として終了コードとメッセージを出す
5. 初期容量は既存定数のまま。**grow が発生しない限り、生成コードの動作・GC メトリクスは
   現行と一致する** (無回帰の根拠)

### 2. フリーリストのサイズクラス化 (Phase B-2)

- 現行: 単一フリーリストの first-fit 線形走査 (worst O(n))
- 変更: サイズクラス別リスト (16/32/64/128/256/512/1024/それ超) へ分割。
  割り当ては該当クラス先頭の pop (O(1))、クラス超過サイズのみ従来走査へフォールバック
- ヘッダにクラス別リスト先頭インデックスの配列を追加 (1. の header 拡張)
- 計測: alloc 時の走査ステップ数カウンタを追加し、`ci-artifacts/gc-metrics/` の
  summary に載せて改善前後を比較する

### 3. G1 precise discrimination の再評価 (Phase B-5、任意)

G1 (意図的に heap range へ入る i64 値の false-mark) は documented limitation を維持する。
1. のヘッダ間接化が入ると判別ビット拡張等の実装コストが下がるため、その完了時点で
「継続」か「精密判別実装」かを再評価する。判定の正本は runtime-stability-spec.md。

### 4. テスト戦略 (TDD)

1. RED: 同時生存オブジェクト > 4096 を作る E2E (深い cons リスト構築等) と
   root 深度 > 32768 の E2E を追加し、現行の失敗挙動を固定
2. GREEN: grow 実装で green 化
3. 既存 GC 契約テスト (gc_metrics_contract、collector telemetry、
   `test_e2e_alloc_metrics_ci_artifact_payload` 等) の全件 green を維持
4. grow 発生時のメトリクス単調性 (collection_count / freed_count) を契約テストへ追加
5. grow 上限到達時に LS4002 診断が返る E2E
6. bootstrap 固定点: 生成コードが変わるため stage chain の再生成・一致検証を実施

## 影響範囲

- 定数 → ヘッダ load の間接化で生成コードのサイズ・実行コストがわずかに増える
  (criterion ベンチ `crates/lsharp-wasm/benches/compiler_pipeline.rs` と
  GC メトリクスで定量化する)
- bootstrap 固定点の再生成が必要 (上記テスト 6)
- native backend (V2-13 系) のメモリレイアウトは別系統でありスコープ外
- wasmgc backend (imp-01) が完成すればこのランタイム自体が不要になる経路もあるが、
  linear backend がデフォルトである間は本改善が有効

## ステータス

設計 (2026-06-12 起草、同日コード検証に基づき具体化)。着手時は TODO.md に Phase A-3 / B-2 として項目を作成する。

2026-07-25 時点で Phase B-2 の Rust/WASI verified slice を実装した。`__alloc` は
16/32/64/128/256/512/1024 bytes と oversize の 8 class に分かれ、small class は
class head の pop、oversize は従来互換の first-fit scan を使う。free node は解放済み
block の payload 先頭 8 bytes (`next`, `capacity`) に保存し、GC sweep は object table
の physical capacity を読み直してから class を選ぶ。bump allocation は既存の
`heap_ptr` / telemetry ABI を保つため aligned requested size だけを物理容量とする。
`__lsharp_gc_free_list_scan_steps` で oversize scan を観測できる。

Evidence: `e2e::runtime_allocator_size_classes::test_e2e_runtime_allocator_reuses_small_blocks_without_linear_scan`、
`e2e::runtime_allocator_size_classes::test_e2e_runtime_allocator_uses_oversize_fallback_scan`、
`cargo check -p lsharp-wasm --tests`。これは Rust driver が生成した core-WASI Wasm の
verified slice であり、I-03 の動的 grow、HTTP/component parity、native stage0
(Mac/Linux)、CI artifact の scan-step 集計、D-10 sentinel の再評価は未完了とする。

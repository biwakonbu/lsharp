# P11-5: ランタイム安定化 + GC 仕様

最終更新: 2026-03-25

## 目的

長寿命プロセス (LSP, REPL, 連続 self-compile) で破綻しないランタイムを実現する。
bump allocator 前提の短命プロセス設計から脱却し、GC 導入に向けた基盤を固める。

---

## P11-5: ランタイム安定化 (本体)

### 仕様

#### S1. M1-M3 gate 再接続

`docs/memory-management-roadmap.md` の M1 (Collector 前提の整備), M2 (Mark-Sweep MVP), M3 (Performance Pass) を Phase 11 の gate として再接続する。

- M1 完了 = P11-5a の collector 導入ゲート完了
- M2 完了 = P11-5b の長寿命ワークロードで GC 有効テスト可能
- M3 完了 = P11-5c の観測メトリクスで性能回帰なしを確認可能

各 gate は後続タスクのブロッカーとして機能し、gate 未達で後続に進まない。

#### S2. GC-safe root 管理

compiler, LSP, REPL が共有するヒープオブジェクトに対して GC-safe root 管理を導入する。

- runtime API として `root_push`, `root_pop`, `root_set` を提供 (docs/runtime-spec.md P11-2c-2 に定義済み)
- GC 導入前は no-op 互換実装を提供し、compiler 側に条件分岐を持ち込まない
- GC-safe point は call site, loop backedge, runtime call 直前の 3 箇所 (docs/runtime-spec.md に定義済み)
- 例外/異常終了経路でも root stack が破壊されないことを保証する

#### S3. 長寿命測定

長寿命 LSP セッション、連続 REPL 実行、自己コンパイル反復で peak memory と回収挙動を測定する。

- 測定項目は P11-5c に定義する収集項目に準拠
- bump allocator 状態での baseline を先に取得し、GC 導入後と比較する
- 測定は CI の簡易モードと手元の詳細モードの 2 段階

#### S4. 完了条件

bump allocator 前提の短命プロセス設計を脱し、長寿命常駐でも破綻しない。具体的には P11-5d の 3 条件すべてを満たすこと。

---

## P11-5a: collector 導入ゲート

### 仕様

#### S5. M1-M3 smoke test 紐付け

Phase M1-M3 の各マイルストーンを compiler/LSP/REPL の smoke test と紐付ける。

| マイルストーン | smoke test 内容 |
|---------------|----------------|
| M1 (Collector 前提の整備) | 全ヒープ型が共通ヘッダを使用、trace 規約の型別検証、root 登録 API の unit test |
| M2 (Mark-Sweep MVP) | free list 回収の E2E、collector 有効で既存 E2E 全通過、collector 無限再入防止 |
| M3 (Performance Pass) | nursery allocation の throughput、write barrier の正しさ、GC 統計取得の検証 |

各 smoke test は `cargo test` (Rust 期) / `lsharp test` (selfhost 期) で実行可能であること。

#### S6. GC 切り替え API

GC 未導入モードと GC 有効モードを同一 API で切り替えられるようにし、比較実験を可能にする。

- runtime 初期化時のフラグで切り替え: `--gc=none` (bump only), `--gc=mark-sweep`, `--gc=generational`
- API 表面は同一。allocator 実装のみ差し替わる
- 切り替えは process 起動時のみ。実行中の動的切り替えは scope 外

#### S7. object header / trace map / root stack 仕様の再掲

backend 仕様書 (docs/native-backend-spec.md, docs/runtime-spec.md) へ再掲し、実装差分を禁止する。

- object header: `[tag: i32, size_or_words: i32, mark/state: i32, next_free_or_aux: i32]`
  - docs/memory-management-roadmap.md Phase 1 に定義済み
- trace map: 型ごとの子参照走査規約
  - String: payload は bytes、子参照なし
  - ADT/Record: フィールドごとに pointer/immediate を判定
  - Vector/HashMap: 要素スロットを走査
  - Closure: captured values を走査
  - Ref Cell: 内部値を走査
- root stack: shadow stack 方式。docs/runtime-spec.md P11-2c-2 に API 定義済み

Wasm backend と native backend で同一レイアウトを使用し、backend 固有の拡張は禁止する。

---

## P11-5b: 長寿命ワークロード

### 仕様

#### S8. 標準 longevity benchmark

以下を標準 longevity benchmark として固定する。

| ワークロード | 反復回数 | 測定対象 |
|-------------|---------|---------|
| 連続 format | 1,000 回 | peak RSS, heap bytes, GC pause |
| 連続 hover | 1,000 回 | peak RSS, heap bytes, GC pause |
| 連続 self-compile | 100 回 | peak RSS, heap bytes, full GC count |

各ワークロードは同一プロセス内で反復実行し、プロセス再起動を挟まない。

#### S9. LSP soak test

LSP セッションで open/change/diagnostics/hover/completion を繰り返す soak test を追加する。

- 最低 1,000 サイクルの open-edit-diagnostics-hover-completion シーケンス
- 各サイクル後に peak RSS を記録
- 単調増加が検出された場合は fail とする (P11-5d S14 の条件)
- テスト用プロジェクトは selfhost compiler 自身のソースを使用

#### S10. REPL GC 検証

REPL は stateful 実装に切り替える場合でも同じ GC 契約で回ることを別系統で検証する。

- 現行 REPL: 入力ごとに新 Wasm module/store を生成 → GC は不要
- stateful REPL: 単一インスタンス内でヒープを積み上げ → GC 必須
- 両モードで同一の root 管理 API を使用し、GC 有効/無効の切り替えが透過的であることを検証
- stateful REPL で 500 回の eval を実行し、メモリ回収を確認

---

## P11-5c: 観測と失敗解析

### 仕様

#### S11. 収集項目

以下のメトリクスを収集項目として固定する。

| メトリクス | 単位 | 収集タイミング |
|-----------|------|--------------|
| peak RSS | bytes | ワークロード完了時 |
| heap bytes | bytes | 各 GC サイクル前後 |
| live object count | 個 | 各 GC サイクル後 |
| GC pause time | milliseconds | 各 GC サイクル |
| full GC count | 回 | ワークロード完了時 |

#### S12. 2 段階観測

| レベル | 環境 | 内容 |
|-------|------|------|
| 簡易 | CI | peak RSS, full GC count, pass/fail 判定のみ |
| 詳細 | 手元ベンチ | 全メトリクス + object tag 別残存数 + GC pause histogram |

CI では実行時間を抑えるため反復回数を 1/10 に縮小してよい (format 100 回、hover 100 回、self-compile 10 回)。

#### S13. メモリリーク検知

メモリリーク検知時は object tag ごとの残存数を出力し、どの型が残ったか追えるようにする。

- GC sweep 時に tag 別の live count を集計
- リーク判定: N 回連続 GC 後に live count が単調増加している tag を検出
- 出力形式: `LEAK SUSPECT: tag=<tag_id> (<type_name>) count=<N> (+<delta> over <cycles> cycles)`
- 検出時は stderr に警告を出力し、CI では exit code 非 0 を返す

---

## P11-5d: 完了条件

### 仕様

#### S14. ヒープ単調増加なし

native LSP/REPL/compiler の長寿命実行でヒープが単調増加しない。

- 判定方法: P11-5b の各ワークロード完了後、最終 10% 区間の heap bytes が最大値を更新し続けていないこと
- 許容: GC サイクルごとの一時的な増加は許容。長期トレンドとして回収が追いついていること
- bump allocator モードでは本条件は適用しない (回収機構がないため)

#### S15. fixed-point 維持

collector 有効時も selfhost bootstrap の fixed-point が崩れない。

- stageN と stageN+1 の出力バイナリが bit-identical であること
- GC の非決定性 (allocation 順序の変動) が出力に影響しないことを保証
- GC 有効/無効両方で fixed-point テストを実行し、結果が一致すること

#### S16. GC クラッシュなし

GC 由来の既知クラッシュが TODO の open issue から消える。

- collector 起動時の SIGSEGV/trap/unreachable がゼロであること
- P11-5b の全ワークロードを GC 有効で完走できること
- root 漏れによる dangling pointer が検出されないこと (false negative = 0)

---

## GC ロードマップとの接続

### Precise Tracing GC (docs/memory-management-roadmap.md Phase 1-3)

P11-5 の中核。Phase 1 (GC 安全なオブジェクトレイアウト), Phase 2 (Root 集合の精密化), Phase 3 (First Collector) が P11-5a の collector 導入ゲートに直結する。

- Phase 1 完了 → S7 の object header/trace map 仕様が実装レベルで検証済み
- Phase 2 完了 → S2 の GC-safe root 管理が機能
- Phase 3 完了 → S8-S10 の長寿命ワークロードで GC 有効テストが可能

### 世代別 GC (docs/memory-management-roadmap.md Phase 4)

P11-5c の M3 (Performance Pass) に対応。First Collector 完了後に着手。

- young generation = bump allocator (現行 `__alloc` の fast path を再利用)
- old generation = non-moving mark-sweep
- minor GC で young のみ回収、write barrier/card marking で old 参照を追跡
- promotion は survivorship 回数またはサイズ閾値で決定
- 完了条件: minor GC が full GC より低コストであること

### Region 最適化 (docs/memory-management-roadmap.md Phase 5)

GC の補助最適化として段階導入。main memory manager にはしない。

- 対象: コンパイラ内部の一時データ、builtins 内の短命バッファ、単一式内の scratch object
- ユーザー可視の一般ヒープを region だけで支える設計にはしない
- 完了条件: region 導入が GC 正しさを壊さない、region 不使用でも同じ意味論を維持する

### WasmGC 最適化バックエンド (docs/memory-management-roadmap.md Phase 6)

optional backend として browser/対応ランタイム向け。mainline の置き換えではない。

- records/ADT/strings を優先移植
- linear memory mainline と同一 AST/型推論/IR を共有
- codegen と runtime ABI を backend ごとに分離
- 比較軸: バイナリサイズ, peak memory, 長寿命 throughput, browser/wasmtime 互換性
- mainline 置き換えは実測が linear memory collector を上回った場合にのみ再評価

---

## 依存関係

```
docs/memory-management-roadmap.md  -- GC 実装の詳細ロードマップ (Phase 0-6)
docs/runtime-spec.md               -- Runtime API, 値表現, Root 管理, GC-safe point
docs/native-backend-spec.md        -- Native backend 仕様
```

## 更新規則

- P11-5 のランタイム安定化仕様はこの文書に一本化する
- GC 実装の詳細は docs/memory-management-roadmap.md を正本とする
- Runtime API の仕様は docs/runtime-spec.md を正本とする
- TODO.md の P11-5 項目はこの文書の仕様番号 (S1-S16) に対応する

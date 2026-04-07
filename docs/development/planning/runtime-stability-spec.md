# P11-5: ランタイム安定化 + GC 仕様

最終更新: 2026-03-25

## 目的

長寿命プロセス (LSP, REPL, 連続 self-compile) で破綻しないランタイムを実現する。
bump allocator 前提の短命プロセス設計から脱却し、GC 導入に向けた基盤を固める。

---

## P11-5: ランタイム安定化 (本体)

### 仕様

#### S1. M1-M3 gate 再接続

`docs/development/planning/memory-management-roadmap.md` の M1 (Collector 前提の整備), M2 (Mark-Sweep MVP), M3 (Performance Pass) を Phase 11 の gate として再接続する。

- M1 完了 = P11-5a の collector 導入ゲート完了
- M2 完了 = P11-5b の長寿命ワークロードで GC 有効テスト可能
- M3 完了 = P11-5c の観測メトリクスで性能回帰なしを確認可能

各 gate は後続タスクのブロッカーとして機能し、gate 未達で後続に進まない。

#### S2. GC-safe root 管理

compiler, LSP, REPL が共有するヒープオブジェクトに対して GC-safe root 管理を導入する。

- runtime API として `root_push`, `root_pop`, `root_set` を提供 (docs/language/runtime-spec.md P11-2c-2 に定義済み)
- GC 導入前は no-op 互換実装を提供し、compiler 側に条件分岐を持ち込まない
- GC-safe point は call site, loop backedge, runtime call 直前の 3 箇所 (docs/language/runtime-spec.md に定義済み)
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

backend 仕様書 (docs/language/native-backend-spec.md, docs/language/runtime-spec.md) へ再掲し、実装差分を禁止する。

- object header: `[tag: i32, size_or_words: i32, mark/state: i32, next_free_or_aux: i32]`
  - docs/development/planning/memory-management-roadmap.md Phase 1 に定義済み
- trace map: 型ごとの子参照走査規約
  - String: payload は bytes、子参照なし
  - ADT/Record: フィールドごとに pointer/immediate を判定
  - Vector/HashMap: 要素スロットを走査
  - Closure: captured values を走査
  - Ref Cell: 内部値を走査
- root stack: shadow stack 方式。docs/language/runtime-spec.md P11-2c-2 に API 定義済み

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

### Precise Tracing GC (docs/development/planning/memory-management-roadmap.md Phase 1-3)

P11-5 の中核。Phase 1 (GC 安全なオブジェクトレイアウト), Phase 2 (Root 集合の精密化), Phase 3 (First Collector) が P11-5a の collector 導入ゲートに直結する。

- Phase 1 完了 → S7 の object header/trace map 仕様が実装レベルで検証済み
- Phase 2 完了 → S2 の GC-safe root 管理が機能
- Phase 3 完了 → S8-S10 の長寿命ワークロードで GC 有効テストが可能

### 世代別 GC (docs/development/planning/memory-management-roadmap.md Phase 4)

P11-5c の M3 (Performance Pass) に対応。First Collector 完了後に着手。

- young generation = bump allocator (現行 `__alloc` の fast path を再利用)
- old generation = non-moving mark-sweep
- minor GC で young のみ回収、write barrier/card marking で old 参照を追跡
- promotion は survivorship 回数またはサイズ閾値で決定
- 完了条件: minor GC が full GC より低コストであること

### Region 最適化 (docs/development/planning/memory-management-roadmap.md Phase 5)

GC の補助最適化として段階導入。main memory manager にはしない。

- 対象: コンパイラ内部の一時データ、builtins 内の短命バッファ、単一式内の scratch object
- ユーザー可視の一般ヒープを region だけで支える設計にはしない
- 完了条件: region 導入が GC 正しさを壊さない、region 不使用でも同じ意味論を維持する

### WasmGC 最適化バックエンド (docs/development/planning/memory-management-roadmap.md Phase 6)

optional backend として browser/対応ランタイム向け。mainline の置き換えではない。

- records/ADT/strings を優先移植
- linear memory mainline と同一 AST/型推論/IR を共有
- codegen と runtime ABI を backend ごとに分離
- 比較軸: バイナリサイズ, peak memory, 長寿命 throughput, browser/wasmtime 互換性
- mainline 置き換えは実測が linear memory collector を上回った場合にのみ再評価

---

## 依存関係

```
docs/development/planning/memory-management-roadmap.md  -- GC 実装の詳細ロードマップ (Phase 0-6)
docs/language/runtime-spec.md               -- Runtime API, 値表現, Root 管理, GC-safe point
docs/language/native-backend-spec.md        -- Native backend 仕様
```

## Sentinel / handle discrimination 監査メモ (CP-05 G1/G2)

L# runtime は heap value を 2 形式で持つ:

1. **raw heap address**: `i64` extended から作った素の i32 ポインタ。`__alloc` 直後など。
2. **tagged handle**: 上位 1 bit (`TAGGED_POINTER_MASK = 1<<63`) を立てた i64。

`crates/lsharp-wasm/src/wasi.rs` の `emit_gc_mark_candidate` (1672-1795) は次の二段階で
discrimination する:

```
if heap_start <= value < heap_ptr (signed)         -> raw pointer として mark
elif value >= TAGGED_POINTER_MASK (unsigned)
     && heap_start <= (value - mask) < heap_ptr    -> tagged handle として mark
else                                                -> skip (scalar / sentinel)
```

`0` / `-1` / `i64::MIN` / 通常の負整数はすべて skip され、`test_e2e_runtime_collector_ignores_legacy_zero_root_slot_sentinel` で固定済み。

### selfhost output parity (G2 監査結果)

selfhost-emitted コードは tag を以下の opcode で立てる:

- `vector-new` (opcode 54) — `selfhost/src/Backend/Wasm/WasmEmit.ls:710-721` (inline `i64.const i64::MIN + i64.add`)
- `vector-push` realloc (opcode 55) — `:568` `emit-vector-push-instr`
- `ref-new` (opcode 56) — `:570` `emit-ref-new-instr`
- `map-new` (opcode 60) — `:577` `emit-map-new-instr`

`string-concat` / `substring` / `read-file` / `command-line-arg` / `int-to-string` /
`read-stdin` は import call → Rust runtime helper が return 直前に
`emit_tagged_pointer_from_*` (`wasi.rs:103, 112`) を打って tag するため、selfhost 側で
追加 tag を打つ必要はない。string literal は data section 起算の raw i32 で、
collector の raw-range path で拾える。

**結論**: selfhost output と Rust output で tagging 規約は parity が取れている。
本節の修正は不要。

### 既知の理論的 edge case (G1)

ユーザーが `i64::MIN + N` (`heap_start <= N < heap_ptr`) という値を意図的に計算して
持つと、subtract 後 heap range に入って collector に false-marked される。実用上
発生する確率はゼロに近く、現状は **documented limitation** として扱う。

将来 precise discrimination を必要とする場合の選択肢:
- α: tag を `0xC000_0000_0000_0000` のような上位 2 bit パターンへ拡張、user int を i63 に制限
- β: inline tag を撤廃し object table の handle 経由のみで trace、raw-range path を撤廃

どちらも CP-05 G3 (compiler-side GC-safe point spill 完全列挙) より影響範囲が広いため、
G3 の進行と切り離して別 slice として扱う。

## CP-05 G3: GC-safe point spill 棚卸し

operand stack は collector から不可視のため、heap value を operand stack 上に
置いたまま allocation を跨ぐと、その値は trace 対象から漏れる。compiler 側で
shadow stack (`root_push` / `root_pop`) に spill する義務があるベィ。現状の
gap を以下 4 slice に分けて段階導入する:

### G3-a: 非自己再帰関数の heap param entry 時 root_push

- 対象: 全 user function の heap-typed parameter (String/Vector/Ref/Map/Closure/ADT/Record)
- 現状: self-TCO が効く関数のみ `SelfTcoRootOps` で entry root を打つ (`crates/lsharp-ir/src/lower/decl.rs:560-611`)
- 不足: 非自己再帰関数では param が unrooted のまま function body 内 alloc を通過
- 修正: `lower_function` entry 時に heap param を全 root_push、return path で root_pop
- 影響範囲最大、最優先

### G3-b: let heap-local の binding scope rooting

- 対象: `(let [x <heap-expr>] body)` の `x`
- 現状: binding 後に body 内で alloc が走ると `x` の local が unrooted
- 修正: heap 型 binding 時に root_push、scope 抜けで pop
- `lsharp-ir/src/lower/expr.rs` の Let 経路を改修

### G3-c: operand stack intermediate spill

- 対象: 二項演算/関数呼び出し引数列で、先に計算した heap value を後続 alloc 越しに保持するケース
- 現状: 一部 builtin 引数のみ `should_root_user_call_argument` で spill (line 1870)
- 不足: ユーザ定義関数の複合引数列、record-set / vector-set の value、closure 内自由変数の捕捉
- 修正: lowering 時に operand stack 上の heap intermediate を local + root_push に spill する general path

### G3-d: pattern match arm body の heap field rooting

- 対象: `(match scrutinee [(Cons h t) ...])` 等で取り出した heap field
- 現状: arm body が alloc を行うと bound field local が unrooted
- 修正: pattern bind 時に heap field を root_push、arm 抜けで pop

### 進行順

1. G3-a を TDD で実装 (RED test: 非自己再帰関数で heap param が GC で回収される回帰テスト)
2. G3-b → G3-c → G3-d の順
3. 各 slice 完了時に selfhost bootstrap fixed-point を再確認

### 棚卸し結果 (2026-04-07)

4 slice すべて RED test を書いたところ **全 GREEN** だったベィ。現状の実装で既に
カバーされていることが確認できた:

| slice | 現在の実装位置 | 状態 |
|-------|---------------|------|
| G3-a (非自己再帰 heap param) | `should_root_user_call_argument` (`crates/lsharp-ir/src/lower/expr.rs:1870`) が direct user call の各 arg を caller 側で root_push。param は caller frame の root stack 経由で transitively 保護される | GREEN |
| G3-b (let heap-local) | `emit_root_push_local` (`crates/lsharp-ir/src/lower/expr.rs:1823`) が heap 型 let binding を binding 時に root_push、scope 抜けで pop | GREEN |
| G3-c (multi-arg operand stack spill) | caller-side spill は arg ごとに root_push を打つ実装 (line 1270/1298)。先行 arg は後続 arg 評価中の GC を生き延びる | GREEN |
| G3-d (pattern match heap field) | match arm の field bind 時にも root が付与され、arm body の alloc を生き延びる | GREEN |

regression guard test は `crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs`
に 4 件追加済み (`test_e2e_runtime_collector_preserves_*`)。

**結論**: CP-05 G3 は新規実装不要。今後の構造改修 (例: 新しい lowering path や
新ビルトイン追加) で root が漏れた場合の retainer として、上記 4 guard test を
維持する。CP-05 のクローズは G1 (documented limitation) と合わせて完了扱いに
できるベィ。

## 更新規則

- P11-5 のランタイム安定化仕様はこの文書に一本化する
- GC 実装の詳細は docs/development/planning/memory-management-roadmap.md を正本とする
- Runtime API の仕様は docs/language/runtime-spec.md を正本とする
- TODO.md の P11-5 項目はこの文書の仕様番号 (S1-S16) に対応する

# L# メモリ管理ロードマップ

最終更新: 2026-03-24

## 目的

【方針】L# の mainline は、linear memory 上の `precise non-moving tracing GC` を採用する。  
【方針】`region inference` は main memory manager ではなく最適化として扱う。  
【方針】`WasmGC` は optional backend として維持し、mainline の値表現・ABI は linear memory 基盤を保持する。

## 現状認識

【事実】現在の L# は linear memory 上の bump allocator を使っている。`__alloc` はグローバル `heap_ptr` を前進させ、必要時に `memory.grow` するだけで解放は行わない。実装は `crates/lsharp-wasm/src/wasi.rs` の `emit_alloc_func` にある。  
【事実】文字列、ADT、Vector、HashMap、Ref Cell、クロージャはすべて linear memory に直接配置される。主要実装は `crates/lsharp-ir/src/lower/expr.rs`、`crates/lsharp-ir/src/lower/decl.rs`、`crates/lsharp-wasm/src/wasi.rs` にある。  
【事実】`IrType::Ref` と GC 命令群は IR 上に存在するが、現行 codegen では `i64` へのフォールバックと no-op 実装が残っている。実装は `crates/lsharp-wasm/src/emit.rs` にある。  
【事実】workspace のローカル依存は `wasmtime = "29"`、`wasm-encoder = "0.245"` である。`Cargo.toml` を参照。  
【事実】2026-03-24 時点の Wasmtime 最新ドキュメントでは `Config::wasm_gc` は既定で有効ではなく、collector の既定は deferred reference counting 系である。cycles を完全には回収しない。mainline の意味論を全面的に委ねる先としてはまだ弱い。  
【事実】現在の REPL は入力ごとに新しい Wasm module/store を生成して破棄する。したがって現行 REPL 自体は単一 Wasm インスタンス内でヒープを積み上げる構造ではない。ただし、長寿命 Wasm インスタンス、将来の stateful REPL、サーバーモードでは回収不能ヒープが問題になる。

## Selfhost rooting 規約

【規約】selfhost の heap 値 (`Vector` / `String` / record など) を割り当てを起こしうる呼び出しの前後で保持する場合、呼び出し前に `root_push` し、最後の使用後に対応する `root_pop` を行う。割り当てを起こしうるか判定しにくい呼び出しは、保守的に root する。

【規約】ループや shadowing された binding の中で root slot の値を更新する場合は `root_set` を使う。slot を先に作ってから allocating value を評価し、評価後に `root_set` で新しい heap 値へ更新する。内側の同名 binding が外側の slot を誤って解放しないよう、push/pop は lexical scope に対応させる。

【検証済み】`crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` の
`test_e2e_selfhost_root_set_preserves_shadowed_slot_during_allocating_value` は、旧値 `42` と allocating value の新値 `7` を shadowed root slot へ設定し、`7\n` を確認する。native stage-chain にも同じ fixture を保持する。

【残件】この guard は Rust/Wasm の current runtime bundle の証拠であり、Linux x86_64 の current-source native stage0、Mac Apple Silicon native artifact、lint による全 selfhost source の静的検査、GC stress mode の証拠ではない。

## 採用方針

### 1. Mainline: precise non-moving tracing GC

【提案】mainline の GC は `shadow stack + precise mark-sweep` を第一段階として導入する。  
【理由】現行の値表現は `i64` のタグ付きワードと linear memory 上のヒープオブジェクトを前提にしているため、moving GC より non-moving GC の方が差し込みやすい。  
【理由】既存のオブジェクトヘッダ `[tag: i32, size: i32, ...]` を拡張しやすく、Sweep 時の走査にも再利用できる。  
【理由】`__alloc`、文字列ランタイム、ADT lowering、Vector/HashMap、クロージャ変換を全面的に捨てずに進化できる。

### 2. Region inference: 最適化として後段導入

【提案】region inference は main memory manager としては採用しない。  
【理由】L# には既にクロージャ、可変参照、Vector/HashMap、共有 ADT があり、escape 解析と寿命境界の推論が設計の中心問題になる。  
【理由】HM 型推論との理論的整合性はあるが、実装難度に対して得られる即効性が低い。  
【提案】region は一時オブジェクト、コンパイラ内部ワーク領域、短命バッファ、式単位の scratch allocation の最適化として段階導入する。

### 3. WasmGC: optional backend

【提案】WasmGC は browser/対応ランタイム向けの optional backend として維持する。  
【理由】records/ADT/strings を WasmGC に自然に写像できる利点はある。  
【理由】ただし、mainline に採用するには値表現、呼出規約、ランタイム境界、テスト戦略を作り直す必要がある。これは「GC を追加する」ではなく「ABI と runtime representation を再設計する」作業である。  
【方針】WasmGC backend は性能・サイズ比較、および browser 配布向け最適化の文脈で評価する。

## 非目標

【非目標】Rust 式 borrow checker を L# の mainline に導入しない。  
【理由】S 式 + HM 推論 + 既存の言語 UX と整合しにくく、静的解析の複雑さが突出する。  
【非目標】reference counting を mainline collector にしない。  
【理由】cycle 問題を抱え、L# のクロージャ/Ref Cell/共有コレクションと相性が悪い。collector 自体の実装参考にはなるが、主方針にはしない。

## 実装ロードマップ

### Phase 0: 既存基盤の固定化

【事実】以下は実装済み。

- bump allocator (`__alloc`)
- タグ付きポインタ (`i64` MSB=1)
- オブジェクトヘッダ (`tag`, `size`)
- 文字列/ADT/Vector/HashMap/Ref Cell/クロージャの linear memory 配置

【完了条件】既存ヒープオブジェクトのレイアウトを文書化し、GC 導入前の正本をこの文書に集約する。

### Phase 1: GC 安全なオブジェクトレイアウトへ移行

【提案】全ヒープオブジェクトのヘッダを GC 用に統一する。

- `tag: i32`
- `size_or_words: i32`
- `mark/state: i32`
- `next_free_or_aux: i32`

【提案】型ごとに `trace` 規約を定義する。

- String: payload は bytes、子参照なし
- ADT/Record: フィールドごとに pointer/immediate を判定
- Vector/HashMap: 要素スロットを走査
- Closure: captured values を走査
- Ref Cell: 内部値を走査

【完了条件】新旧ヘッダの混在を禁止し、全ヒープ型が共通ヘッダを使う。

### Phase 2: Root 集合の精密化

【提案】`shadow stack` を導入する。GC 対象値を持つローカル・引数・一時値を明示的に root として登録する。  
【提案】WASI helper / builtin 関数境界でも root を失わないよう、呼出規約を GC-safe に変更する。  
【提案】スタックマシン IR のまま実装する場合は、GC が入り得る call site の前後で一時値をローカルへ spill して root 登録する。

【完了条件】

- collector 開始時に root set を完全列挙できる
- false positive は許容しても false negative は許容しない
- `memory.grow` と collector 起動が両立する

### Phase 3: First Collector

【提案】first collector は `precise non-moving mark-sweep`。

- allocation failure またはしきい値超過で GC 起動
- mark: shadow stack から到達可能オブジェクトを走査
- sweep: linear memory 上を走査して free list を再構築
- allocation: まず free list、失敗時は bump + `memory.grow`

【理由】moving を避けることで、既存の生ポインタ前提コードを大規模に壊さず導入できる。

【完了条件】

- 長寿命 Wasm インスタンスで使用量が回復する
- String/ADT/Vector/HashMap/Closure/Ref Cell を含む E2E を collector 有効状態で実行できる
- collector 無限再入を防ぐ

### Phase 4: Generational Optimization

【提案】young generation は bump allocator、old generation は non-moving mark-sweep とする。  
【提案】minor GC では young のみを回収し、old 参照には write barrier/card marking を導入する。  
【提案】promotion は survivorship 回数またはサイズ閾値で決める。

【理由】L# の一時オブジェクトは短命である可能性が高く、現行 bump allocator の速さを捨てずに済む。

【完了条件】

- minor GC が full GC より低コストである
- 既存 `__alloc` fast path を nursery allocation に再利用できる

### Phase 5: Region Optimization

【提案】region は次の用途に限定して段階導入する。

- コンパイラ内部の一時データ
- builtins 内の短命バッファ
- 単一式・単一関数内で escape しない scratch object

【提案】ユーザー可視の一般ヒープを region だけで支える設計にはしない。

【完了条件】

- region 導入が GC 正しさを壊さない
- region 不使用でも同じ意味論を維持する

### Phase 6: Optional WasmGC Backend

【提案】別バックエンドとして WasmGC 実装を進める。

- records/ADT/strings を優先移植
- linear memory mainline と同一 AST/型推論/IR を共有
- codegen と runtime ABI を backend ごとに分離

【提案】比較軸を固定する。

- バイナリサイズ
- peak memory
- 長寿命ワークロードでの throughput
- browser / wasmtime 互換性

【完了条件】mainline を置き換えるかどうかは、実測が linear memory collector を上回った場合にのみ再評価する。

## マイルストーン別タスク分解

### M1: Collector 前提の整備

- オブジェクトヘッダの統一
- ヒープオブジェクトごとの `trace` 仕様策定
- root 登録 API の導入
- GC-safe builtin 呼出規約の設計

### M2: Mark-Sweep MVP

- free list 実装
- mark bit/state 管理
- mark/sweep ループ
- allocation slow path
- E2E テスト追加

### M3: Performance Pass

- nursery 導入
- write barrier
- promotion policy
- GC 統計取得

### M4: Optional Backends

- region 最適化
- WasmGC backend
- benchmark と比較表更新

## 成功指標

【提案】最低限、以下を満たしたら mainline GC 導入成功とみなす。

- 同一 Wasm インスタンスで長時間動かしてもヒープが単調増加しない
- 現在の E2E 群が collector 有効状態で安定通過する
- バンプ allocator 単体比で大幅な回帰を起こさない
- builtins と user code の両方で pointer safety を維持する

## 更新規則

【方針】メモリ管理の正本はこの文書に一本化する。  
【方針】旧 `docs/type-system-roadmap.md` は廃止し、今後の GC/region/WasmGC 方針はこの文書だけを更新する。  
【方針】`TODO.md` の GC 項目はこの文書のマイルストーン名に合わせて管理する。

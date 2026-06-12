# imp-01: WasmGC 完全移行 (v2-07 補遺)

> 対象 issue: [D-01](../../../../ISSUES.md#d-01) (i64 フォールバック)、[D-02](../../../../ISSUES.md#d-02) (GADT)、
> [D-03](../../../../ISSUES.md#d-03) (HKT)、[D-04](../../../../ISSUES.md#d-04) (Computation Expression)、
> [D-06](../../../../ISSUES.md#d-06) (動的ディスパッチ)、[D-09](../../../../ISSUES.md#d-09) (selfhost ADT 表現)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase B-1
>
> **正本**: [v2-designs/v2-07-wasmgc-optional-backend.md](../v2-designs/v2-07-wasmgc-optional-backend.md)。
> 本書は v2-07 の方針 (バックエンド構成・型マッピング・優先順位・`--backend=wasmgc` フラグ) を変更せず、
> 「現行コードのどこを、どの順で、どう無回帰に置き換えるか」の補遺を与える。

## 現状の正確な把握 (2026-06-12 コード検証済み)

WasmGC への移行に必要な **IR レベルの語彙は既に存在する**。欠けているのは emit 層のみ:

| 層 | 現状 | 場所 |
|----|------|------|
| IR 型 | `IrType::Ref(u32)` が GC 型インデックスを保持済み | `crates/lsharp-ir/src/lib.rs` (IrType enum) |
| IR 命令 | `StructNew(u32)` / `StructGet(u32,u32)` / `StructSet(u32,u32)` / `RefCast(u32)` が定義済み | `crates/lsharp-ir/src/lib.rs:213-216` |
| IR モジュール | `Module.gc_types` フィールドが GC 型定義を保持する枠を持つ | `crates/lsharp-ir/src/cache.rs:9-15` (empty_module 参照) |
| 型変換 | `ir_to_wasm_valtype` が `Ref(_) => ValType::I64` へフォールバック | `crates/lsharp-wasm/src/emit.rs:16` |
| 命令 emit | `StructNew(_) => I64Const(0)` / `StructGet => nop` / `StructSet => Drop;Drop;I64Const(0)` / `RefCast => nop` のスタブ | `crates/lsharp-wasm/src/emit.rs:195-212` |
| 実動経路 | レコード/ADT の実体はリニアメモリ allocator + mark-sweep GC で動作 (`emit_wasm_wasi`) | `crates/lsharp-wasm/src/wasi.rs:102` (preview1), `:742` (component) |
| トレイト | マングル名 `TraitName_TypeName_methodName` による静的解決 (lowering 時) | `crates/lsharp-ir/src/lower/` |
| クロージャ | 自由変数解析 + lambda lifting | `crates/lsharp-ir/src/lower/closure.rs`, `lower/mod.rs` |

つまり `emit.rs:195-212` のスタブは「リニアメモリ経路では StructNew 系 IR 命令が
生成されない」前提で安全に nop 化されているもので、WasmGC backend はこの 4 命令を
本物の GC 命令に変換する emitter を新設すれば成立する。

## 依存バージョンの前提 (着手前の必須確認)

- workspace は `wasm-encoder = "0.245"` / `wasmtime = "29"` (`Cargo.toml:24-26`)
- リポジトリ内に `Config::wasm_gc(true)` 等の GC proposal 設定は存在しない (未使用)
- **Step 0 として確認すること**:
  1. wasm-encoder 0.245 の GC 型エンコード API (`SubType` / `CompositeType` / `StructType` /
     `FieldType` / `StorageType`) で必要な型セクションが出力できるか
  2. wasmtime 29 の `Config::wasm_gc(true)` で `struct.new` / `struct.get` / `ref.cast` を
     含むモジュールが実行できるか (最小 .wat を手書きして wasmtime API で実行する
     スパイクテストを 1 本書く)
  3. 不足する場合は wasmtime / wasm-encoder の更新を先行タスク化し、
     更新後に既存 E2E 全件 green を確認する

## 実装戦略

### Stage 0: backend フラグの配線

- CLI には既に 4 target (wasi-preview1 / wasi-component / web-wasm / native) の分岐が
  `crates/lsharp-driver/src/main.rs` の compile コマンド処理にある。ここに
  `--backend=wasmgc` (デフォルト: linear) を追加し、`lsharp-wasm` の emit 入口で分岐する
- 新規ファイル `crates/lsharp-wasm/src/wasmgc.rs` に `emit_wasm_wasmgc()` を新設
  (`emit_wasm_wasi` と同じシグネチャ。imp-06 のファイル分割方針に合わせ 800 行以内で分割)

### Stage 1: Records / ADT → struct 型

1. lowering: 現在リニアメモリ allocator 呼び出しへ落ちているレコード/ADT 構築・
   フィールドアクセスを、wasmgc backend 選択時は `StructNew` / `StructGet` /
   `StructSet` / `RefCast` IR 命令 + `Module.gc_types` への型登録で出力する
   (IR 命令は定義済みのため lowering の分岐追加のみ)
2. emit: `wasmgc.rs` で型セクションに struct 型を出力し、4 命令を
   `struct.new type_idx` / `struct.get type_idx field_idx` / `struct.set` / `ref.cast` へ変換。
   `ir_to_wasm_valtype` の wasmgc 版は `Ref(idx) => ValType::Ref(RefType { heap_type: Concrete(idx), nullable: true })`
3. ADT は v2-07 のとおり tagged struct (フィールド 0 にタグ i32、以降ペイロード)。
   パターンマッチの lowering はタグ読み出し (`StructGet(t, 0)`) + `RefCast` で分岐
4. 検証: `examples/gadt.ls` の「型チェックのみ」スタブを外し、
   `--backend=wasmgc` での実行 E2E を追加 (D-02 の resolved 条件)

### Stage 2: Strings → array i8

- 文字列リテラル/操作を `array.new_data` / `array.get_u` / `array.len` へ。
  WASI fd_write へ渡す際は array → リニアメモリへのコピーが必要
  (`array.copy` 不可のため要素ループまたは `array.init_data` の逆操作を helper 化)

### Stage 3: Closures → funcref + env struct

- 現行の lambda lifting (`lower/closure.rs`) は維持し、env をリニアメモリ tuple から
  struct 型へ置換。呼び出しは `call_ref` (typed funcref)。
  `IrType::FuncRef` は既存のため IR 拡張は env の型インデックス保持のみ
- 検証: `examples/hkt.ls` / `examples/computation.ls` の実行 E2E (D-03 / D-04 の resolved 条件)

### Stage 4: トレイト vtable

- 現行のマングル名静的解決 (`TraitName_TypeName_methodName`) は維持 (性能上有利)
- 追加で「trait object 型」を導入する場合のみ vtable struct
  (メソッドごとの funcref フィールド) + `call_ref` を生成。
  言語構文 (存在型) の設計は本書スコープ外とし、まず IR/emit の機構のみ用意する
  (D-06 の resolved 条件は「動的ディスパッチの最小 E2E」)

### Stage 5: selfhost ADT 表現の切替 (D-09)

- selfhost コンパイラ (selfhost/src/) の整数タグ + Vector 表現を struct ベースへ移行。
  bootstrap 固定点 (stage chain) の再生成・一致検証が必須のため、Stage 1-4 が
  安定した後に単独で計画する (本書では方針のみ固定)

## 無回帰戦略

- デフォルト backend は linear のまま。既存 E2E は全件無変更で走り続ける
- wasmgc backend の E2E は既存シナリオのパラメタライズ追加とし、
  stdout の byte 一致を両 backend で比較する差分テストを置く
- `emit.rs:195-212` のスタブは linear backend が StructNew 系命令を受け取らない限り
  そのまま残してよい。Stage 1 で lowering が分岐した後、linear 経路で当該命令が
  生成されないことを debug_assert で固定し、最終的に
  「wasmgc backend では `crates/lsharp-wasm/src/emit.rs` の TODO 4 件に到達しない」
  ことをもって D-01 を resolved とする

## 完了条件 (issue との対応)

| Stage | 解消 issue | 証跡 |
|-------|-----------|------|
| 1 | D-01 (主要部), D-02 | gadt.ls 実行 E2E、struct 型セクションのスナップショット |
| 2 | D-01 (残部) | 文字列系 E2E の両 backend green |
| 3 | D-03, D-04 | hkt.ls / computation.ls 実行 E2E |
| 4 | D-06 | 動的ディスパッチ最小 E2E |
| 5 | D-09 | bootstrap 固定点の wasmgc backend 一致 |

## ステータス

設計 (2026-06-12 起草、同日コード検証に基づき具体化)。
着手時は TODO.md に Phase B-1 として Stage 単位の項目を作成する。

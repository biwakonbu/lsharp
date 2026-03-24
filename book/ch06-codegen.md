# コード生成 -- WebAssembly バイナリを出力する

## WebAssembly とは

WebAssembly (Wasm) は、ブラウザ上で動作するバイナリ形式のプログラム表現である。元々はブラウザ向けだが、**WASI (WebAssembly System Interface)** の登場により、サーバーやエッジ環境でも実行可能になった。

L# は WASI をターゲットにしている。コンパイルされた `.wasm` ファイルは `wasmtime` で直接実行できる:

```bash
$ cargo run -- compile examples/fib.ls -o fib.wasm
$ wasmtime fib.wasm
55
```

## Wasm の構造

Wasm バイナリ (`.wasm`) はいくつかの**セクション**で構成される:

| セクション | 内容 |
|------------|------|
| Type | 関数の型シグネチャ |
| Import | 外部から取り込む関数 (WASI 等) |
| Function | 関数定義 (型インデックス) |
| Table | 間接呼び出し用テーブル (クロージャ用) |
| Memory | 線形メモリの定義 |
| Global | グローバル変数 (ヒープポインタ等) |
| Export | 外部に公開する関数 |
| Element | テーブル初期化データ |
| Code | 関数の実体 (命令列) |
| Data | メモリの初期データ |

各セクションはバイナリ内で順番に配置され、相互にインデックスで参照し合う。たとえば Function セクションの各エントリは Type セクション内の型シグネチャをインデックスで参照する。

## wasm-encoder API の詳細

L# は `wasm-encoder` クレートを使って Wasm バイナリを構築する (`crates/lsharp-wasm/src/wasi.rs`)。`wasm-encoder` はセクション単位で Wasm バイナリを組み立てる低レベル API を提供する。

### Module -- 全体のコンテナ

```rust
let mut wasm_module = wasm_encoder::Module::new();
```

`wasm_encoder::Module` が最上位のコンテナである。各セクションオブジェクトを構築した後、`section()` メソッドで追加していく。最終的に `finish()` メソッドでバイトベクタとして出力する。

### TypeSection -- 関数型シグネチャ

```rust
let mut types = TypeSection::new();

// 関数型: (i32, i32, i32, i32) -> i32
types.ty().function(vec![ValType::I32; 4], vec![ValType::I32]);

// GC 型は SubType として登録
types.ty().subtype(&SubType {
    is_final: true,
    supertype_idx: None,
    composite_type: CompositeType {
        inner: CompositeInnerType::Struct(StructType {
            fields: wasm_fields.into_boxed_slice(),
        }),
        shared: false,
        descriptor: None,
        describes: None,
    },
});

wasm_module.section(&types);
```

TypeSection には関数型シグネチャと GC 型の両方が格納される。GC 型が先に登録されるため、関数型のインデックスは GC 型の数だけオフセットされる。`types.len()` メソッドで現在のインデックスを取得しながら登録していく。

### ImportSection -- 外部関数のインポート

```rust
let mut imports = ImportSection::new();
imports.import(
    "wasi_snapshot_preview1",
    "fd_write",
    EntityType::Function(fd_write_type_idx),
);
```

`EntityType::Function` で型インデックスを指定する。import された関数は関数インデックス空間の先頭から順番に割り当てられる。

### FunctionSection -- 関数定義

```rust
let mut functions = FunctionSection::new();
functions.function(print_type_idx);  // ヘルパー関数
for &type_idx in &user_type_indices {
    functions.function(type_idx);    // ユーザー関数
}
functions.function(start_type_idx);  // _start
```

FunctionSection は関数の型インデックスのみを宣言する。関数本体は CodeSection に格納される。宣言順序と CodeSection 内の関数本体の順序は一致する必要がある。

### CodeSection -- 関数本体

```rust
let mut codes = CodeSection::new();

// ローカル変数宣言 + 命令列
let mut f = wasm_encoder::Function::new(vec![
    (1, ValType::I32),  // ローカル変数1つ: i32 型
    (1, ValType::I64),  // ローカル変数1つ: i64 型
]);
f.instruction(&wasm_encoder::Instruction::I64Const(42));
f.instruction(&wasm_encoder::Instruction::End);
codes.function(&f);

wasm_module.section(&codes);
```

`wasm_encoder::Function::new()` にはローカル変数の型と個数のペアを渡す。パラメータはローカル変数として暗黙的に含まれるため、明示的に宣言するのは追加のローカル変数のみである。

### MemorySection / DataSection -- 線形メモリ

```rust
let mut memories = MemorySection::new();
memories.memory(MemoryType {
    minimum: 1,       // 最小 1 ページ (64KB)
    maximum: None,    // 上限なし
    memory64: false,
    shared: false,
    page_size_log2: None,
});

let mut data = DataSection::new();
data.active(
    0,  // メモリインデックス
    &wasm_encoder::ConstExpr::i32_const(0),  // 開始アドレス
    b"\n".iter().copied(),  // データ
);
```

## emit_wasm_wasi の処理フロー

`emit_wasm_wasi` 関数は IR モジュール (`Module`) を受け取り、WASI 対応の Wasm バイナリを生成する。処理は以下の順序で進む。

### メモリレイアウト定数

線形メモリの先頭領域は以下のように固定的に使用される:

```
アドレス     用途                サイズ
─────────   ──────────────────  ──────
0  (NEWLINE_ADDR)    改行文字 '\n'          1 byte
16 (IOV_ADDR)        iovec 構造体           8 bytes (ptr: i32 + len: i32)
24 (NWRITTEN_ADDR)   書き込みバイト数        4 bytes
28~275               数値変換バッファ        248 bytes
276 (BUF_END)        バッファ末尾
512~                 文字列定数データ
ヒープ開始~          動的メモリ確保領域
```

`NEWLINE_ADDR` に改行文字が格納されており、`__print_i64` の最後に改行を出力する際にこのアドレスを参照する。`IOV_ADDR` は WASI の `fd_write` が要求する `iovec` 構造体 (バッファポインタ + 長さ) のために確保されている。

ヒープ開始位置は文字列定数のサイズに応じて動的に計算される:

```rust
let heap_start = ((512 + total_string_data_size) + 7) & !7;  // 8 バイトアラインメント
```

### 関数インデックスの割り当て

Wasm の関数インデックスは以下の順序で割り当てられる:

```
インデックス    関数
──────────     ─────────────────────
0              fd_write          (WASI import)
1              proc_exit         (WASI import)
2              args_get          (WASI import)
3              args_sizes_get    (WASI import)
4              fd_read           (WASI import)
5              fd_close          (WASI import)
6              path_open         (WASI import)
7              fd_seek           (WASI import)
8              fd_filestat_get   (WASI import)
──── ここまで WASI import (WASI_IMPORT_COUNT = 9) ────
9              __print_i64       (IR ヘルパー)
10             __alloc           (IR ヘルパー)
11             __string_concat   (IR ヘルパー)
12             __string_eq       (IR ヘルパー)
13             __print_string    (IR ヘルパー)
14             __int_to_string   (IR ヘルパー)
15             __read_file       (IR ヘルパー)
16             __write_file      (IR ヘルパー)
17             __file_exists     (IR ヘルパー)
18             __command_line_args (IR ヘルパー)
19             __fnv1a_hash      (IR ヘルパー)
──── ここまでヘルパー関数 (user_func_base = 20) ────
20             ユーザー関数 0
21             ユーザー関数 1
...
20+N-1         ユーザー関数 N-1
20+N           _start            (エントリポイント)
```

IR 側では関数インデックスが `IR_IMPORT_COUNT (12)` からの相対値で管理されるため、`emit_instructions_wasi` でユーザー関数のインデックスを Wasm 側の `user_func_base + (ir_idx - IR_IMPORT_COUNT)` にリマップする。

### セクション生成の順序

`emit_wasm_wasi` は以下の順序でセクションを組み立てる:

1. **Type Section**: GC 型定義 → WASI 関数型 → ヘルパー関数型 → ユーザー関数型 → `_start` 型 → `CallIndirect` 用の型
2. **Import Section**: 9 個の WASI 関数
3. **Function Section**: ヘルパー関数 → ユーザー関数 → `_start`
4. **Table Section**: クロージャ使用時のみ (`CallIndirect` が存在する場合)
5. **Memory Section**: 1 ページ (64KB)、上限なし
6. **Global Section**: ヒープポインタ (`heap_start` で初期化、mutable)
7. **Export Section**: `memory` と `_start`
8. **Element Section**: クロージャ使用時のみ (テーブルの初期化)
9. **Code Section**: 全関数の本体
10. **Data Section**: 改行文字 + 文字列定数

## IR 命令から Wasm 命令への変換

IR 命令は `crates/lsharp-wasm/src/emit.rs` の `emit_instructions_common` 関数で Wasm 命令に変換される。変換は大きく分けて以下のカテゴリに分類される。

### 定数・ローカル変数

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `I64Const(n)` | `i64.const n` | 64bit 整数定数 |
| `F64Const(n)` | `f64.const n` | 64bit 浮動小数点定数 |
| `I32Const(n)` | `i32.const n` | 32bit 整数定数 |
| `LocalGet(i)` | `local.get i` | ローカル変数読み取り |
| `LocalSet(i)` | `local.set i` | ローカル変数書き込み |
| `LocalTee(i)` | `local.tee i` | 書き込み + スタックに残す |

### 算術・比較演算

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `I64Add` | `i64.add` | 整数加算 |
| `I64Sub` | `i64.sub` | 整数減算 |
| `I64Mul` | `i64.mul` | 整数乗算 |
| `I64Div` | `i64.div_s` | 整数除算 (符号付き) |
| `I64Rem` | `i64.rem_s` | 整数剰余 (符号付き) |
| `F64Add` | `f64.add` | 浮動小数点加算 |
| `F64Sub` | `f64.sub` | 浮動小数点減算 |
| `F64Mul` | `f64.mul` | 浮動小数点乗算 |
| `F64Div` | `f64.div` | 浮動小数点除算 |
| `I64Eq` | `i64.eq` | 整数等値比較 (結果: i32) |
| `I64Ne` | `i64.ne` | 整数非等値比較 |
| `I64LtS` | `i64.lt_s` | 未満 (符号付き) |
| `I64GtS` | `i64.gt_s` | 超過 (符号付き) |
| `I64LeS` | `i64.le_s` | 以下 (符号付き) |
| `I64GeS` | `i64.ge_s` | 以上 (符号付き) |

### 論理・ビット演算

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `I32Eqz` | `i32.eqz` | ゼロ判定 (i32) |
| `I32And` | `i32.and` | ビット AND (i32) |
| `I32Or` | `i32.or` | ビット OR (i32) |
| `I64And` | `i64.and` | ビット AND (i64) |
| `I64Or` | `i64.or` | ビット OR (i64) |
| `I64Xor` | `i64.xor` | ビット XOR (i64) |
| `I32Shl` | `i32.shl` | 左シフト (i32) |
| `I32ShrU` | `i32.shr_u` | 右シフト 符号なし (i32) |
| `I64Shl` | `i64.shl` | 左シフト (i64) |
| `I64ShrU` | `i64.shr_u` | 右シフト 符号なし (i64) |

### 型変換

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `I64ExtendI32S` | `i64.extend_i32_s` | i32 → i64 (符号拡張) |
| `I64ExtendI32U` | `i64.extend_i32_u` | i32 → i64 (ゼロ拡張) |
| `I32WrapI64` | `i32.wrap_i64` | i64 → i32 (切り捨て) |

### 制御フロー

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `Call(i)` | `call i` | 直接関数呼び出し |
| `CallImport(i)` | `call i` (リマップ後) | import 関数呼び出し |
| `If(ty)` | `if (result ty)` | 条件分岐 (結果型付き) |
| `IfEmpty` | `if` | 条件分岐 (結果型なし) |
| `Else` | `else` | else 節 |
| `End` | `end` | ブロック終端 |
| `Block(ty)` | `block (result ty)` | ブロック開始 |
| `BlockEmpty` | `block` | ブロック開始 (結果型なし) |
| `Loop(ty)` | `loop (result ty)` | ループ開始 |
| `LoopEmpty` | `loop` | ループ開始 (結果型なし) |
| `Br(n)` | `br n` | 無条件分岐 |
| `BrIf(n)` | `br_if n` | 条件分岐 |
| `Return` | `return` | 関数から返る |
| `Unreachable` | `unreachable` | 到達不能 (トラップ) |
| `Drop` | `drop` | スタックトップを破棄 |

### メモリ操作

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `I32Load { offset }` | `i32.load offset=N` | 32bit 読み取り |
| `I32Store { offset }` | `i32.store offset=N` | 32bit 書き込み |
| `I32Load8U { offset }` | `i32.load8_u offset=N` | 8bit 読み取り (符号なし) |
| `I32Store8 { offset }` | `i32.store8 offset=N` | 8bit 書き込み |
| `I64Load { offset }` | `i64.load offset=N` | 64bit 読み取り |
| `I64Store { offset }` | `i64.store offset=N` | 64bit 書き込み |
| `MemoryGrow` | `memory.grow` | メモリ拡張 |
| `MemorySize` | `memory.size` | メモリサイズ取得 |
| `MemoryCopy` | `memory.copy` | メモリコピー |
| `MemoryFill` | `memory.fill` | メモリ充填 |

### GC 命令 (WasmGC)

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `StructNew(type_idx)` | `struct.new type_idx` | GC struct の生成 |
| `StructGet(type_idx, field_idx)` | `struct.get type_idx field_idx` | struct フィールド読み取り |
| `StructSet(type_idx, field_idx)` | `struct.set type_idx field_idx` | struct フィールド書き込み |
| `RefCast(type_idx)` | `ref.cast type_idx` | ダウンキャスト |
| `RefFunc(func_idx)` | `ref.func func_idx` | 関数参照の取得 |
| `CallRef(type_idx)` | `call_ref type_idx` | funcref 経由の間接呼び出し |

### グローバル変数・間接呼び出し

| IR 命令 | Wasm 命令 | 説明 |
|---------|-----------|------|
| `GlobalGet(idx)` | `global.get idx` | グローバル変数読み取り |
| `GlobalSet(idx)` | `global.set idx` | グローバル変数書き込み |
| `CallIndirect(param_count)` | `call_indirect type_idx, 0` | テーブル経由の間接呼び出し |
| `FuncIdx(idx)` | `i32.const idx` | 関数インデックスをスタックに積む |
| `StringConst(idx)` | (インライン展開済み) | 文字列定数参照 |

`CallImport` は IR 側のインデックスから Wasm 側のインデックスへのリマップが必要である。リマップは `emit_instructions_wasi` 内のクロージャで行われる。

## __print_i64 のアルゴリズム

`__print_i64` は i64 の値を 10 進文字列に変換して標準出力に書き出す関数である。ランタイムライブラリを持たない L# において、この関数は Wasm 命令で直接実装される。

### アルゴリズムの概要

```
入力: value (i64)
ローカル変数: buf_pos (i32), is_neg (i32), abs_val (i64)

1. buf_pos = BUF_END (= 276)   // バッファ末尾から書き始める
2. abs_val = value
3. is_neg = 0

4. if value < 0:
     is_neg = 1
     abs_val = 0 - value        // 絶対値を取る

5. if abs_val == 0:
     buf_pos -= 1
     memory[buf_pos] = '0' (48)
   else:
     while abs_val != 0:
       buf_pos -= 1
       digit = abs_val % 10
       memory[buf_pos] = digit + '0' (48)
       abs_val = abs_val / 10

6. if is_neg:
     buf_pos -= 1
     memory[buf_pos] = '-' (45)

7. iovec.ptr = buf_pos
   iovec.len = BUF_END - buf_pos
   fd_write(stdout=1, iovec_addr, iovec_count=1, nwritten_addr)

8. iovec.ptr = NEWLINE_ADDR (= 0)
   iovec.len = 1
   fd_write(stdout=1, iovec_addr, iovec_count=1, nwritten_addr)
```

### Wasm 命令レベルの擬似コード

以下は実際の Wasm 命令に近い擬似コードである:

```wasm
;; パラメータ: $value (i64)
;; ローカル: $buf_pos (i32), $is_neg (i32), $abs_val (i64)

;; ステップ 1-3: 初期化
i32.const 276          ;; BUF_END
local.set $buf_pos
local.get $value
local.set $abs_val
i32.const 0
local.set $is_neg

;; ステップ 4: 負数チェック
local.get $value
i64.const 0
i64.lt_s
if
  i32.const 1
  local.set $is_neg
  i64.const 0
  local.get $value
  i64.sub
  local.set $abs_val
end

;; ステップ 5: 桁変換ループ
local.get $abs_val
i64.eqz
if                     ;; ゼロの場合
  local.get $buf_pos
  i32.const 1
  i32.sub
  local.set $buf_pos
  local.get $buf_pos
  i32.const 48         ;; '0'
  i32.store8
else                   ;; 非ゼロの場合
  block
    loop
      local.get $abs_val
      i64.eqz
      br_if 1          ;; abs_val == 0 なら抜ける
      ;; 末尾から 1 桁ずつ書き込む
      local.get $buf_pos
      i32.const 1
      i32.sub
      local.set $buf_pos
      local.get $buf_pos
      local.get $abs_val
      i64.const 10
      i64.rem_u        ;; 下 1 桁
      i32.wrap_i64
      i32.const 48     ;; + '0'
      i32.add
      i32.store8
      local.get $abs_val
      i64.const 10
      i64.div_u        ;; 次の桁へ
      local.set $abs_val
      br 0             ;; ループ継続
    end
  end
end

;; ステップ 6: マイナス記号
local.get $is_neg
if
  local.get $buf_pos
  i32.const 1
  i32.sub
  local.set $buf_pos
  local.get $buf_pos
  i32.const 45         ;; '-'
  i32.store8
end

;; ステップ 7: fd_write で数値を出力
i32.const 16           ;; IOV_ADDR
local.get $buf_pos
i32.store              ;; iovec.ptr = buf_pos
i32.const 20           ;; IOV_ADDR + 4
i32.const 276          ;; BUF_END
local.get $buf_pos
i32.sub
i32.store              ;; iovec.len = BUF_END - buf_pos
i32.const 1            ;; stdout
i32.const 16           ;; IOV_ADDR
i32.const 1            ;; iovec_count
i32.const 24           ;; NWRITTEN_ADDR
call $fd_write
drop

;; ステップ 8: 改行を出力
i32.const 16           ;; IOV_ADDR
i32.const 0            ;; NEWLINE_ADDR
i32.store              ;; iovec.ptr = 0 (改行文字のアドレス)
i32.const 20           ;; IOV_ADDR + 4
i32.const 1
i32.store              ;; iovec.len = 1
i32.const 1            ;; stdout
i32.const 16           ;; IOV_ADDR
i32.const 1            ;; iovec_count
i32.const 24           ;; NWRITTEN_ADDR
call $fd_write
drop
```

この「バッファ末尾から逆順に桁を書き込む」方式により、桁数をあらかじめ計算する必要がなくなる。変換後に `buf_pos` から `BUF_END` までが数値の文字列表現になっている。

## GC セクションの生成

WasmGC の型定義は TypeSection の先頭に配置される。IR の `GcTypeDef` 構造体が Wasm の GC 型セクションに変換される。

### GcTypeDef の構造

```rust
/// GC 型定義（WasmGC struct/array 用）
pub struct GcTypeDef {
    pub name: String,        // 型名 (例: "Point", "Option.Some")
    pub kind: GcTypeKind,    // Struct または Array
}

pub enum GcTypeKind {
    Struct(Vec<GcField>),    // 構造体型 (レコード, ADT バリアント)
    Array(IrType),           // 配列型 (文字列等)
}

pub struct GcField {
    pub name: String,        // フィールド名
    pub ty: IrType,          // フィールドの IR 型
    pub mutable: bool,       // 可変フラグ
}
```

### Struct 型の変換

```rust
for gc_type in &module.gc_types {
    match &gc_type.kind {
        GcTypeKind::Struct(fields) => {
            let wasm_fields: Vec<FieldType> = fields.iter()
                .map(|f| FieldType {
                    element_type: StorageType::Val(ir_to_wasm_valtype(f.ty)),
                    mutable: f.mutable,
                })
                .collect();
            types.ty().subtype(&SubType {
                is_final: true,
                supertype_idx: None,
                composite_type: CompositeType {
                    inner: CompositeInnerType::Struct(StructType {
                        fields: wasm_fields.into_boxed_slice(),
                    }),
                    shared: false,
                    descriptor: None,
                    describes: None,
                },
            });
        }
        // ...
    }
}
```

たとえば L# のレコード型 `(type Point (record (: x Float) (: y Float)))` は以下のように変換される:

```
IR GcTypeDef:
  name: "Point"
  kind: Struct([
    GcField { name: "x", ty: F64, mutable: false },
    GcField { name: "y", ty: F64, mutable: false },
  ])

Wasm Type Section:
  (type $Point (sub final (struct
    (field $x (mut f64))
    (field $y (mut f64)))))
```

### Array 型の変換

配列型は文字列やリスト等に使用される:

```rust
GcTypeKind::Array(elem_ty) => {
    types.ty().subtype(&SubType {
        is_final: true,
        supertype_idx: None,
        composite_type: CompositeType {
            inner: CompositeInnerType::Array(ArrayType(FieldType {
                element_type: StorageType::Val(ir_to_wasm_valtype(*elem_ty)),
                mutable: true,
            })),
            shared: false,
            descriptor: None,
            describes: None,
        },
    });
}
```

### IR 型から Wasm 型への変換

`ir_to_wasm_valtype` 関数 (`crates/lsharp-wasm/src/emit.rs`) が IR 型を Wasm の `ValType` に変換する:

```rust
pub fn ir_to_wasm_valtype(ty: IrType) -> ValType {
    match ty {
        IrType::I64 => ValType::I64,
        IrType::F64 => ValType::F64,
        IrType::I32 => ValType::I32,
        IrType::Ref(_) => ValType::I64,     // MVP: GC 参照は i64 にフォールバック
        IrType::FuncRef => ValType::FUNCREF,
    }
}
```

現在の MVP 実装では `IrType::Ref` は `i64` にフォールバックしている。WasmGC 本格対応時には `ValType::Ref(RefType { nullable: true, heap_type: HeapType::Concrete(idx) })` に変換される予定である。

## _start エントリポイント

WASI ではプログラムのエントリポイントとして `_start` 関数がエクスポートされる:

```wasm
(func $_start
  call $main    ;; ユーザーの main 関数を呼び出す
  drop          ;; 戻り値を破棄
)
(export "_start" (func $_start))
```

`main` 関数が `Unit` (i64 の 0) を返すが、`_start` は戻り値を持たないため `drop` で破棄する。`main` 関数が見つからない場合、`_start` は空の関数になる。

## コンパイルパイプライン全体

L# ドライバー (`crates/lsharp-driver/src/main.rs`) が以下のパイプラインを実行する:

```
1. ソースファイルを読み込む
2. Lexer.tokenize()     → トークン列
3. Parser.parse_program() → AST
4. Infer.infer_program()  → 型チェック済み環境
5. Lower.lower_program()  → IR モジュール
6. emit_wasm_wasi()       → Wasm バイナリ
7. ファイルに書き出す
```

各段階が独立したクレートに分離されているため、テストや拡張が容易である。

## テスト戦略

Wasm コード生成のテストは 2 層で行う:

### 1. バイナリ生成テスト

```rust
#[test]
fn test_codegen_arithmetic() {
    let wasm = compile("(defn main [] (print (+ 1 2)))");
    assert!(!wasm.is_empty());  // バイナリが生成される
}
```

### 2. E2E 実行テスト

`wasmtime` を使って実際に Wasm を実行し、出力を検証する:

```rust
#[test]
fn test_wasi_fib() {
    let output = compile_and_run(
        "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
         (defn main [] (print (fib 10)))"
    );
    assert_eq!(output.trim(), "55");
}
```

これにより、パイプライン全体の正しさを一気通貫で検証できる。

## まとめ

L# のコード生成は以下の特徴を持つ:

- **WASI ターゲット**: ブラウザ外でも実行可能
- **IR との体系的な対応**: 60 以上の IR 命令が Wasm 命令に変換される
- **自己完結**: ランタイムライブラリなし、必要な機能 (`__print_i64`, `__alloc`, `__string_concat` 等) は Wasm 内に埋め込み
- **wasm-encoder**: Rust クレートによる安全なバイナリ構築
- **段階的な関数インデックス管理**: WASI import → IR ヘルパー → ユーザー関数の 3 層構造

## WasmGC への移行

現在の L# コンパイラは全ての値を `i64` や `f64` のプリミティブ型で表現している (MVP 方式)。しかし、レコード型や ADT を効率的に扱うには **WasmGC** (Garbage Collection 拡張) が必要になる。

WasmGC は 2025 年時点で全主要ブラウザ・ランタイムで安定サポート済みである:

- Chrome v119+, Firefox v120+, Safari v18.2+
- wasmtime, wasmer v6.0+ でフルサポート

L# の IR には `StructNew`, `StructGet`, `StructSet`, `RefCast` といった GC 命令が既に定義されている (第 5 章参照)。現在は MVP として `i64` にフォールバックしているが、今後 `wasm-encoder` の GC API (`StructType`, `ArrayType`, `SubType`) を使って本格的な WasmGC コード生成に移行する計画である。

これで L# コンパイラの「ソースコードから実行可能バイナリまで」のパイプライン全体を見てきた。次章からは、型システムの拡張ロードマップに踏み込んでいく。

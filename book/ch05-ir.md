# 中間表現 -- AST から命令列へ

## なぜ中間表現が必要か

型チェック済みの AST を直接 WebAssembly に変換することもできるが、**中間表現 (Intermediate Representation, IR)** を挟むことで大きな利点が得られる:

1. **関心の分離**: AST は「言語の構文」を、IR は「実行の意味」を表現する
2. **最適化の基盤**: IR レベルで定数畳み込みやデッドコード除去を行える
3. **複数ターゲット**: 同じ IR から Wasm、ネイティブコード、LLVM IR など複数の出力に変換できる

## L# の IR 設計

L# の IR はスタックマシンベースのフラットな命令列である (`crates/lsharp-ir/src/lib.rs`)。WebAssembly 自体がスタックマシンなので、変換が自然になる:

```rust
pub struct Module {
    pub functions: Vec<Function>,
    pub gc_types: Vec<GcTypeDef>,  // GC 型定義 (WasmGC struct 用)
    pub imports: Vec<ImportFunc>,   // import 関数定義
}

pub struct Function {
    pub name: String,
    pub params: Vec<IrType>,
    pub result: IrType,
    pub locals: Vec<IrType>,    // ローカル変数の型
    pub body: Vec<Instruction>, // フラットな命令列
    pub is_export: bool,
}
```

`gc_types` はレコード型や ADT の WasmGC 構造体定義を格納する。`imports` は WASI の `fd_write` のような外部関数のシグネチャを保持する。

### IR の型

IR は 4 つの型をサポートする:

```rust
pub enum IrType {
    I64,      // 64bit 整数 (Int, Bool)
    F64,      // 64bit 浮動小数点 (Float)
    I32,      // 32bit 整数 (比較結果、制御フロー)
    Ref(u32), // GC 参照型 (WasmGC struct/array への参照)
}
```

`Bool` は `I64` で表現する (0 = false, 1 = true)。`Ref(u32)` は WasmGC の構造体 (struct) や配列 (array) への参照を表す。引数の `u32` は GC 型定義のインデックスで、レコード型や ADT のバリアントに対応する。

### 命令セット

IR の命令は WebAssembly の命令と概ね対応する:

```rust
pub enum Instruction {
    // 定数をスタックに積む
    I64Const(i64),    // 整数定数
    F64Const(f64),    // 浮動小数点定数

    // ローカル変数操作
    LocalGet(u32),    // ローカル変数の値をスタックに積む
    LocalSet(u32),    // スタックの値をローカル変数に格納
    LocalTee(u32),    // 値を格納しつつスタックにも残す

    // 算術演算 (スタックから2値を取り、結果を積む)
    I64Add, I64Sub, I64Mul, I64Div, I64Rem,
    F64Add, F64Sub, F64Mul, F64Div,

    // 比較 (結果は i32: 0 or 1)
    I64Eq, I64Ne, I64LtS, I64GtS, I64LeS, I64GeS,

    // 制御フロー
    Call(u32),        // 関数呼び出し
    CallImport(u32),  // import 関数呼び出し
    If(IrType),       // 条件分岐
    Else, End,
    Block(IrType),    // ブロック
    Loop(IrType),     // ループ
    Br(u32),          // 無条件分岐
    BrIf(u32),        // 条件分岐
    Return,
    Unreachable,
    Drop,

    // GC 命令 (WasmGC -- 第 7 章で詳述)
    StructNew(u32),         // struct.new type_idx
    StructGet(u32, u32),    // struct.get type_idx field_idx
    StructSet(u32, u32),    // struct.set type_idx field_idx
    RefCast(u32),           // ref.cast type_idx (ダウンキャスト)
}
```

GC 命令はレコード型と ADT の WasmGC 表現に使用される。`StructNew` はフィールド値をスタックから取得して GC ヒープに構造体を割り当て、`StructGet`/`StructSet` はフィールドの読み書きを行う。`RefCast` は ADT のパターンマッチでバリアントのダウンキャストに使う。

## スタックマシンの動作

IR はスタックマシンとして動作する。`(+ 1 2)` の命令列を追跡してみよう:

```
命令             スタック (底 → 頂)
─────────────   ─────────────────
i64.const 1     [1]
i64.const 2     [1, 2]
i64.add         [3]
```

1. `i64.const 1`: 値 1 をスタックに積む
2. `i64.const 2`: 値 2 をスタックに積む
3. `i64.add`: スタックから 2 値を取り出し、加算結果 3 を積む

ネストした式 `(+ (* 2 3) 4)` も自然に処理される:

```
命令             スタック
─────────────   ─────────────────
i64.const 2     [2]
i64.const 3     [2, 3]
i64.mul         [6]
i64.const 4     [6, 4]
i64.add         [10]
```

## Lowering -- AST から IR への変換

AST を IR に変換する処理を **lowering (降位)** と呼ぶ (`crates/lsharp-ir/src/lower.rs`)。

### 変換コンテキスト

各関数の変換中、ローカル変数のマッピングを管理する:

```rust
struct FuncCtx {
    name: String,
    locals_map: HashMap<String, u32>,  // 変数名 → インデックス
    locals: Vec<IrType>,               // 追加ローカル変数の型
    body: Vec<Instruction>,            // 生成された命令列
    param_count: u32,                  // パラメータ数
    next_local: u32,                   // 次のローカル変数インデックス
}
```

### リテラルの変換

最も単純な変換:

```rust
Expr::Lit(_, Literal::Int(n))   => ctx.emit(Instruction::I64Const(n));
Expr::Lit(_, Literal::Float(n)) => ctx.emit(Instruction::F64Const(n));
Expr::Lit(_, Literal::Bool(b))  => ctx.emit(Instruction::I64Const(b as i64));
Expr::Lit(_, Literal::Unit)     => ctx.emit(Instruction::I64Const(0));
```

`Bool` は `I64` にマッピングされ、`true` は 1、`false` は 0 になる。`Unit` も 0 で表現する。

### 変数参照

```rust
Expr::Var(_, name) => {
    let idx = ctx.locals_map[name];
    ctx.emit(Instruction::LocalGet(idx));
}
```

変数名をローカル変数のインデックスに解決し、`LocalGet` 命令を発行する。

### 算術式

```rust
// (+ x y) の変換
Expr::App(_, func, args) if func == "+" => {
    self.lower_expr(ctx, &args[0])?;  // x をスタックに
    self.lower_expr(ctx, &args[1])?;  // y をスタックに
    ctx.emit(Instruction::I64Add);     // 加算
}
```

S 式の `(op arg1 arg2)` が自然にスタックマシンの「引数を積んでから演算」に変換される。

### if 式

```rust
// (if cond then else) の変換
Expr::If(_, cond, then, else_) => {
    self.lower_expr(ctx, cond)?;       // 条件を評価
    ctx.emit(Instruction::I32WrapI64); // i64 → i32 に変換
    ctx.emit(Instruction::If(result_type));
    self.lower_expr(ctx, then)?;       // then 節
    ctx.emit(Instruction::Else);
    self.lower_expr(ctx, else_)?;      // else 節
    ctx.emit(Instruction::End);
}
```

Wasm の `if` 命令は条件を `i32` で受け取るため、`Bool` (i64) から `i32` への変換が必要になる。

### let 束縛

```rust
// (let [x 10 y 20] (+ x y)) の変換
Expr::Let(_, bindings, body) => {
    for (pat, val) in bindings {
        self.lower_expr(ctx, val)?;      // 値を評価
        let idx = ctx.alloc_local(IrType::I64);  // ローカル変数を確保
        ctx.emit(Instruction::LocalSet(idx));      // 値を格納
        ctx.locals_map.insert(name, idx);          // 名前を登録
    }
    self.lower_expr(ctx, body)?;  // 本体を評価
}
```

### 関数呼び出し

```rust
Expr::App(_, func, args) => {
    // 引数を左から順にスタックに積む
    for arg in args {
        self.lower_expr(ctx, arg)?;
    }

    // 関数インデックスで呼び出し
    let func_idx = self.func_indices[func_name];
    if func_idx < self.import_count {
        ctx.emit(Instruction::CallImport(func_idx));
    } else {
        ctx.emit(Instruction::Call(func_idx));
    }
}
```

`print` のような import 関数と、ユーザー定義関数で呼び出し命令が異なる。

## IR のテスト

L# では **insta** クレートによるスナップショットテストで IR 出力を検証する:

```rust
#[test]
fn test_lower_arithmetic() {
    let ir = lower("(defn add [x y] (+ x y))");
    insta::assert_snapshot!(ir.dump());
}
```

スナップショットテストは「期待される出力」をファイルに保存し、実際の出力と比較する。IR の変更が意図的なものかどうかを素早く検証できる。

## IR の出力例

`cargo run -- compile --emit-ir examples/fib.ls` で IR を確認できる:

```
fn fib(i64) -> i64:
  locals: i64, i64
  local.get 0
  i64.const 1
  i64.le_s
  i32.wrap_i64
  if (i64)
    local.get 0
  else
    local.get 0
    i64.const 1
    i64.sub
    call 1
    local.get 0
    i64.const 2
    i64.sub
    call 1
    i64.add
  end
```

## まとめ

IR は AST と機械語の間を橋渡しする:

- **スタックマシン**: WebAssembly と自然に対応する命令体系
- **フラットな命令列**: 木構造から線形構造への変換
- **型の単純化**: 言語の豊かな型から 3 つの機械型 (i64, f64, i32) へ

次章では、この IR を実際の WebAssembly バイナリに変換する**コード生成**を見ていく。

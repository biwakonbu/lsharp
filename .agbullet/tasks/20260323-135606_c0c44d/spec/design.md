# 設計書: L# 全 Phase (0-9) 実装完了

## 1. アーキテクチャ概要

既存のコンパイラパイプライン (Lexer -> Parser -> 型推論 -> IR Lowering -> Wasm Codegen) を維持しつつ、リニアメモリベースのランタイムを段階的に構築する。

```
Source (.ls)
  -> Lexer (lsharp-syntax) -> Token列
  -> Parser (lsharp-syntax) -> AST
  -> Type Inference (lsharp-types) -> 型付き AST
  -> Closure Analysis (lsharp-ir/closure.rs) [Phase 3 で追加]
  -> Lowering (lsharp-ir/lower/) -> IR Module
  -> Module Linking (lsharp-ir) [Phase 6 で有効化]
  -> Codegen (lsharp-wasm) -> .wasm バイナリ
  -> WASI Runner (wasmtime) -> 実行
```

### メモリレイアウト (Phase 0 で確立)

```
Linear Memory:
  [0..15]    : 改行文字 + パディング
  [16..23]   : iovec 構造体
  [24..27]   : nwritten
  [28..275]  : 数値変換バッファ
  [276..511] : 予約
  [512..N]   : 文字列定数 (data section)
  [N+1..]    : ヒープ領域 ($heap_ptr の初期値 = N+1)
```

### タグ付きワード (Phase 0-3 で確立)

```
i64 値の解釈:
  - 最上位ビット = 0: 即値整数 (63-bit signed integer)
  - 最上位ビット = 1: ヒープポインタ (下位32ビットがアドレス)

ヒープオブジェクトヘッダ:
  [offset+0] i32: オブジェクトタグ
  [offset+4] i32: サイズ (バイト数)
  [offset+8..] : ペイロード

オブジェクトタグ:
  0 = 予約
  1 = String (len: i32, bytes: [u8])
  2 = Record (field_count: i32, fields: [i64])
  3 = ADT (variant_tag: i32, field_count: i32, fields: [i64])
  4 = Closure (func_idx: i32, env_count: i32, captured: [i64])
  5 = Vector (len: i32, cap: i32, data_ptr: i32)
  6 = HashMap (size: i32, cap: i32, buckets_ptr: i32)
  7 = Ref (value: i64)
```

## 2. コンポーネント設計

### 2.1 Phase 0: lower/ モジュール分割

| ファイル | 責務 | 推定行数 |
|---------|------|---------|
| `lower/mod.rs` | `Lower` struct 定義、`lower_program()`、型登録、公開 API | ~400 |
| `lower/expr.rs` | `lower_expr()`、`emit_binop()`、式の IR 変換 | ~400 |
| `lower/pattern.rs` | `lower_match_arms()`、パターンマッチの IR 生成 | ~150 |
| `lower/decl.rs` | `lower_function()`、ジェネレータ群 (アクセサ、コンストラクタ、制約チェック) | ~400 |
| `lower/tests.rs` | テストコード (insta スナップショット含む) | ~600 |

**共有する型**:
- `Lower` struct: `pub(crate)` フィールドで全サブモジュールからアクセス
- `FuncCtx`: `lower/expr.rs` と `lower/pattern.rs` で共有

**分割手順**:
1. `lower/` ディレクトリ作成、`mod.rs` に `Lower` struct 移動
2. `impl Lower` ブロックを機能別にファイルに分配
3. 各ファイルで `use super::*` または明示的 import
4. insta スナップショットファイルの参照パス更新
5. `cargo test` で 422 テスト全パス確認

### 2.2 Phase 0: Bump Allocator (`wasi.rs`)

```
__alloc(size: i32) -> i32:
  1. global.get $heap_ptr
  2. size を 8 バイトアラインメントに切り上げ
  3. 新しい heap_ptr = 現在の heap_ptr + aligned_size
  4. 新しい heap_ptr がメモリサイズを超える場合:
     - 必要ページ数を計算
     - memory.grow で拡張
     - 拡張失敗時は unreachable (OOM)
  5. global.set $heap_ptr (新しい値)
  6. 元の heap_ptr を返す (確保した領域の先頭)
```

**実装箇所**: `wasi.rs` の `WasiCodegen::emit_module()` にインライン Wasm 関数として埋め込み

### 2.3 Phase 0: メモリ操作 IR 命令 (`lib.rs`, `emit.rs`)

`Instruction` enum に追加:
```rust
// lib.rs
I32Load { offset: u32 },
I32Store { offset: u32 },
I32Load8U { offset: u32 },
I32Store8 { offset: u32 },
I64Load { offset: u32 },
I64Store { offset: u32 },
```

`emit.rs` に対応する変換:
```rust
// emit.rs
Instruction::I32Load { offset } => {
    func.instruction(&wasm_encoder::Instruction::I32Load(
        wasm_encoder::MemArg { offset: *offset as u64, align: 2, memory_index: 0 }
    ));
}
```

### 2.4 Phase 1: 文字列ランタイム

**ビルトイン関数の実装場所**: `wasi.rs` にヘルパー関数として Wasm で直接生成

| 関数 | 実装概要 |
|------|---------|
| `string-length` | ヒープの String オブジェクトから len フィールド読み出し |
| `string-concat` | 新 String を alloc、両方の bytes をコピー |
| `string-char-at` | offset + 8 + index から I32Load8U |
| `substring` | 新 String を alloc、部分バイト列をコピー |
| `string-eq` | len 比較 + バイト列の逐次比較 |
| `int-to-string` | 既存の数値変換ロジック活用、結果をヒープ String に |
| `print-string` | String の bytes を fd_write で出力 |

**型推論への追加**: `infer.rs` のビルトイン関数テーブルに型シグネチャを登録

**IR Lowering への追加**: `lower/expr.rs` でビルトイン関数呼び出しを IR の `Call` 命令に変換

### 2.5 Phase 2: コレクション

**ADT リニアメモリ化 (P2-1)**:
- `lower/decl.rs` の `generate_adt_constructor` を改修
- コンストラクタ: `__alloc(8 + 4 + field_count * 8)` -> ヘッダ書き込み -> フィールド書き込み -> ポインタ返却
- パターンマッチ: ポインタからタグ読み出し -> 分岐 -> フィールド読み出し

**Vector (P2-2)**:
```
vector-new(cap):
  alloc(header=8, len=4, cap=4, data_ptr=4)  // 20 bytes
  data = alloc(cap * 8)
  書き込み: tag=5, size=20, len=0, cap=cap, data_ptr=data

vector-push(v, x):
  len = load(v+8), cap = load(v+12)
  if len >= cap: リアロケーション (cap * 2)
  store(data_ptr + len * 8, x)
  store(v+8, len+1)
```

**HashMap (P2-3)**:
```
バケット構造: [hash: i32, key: i64, value: i64, next_ptr: i32]
衝突解決: チェイン法 (linked list)
ハッシュ関数: FNV-1a (offset_basis=2166136261, prime=16777619)
初期容量: 16、負荷率 0.75 で 2 倍拡張
```

### 2.6 Phase 3: クロージャ

**自由変数解析 (`closure.rs`)**:
```rust
pub fn free_variables(expr: &Expr, bound: &HashSet<String>) -> Vec<String> {
    // AST を再帰的に走査
    // let/fn のパラメータは bound に追加
    // 変数参照が bound に含まれなければ自由変数
}
```

**Lambda Lifting**:
1. `lower_expr()` で `Expr::Lambda` を検出
2. 自由変数を解析
3. 新しいトップレベル関数を生成 (元のパラメータ + 環境パラメータ)
4. クロージャオブジェクト (tag=4) をヒープに確保
5. 呼び出し側: `call_indirect` でクロージャを実行

**Wasm テーブル**: `funcref` テーブルを追加、`call_indirect` で間接呼び出し

### 2.7 Phase 4-5: エラー処理 & WASI

**Option/Result**: Phase 2-1 の ADT リニアメモリ化が前提。型レベルでは既に定義可能、ランタイム動作の有効化のみ。

**Ref Cell**: `__alloc(16)` で tag=7 オブジェクトを確保、`ref-get`/`ref-set` は I64Load/I64Store。

**WASI 拡張**: `wasi.rs` の `emit_imports()` に WASI 関数を追加。`wasi_runner.rs` に `WasiCtxBuilder::preopened_dir()` を追加。

### 2.8 Phase 6: マルチファイルコンパイル

**モジュール探索規約**:
```
(import Foo)  ->  ./Foo.ls
(import Foo.Bar)  ->  ./Foo/Bar.ls
```

**コンパイルフロー**:
1. エントリファイルから import 宣言を収集
2. `ModuleGraph` に追加、循環依存チェック
3. トポロジカルソート順に各モジュールをコンパイル
4. export シンボルの型環境を次モジュールに注入
5. `link_modules()` で全 IR を結合
6. 単一 .wasm を生成

### 2.9 Phase 7-8: 標準ライブラリ & セルフホスティング

**stdlib 構造**:
- 各ファイルは `(module Name ...)` で宣言
- ビルトイン関数のラッパーと純粋関数の組み合わせ
- `lsharp test stdlib/` で自動テスト

**セルフホスティング戦略**:
1. 最小サブセット: `let` / 再帰 / `if` / `match` / ADT / Record / モジュール
2. Rust 版と L# 版の出力比較テストで正しさを検証
3. stage1.wasm (Rust でコンパイル) -> stage2.wasm (stage1 でコンパイル) の固定点検証

### 2.10 Phase 9: エコシステム

**REPL**: `wasmtime` をインプロセスで使用、式を受け取り -> パイプライン実行 -> 結果表示
**LSP**: `tower-lsp` ベース、型推論結果をキャッシュして応答
**パッケージマネージャ**: `lsharp.toml` + Git clone + `module_graph.rs` で依存解決

## 3. データ設計

### 3.1 IR 拡張 (lib.rs)

```rust
// 追加する Instruction バリアント
pub enum Instruction {
    // ... 既存 ...
    I32Load { offset: u32 },
    I32Store { offset: u32 },
    I32Load8U { offset: u32 },
    I32Store8 { offset: u32 },
    I64Load { offset: u32 },
    I64Store { offset: u32 },
    I32WrapI64,        // i64 -> i32 変換 (ポインタ抽出)
    I64ExtendI32U,     // i32 -> i64 変換 (タグ付け)
    I32Const(i32),     // i32 定数
    I32Add,            // i32 加算
    I32Sub,            // i32 減算
    I32Mul,            // i32 乗算
    I32GtU,            // i32 符号なし比較
    I32GeU,            // i32 符号なし比較
    I32And,            // i32 ビットAND
    I32Or,             // i32 ビットOR
    I32Shl,            // i32 左シフト
    I32ShrU,           // i32 符号なし右シフト
    MemoryGrow,        // memory.grow
    MemorySize,        // memory.size
    CallIndirect { type_idx: u32 },  // call_indirect (クロージャ用)
}

// 追加する IrType バリアント (必要に応じて)
pub enum IrType {
    // ... 既存 ...
    I32,  // メモリ操作用
}
```

### 3.2 ビルトイン関数テーブル

Phase ごとに追加されるビルトイン関数:

| Phase | 関数名 | 型シグネチャ |
|-------|--------|-------------|
| P0 | `__alloc` | `(i32) -> i32` (内部関数) |
| P1 | `string-length` | `(String) -> Int` |
| P1 | `string-concat` | `(String, String) -> String` |
| P1 | `string-char-at` | `(String, Int) -> Int` |
| P1 | `substring` | `(String, Int, Int) -> String` |
| P1 | `string-eq` | `(String, String) -> Bool` |
| P1 | `int-to-string` | `(Int) -> String` |
| P1 | `print-string` | `(String) -> Unit` |
| P1 | `print-int` | `(Int) -> Unit` |
| P2 | `vector-new` | `(Int) -> Vector` |
| P2 | `vector-push` | `(Vector, a) -> Vector` |
| P2 | `vector-get` | `(Vector, Int) -> a` |
| P2 | `vector-set` | `(Vector, Int, a) -> Vector` |
| P2 | `vector-length` | `(Vector) -> Int` |
| P2 | `map-new` | `() -> Map` |
| P2 | `map-insert` | `(Map, String, a) -> Map` |
| P2 | `map-get` | `(Map, String) -> Option a` |
| P2 | `map-contains?` | `(Map, String) -> Bool` |
| P2 | `map-remove` | `(Map, String) -> Map` |
| P2 | `map-size` | `(Map) -> Int` |
| P4 | `ref-new` | `(a) -> Ref a` |
| P4 | `ref-get` | `(Ref a) -> a` |
| P4 | `ref-set` | `(Ref a, a) -> Unit` |
| P5 | `read-file` | `(String) -> String` |
| P5 | `write-file` | `(String, String) -> Unit` |
| P5 | `file-exists?` | `(String) -> Bool` |

### 3.3 データフロー

```
ユーザーソースコード
  |
  v
[Parser] -- 新構文は Phase 0-6 では追加不要 (既存 AST で対応)
  |
  v
[型推論] -- ビルトイン関数の型シグネチャを追加
  |
  v
[自由変数解析] -- Phase 3 で追加
  |
  v
[IR Lowering] -- メモリ操作命令、ビルトイン呼び出し、クロージャ変換を追加
  |
  v
[IR Module] -- Instruction enum 拡張
  |
  v
[Module Linking] -- Phase 6 で有効化
  |
  v
[Wasm Codegen] -- メモリ操作命令の変換、Bump Allocator 埋め込み、WASI import 追加
  |
  v
[.wasm] -- リニアメモリベースのランタイム付き
```

## 4. インターフェース設計

### 4.1 内部 API (クレート間)

**lsharp-ir -> lsharp-wasm**:
- `Module` struct の `instructions` フィールドにメモリ操作命令が含まれるようになる
- 既存の `link_modules()` API は変更なし

**lsharp-types -> lsharp-ir**:
- ビルトイン関数の型情報は `TypeEnv` 経由で共有
- クロージャの自由変数情報は新規 API `closure::free_variables()` で取得

### 4.2 CLI API (lsharp-driver)

Phase 6 以降:
```bash
# マルチファイルコンパイル
cargo run -- compile src/Main.ls -o output.wasm

# stdlib を含むコンパイル
cargo run -- compile --stdlib src/Main.ls -o output.wasm
```

Phase 9:
```bash
cargo run -- repl          # REPL 起動
cargo run -- lsp           # LSP サーバー起動
cargo run -- pkg install   # パッケージインストール
```

## 5. エラーハンドリング

- メモリ不足 (OOM): `memory.grow` 失敗時は `unreachable` トラップ (Phase 0)
- 範囲外アクセス: Vector/String の境界チェックで trap (Phase 1-2)
- HashMap キー未発見: `Option::None` を返す (Phase 2)
- ファイル操作エラー: WASI エラーコードを `Result` に変換 (Phase 5)
- モジュール未発見: コンパイルエラーとして `miette` で報告 (Phase 6)

## 6. テスト戦略

### 6.1 ユニットテスト

| 対象 | テストファイル | 内容 |
|------|--------------|------|
| lower/ 分割 | `lower/tests.rs` | 既存 422 テストの移行 |
| メモリ操作命令 | `lib.rs` テスト | IR 命令の構築と検証 |
| 自由変数解析 | `closure.rs` テスト | Lambda パターンの自由変数抽出 |
| モジュール探索 | `module_graph.rs` テスト | ファイルパス解決 |

### 6.2 E2E テスト (`e2e.rs`)

Phase ごとに追加する E2E テスト:

| Phase | テスト内容 | 推定テスト数 |
|-------|----------|------------|
| P0 | メモリ確保・書き込み・読み出し、タグ判定 | 5-8 |
| P1 | 文字列ビルトイン各関数、print 多相化 | 10-15 |
| P2 | ADT 構築・パターンマッチ、Vector CRUD、HashMap CRUD | 15-20 |
| P3 | クロージャキャプチャ、高階関数 | 8-12 |
| P4 | Option/Result パターンマッチ、Ref Cell | 5-8 |
| P5 | ファイル読み書き、引数取得 | 5-8 |
| P6 | マルチファイルコンパイル・実行 | 5-8 |
| P7 | stdlib のコンパイル・実行 | 10-15 |
| P8 | セルフホスティング検証 | 5-10 |

### 6.3 回帰テスト

- 全 Phase を通じて `cargo test` が常にパスすること
- insta スナップショットの更新は意図的な変更時のみ

## 7. 実装優先順位

### Tier 1: 基盤 (Phase 0) -- 全ての前提

1. **P0-0**: lower.rs リファクタリング (他全ての前提作業)
2. **P0-1 + P0-2**: Bump Allocator + メモリ操作 IR (並列実行可能)
3. **P0-3**: タグ付きワード

### Tier 2: ランタイム機能 (Phase 1-3) -- 一部並列可能

4. **P1-1 + P2-1 + P3-1**: 文字列ランタイム + ADT リニアメモリ + 自由変数解析 (並列実行可能)
5. **P1-2 + P1-3**: 文字列ヒープ化 + print 多相化
6. **P2-2 + P2-3**: Vector + HashMap (並列実行可能)
7. **P3-2 + P3-3**: クロージャ変換 + 高階関数

### Tier 3: 応用機能 (Phase 4-6) -- 一部並列可能

8. **P4-1 + P4-2 + P5-1**: Result/Option + Ref Cell + WASI import (並列実行可能)
9. **P5-2**: ファイル操作
10. **P6-1 -> P6-2 -> P6-3**: モジュールシステム (順序依存)

### Tier 4: 完成 (Phase 7-9) -- 順序依存

11. **P7**: 標準ライブラリ
12. **P8-1 -> P8-2 -> P8-3 -> P8-4 -> P8-5**: セルフホスティング (順序依存)
13. **P9**: エコシステム (REPL/LSP/パッケージマネージャ)

### 並列実行マップ

```
時間軸 ->

[P0-0] -> [P0-1 | P0-2] -> [P0-3] -> [P1-1 | P2-1 | P3-1] -> [P1-2 | P2-2 | P3-2]
                                       [P6-1]                   [P1-3 | P2-3 | P3-3]
                                                                 [P6-2]
                                                    -> [P4-1 | P4-2 | P5-1] -> [P5-2]
                                                       [P6-3]
                                                                    -> [P7] -> [P8] -> [P9]
```

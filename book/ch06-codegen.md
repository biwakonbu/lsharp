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
| Memory | 線形メモリの定義 |
| Export | 外部に公開する関数 |
| Code | 関数の実体 (命令列) |
| Data | メモリの初期データ |

## L# の Wasm コード生成

L# は `wasm-encoder` クレートを使って Wasm バイナリを構築する (`crates/lsharp-wasm/src/wasi.rs`):

### WASI 固有の設定

WASI モードでは、標準出力への書き込みに `fd_write` を使う:

```rust
// fd_write のインポート
// (import "wasi_snapshot_preview1" "fd_write"
//   (func $fd_write (param i32 i32 i32 i32) (result i32)))
```

これは WASI の規約で、ファイルディスクリプタへの書き込みを行う関数である。

### メモリレイアウト

WASI モードでは固定のメモリレイアウトを使用する:

```
アドレス    内容
─────────  ──────────
0          改行文字 '\n'
16         iovec 構造体 (ポインタ + 長さ)
24         nwritten (書き込みバイト数)
256~       数値変換バッファ
```

`print` 関数は整数を文字列に変換し、`fd_write` で標準出力に書き出す。この変換処理は `__print_i64` ヘルパー関数として Wasm 内に生成される。

### IR から Wasm への変換

IR 命令は概ね Wasm 命令に直接対応する:

| IR 命令 | Wasm 命令 |
|---------|-----------|
| `I64Const(n)` | `i64.const n` |
| `LocalGet(i)` | `local.get i` |
| `I64Add` | `i64.add` |
| `Call(i)` | `call i` |
| `If(ty)` | `if (result ty)` |

一部の IR 命令は Wasm の命令と 1:1 ではない。たとえば `CallImport` は Wasm では通常の `call` 命令だが、関数インデックスが import 関数を指す。

### _start エントリポイント

WASI ではプログラムのエントリポイントとして `_start` 関数がエクスポートされる:

```wasm
(func $_start
  call $main    ;; ユーザーの main 関数を呼び出す
  drop          ;; 戻り値を破棄
)
(export "_start" (func $_start))
```

`main` 関数が `Unit` (i64 の 0) を返すが、`_start` は戻り値を持たないため `drop` で破棄する。

## 数値出力の仕組み

L# の `print` は整数を標準出力に表示する。Wasm 内でこれを実現するために、整数を10進文字列に変換するヘルパー関数 `__print_i64` が自動生成される:

```
__print_i64 の動作:
1. 整数を受け取る
2. 負数ならマイナス記号を出力
3. 各桁を '0'~'9' の ASCII 文字に変換
4. メモリに逆順で格納
5. fd_write で標準出力に書き出す
6. 改行を出力
```

この処理はすべて Wasm 命令で実装される。ランタイムライブラリを持たない L# にとって、このようなヘルパー関数は Wasm バイナリ内に埋め込む必要がある。

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
- **IR との 1:1 対応**: スタックマシンモデルで自然な変換
- **自己完結**: ランタイムライブラリなし、必要な機能は Wasm 内に埋め込み
- **wasm-encoder**: Rust クレートによる安全なバイナリ構築

## WasmGC への移行

現在の L# コンパイラは全ての値を `i64` や `f64` のプリミティブ型で表現している (MVP 方式)。しかし、レコード型や ADT を効率的に扱うには **WasmGC** (Garbage Collection 拡張) が必要になる。

WasmGC は 2025 年時点で全主要ブラウザ・ランタイムで安定サポート済みである:

- Chrome v119+, Firefox v120+, Safari v18.2+
- wasmtime, wasmer v6.0+ でフルサポート

L# の IR には `StructNew`, `StructGet`, `StructSet`, `RefCast` といった GC 命令が既に定義されている (第 5 章参照)。現在は MVP として `i64` にフォールバックしているが、今後 `wasm-encoder` の GC API (`StructType`, `ArrayType`, `SubType`) を使って本格的な WasmGC コード生成に移行する計画である。

これで L# コンパイラの「ソースコードから実行可能バイナリまで」のパイプライン全体を見てきた。次章からは、型システムの拡張ロードマップに踏み込んでいく。

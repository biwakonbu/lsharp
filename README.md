# L# (lsharp)

F# のような強い静的型付けと Lisp の S 式構文を融合したプログラミング言語。WebAssembly をターゲットとし、ブラウザ・サーバー・エッジで同一コードが動く世界を目指す。

## Features

- **純粋 S 式構文** - Clojure 風のシンプルな文法
- **Hindley-Milner 型推論** - 明示的な型注釈なしで型安全
- **代数的データ型** - パターンマッチ付き
- **WebAssembly 出力** - WASI 対応、wasmtime で直接実行可能

## Quick Start

```bash
# ビルド
cargo build

# ソースファイルの型チェック
cargo run -- check examples/fib.ls

# Wasm にコンパイル
cargo run -- compile examples/fib.ls -o fib.wasm

# wasmtime で実行
wasmtime fib.wasm
# => 55
```

## Examples

```lisp
;; フィボナッチ数列
(defn fib [n]
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(defn main []
  (print (fib 10)))
```

```lisp
;; 代数的データ型
(type (Option a)
  (Some a)
  None)

(defn unwrap-or [opt default]
  (match opt
    [(Some x) x]
    [None default]))
```

## Architecture

```
L# source (.ls)
  -> Lexer (lsharp-syntax)
  -> Parser (lsharp-syntax)
  -> Type Inference (lsharp-types) [Hindley-Milner]
  -> IR (lsharp-ir)
  -> Wasm codegen (lsharp-wasm) [WASI]
  -> .wasm binary
```

## License

MIT

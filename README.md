# L# (lsharp)

F# のような強い静的型付けと Lisp の S 式構文を融合したプログラミング言語。現在は共通の frontend / IR を軸に、Wasm/WASI backend と native backend を併走で育てており、Rust 製ツールチェインと L# selfhost 実装の両方から同じパイプラインを検証している。

## Features

- **純粋 S 式構文** - Clojure 風のシンプルな文法
- **Hindley-Milner 型推論** - 明示的な型注釈なしで型安全
- **代数的データ型** - パターンマッチ付き
- **マルチバックエンド** - 共通 IR から Wasm/WASI と native の両経路を検証

## Quick Start

```bash
# 開発用 CLI をビルド
cargo build -p lsharp-driver

# ソースファイルの型チェック
target/debug/lsharp check examples/fib.ls

# Wasm/WASI backend でコンパイル
target/debug/lsharp compile examples/fib.ls -o fib.wasm

# wasmtime で実行
wasmtime fib.wasm
# => 55
```

native 配布物や selfhost compiler への経路を試す場合は、`LSHARP_PATH` で委譲先を差し替えられる。

```bash
# 既存 CLI から外部 compiler / native 配布物へ委譲
LSHARP_PATH=/path/to/native/lsharp target/debug/lsharp --version
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

```text
L# source (.ls)
  -> Frontend
     - Lexer / Parser          (crates/lsharp-syntax, selfhost/Lexer.ls, selfhost/Parser.ls)
     - MacroExpand / TypeInfer (crates/lsharp-types, selfhost/MacroExpand.ls, selfhost/TypeInfer.ls)
  -> Lowering / IR             (crates/lsharp-ir, selfhost/Lower*.ls, selfhost/Compiler.ls)
  -> Backend split
     - Wasm/WASI               (crates/lsharp-wasm, selfhost/WasmEmit.ls, selfhost/WasiBackend.ls)
       -> .wasm binary
     - Native/AOT              (selfhost/NativeCodegen.ls, selfhost/NativeEmit.ls, selfhost/NativeTarget.ls)
       -> native artifact / release binary
```

現在の公開 CLI は主に `crates/lsharp-driver` が担い、`check` / `compile` / `parse` などの入口を提供する。selfhost 側では `selfhost/Cli.ls`, `selfhost/LspServer.ls`, `selfhost/DocTools.ls`, `selfhost/TestRunner.ls` が対応するツール群で、default path migration の進行に合わせて native 配布物へ寄せていく。

## Build / Use Paths

| 用途 | 現在の実用経路 | 主な実装 |
|------|----------------|----------|
| 日常開発・型検査 | `cargo build -p lsharp-driver` → `target/debug/lsharp check ...` | `crates/lsharp-driver`, `crates/lsharp-types` |
| Wasm 生成・実行 | `target/debug/lsharp compile ... -o out.wasm` → `wasmtime out.wasm` | `crates/lsharp-wasm`, `selfhost/WasmEmit.ls` |
| native compiler / 配布物の接続確認 | `LSHARP_PATH=/path/to/lsharp target/debug/lsharp --version` | `crates/lsharp-driver`, `scripts/ci/default-path-smoke.sh` |
| selfhost / stdlib の固定入力 compile gate | `bash scripts/ci/compile-phase11-inputs.sh` | `selfhost/Compiler.ls`, `selfhost/Native*`, `selfhost/Wasi*` |

## License

MIT

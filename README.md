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

# 公開 CLI の基本動線: compile で frontend 検証と Wasm 出力をまとめて行う
target/debug/lsharp compile examples/fib.ls -o fib.wasm

# wasmtime で実行
wasmtime fib.wasm
# => 55
```

公開 CLI は `compile` を中心に整理しており、`parse` / `check` / `fmt` はエディタ連携や AI 連携で使う
LSP / MCP の内部 API として扱う。AST・型情報・formatting の詳細確認は、CLI を直叩きする代わりに
`lsharp lsp` / `lsharp mcp-server` を経由する想定である。

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

現在の公開 CLI は主に `crates/lsharp-driver` の `compile` を入口にし、`parse` / `check` / `fmt` は
`lsharp lsp` / `lsharp mcp-server` が利用する内部 API 側へ寄せていく。selfhost 側では
`selfhost/Cli.ls`, `selfhost/LspServer.ls`, `selfhost/DocTools.ls`, `selfhost/TestRunner.ls` が対応する
ツール群で、default path migration の進行に合わせて native 配布物へ寄せていく。

## Build / Use Paths

| 用途 | 現在の実用経路 | 主な実装 |
|------|----------------|----------|
| 日常開発・公開 CLI | `cargo build -p lsharp-driver` → `target/debug/lsharp compile ... -o out.wasm` | `crates/lsharp-driver`, `crates/lsharp-types`, `crates/lsharp-wasm` |
| Wasm 生成・実行 | `target/debug/lsharp compile ... -o out.wasm` → `wasmtime out.wasm` | `crates/lsharp-wasm`, `selfhost/WasmEmit.ls` |
| IDE / AI 連携 | `target/debug/lsharp lsp` / `target/debug/lsharp mcp-server` | `crates/lsharp-lsp`, internal parse/check/fmt APIs |
| native compiler / 配布物の接続確認 | `LSHARP_PATH=/path/to/lsharp target/debug/lsharp --version` | `crates/lsharp-driver`, `scripts/ci/default-path-smoke.sh` |
| selfhost / stdlib の固定入力 compile gate | `bash scripts/ci/compile-phase11-inputs.sh` | `selfhost/Compiler.ls`, `selfhost/Native*`, `selfhost/Wasi*` |

## License

MIT

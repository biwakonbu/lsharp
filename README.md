# L# (lsharp)

L# は **Lisp の S 式構文**、**F# 系の型指向**、そして **L# 独自の型 / メタデータ設計** を組み合わせたプログラミング言語です。現在の公開ターゲットは WebAssembly/WASI で、同じ frontend / IR を基準に native backend と selfhost toolchain も並走で育てています。

- 構文は Lisp 的に小さく保つ
- 型は Hindley-Milner を土台に強く保つ
- さらに trait、制約付き型、metadata、計算式などで「型に意味を持たせる」方向を探る

> 現在の通常導線は Rust 製 `lsharp` バイナリによる `compile` / `test` / `lsp` / `mcp-server` です。native backend と selfhost default path は移行中で、`LSHARP_PATH` を使うと外部 compiler / 配布物への委譲を試せます。

## Core Language

- **純粋 S 式構文** - Clojure 風の小さな文法で関数・型・module を表現
- **Hindley-Milner 型推論** - 明示的な型注釈なしでも多くの型が推論される
- **ADT / record / pattern match** - 代数的データ型、レコード型、`match` を標準で提供
- **module system** - `(module Name)` / `(import Name)` / `(open Name)` による構成
- **Wasm/WASI を公開ターゲット** - `compile` を起点に `.wasm` を生成して `wasmtime` で実行

## Type-Oriented Design

L# は「Lisp + F#」で終わらず、型の表現力そのものを広げる方向を取っています。

- **trait + `:where`** - アドホック多相と制約付き多相
- **`type-constrained`** - 値の意味的な制約を型定義に載せる
- **構造化 metadata** - `:doc`, `:params`, `:returns`, `:example`, `:invariant`, `:transitions`
- **metadata-driven test / docs** - `test` や `doc` が metadata を直接利用
- **高度な型機能を探求中** - 高カインド型、GADT、computation expressions は一部実装 / 検証中

特に `test` コマンドで `:example` / `:invariant` を実行できる点は、L# の「型とドキュメントと検証を近づける」方向性をよく表しています。

## Quick Start

```bash
# 開発用 CLI をビルド
cargo build -p lsharp-driver

# 公開 CLI の基本動線: compile で format/check/codegen をまとめて通す
target/debug/lsharp compile examples/fib.ls -o fib.wasm

# wasmtime で実行
wasmtime fib.wasm
# => 55
```

metadata を使ったテストも実行できます。

```bash
# :example / :invariant を自動検証
target/debug/lsharp test examples/metadata.ls
```

IDE / AI 連携は次の入口を使います。

```bash
# IDE 向け
target/debug/lsharp lsp

# AI 向け
target/debug/lsharp mcp-server
```

公開 CLI は `compile` を中心に整理しており、`parse` / `check` / `fmt` は LSP / MCP の内部 API として扱います。AST・型情報・formatting の詳細確認は、CLI を直叩きする代わりに `lsharp lsp` / `lsharp mcp-server` を経由する想定です。

移行中の外部 compiler / native 配布物への委譲を試す場合は、`LSHARP_PATH` を使います。

```bash
LSHARP_PATH=/path/to/lsharp target/debug/lsharp --version
```

## Language Snapshot

```lisp
;; フィボナッチ
(defn fib [n]
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))
```

```lisp
;; ADT + pattern match
(type (Option a)
  (Some a)
  None)

(defn unwrap-or [opt default]
  (match opt
    [(Some x) x]
    [None default]))
```

```lisp
;; record + constrained type
(type Point (record (: x Int) (: y Int)))

(type-constrained Percentage Int
  :constraints [(>= 0) (<= 100)])
```

## Architecture

```text
L# source (.ls)
  -> Frontend
     - Syntax / MacroExpand           (crates/lsharp-syntax, selfhost/src/Syntax)
     - Types / Metadata / Constraints (crates/lsharp-types, selfhost/src/Types)
  -> Lowering / IR                    (crates/lsharp-ir, selfhost/src/IR)
  -> Codegen
     - Wasm/WASI  (公開経路)          (crates/lsharp-wasm, selfhost/src/Backend/Wasm)
     - Native/AOT (並走開発中)        (selfhost/src/Backend/Native)
  -> Tooling
     - CLI / docs / package           (crates/lsharp-driver, selfhost/src/App, selfhost/src/Tools)
     - LSP                            (crates/lsharp-lsp, selfhost/src/Tools/Lsp)
```

`selfhost/src/**` が selfhost 側の canonical source root です。公開 CLI の default path はまだ主に Rust 実装ですが、`LSHARP_PATH` による process-entry delegation と parity 検証を進めています。

## Build / Use Paths

| 用途 | 現在の推奨経路 | 補足 |
|------|----------------|------|
| 日常開発・公開 CLI | `cargo build -p lsharp-driver` → `target/debug/lsharp compile ... -o out.wasm` | 現在の通常導線 |
| metadata test | `target/debug/lsharp test examples/metadata.ls` | `:example` / `:invariant` を自動検証 |
| IDE / AI 連携 | `target/debug/lsharp lsp` / `target/debug/lsharp mcp-server` | LSP / MCP の入口 |
| 外部 compiler / native 配布物の接続確認 | `LSHARP_PATH=/path/to/lsharp target/debug/lsharp --version` | default-path migration の検証導線 |

## Current Status

- **公開ターゲット**: Wasm/WASI
- **安定寄りのコア**: HM 型推論、ADT、record、module、static trait dispatch、metadata-driven docs/tests
- **並走開発 / 移行中**: native backend、selfhost cutover、AI / package ecosystem、高度な型機能の parity
- **注意点**: 高カインド型、GADT、computation expressions は README で方向性として触れていますが、全面的に runtime-ready と断定しない段階です

## Learn More

- 利用者向けの導線: [`docs/guides/quick-start.md`](docs/guides/quick-start.md), [`docs/guides/language-reference.md`](docs/guides/language-reference.md)
- 言語の背景と実装の読み物: [`book/ch01-introduction.md`](book/ch01-introduction.md), [`book/ch10-traits.md`](book/ch10-traits.md), [`book/ch11-advanced-types.md`](book/ch11-advanced-types.md), [`book/ch15-selfhosting.md`](book/ch15-selfhosting.md)
- compiler / backend の契約: [`docs/language/README.md`](docs/language/README.md)
- 現在のロードマップ: [`TODO.md`](TODO.md), [`docs/development/operations/default-path-migration.md`](docs/development/operations/default-path-migration.md), [`docs/development/planning/phase12-package-ai-ecosystem-roadmap.md`](docs/development/planning/phase12-package-ai-ecosystem-roadmap.md)

## License

MIT

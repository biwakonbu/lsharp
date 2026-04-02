# L# (lsharp)

L# は **Lisp の S 式構文**、**F# 系の型指向**、そして **L# 独自の型 / メタデータ設計** を組み合わせたプログラミング言語です。現在の公開ターゲットは WebAssembly/WASI 系で、配布は **Rust 製 host launcher + 埋め込み selfhost guest component** を前提に整理しています。

- 構文は Lisp 的に小さく保つ
- 型は Hindley-Milner を土台に強く保つ
- さらに trait、制約付き型、metadata、計算式などで「型に意味を持たせる」方向を探る

> 現在の通常導線は Rust 製 `lsharp` host launcher による `compile` / `test` / `lsp` / `mcp-server` です。`LSHARP_PATH` を使うと、埋め込み guest component の代わりに外部 compiler / 配布物への process-entry delegation を試せます。

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
# 展開した release archive へ移動
cd lsharp-<version>-<target>

# checksum を検証 (macOS)
shasum -a 256 -c checksums.txt
# Linux では: sha256sum -c checksums.txt

# packaged lsharp を優先して使う
export PATH="$PWD:$PATH"
```

```bash
cat > hello.ls <<'EOF'
(defn main [] 42)
EOF

# 公開 CLI の基本動線: compile で format/check/codegen をまとめて通す
lsharp compile hello.ls -o hello.wasm
```

metadata を使ったテストとドキュメント生成も、同じ packaged `lsharp` だけで実行できます。

```bash
cat > metadata.ls <<'EOF'
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
EOF

# :example / :invariant を自動検証
lsharp test metadata.ls

# HTML ドキュメントを生成
lsharp doc metadata.ls -o metadata.html
```

IDE / AI 連携は次の入口を使います。

```bash
# IDE 向け
lsharp lsp

# AI 向け
lsharp mcp-server
```

公開 CLI は `compile` を中心に整理しており、`parse` / `check` / `fmt` は LSP / MCP の内部 API として扱います。AST・型情報・formatting の詳細確認は、CLI を直叩きする代わりに `lsharp lsp` / `lsharp mcp-server` を経由する想定です。

既定では `lsharp` host launcher が build-time に埋め込んだ guest component を使います。外部 compiler / 配布物への委譲を試す場合は、`LSHARP_PATH` を使います。

```bash
LSHARP_PATH=/path/to/lsharp lsharp --version
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
     - `wasi-component` (default)     (crates/lsharp-wasm, selfhost/src/Backend/Wasm)
     - `web-wasm`                     (Rust host fallback + browser-oriented core wasm)
     - bootstrap core wasm            (`stageN.wasm` fixed-point 検証用)
  -> Host launcher / distribution
     - Rust host launcher             (crates/lsharp-driver, Wasmtime + host capabilities)
     - Embedded guest component       (selfhost/src/App/EmbeddedCli.ls)
  -> Tooling surface
     - CLI / docs / package           (crates/lsharp-driver, selfhost/src/App, selfhost/src/Tools)
     - LSP                            (crates/lsharp-lsp, selfhost/src/Tools/Lsp)
```

`selfhost/src/**` が selfhost 側の canonical source root です。日常の `lsharp` 実行では Rust host launcher が capability を提供し、`parse` / `check` / `compile` / `build` / `test` / `review` / `doc-ack` / `doc-check` / `fmt` の default path は埋め込み guest component が担当します。`review` は simple text surface に加えて `--json` / `--format json` も embedded guest で処理します。`install` / `repl` / `lsp` / `doc` と `compile` / `build` の Rust-only fallback (`--emit-ir`, `web-wasm`, `native`) は host launcher 側に残り、`LSHARP_DISABLE_EMBEDDED_COMPONENT=1` は guest-backed `review` / simple `doc-ack` / simple `doc-check` を host 別契約へ落とさず external selfhost hint へ戻す safety valve として使います。

## Build / Use Paths

| 用途 | 現在の推奨経路 | 補足 |
|------|----------------|------|
| 日常開発・公開 CLI | `cargo build -p lsharp-driver` → `target/debug/lsharp compile ... -o out.component.wasm` | `--target wasi-component` が既定。single-binary 配布では host launcher がこの guest component を内蔵する |
| browser 向け core wasm | `target/debug/lsharp compile --target web-wasm ... -o out.wasm` | `web-wasm` は browser 向け core `.wasm`。現時点では host launcher の Rust fallback 経路が担う |
| metadata test | `target/debug/lsharp test examples/metadata.ls` | `:example` / `:invariant` を自動検証 |
| IDE / AI 連携 | `target/debug/lsharp lsp` / `target/debug/lsharp mcp-server` | LSP / MCP の入口 |
| 外部 compiler / 配布物の接続確認 | `LSHARP_PATH=/path/to/lsharp target/debug/lsharp --version` | 埋め込み guest の代わりに external host launcher / Wasm artifact へ委譲できる |

## Current Status

- **公開ターゲット**: Wasm/WASI
- **安定寄りのコア**: HM 型推論、ADT、record、module、static trait dispatch、metadata-driven docs/tests
- **現在の配布モデル**: Rust host launcher + embedded guest component による single-binary distribution
- **bootstrap の読み方**: `stage0 -> stage1 -> stage2 -> stage3` のうち、fixed-point の意味は `stage2.wasm == stage3.wasm`。最小 subset の `stage1 -> stage2` 実生成は確認済みだが、full input set の fixed-point は引き続き追跡中
- **移行中**: selfhost cutover の残タスク、AI / package ecosystem、高度な型機能の parity
- **deferred**: native backend の常用配布経路は Phase 13+ の探求項目として保持
- **注意点**: 高カインド型、GADT、computation expressions は README で方向性として触れていますが、全面的に runtime-ready と断定しない段階です

## Learn More

- 利用者向けの導線: [`docs/guides/quick-start.md`](docs/guides/quick-start.md), [`docs/guides/language-reference.md`](docs/guides/language-reference.md)
- 言語の背景と実装の読み物: [`book/ch01-introduction.md`](book/ch01-introduction.md), [`book/ch10-traits.md`](book/ch10-traits.md), [`book/ch11-advanced-types.md`](book/ch11-advanced-types.md), [`book/ch15-selfhosting.md`](book/ch15-selfhosting.md)
- compiler / backend の契約: [`docs/language/README.md`](docs/language/README.md)
- 現在のロードマップ: [`TODO.md`](TODO.md), [`docs/development/operations/default-path-migration.md`](docs/development/operations/default-path-migration.md), [`docs/development/planning/phase12-package-ai-ecosystem-roadmap.md`](docs/development/planning/phase12-package-ai-ecosystem-roadmap.md)

## License

MIT

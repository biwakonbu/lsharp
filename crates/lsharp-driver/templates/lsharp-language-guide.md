---
name: lsharp-language-guide
description: L# 0.1.0 でアプリケーションやライブラリを書くユーザー向けに、Quick Start、CLI、構文、型、metadata、module、stdlib、MCP ツール、配布ターゲット、既知の制限を案内する。Use when user asks how to build software with L#, write L# syntax, use the CLI, structure modules/packages, run metadata tests/docs, inspect stdlib APIs, or use L# through MCP tools.
context: project
---

# L# Language Guide

## 概要

L# 0.1.0 は S 式構文、Hindley-Milner 型推論、ADT、record、pattern match、module、metadata-driven test/docs を持つ関数型言語です。アプリケーションやライブラリを書く時は、まず `lsharp compile` / `lsharp test` / `lsharp doc` を中心に案内してください。

公開 CLI は `compile` を基本導線にします。`parse` / `check` / `fmt` は主に LSP / MCP / 内部 tooling 側の用途として扱い、通常ユーザーへは `lsharp lsp` または `lsharp mcp-server` 経由を案内します。

## Docs SSOT

この file は AI セッション向けの要約です。人間向けユーザーガイドの正本は `docs/guides/`、公開サイトの表示順と output path の正本は `docs/site.toml` です。内容を更新する時は、先に `docs/guides/` を更新してから、この template を同期してください。

主要 guide:

- `docs/guides/quick-start.md`
- `docs/guides/language-reference.md`
- `docs/guides/package-layout.md`
- `docs/guides/metadata-driven-development.md`
- `docs/guides/ide-setup.md`
- `docs/guides/deployment-targets.md`
- `docs/guides/stdlib-guide.md`
- `docs/guides/error-reference.md`
- `docs/guides/examples.md`

エラーコードリファレンスは `LS####` と legacy `E0001`-`E0005` alias の正本です。CLI / LSP / MCP の全診断へ `LS####` を貫通させる作業は `I-02` / `imp-02` の範囲として扱います。

## Quick Start

最小プログラム:

```lisp
(defn main [] 42)
```

基本コマンド:

```bash
lsharp --version
lsharp compile hello.ls -o hello.component.wasm
lsharp test metadata.ls
lsharp doc metadata.ls -o metadata.html
lsharp lsp
lsharp mcp-server
```

`compile` は format/check/codegen をまとめて通す公開向け入口です。出力拡張子や `--target` に応じて Wasm component、WASI preview1 wasm、web wasm、native artifact を扱います。

## CLI Workflows

- `lsharp compile <file> -o <out>`: ソースを成果物へコンパイルします。通常はこの導線を最初に使います。
- `lsharp build <file> --output <out>`: `compile` の alias として使えます。
- `lsharp test <file>`: `:example` / `:invariant` metadata を実行します。
- `lsharp doc <file> -o <html>`: `:doc` / `:params` / `:returns` / `:example` metadata から HTML を生成します。
- `lsharp review <file>`: metadata や doc freshness を review 用 text/YAML として確認します。
- `lsharp doc-site -o _site`: guide と stdlib API を含む静的 docs site を生成します。
- `lsharp language-guide`: この skill と同じユーザー向け Markdown を標準出力へ出します。
- `lsharp claude-plugin`: Claude Code に `lsharp mcp-server` 設定とこの skill をインストールします。

## 構文早見表

L# は Clojure 風の S 式構文です。括弧の先頭に form 名や関数名を置きます。

- 関数定義: `(defn name [params] body)`
- ラムダ: `(fn [params] body)`
- let 束縛: `(let [x 1 y 2] (+ x y))`
- 条件分岐: `(if cond then else)`
- 逐次実行: `(do expr1 expr2 result)`
- パターンマッチ: `(match expr [pattern body] ...)`
- 型注釈: `(ann expr Type)`
- record field access: `(. record field)` または実装が提供する field access form
- ADT: `(type (Option a) (Some a) None)`
- Record: `(type Point (record (: x Int) (: y Int)))`
- Module: `(module Name)` / `(import Name)` / `(open Name)`

## 型システム

- プリミティブ: `Int`, `Float`, `String`, `Bool`, `Unit`
- 型推論: Hindley-Milner
- 関数型: `A -> B`
- 多相: `(Option a)`, `(Result a e)`
- ADT と record は型定義として扱います。
- trait と `:where` は static dispatch / constrained polymorphism の用途で使います。
- `type-constrained` は値の意味的制約を型定義へ載せるために使います。

## Data Modeling

ADT:

```lisp
(type (Option a)
  (Some a)
  None)

(defn unwrap-or [opt default]
  (match opt
    [(Some x) x]
    [None default]))
```

Record:

```lisp
(type Point
  (record (: x Int) (: y Int)))
```

制約付き型:

```lisp
(type-constrained Percentage Int
  :constraints [(>= 0) (<= 100)])
```

## Metadata-Driven Development

関数には実行可能な例、ドキュメント、戻り値説明、invariant を metadata として書けます。ユーザーには、テストとドキュメントを source に近づける目的で使うと説明してください。

```lisp
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
```

使うコマンド:

```bash
lsharp test metadata.ls
lsharp doc metadata.ls -o metadata.html
lsharp review metadata.ls
```

## Modules And Packages

- ファイルパスと module 名を対応させます。例: `src/Foo/Bar.ls` は `(import Foo.Bar)` で参照します。
- `(module Name)` はファイルの module 宣言です。
- `(import Name)` は module を読み込みます。
- `(open Name)` は module 内の名前を現在 scope で使いやすくします。
- package 境界では `lsharp.toml` の `[project.exports]` が公開 module を制御します。
- 非公開にしたい symbol は `private` を使います。
- GitHub package dependency は `lsharp add github.com/user/repo --tag <tag>` で追加する設計です。

## stdlib

- `Core`: 基本演算、比較、制御補助
- `List`: list 操作
- `Vector`: indexed sequence 操作
- `Map`, `Set`: collection
- `String`, `Char`: text 操作
- `IO`, `Path`: file/path 系 helper
- `Json`: JSON 値と変換
- `Debug`: debug 出力や確認用 helper

stdlib API の詳細は MCP の `lsharp_stdlib_api`、package API は `lsharp_package_api` を使って確認します。

## MCP ツール

- `lsharp_hover`: symbol の型や `:doc` を取得します。
- `lsharp_completion`: 補完候補を取得します。
- `lsharp_check`: L# source を型チェックします。
- `lsharp_package_api`: package API 一覧を取得します。
- `lsharp_stdlib_api`: stdlib API 一覧を取得します。

Claude Code で使う場合:

```bash
lsharp claude-plugin
```

このコマンドは `~/.claude/settings.json` に `lsharp mcp-server` を登録し、`~/.claude/skills/lsharp-language-guide/SKILL.md` にこのガイドを配置します。

## Deployment Targets

- 既定の公開 compile target は `wasi-component` です。
- `--target wasi-preview1` で preview1 wasm を出せます。
- `--target web-wasm` は browser 向け core wasm の Rust host fallback 経路です。
- `--target native` は native backend 経路です。native product scope は Linux x86_64 と Mac Apple Silicon を優先対象にしてください。
- Windows native や Intel Mac native は、この skill では通常の対応対象として案内しません。

## Known Limits

- 高カインド型、GADT、computation expressions は方向性として存在しますが、全機能を production-ready と断定しないでください。
- native-only / pure selfhosting 配布は進行中の領域です。通常ユーザーには host launcher + embedded guest component の導線を案内してください。
- `parse` / `check` / `fmt` の CLI 直叩きは公開 primary flow ではありません。詳細な補助操作は LSP / MCP を使います。
- 実行環境や release artifact の最新状態を断定する前に、repo の README、TODO、release docs、`lsharp --version` を確認してください。

## よく使う例

フィボナッチ:

```lisp
(defn fib [n]
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))
```

metadata 付き関数:

```lisp
(defn add
  [x y]
  :doc "2 つの整数を足す。"
  :params [(x "左辺") (y "右辺")]
  :returns "合計"
  :example [(= (add 1 2) 3)]
  (+ x y))
```

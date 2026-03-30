# Phase 12: モジュール・パッケージ & AI フレンドリーエコシステム ロードマップ

> このドキュメントは「完成後にどう使えるか」を起点に、各機能の実装方針を解説する。
> 仕様の詳細ではなく、ユーザー・AI エージェント・ライブラリ作者の視点から全体像を掴むための文書。

---

## 目次

1. [全体像: 完成後の世界](#1-全体像-完成後の世界)
2. [アーキテクチャ: lsharp-mcp](#2-アーキテクチャ-lsharp-mcp)
3. [P12-A: AI 連携基盤 (MCP Server + api.json)](#3-p12-a-ai-連携基盤)
4. [P12-B: パッケージシステムコア](#4-p12-b-パッケージシステムコア)
5. [P12-C: パッケージ配布 & エコシステム](#5-p12-c-パッケージ配布--エコシステム)
6. [依存関係と実装順序](#6-依存関係と実装順序)
7. [既存基盤との関係](#7-既存基盤との関係)

---

## 1. 全体像: 完成後の世界

### 設計思想

**llms.txt のような静的ファイルに頼らない。**

静的ファイルでは「今このプロジェクトが使っているパッケージの、このバージョンの API」を取得できない。
代わりに、L# エコシステム全体を 1 つの **MCP Server (`lsharp-mcp`)** で提供する:

- AI は MCP 経由で言語仕様もパッケージ API もバージョン指定で取得する
- `lsharp.toml` を読んで使用中のパッケージを自動認識する
- パッケージは **GitHub リポジトリ + タグ** のみで配布 (レジストリサーバーは立てない)
- **`lsharp` バイナリ 1 つに全てが入る**: コンパイラ・LSP・MCP Server・Claude Code プラグイン・stdlib
- Claude Code, Cursor, Codex, Gemini CLI など MCP 対応ツールならどれでも使える

### バージョニング

**コンパイラバージョン = 言語バージョン** (semver)。edition 制度なし。

```bash
$ lsharp --version
lsharp 0.1.0
```

```toml
# lsharp.toml — パッケージが要求する最低コンパイラバージョン
[project]
lsharp-version = ">=0.1.0"
```

stdlib はコンパイラに同梱され、常にコンパイラと同じバージョン。

### `lsharp` バイナリの構成

```
lsharp (単一バイナリ)
  ├── compile              # format → check → codegen を一括実行 (唯一の CLI コマンド)
  │     ├── -o foo.wasm    #   → Wasm 出力
  │     └── -o foo         #   → Native 出力
  ├── lsp                  # LSP サーバー (IDE 向け — check/hover/completion を提供)
  ├── mcp-server           # MCP サーバー (AI 向け — LSP をバックエンドに使う)
  ├── claude-plugin        # Claude Code プラグイン (MCP Server 登録 + Agent Skills インストール)
  ├── init / install / add # パッケージ管理
  ├── doc --json / doc-site # ドキュメント生成
  └── stdlib (埋め込み)     # 標準ライブラリ (バイナリに含む)
```

**CLI は `compile` に統一。** `check` / `format` / `parse` は CLI サブコマンドとしては廃止し、
LSP / MCP の内部 API としてのみ残す。CI は `lsharp compile` 一発で format + check + codegen が走る。

`lsharp` をインストールするだけで、IDE 連携も AI 連携もパッケージ管理も全て使える。

### AI エージェントの体験 (2 層構造)

**第 1 層: Agent Skills (常駐コンテキスト)**

`lsharp claude-plugin` を実行すると、Claude Code に L# の Agent Skills がインストールされる。
AI は会話開始時点で L# の構文・型システム・パターン・イディオムを「知っている」状態になる。

```
;; Agent Skills としてコンテキストに常駐する情報:
- L# は S 式構文 + Hindley-Milner 型推論の関数型言語
- (defn name [params] body) で関数定義
- (type Point (record (: x Int) (: y Int))) でレコード型
- (match expr [pattern body] ...) でパターンマッチ
- (import Module) でモジュール読み込み
- stdlib: Core, List, Map, Set, Vector, String, Char, IO, Json, Path, Debug
- ... (概要レベルの全情報)
```

**第 2 層: MCP ツール (オンデマンド)**

具体的な API・型チェック・パッケージ情報は MCP ツールで動的に取得する。

```
;; ユーザーがプロジェクトを開く (lsharp.toml に my-geometry = "0.1.0" がある)

AI: L# の構文は Agent Skills で既に知っている
AI: (MCP 経由でプロジェクト固有情報を取得)
  → lsharp_project_context   → 使用中パッケージ一覧: [my-geometry@0.1.0]
  → lsharp_package_api        → my-geometry の全関数・型・使用例

AI: コードを書く
  → lsharp_check              → 型チェック OK
  → lsharp_hover              → distance の型: Point -> Point -> Int
```

```lisp
;; AI が生成するコード
(module App)
(import Geometry)

(defn main []
  (let [a {Point x 0 y 0}
        b {Point x 3 y 4}]
    (print (distance a b))))
```

**ポイント:**
- **概要は Agent Skills で常駐** → ツール呼び出し不要、レイテンシゼロ
- **詳細は MCP で動的取得** → パッケージ・バージョンごとに正確な情報

### ライブラリ作者の体験

```bash
$ lsharp init my-geometry
$ cat src/Geometry.ls
```

```lisp
(module Geometry)

(type Point (record (: x Int) (: y Int)))

(defn distance
  :doc "2 点間のユークリッド距離を計算する"
  :params [(p1 "始点") (p2 "終点")]
  :returns "距離 (非負整数)"
  :example (distance {Point x 0 y 0} {Point x 3 y 4})
  [p1 p2]
  (let [dx (- (Point.x p2) (Point.x p1))
        dy (- (Point.y p2) (Point.y p1))]
    (+ (* dx dx) (* dy dy))))
```

```bash
# api.json を生成 (MCP Server がこれを配信する)
$ lsharp doc --json
  Generated docs/api.json

# パッケージ公開
$ lsharp publish
```

### ライブラリ利用者の体験

```bash
$ lsharp add my-geometry
$ lsharp install
```

```lisp
(import Geometry)
(defn main []
  (print (distance {Point x 0 y 0} {Point x 3 y 4})))
```

---

## 2. アーキテクチャ: lsharp-mcp (LSP-over-MCP)

### 設計原則: LSP をバックエンドにする

**MCP Server は LSP の薄いラッパーである。**

型チェック・hover・補完・診断・フォーマットなどの言語機能は全て既存の LSP (`lsharp-lsp`) に委譲する。
MCP Server が独自にコンパイラパイプラインを呼び出すことはない。
これにより:

- **コンパイラロジックの重複ゼロ** — 型チェック・診断が 1 箇所で管理される
- **LSP の改善が AI にも自動反映** — LSP を修正すれば MCP 経由の AI も恩恵を受ける
- **IDE ユーザーと AI が同じ品質の分析を受けられる**

### 全体構成

```
AI Agent (Claude Code / Cursor / Codex / Gemini CLI)
  │
  │ MCP protocol (stdio or HTTP)
  ▼
lsharp-mcp (ローカル MCP Server — 薄いラッパー)
  │
  ├── LsharpBackend (LSP) ← 型チェック・診断・hover・定義ジャンプ・補完・フォーマット
  │     └── parse_and_check(), hover(), completion(), formatting() を内部呼び出し
  ├── api.json reader     ← パッケージ API ドキュメント (バージョン指定取得)
  ├── lsharp.toml reader  ← プロジェクト情報・依存一覧
  └── registry client     ← パッケージ検索 (未キャッシュ時のフォールバック)
```

**ポイント:** `lsharp-mcp` は `lsharp-lsp` の `LsharpBackend` をライブラリとして組み込む。
JSON-RPC で LSP プロセスと通信するのではなく、Rust レベルで直接メソッドを呼ぶ。
LSP の `pub` API (`parse_and_check`, `find_definition`, `find_references`, `format_source`) が
MCP ツールのバックエンドになる。

### MCP ツール一覧

AI が呼び出せるツール:

| ツール名 | 引数 | バックエンド | 戻り値 | 用途 |
|---------|------|-------------|--------|------|
| `lsharp_check` | `source` or `file` | **LSP** (parse_and_check) | 診断結果 (エラー・警告) | コードを検証する |
| `lsharp_hover` | `file`, `line`, `col` | **LSP** (hover) | 型情報 + :doc メタデータ | シンボルの詳細を調べる |
| `lsharp_completion` | `file`, `line`, `col` | **LSP** (completion) | 補完候補一覧 | コード補完を取得する |
| `lsharp_format` | `source` | **LSP** (format_source) | フォーマット済みソース | コードを整形する |
| `lsharp_definition` | `file`, `line`, `col` | **LSP** (find_definition) | 定義位置 | シンボルの定義に飛ぶ |
| `lsharp_references` | `file`, `line`, `col` | **LSP** (find_references) | 参照位置一覧 | シンボルの使用箇所を探す |
| `lsharp_project_context` | なし | lsharp.toml | プロジェクト情報 + 依存一覧 | プロジェクト状態を把握する |
| `lsharp_package_api` | `name`, `version?` | api.json | パッケージの全関数・型・使用例 | パッケージ API を理解する |
| `lsharp_stdlib_api` | `module?` | api.json | stdlib の全/指定モジュール API | 標準ライブラリを使う |
| `lsharp_search` | `query` | registry | マッチするパッケージ一覧 | パッケージを探す |
| `lsharp_compile_run` | `source` or `file` | compile (format+check+codegen) + wasmtime | コンパイル + 実行結果 | コードを動かす |
| `lsharp_errors` | `error_code` | 静的辞書 | エラーの説明と対処法 | エラーを理解する |

**LSP バックエンド (6 ツール)** vs **MCP 独自 (6 ツール)** で半々。
言語機能は LSP に集約し、パッケージ・プロジェクト管理は MCP 独自。

### AI のワークフロー (自動)

```
1. プロジェクトを開く
   AI → lsharp_project_context
   戻り値: { packages: [{ name: "my-geometry", version: "0.1.0" }] }

2. 使用パッケージの API を取得
   AI → lsharp_package_api(name: "my-geometry", version: "0.1.0")
   戻り値: { modules: [{ name: "Geometry", functions: [...], types: [...] }] }

3. コードを書く

4. 型チェック (LSP 経由)
   AI → lsharp_check(source: "(defn main [] (print (distance ...)))")
   戻り値: { ok: true, diagnostics: [] }
   (失敗時: { ok: false, diagnostics: [{ line: 1, message: "type mismatch", code: "E0005" }] })

5. エラーの詳細を調べる (必要に応じて)
   AI → lsharp_errors(error_code: "E0005")
   戻り値: { description: "...", fix: "..." }

6. シンボルの型を確認 (必要に応じて)
   AI → lsharp_hover(file: "src/Main.ls", line: 3, col: 10)
   戻り値: { type: "Point -> Point -> Int", doc: "2 点間のユークリッド距離を計算する" }
```

### MCP Server の設定例

**Claude Code (settings.json):**

```json
{
  "mcpServers": {
    "lsharp": {
      "command": "lsharp",
      "args": ["mcp-server"],
      "env": {}
    }
  }
}
```

**Cursor (.cursor/mcp.json):**

```json
{
  "mcpServers": {
    "lsharp": {
      "command": "lsharp",
      "args": ["mcp-server"]
    }
  }
}
```

`lsharp mcp-server` は `lsharp` バイナリのサブコマンドとして実装する。
別途インストール不要で、言語のツールチェインに MCP Server が組み込まれている形。

### LSP の拡張ロードマップ

MCP から活用するために、LSP に以下の機能追加が必要:

| 機能 | 現状 | 追加内容 |
|------|------|---------|
| hover | 枠だけ (TODO) | AST の `:doc` メタデータ + 型推論結果を返す |
| completion | 未実装 | スコープ内シンボル + import 候補 |
| diagnostics | 実装済み | そのまま MCP に委譲 |
| definition | 実装済み | そのまま MCP に委譲 |
| references | 実装済み | そのまま MCP に委譲 |
| formatting | 実装済み | そのまま MCP に委譲 |

---

## 3. P12-A: AI 連携基盤

### A-1. api.json スキーマと生成コマンド

**完成後の使い方:**

```bash
$ lsharp doc --json
  Generated docs/api.json

$ lsharp doc --json src/Geometry.ls   # 単一ファイル
```

**生成される api.json:**

```json
{
  "package": "my-geometry",
  "version": "0.1.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": "2D 幾何ライブラリ",
      "functions": [
        {
          "name": "distance",
          "signature": "Point -> Point -> Int",
          "params": [
            { "name": "p1", "type": "Point", "doc": "始点" },
            { "name": "p2", "type": "Point", "doc": "終点" }
          ],
          "returns": { "type": "Int", "doc": "距離 (非負整数)" },
          "doc": "2 点間のユークリッド距離を計算する",
          "example": "(distance {Point x 0 y 0} {Point x 3 y 4})"
        }
      ],
      "types": [
        {
          "name": "Point",
          "kind": "record",
          "fields": [
            { "name": "x", "type": "Int" },
            { "name": "y", "type": "Int" }
          ]
        }
      ]
    }
  ]
}
```

**情報の流れ:**

```
Source (.ls)
  → Parser    → AST + Metadata (:doc, :params, :returns, :example)
  → TypeInfer → 型シグネチャ (Point -> Point -> Int)
  → api_doc.rs → api.json
  → lsharp-mcp → AI に配信
```

**実装方針:**

- `crates/lsharp-driver/src/api_doc.rs` を新規作成
- AST の `Metadata` (`:doc`, `:params`, `:returns`, `:example`) + 型推論結果を結合
- `docs/schemas/knowledge.schema.json` を拡張してスキーマ定義

**修正対象:** `docs/schemas/knowledge.schema.json`, `crates/lsharp-driver/src/main.rs`, 新規 `crates/lsharp-driver/src/api_doc.rs`

---

### A-2. lsharp-mcp Server 実装 (LSP-over-MCP)

**完成後の使い方 (AI 側):**

```
;; Claude Code が自動的にツールとして認識する

;; --- LSP バックエンドツール (言語機能) ---
Tool: lsharp_check { source: "(defn main [] (print (+ 1 \"hello\")))" }
  → { ok: false, diagnostics: [{ line: 1, col: 26, code: "E0005", message: "type mismatch: Int vs String" }] }

Tool: lsharp_hover { file: "src/Main.ls", line: 3, col: 10 }
  → { type: "Point -> Point -> Int", doc: "2 点間のユークリッド距離を計算する" }

Tool: lsharp_completion { file: "src/Main.ls", line: 5, col: 3 }
  → [{ label: "distance", type: "Point -> Point -> Int" }, { label: "Point", kind: "type" }]

Tool: lsharp_format { source: "(defn   main  []  ( + 1 2))" }
  → "(defn main [] (+ 1 2))"

;; --- MCP 独自ツール (パッケージ・プロジェクト管理) ---
Tool: lsharp_project_context
  → { name: "my-app", dependencies: [{ name: "my-geometry", version: "0.1.0" }] }

Tool: lsharp_package_api { name: "my-geometry", version: "0.1.0" }
  → api.json の中身をそのまま返す

Tool: lsharp_stdlib_api { module: "List" }
  → List モジュールの全関数・型情報

Tool: lsharp_compile_run { file: "src/Main.ls" }
  → { ok: true, stdout: "25", exit_code: 0 }

Tool: lsharp_errors { error_code: "E0005" }
  → { code: "E0005", name: "type mismatch", description: "...", fix: "..." }
```

**実装方針:**

- `lsharp mcp-server` サブコマンドとして実装 (stdio transport)
- MCP SDK (Rust) を使用するか、JSON-RPC を直接実装
- **LSP バックエンドツール** は `lsharp-lsp` の `LsharpBackend` をライブラリとして組み込み:
  - `lsharp_check` → `util::parse_and_check()` を呼び出し、`Diagnostic` を JSON 変換
  - `lsharp_hover` → `LsharpBackend::hover()` を呼び出し (要: hover の実装完了)
  - `lsharp_completion` → `LsharpBackend::completion()` を呼び出し (要: completion の新規実装)
  - `lsharp_format` → `format::format_source()` を呼び出し
  - `lsharp_definition` → `util::find_definition()` を呼び出し
  - `lsharp_references` → `references::find_references()` を呼び出し
- **MCP 独自ツール** は LSP を経由しない:
  - `lsharp_project_context` → `lsharp.toml` を parse
  - `lsharp_package_api` → `.lsharp/packages/<name>/docs/api.json` を読む
  - `lsharp_stdlib_api` → stdlib の api.json を読む
  - `lsharp_search` → レジストリ HTTP API (C-1 完了後)、それまではローカルのみ
  - `lsharp_compile_run` → 内部で compile + wasmtime 実行
  - `lsharp_errors` → エラーコード辞書 (静的データ)

**前提: LSP の拡張が必要**

A-2 の前に LSP 側で以下を完了する必要がある:
- hover 実装 (現在 TODO): AST の `:doc` + 型推論結果を返す
- completion 新規実装: スコープ内シンボル + import 候補

**修正対象:** `crates/lsharp-driver/src/main.rs`, `crates/lsharp-lsp/src/lib.rs`, 新規 `crates/lsharp-driver/src/mcp_server.rs`

---

### A-3. stdlib メタデータ整備

**現状:**

```lisp
;; Core.ls (現状)
;; 絶対値
(defn abs [x] (if (< x 0) (- 0 x) x))
```

**完成後:**

```lisp
;; Core.ls (完成後)
(defn abs
  :doc "整数の絶対値を返す"
  :params [(x "対象の整数")]
  :returns "x の絶対値 (非負整数)"
  :example (abs (- 0 5))
  [x]
  (if (< x 0) (- 0 x) x))
```

**AI から MCP 経由で取得した場合の見え方:**

```
Tool: lsharp_stdlib_api { module: "Core" }
→ {
    module: "Core",
    doc: "基本ユーティリティ: Bool 操作、数学関数、Option/Result 型",
    functions: [
      { name: "abs", signature: "Int -> Int", doc: "整数の絶対値を返す", ... },
      { name: "max", signature: "Int -> Int -> Int", doc: "2 値の最大値", ... },
      { name: "unwrap", signature: "Option a -> a -> a", doc: "Option から値を取り出す", ... },
      ...
    ],
    types: [
      { name: "Option", kind: "adt", variants: ["Some a", "None"], ... },
      { name: "Result", kind: "adt", variants: ["Ok a", "Err e"], ... }
    ]
  }
```

**対象:** 11 モジュール × 平均 10 関数 = 約 110 関数にメタデータ付与

**修正対象:** `stdlib/*.ls` 全 11 ファイル

---

### A-4. 言語リファレンスと利用ガイド

**完成後の 2 層構造:**

| 層 | 配信手段 | 内容 | AI の体験 |
|---|---------|------|----------|
| 概要 | **Claude Code Agent Skills** | 構文・型システム・パターン・イディオム・stdlib 一覧 | コンテキストに常駐。ツール呼び出し不要 |
| 詳細 | **MCP ツール** (LSP バックエンド) | 関数シグネチャ・型チェック・hover・補完 | 必要時にオンデマンド取得 |

**Agent Skills (概要 — コンテキスト常駐):**

`lsharp claude-plugin` を実行すると、以下の Agent Skills がインストールされる:

```markdown
# L# (lsharp) Language Guide

## 概要
S 式構文 + Hindley-Milner 型推論の関数型言語。WebAssembly (WASI) / Native をターゲットとする。

## 構文早見表
- 関数定義: (defn name [params] body)
- ラムダ: (fn [params] body)
- let 束縛: (let [x 1 y 2] (+ x y))
- 条件分岐: (if cond then else)
- 逐次実行: (do expr1 expr2 ... result)
- パターンマッチ: (match expr [pattern body] ...)
- ADT 定義: (type (Option a) (Some a) None)
- レコード定義: (type Point (record (: x Int) (: y Int)))
- レコードリテラル: {Point x 1 y 2}
- フィールドアクセス: (Point.x p)
- モジュール: (module Name) / (import Module)
- トレイト: (trait (Show a) (defn show [a] : String))
- メタデータ: :doc "説明" :params [(x "説明")] :returns "説明"

## 型システム
- プリミティブ: Int, Float, String, Bool, Unit
- 推論: Hindley-Milner (型注釈は任意)
- 関数: 全てカリー化 (Int -> Int -> Int)
- 多相: パラメトリック多相 (Option a), (Result a e)

## stdlib モジュール
Core, List, Map, Set, Vector, String, Char, IO, Json, Path, Debug

## よくあるパターン
- Option: (match opt [(Some x) x] [None default])
- リスト変換: (map f (filter pred xs))
- レコード更新: {(original) | x 10}

## MCP ツールで詳細を取得
- lsharp_hover: シンボルの型と :doc を取得
- lsharp_completion: スコープ内の補完候補
- lsharp_check: 型チェック
- lsharp_package_api: パッケージの全 API
```

**MCP ツール (詳細 — オンデマンド):**

```
;; AI: distance 関数の詳細を知りたい → lsharp_hover で型 + :doc を取得
;; AI: 何が使えるか知りたい → lsharp_completion でスコープ内の候補を取得
;; AI: パッケージの API を知りたい → lsharp_package_api で全関数・型を取得
```

**人間向けドキュメント:**

Agent Skills と同じ情報源から人間向け Markdown も生成する:

```
docs/guides/
  quick-start.md        # 5 分チュートリアル (hello → fib → ADT → record → module)
  language-reference.md  # 構文・型・モジュール完全リファレンス
```

**実装方針:**

- Agent Skills のテンプレートは `crates/lsharp-driver/` にテキストとして同梱
- `lsharp claude-plugin` が Claude Code の Agent Skills ディレクトリにインストール
- 個々のシンボル情報は LSP hover/completion 経由で動的に提供 (MCP がラップ)
- Markdown ガイドは同じテンプレートから生成

---

### A-5. ドキュメントサイト生成

**完成後:**

```bash
$ lsharp doc-site --output _site/
  Language reference ... ok
  Stdlib API (11 modules) ... ok
  Guides (3 pages) ... ok
  Site generated: _site/
```

```
_site/
  index.html
  guides/
    quick-start.html
    language-reference.html
  api/
    Core.html
    List.html
    ...
    stdlib.json          # 機械可読 API (MCP Server がリモートで配信する形式と同一)
```

**実装方針:** 既存 HtmlTemplate.ls / HtmlLayout.ls パイプラインを拡張

---

## 4. P12-B: パッケージシステムコア

> `package` は公開・配布単位を指す。`selfhost` は公開 package ではなく、同じ `src/` / dotted import 規約を使う **内部 source root** として扱う。
> 正本 entrypoint は `selfhost/src/App/Main.ls`。仕様上の selfhost source tree 基準は `selfhost/src/**` とする。
> 現時点の実装では、Rust 側は package src / `.lsharp/packages/*/src` / stdlib の探索順を満たし、selfhost 側も `src/` 祖先 discovery・dotted local import・stdlib fallback に加えて `.lsharp/module-index/*.path` 経由の installed package 解決まで反映済みである。flat な旧互換コピーは撤去済みで、repo 内参照も `selfhost/src/**` 基準へ統一した。

### B-1. lsharp.toml スキーマ拡張

**完成後の lsharp.toml:**

```toml
[project]
name = "my-geometry"
version = "0.1.0"
description = "2D 幾何プリミティブと距離計算"
license = "MIT"
authors = ["Author Name <email>"]
repository = "https://github.com/user/my-geometry"
keywords = ["geometry", "math", "2d"]
lsharp-version = ">=0.1.0"
entry = "src/Main.ls"

# 公開するモジュール (省略時は全モジュール公開)
[project.exports]
modules = ["Geometry", "Geometry.Vec2"]

[dependencies]
math-core = "1.0.0"

[dependencies.my-utils]
git = "https://github.com/user/my-utils.git"
tag = "v2.0"

[dev-dependencies]
test-helpers = "0.1.0"
```

**現状との差分:**

```
[project] に追加: description, license, authors, repository, keywords, lsharp-version
新規セクション:    [project.exports], [dev-dependencies]
削除:              [project.ai] → 不要 (MCP Server が api.json から動的に提供)
```

**実装方針:**

- `crates/lsharp-driver/src/config.rs` の `ProjectConfig` に新フィールドを `#[serde(default)]` で追加
- 後方互換: 既存の lsharp.toml はそのまま動く

---

### B-2. パッケージディレクトリ規約と lsharp init

**完成後:**

```bash
$ lsharp init my-lib
  Created my-lib/lsharp.toml
  Created my-lib/src/Main.ls
  Created my-lib/.gitignore
```

**標準レイアウト:**

```
my-package/
  lsharp.toml              # パッケージ定義 (必須)
  src/                     # ソースコード
    Main.ls                #   エントリポイント
    MyModule.ls            #   → (import MyModule) で参照
    MyModule/
      Sub.ls               #   → (import MyModule.Sub) で参照
  examples/
  tests/
  docs/
    api.json               # lsharp doc --json で生成 (MCP Server が読む)
```

**モジュール解決:**

```
(import Utils) の探索順:
  1. src/Utils.ls (同パッケージ)
  2. .lsharp/packages/*/src/Utils.ls (依存パッケージ)
  3. <stdlib-path>/Utils.ls (標準ライブラリ)
```

**内部 source root への適用:**

```text
selfhost/
  src/
    App/
      Main.ls
    Syntax/
      Token.ls
```

- `selfhost` は publish 対象ではないが、`selfhost/src/App/Main.ls` を entry にすると `(import Syntax.Token)` は `selfhost/src/Syntax/Token.ls` を解決する
- `lsharp.toml` が無い場合も、entry から最も近い `src/` 祖先を source root として扱う

---

### B-3. 可視性制御の実装

**完成後:**

```lisp
;; Geometry.ls
(module Geometry)
(defn distance [p1 p2] ...)         ;; 公開
(private
  (defn internal-helper [x] ...))   ;; 非公開
```

```lisp
;; app.ls
(import Geometry :only [distance])
(distance a b)                       ;; OK
(internal-helper 42)                 ;; ERROR: private
```

**3 段階の制御:**

| レベル | 制御対象 | 仕組み |
|--------|----------|--------|
| シンボル | `(import M :only [f])` | 未列挙シンボルの参照をエラー |
| 宣言 | `(private (defn ...))` | 他モジュールから private 参照をエラー |
| モジュール | `[project.exports]` | 外部パッケージから非公開モジュールの import をエラー |

---

### B-4. stdlib 自動リンク

**完成後:**

```lisp
;; 設定なしで使える
(import List)
(defn main []
  (print (length (Cons 1 (Cons 2 Nil)))))
```

**公開 package の解決順序:** local src/ → .lsharp/packages/ → stdlib/ → $LSHARP_STDLIB_PATH

**内部 source root (`selfhost`) の現行方針:**

- `selfhost/src/**` を local source root として優先する
- dotted module 名は `/` に変換して解決する
- flat fallback は持たず、正本は nested path のみとする
- installed package は `lsharp install` が生成する `.lsharp/module-index/*.path` を経由して解決する

---

### B-5. 依存関係解決とインストール

**完成後:**

```bash
$ lsharp add github.com/user/geometry-utils --tag v0.2.0
  Added geometry-utils to lsharp.toml

$ lsharp install
  Cloning geometry-utils@v0.2.0 ...
    math-core@v1.0.3 (transitive)
  Generating api.json ... ok
  Lock file written: .lsharp/lock.toml
```

**インストール後のディレクトリ:**

```
.lsharp/
  lock.toml
  packages/
    geometry-utils-a1b2c3/
      lsharp.toml
      src/
      docs/api.json          # MCP Server はこれを読んで AI に返す
    math-core-d4e5f6/
      lsharp.toml
      src/
      docs/api.json
```

**lock.toml:**

```toml
[metadata]
lsharp-version = "0.1.0"
generated = "2026-03-27T10:00:00Z"

[[package]]
name = "geometry-utils"
version = "0.2.0"
source = "git+https://github.com/user/geometry-utils.git#tag=v0.2.0"
checksum = "sha256:abc123..."
```

**バージョン解決:**

| 記法 | 意味 |
|------|------|
| `"1.0.0"` | >=1.0.0, <2.0.0 (Cargo 互換) |
| `"=1.0.0"` | 完全一致 |
| `">=1.2.0"` | 1.2.0 以上 |

---

## 5. P12-C: パッケージ配布 & エコシステム

### 設計方針: GitHub リポジトリのみ

**レジストリサーバーは立てない。** GitHub リポジトリ + Git タグが唯一の配布手段。

理由:
- ユーザーが少ない段階でサーバー運用は過剰
- GitHub は既にバージョン管理・認証・可用性を提供している
- `git clone --depth 1 --branch <tag>` で十分高速に取得できる

### C-1. GitHub ベースのパッケージ配布

**lsharp.toml での依存宣言:**

```toml
[dependencies.my-geometry]
git = "https://github.com/user/my-geometry.git"
tag = "v0.1.0"
```

```bash
$ lsharp install
  Cloning my-geometry@v0.1.0 from github.com/user/my-geometry ...
  Generating api.json ... ok
  Lock file written: .lsharp/lock.toml
```

`lsharp install` は以下を行う:
1. `git clone --depth 1 --branch <tag>` でソースを `.lsharp/packages/` に取得
2. `lsharp doc --json` を自動実行して `docs/api.json` を生成
3. MCP Server はこの api.json を読んで AI に返す

### C-2. パッケージ検証 (`lsharp check-package`)

```bash
$ lsharp check-package
  Validating lsharp.toml ... ok
  Generating api.json ... ok
  Comparing with v0.1.0 (previous tag) ...
    + added: rotate (Geometry.Vec2)
    ~ changed: distance return type Int → Float  ⚠ BREAKING
  checksum: sha256:abc123...
```

パッケージ作者が `git tag` する前にローカルで実行する検証コマンド。
`lsharp publish` のような中央登録は不要。

### C-3. API diff & 互換性チェック

```bash
$ lsharp api-diff v0.1.0 v0.2.0
  Added:    + Geometry.Vec2.rotate : Vec2 -> Float -> Vec2
  Changed:  ~ Geometry.distance : Int → Float  ⚠ BREAKING
  Removed:  (none)
```

2 つの Git タグ間で api.json を比較する。

### C-4. AI パッケージ理解サポート

**MCP 経由:**

```
Tool: lsharp_package_api { name: "my-geometry" }
→ (インストール済みパッケージの api.json を返す)
```

**CLI 経由 (人間向け):**

```bash
$ lsharp info my-geometry
  Package: my-geometry@0.2.0
  Source: github.com/user/my-geometry
  Modules: Geometry, Geometry.Vec2
  Functions:
    distance : Point -> Point -> Float  — 2 点間の距離
    rotate   : Vec2 -> Float -> Vec2    — ベクトル回転
```

**AI のパッケージ利用フロー:**

```
1. ユーザー: 「my-geometry パッケージを使いたい」
2. AI: lsharp.toml の [dependencies] に git URL + tag を追記
3. AI → lsharp_compile_run でインストール + コード検証
4. 完成
```

注意: レジストリがないため、パッケージ検索 (`lsharp_search`) はインストール済みパッケージのみ対象。
新しいパッケージの発見は GitHub 上での検索やコミュニティ情報に委ねる。

---

## 6. 依存関係と実装順序

```
Phase 12-A (AI 連携基盤) ← Phase 11 と独立して着手可能
│
├─ A-1 api.json スキーマ + 生成     ← 最初に着手 (全ての土台)
├─ A-1.5 LSP 拡張 (hover + completion) ← A-2 の前提条件
├─ A-2 lsharp-mcp Server 実装       ← A-1, A-1.5 完了後 (LSP + api.json を配信)
├─ A-3 stdlib メタデータ整備        ← A-1 完了後 (api.json で検証)
├─ A-4 言語リファレンス + ガイド    ← A-2 と並行可能
└─ A-5 ドキュメントサイト           ← A-1〜A-4 完了後

Phase 12-B (パッケージコア) ← A-1, A-2 完了後に着手推奨
│
├─ B-1 lsharp.toml 拡張            ← 最初に着手 (config.rs のみ)
├─ B-2 ディレクトリ規約 + init      ← B-1 完了後
├─ B-3 可視性制御                   ← B-1 完了後
├─ B-4 stdlib 自動リンク            ← B-2 完了後
└─ B-5 依存解決 + install           ← B-1, B-2, B-4 完了後

Phase 12-C (配布 — GitHub only) ← B-5 完了後に着手推奨
│
├─ C-1 GitHub ベース配布            ← B-5 完了後 (git clone + api.json 自動生成)
├─ C-2 check-package                ← C-1, A-1 完了後
├─ C-3 API diff                     ← A-1 完了後 (api.json 比較)
└─ C-4 info                         ← C-1 完了後
```

**MVP (最小実用セット):**

A-1 (api.json) + A-1.5 (LSP hover/completion) + A-2 (lsharp-mcp) + A-3 (stdlib メタデータ) の 4 つで、
AI が MCP 経由で L# の型チェック・hover・補完と stdlib API を利用し、正しいコードを書ける状態になる。
パッケージシステムがなくても、この 4 つで AI 連携の最低ラインを達成できる。

---

## 7. 既存基盤との関係

### 拡張するもの

| 既存 | 拡張内容 |
|------|---------|
| `lsharp-lsp` (lib.rs) | hover 実装完了、completion 新規追加、MCP から直接呼び出し可能に |
| `config.rs` (lsharp.toml) | `[project.exports]`, `[dev-dependencies]` 追加 |
| `module_graph.rs` | 検索パスに `src/`, `.lsharp/packages/`, `stdlib/` 追加 |
| `knowledge.schema.json` | 型シグネチャ・パラメータ docs フィールド追加 |
| `ast.rs` (Metadata) | `:doc` / `:params` / `:returns` をパイプライン全体で伝搬 |
| `stdlib/*.ls` | 全関数にメタデータ付与 |
| CLI (main.rs) | `mcp-server`, `claude-plugin`, `doc --json`, `doc-site`, `init`, `install`, `add`, `check-package`, `info`, `api-diff` 追加 |

### 新規作成するもの

| ファイル | 役割 |
|---------|------|
| `crates/lsharp-driver/src/mcp_server.rs` | MCP Server (コア — LSP ラッパー + パッケージ管理ツール) |
| `crates/lsharp-lsp/src/completion.rs` | LSP completion 実装 (新規) |
| `crates/lsharp-driver/src/api_doc.rs` | AST + 型情報 → api.json 生成 |
| `crates/lsharp-driver/src/init.rs` | `lsharp init` スキャフォールド |
| `crates/lsharp-driver/src/resolver.rs` | 依存解決・ダウンロード・lock.toml |
| `docs/guides/quick-start.md` | 5 分チュートリアル |
| `docs/guides/language-reference.md` | 言語リファレンス |

### 旧設計から削除したもの

| 削除 | 理由 |
|------|------|
| `llms.txt` | MCP Server が動的に情報を提供するため不要 |
| `[project.ai]` セクション | api.json + lsharp.toml の description で十分 |
| `crates/lsharp-driver/src/llms.rs` | llms.txt 生成が不要になったため |
| HTTP レジストリサーバー | GitHub リポジトリ + Git タグで配布。サーバー運用は過剰 |
| `lsharp publish` | 中央レジストリがないため不要。`git tag` + `git push --tags` で公開 |
| `lsharp_search` リモート検索 | レジストリがないため。インストール済みパッケージのローカル検索のみ |
| `lsharp_language_reference` MCP ツール | Agent Skills で概要を常駐提供するため、MCP ツールとしては不要 |
| MCP リソース `lsharp://language-reference` | 同上。Agent Skills に統合 |
| CLI `check` / `format` / `parse` | `compile` に統合。LSP / MCP の内部 API としてのみ存続 |

### 変更しないもの

- Phase 11 の bootstrap / native / GC 関連コード
- 既存の E2E テスト群
- selfhost/src/** コンパイラモジュール群 (Phase 12 では Rust 側で実装し、将来 selfhost へ移行)

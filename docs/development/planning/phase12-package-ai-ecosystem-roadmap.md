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
- ローカルキャッシュ → リモートレジストリの 2 段構えでドキュメントを解決する
- Claude Code, Cursor, Codex, Gemini CLI など MCP 対応ツールならどれでも使える

### AI エージェントの体験

```
;; ユーザーがプロジェクトを開く (lsharp.toml に my-geometry = "0.1.0" がある)

AI: (MCP 経由で自動取得)
  → lsharp_project_context   → 使用中パッケージ一覧: [my-geometry@0.1.0]
  → lsharp_language_reference → L# の構文・型システム・パターン
  → lsharp_package_api        → my-geometry の全関数・型・使用例

AI: 情報が揃ったので正しいコードを書ける
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

**ポイント: AI は静的ファイルを読まない。MCP ツールを呼ぶだけ。**

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

## 2. アーキテクチャ: lsharp-mcp

### 全体構成

```
AI Agent (Claude Code / Cursor / Codex / Gemini CLI)
  │
  │ MCP protocol (stdio or HTTP)
  ▼
lsharp-mcp (ローカル MCP Server)
  │
  ├── lsharp.toml を読む → 使用中パッケージ + バージョンを把握
  ├── .lsharp/packages/ を読む → ローカルキャッシュの api.json を返す
  ├── stdlib/ を読む → 標準ライブラリの API を返す
  └── registry (HTTP) → 未キャッシュのパッケージ情報をフェッチ
```

### MCP ツール一覧

AI が呼び出せるツール:

| ツール名 | 引数 | 戻り値 | 用途 |
|---------|------|--------|------|
| `lsharp_language_reference` | なし | L# 構文・型・パターン全リファレンス | 言語を理解する |
| `lsharp_project_context` | なし | lsharp.toml の内容 + 依存一覧 | プロジェクト状態を把握する |
| `lsharp_package_api` | `name`, `version?` | パッケージの全関数・型・使用例 | パッケージ API を理解する |
| `lsharp_stdlib_api` | `module?` | stdlib の全/指定モジュール API | 標準ライブラリを使う |
| `lsharp_search` | `query` | マッチするパッケージ一覧 | パッケージを探す |
| `lsharp_check` | `source` | 型チェック結果 + エラー | コードを検証する |
| `lsharp_compile_run` | `source` or `file` | コンパイル + 実行結果 | コードを動かす |
| `lsharp_errors` | `error_code` | エラーの説明と対処法 | エラーを理解する |

### AI のワークフロー (自動)

```
1. プロジェクトを開く
   AI → lsharp_project_context
   戻り値: { packages: [{ name: "my-geometry", version: "0.1.0" }] }

2. 言語仕様を把握 (初回のみ)
   AI → lsharp_language_reference
   戻り値: L# の構文早見表・型システム・import パターン

3. 使用パッケージの API を取得
   AI → lsharp_package_api(name: "my-geometry", version: "0.1.0")
   戻り値: {
     modules: [{
       name: "Geometry",
       functions: [{ name: "distance", signature: "Point -> Point -> Int", ... }],
       types: [{ name: "Point", kind: "record", fields: [...] }]
     }]
   }

4. コードを書く (上記情報に基づいて)

5. 検証
   AI → lsharp_check(source: "...")
   戻り値: { ok: true } or { errors: [...] }
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

### A-2. lsharp-mcp Server 実装

**完成後の使い方 (AI 側):**

```
;; Claude Code が自動的にツールとして認識する
Tool: lsharp_language_reference
  → L# の構文・型システム・パターンの完全リファレンスを返す

Tool: lsharp_package_api { name: "my-geometry", version: "0.1.0" }
  → api.json の中身をそのまま返す

Tool: lsharp_project_context
  → { name: "my-app", dependencies: [{ name: "my-geometry", version: "0.1.0" }] }

Tool: lsharp_stdlib_api { module: "List" }
  → List モジュールの全関数・型情報

Tool: lsharp_search { query: "geometry" }
  → [{ name: "my-geometry", version: "0.1.0", summary: "2D 幾何ライブラリ" }]

Tool: lsharp_check { source: "(defn main [] (print (+ 1 \"hello\")))" }
  → { ok: false, errors: [{ code: "E0005", message: "type mismatch: Int vs String" }] }

Tool: lsharp_compile_run { file: "src/Main.ls" }
  → { ok: true, stdout: "25", exit_code: 0 }

Tool: lsharp_errors { error_code: "E0005" }
  → { code: "E0005", name: "type mismatch", description: "...", fix: "..." }
```

**実装方針:**

- `lsharp mcp-server` サブコマンドとして実装 (stdio transport)
- MCP SDK (Rust) を使用するか、JSON-RPC を直接実装
- 各ツールは既存の lsharp CLI 機能を呼び出すラッパー:
  - `lsharp_language_reference` → 静的テンプレート (コンパイラ同梱)
  - `lsharp_package_api` → `.lsharp/packages/<name>/docs/api.json` を読む
  - `lsharp_project_context` → `lsharp.toml` を parse
  - `lsharp_stdlib_api` → stdlib の api.json を読む
  - `lsharp_search` → レジストリ HTTP API (C-1 完了後)、それまではローカルのみ
  - `lsharp_check` → 内部で parse + type check
  - `lsharp_compile_run` → 内部で compile + wasmtime 実行
  - `lsharp_errors` → エラーコード辞書 (静的データ)

**修正対象:** `crates/lsharp-driver/src/main.rs`, 新規 `crates/lsharp-driver/src/mcp_server.rs`

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

**完成後:**

AI が `lsharp_language_reference` ツールを呼ぶと返ってくるデータ:

```json
{
  "language": "L# (lsharp)",
  "description": "S 式構文 + Hindley-Milner 型推論の関数型言語。WebAssembly (WASI) ターゲット。",
  "syntax": {
    "function_def": "(defn name [params] body)",
    "lambda": "(fn [params] body)",
    "let": "(let [x 1 y 2] (+ x y))",
    "if": "(if cond then else)",
    "do": "(do expr1 expr2 ... result)",
    "match": "(match expr [pattern body] ...)",
    "adt": "(type (Option a) (Some a) None)",
    "record_def": "(type Point (record (: x Int) (: y Int)))",
    "record_lit": "{Point x 1 y 2}",
    "field_access": "(Point.x p)",
    "module": "(module Name)",
    "import": "(import Module)",
    "import_alias": "(import Module :as Alias)",
    "import_only": "(import Module :only [sym1 sym2])",
    "trait_def": "(trait (Show a) (defn show [a] : String))",
    "trait_impl": "(impl (Show Int) (defn show [x] (int-to-string x)))",
    "metadata": ":doc \"説明\" :params [(x \"説明\")] :returns \"説明\""
  },
  "type_system": {
    "primitives": ["Int", "Float", "String", "Bool", "Unit"],
    "inference": "Hindley-Milner (型注釈は任意)",
    "functions": "全てカリー化 (Int -> Int -> Int)",
    "polymorphism": "パラメトリック多相 (Option a), (Result a e)",
    "records": "構造的レコード型 {Point x Int y Int}"
  },
  "stdlib_modules": ["Core", "List", "Map", "Set", "Vector", "String", "Char", "IO", "Json", "Path", "Debug"],
  "common_errors": [
    { "code": "E0001", "name": "undefined symbol", "fix": "(import Module) を追加" },
    { "code": "E0005", "name": "type mismatch", "fix": "引数の型を確認" },
    { "code": "E0006", "name": "pattern mismatch", "fix": "match のパターンを見直す" }
  ],
  "idioms": [
    { "name": "Option のアンラップ", "code": "(match opt [(Some x) x] [None default])" },
    { "name": "リスト変換", "code": "(let [result (map f (filter pred xs))] result)" },
    { "name": "レコード更新", "code": "{(original) | x 10}" }
  ]
}
```

同じデータを人間向け Markdown (`docs/guides/`) としても生成する:

```
docs/guides/
  quick-start.md        # 5 分チュートリアル (hello → fib → ADT → record → module)
  language-reference.md  # 構文・型・モジュール完全リファレンス
  ai-guide.md           # AI エージェント向け利用ガイド
```

**実装方針:**

- リファレンスデータは構造化 JSON として `crates/lsharp-driver/` に同梱
- MCP Server はこの JSON をそのまま返す
- Markdown ガイドは同じ JSON から生成 (または手書き + JSON を正本として同期)

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

**解決順序:** local src/ → .lsharp/packages/ → stdlib/ → $LSHARP_STDLIB_PATH

---

### B-5. 依存関係解決とインストール

**完成後:**

```bash
$ lsharp add geometry-utils
  Added geometry-utils = "0.2.0" to lsharp.toml

$ lsharp install
  Resolving dependencies...
    geometry-utils@0.2.0
    math-core@1.0.3 (transitive)
  Downloading ... ok
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

### C-1. パッケージレジストリプロトコル

**Phase 1 (Git-tag ベース):**

レジストリなし。Git リポジトリ + タグが source of truth:

```toml
[dependencies.my-geometry]
git = "https://github.com/user/my-geometry.git"
tag = "v0.1.0"
```

**Phase 2 (HTTP レジストリ):**

```
GET https://registry.lsharp.dev/api/v1/packages/my-geometry
  → { "versions": ["0.1.0", "0.2.0"], "latest": "0.2.0" }

GET https://registry.lsharp.dev/api/v1/packages/my-geometry/0.1.0/api.json
  → (api.json をそのまま返す — MCP Server がリモートフォールバックとして使う)
```

MCP Server はローカルに api.json がなければレジストリから取得する。
AI は MCP 経由でアクセスするので、レジストリの存在を意識しない。

---

### C-2. パッケージ公開と検証

```bash
$ lsharp publish
  Validating lsharp.toml ... ok
  Generating api.json ... ok
  Comparing with v0.1.0 ...
    + added: rotate (Geometry.Vec2)
    ~ changed: distance return type Int → Float  ⚠ BREAKING
  ⚠ Breaking change in minor version. Consider 1.0.0.
  Proceed? [y/N]
```

---

### C-3. API diff & 互換性チェック

```bash
$ lsharp api-diff v0.1.0 v0.2.0
  Added:    + Geometry.Vec2.rotate : Vec2 -> Float -> Vec2
  Changed:  ~ Geometry.distance : Int → Float  ⚠ BREAKING
  Removed:  (none)
  Verdict:  ⚠ BREAKING — semver major bump required
```

---

### C-4. AI パッケージ検索・理解サポート

**MCP 経由:**

```
Tool: lsharp_search { query: "geometry distance" }
→ [
    { name: "my-geometry", version: "0.2.0", summary: "2D 幾何ライブラリ" },
    { name: "geo-3d", version: "0.1.0", summary: "3D 幾何ライブラリ" }
  ]

Tool: lsharp_package_api { name: "my-geometry" }
→ (api.json の完全な中身)
```

**CLI 経由 (人間向け):**

```bash
$ lsharp search "geometry distance"
  my-geometry  0.2.0  — 2D 幾何ライブラリ

$ lsharp info my-geometry
  Package: my-geometry@0.2.0
  Modules: Geometry, Geometry.Vec2
  Functions:
    distance : Point -> Point -> Float  — 2 点間の距離
    rotate   : Vec2 -> Float -> Vec2    — ベクトル回転
```

**AI のパッケージ発見→利用フロー:**

```
1. ユーザー: 「距離計算がしたい」
2. AI → lsharp_search({ query: "distance" })
   → my-geometry を発見
3. AI → lsharp_package_api({ name: "my-geometry" })
   → 全 API を取得
4. AI: lsharp.toml に追加するコードを生成
   → [dependencies] に my-geometry = "0.2.0"
5. AI → lsharp_compile_run でコードを検証
6. 完成
```

---

## 6. 依存関係と実装順序

```
Phase 12-A (AI 連携基盤) ← Phase 11 と独立して着手可能
│
├─ A-1 api.json スキーマ + 生成     ← 最初に着手 (全ての土台)
├─ A-2 lsharp-mcp Server 実装       ← A-1 完了後 (api.json を配信)
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

Phase 12-C (配布エコシステム) ← B-5 完了後に着手推奨
│
├─ C-1 レジストリプロトコル         ← B-5 と並行設計可能
├─ C-2 publish                      ← C-1, A-1 完了後
├─ C-3 API diff                     ← A-1 完了後 (api.json 比較)
└─ C-4 search / info                ← C-1 完了後
```

**MVP (最小実用セット):**

A-1 (api.json) + A-2 (lsharp-mcp) + A-3 (stdlib メタデータ) の 3 つで、
AI が MCP 経由で L# の言語仕様と stdlib API を取得し、正しいコードを書ける状態になる。
パッケージシステムがなくても、この 3 つで AI 連携の最低ラインを達成できる。

---

## 7. 既存基盤との関係

### 拡張するもの

| 既存 | 拡張内容 |
|------|---------|
| `config.rs` (lsharp.toml) | `[project.exports]`, `[dev-dependencies]` 追加 |
| `module_graph.rs` | 検索パスに `src/`, `.lsharp/packages/`, `stdlib/` 追加 |
| `knowledge.schema.json` | 型シグネチャ・パラメータ docs フィールド追加 |
| `ast.rs` (Metadata) | `:doc` / `:params` / `:returns` をパイプライン全体で伝搬 |
| `stdlib/*.ls` | 全関数にメタデータ付与 |
| CLI (main.rs) | `mcp-server`, `doc --json`, `doc-site`, `init`, `install`, `add`, `publish`, `search`, `info`, `api-diff` 追加 |

### 新規作成するもの

| ファイル | 役割 |
|---------|------|
| `crates/lsharp-driver/src/mcp_server.rs` | MCP Server (コア — 全 AI ツールのエントリポイント) |
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

### 変更しないもの

- Phase 11 の bootstrap / native / GC 関連コード
- 既存の E2E テスト群
- selfhost/ コンパイラモジュール群 (Phase 12 では Rust 側で実装し、将来 selfhost へ移行)

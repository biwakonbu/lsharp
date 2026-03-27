# Phase 12: モジュール・パッケージ & AI フレンドリーエコシステム ロードマップ

> このドキュメントは「完成後にどう使えるか」を起点に、各機能の実装方針を解説する。
> 仕様の詳細ではなく、ユーザー・AI エージェント・ライブラリ作者の視点から全体像を掴むための文書。

---

## 目次

1. [全体像: 完成後の世界](#1-全体像-完成後の世界)
2. [P12-A: AI フレンドリードキュメント基盤](#2-p12-a-ai-フレンドリードキュメント基盤)
3. [P12-B: パッケージシステムコア](#3-p12-b-パッケージシステムコア)
4. [P12-C: パッケージ配布 & エコシステム](#4-p12-c-パッケージ配布--エコシステム)
5. [依存関係と実装順序](#5-依存関係と実装順序)
6. [既存基盤との関係](#6-既存基盤との関係)

---

## 1. 全体像: 完成後の世界

### ライブラリ作者の体験

```bash
# 新しいパッケージを作成
$ lsharp init my-geometry
  Created my-geometry/
    lsharp.toml
    src/Main.ls
    .gitignore

# コードを書く
$ cat src/Geometry.ls
```

```lisp
(module Geometry)

(import Core)

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
# API ドキュメントを生成
$ lsharp doc --json
  Generated docs/api.json

$ lsharp doc --llms
  Generated llms.txt

# パッケージ公開
$ lsharp publish
  Validating lsharp.toml ... ok
  Generating api.json ... ok
  Generating llms.txt ... ok
  Checking semver compatibility ... ok
  Published my-geometry@0.1.0
```

### ライブラリ利用者の体験

```bash
# 依存関係を追加
$ lsharp add my-geometry
  Added my-geometry = "0.1.0" to lsharp.toml

$ lsharp install
  Resolving dependencies...
  Downloading my-geometry@0.1.0 ... ok
  Lock file updated: .lsharp/lock.toml
```

```lisp
;; app.ls
(module App)
(import Geometry)

(defn main []
  (let [a {Point x 0 y 0}
        b {Point x 3 y 4}]
    (print (distance a b))))
```

```bash
$ lsharp compile app.ls -o app.wasm && wasmtime app.wasm
25
```

### AI エージェントの体験

AI は `llms.txt` と `api.json` を読むだけで正しいコードを書ける:

```
# AI への指示: 「my-geometry パッケージを使って 2 点間の距離を計算して」

# AI は以下を参照する:
# 1. llms.txt  → L# の構文・型システム・import パターンを理解
# 2. api.json  → distance 関数のシグネチャ・パラメータ・使用例を取得
# 3. コードを生成 (上の app.ls と同等)
```

---

## 2. P12-A: AI フレンドリードキュメント基盤

### A-1. llms.txt テンプレートと生成コマンド

**完成後の使い方:**

```bash
$ lsharp doc --llms
  Generated llms.txt
```

生成される `llms.txt` の中身:

```markdown
# L# (lsharp)

> S 式構文 + Hindley-Milner 型推論の関数型言語。WebAssembly (WASI) をターゲットとする。

## 構文早見表

### 基本

  (defn name [params] body)           ;; 関数定義
  (fn [params] body)                  ;; 無名関数 (ラムダ)
  (let [x 1 y 2] (+ x y))            ;; ローカル束縛
  (if cond then else)                 ;; 条件分岐
  (do expr1 expr2 ... result)         ;; 逐次実行 (最後の式が戻り値)
  (match expr [pattern body] ...)     ;; パターンマッチ

### 型定義

  (type (Option a) (Some a) None)                     ;; ADT (代数的データ型)
  (type Point (record (: x Int) (: y Int)))            ;; レコード型
  (type-alias Name String)                             ;; 型エイリアス
  {Point x 1 y 2}                                      ;; レコードリテラル
  (Point.x p)                                          ;; フィールドアクセス

### モジュール

  (module Name)                                        ;; モジュール宣言
  (import Module)                                      ;; インポート
  (import Module :as Alias)                            ;; エイリアス付きインポート
  (import Module :only [sym1 sym2])                    ;; 選択的インポート
  (import Module :open)                                ;; 全シンボルを展開

### トレイトとメタデータ

  (trait (Show a) (defn show [a] : String))            ;; トレイト定義
  (impl (Show Int) (defn show [x] (int-to-string x))) ;; トレイト実装
  :doc "説明"                                           ;; ドキュメントメタデータ
  :params [(x "説明")]                                  ;; パラメータ説明
  :returns "説明"                                       ;; 戻り値説明

## 型システム

- Hindley-Milner 型推論 (型注釈は任意)
- プリミティブ型: Int, Float, String, Bool, Unit
- 関数型: (Int -> Int -> Int) — 全てカリー化
- パラメトリック多相: (Option a), (Result a e)
- レコード型: {Point x Int y Int}

## 標準ライブラリ

Core, List, Map, Set, Vector, String, Char, IO, Json, Path, Debug

## パッケージ: my-geometry

### 概要
2D 幾何プリミティブと距離計算

### 使い方
  (import Geometry)
  (distance {Point x 0 y 0} {Point x 3 y 4})

### 公開 API
- distance : Point -> Point -> Int  — 2 点間のユークリッド距離
- Point : record { x: Int, y: Int } — 2D 座標点
```

**実装方針:**

- コンパイラに静的な L# 言語テンプレートを同梱する (構文早見表・型システム・stdlib 一覧)
- `lsharp.toml` の `[project.ai]` セクションからパッケージ固有情報を読む
- `api.json` (A-2) から公開 API 一覧を読んでテンプレート末尾に追記する
- `crates/lsharp-driver/src/llms.rs` を新規作成し、テンプレート結合ロジックを実装

**修正対象:** `crates/lsharp-driver/src/main.rs`, 新規 `crates/lsharp-driver/src/llms.rs`

---

### A-2. 機械可読 API リファレンス (api.json)

**完成後の使い方:**

```bash
$ lsharp doc --json
  Generated docs/api.json

$ lsharp doc --json src/Geometry.ls   # 単一ファイル指定も可
```

生成される `api.json` の中身:

```json
{
  "package": "my-geometry",
  "version": "0.1.0",
  "ai_summary": "2D 幾何プリミティブと距離計算",
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
          "example": "(distance {Point x 0 y 0} {Point x 3 y 4})",
          "since": "0.1.0"
        }
      ],
      "types": [
        {
          "name": "Point",
          "kind": "record",
          "fields": [
            { "name": "x", "type": "Int" },
            { "name": "y", "type": "Int" }
          ],
          "doc": null
        }
      ]
    }
  ]
}
```

**実装方針:**

- `lsharp-syntax` が parse した AST の `Metadata` (`:doc`, `:params`, `:returns`, `:example`) を活用
- `lsharp-types` の型推論結果から関数シグネチャ (`Point -> Point -> Int`) を文字列化
- 両者を結合して JSON を生成するモジュール `crates/lsharp-driver/src/api_doc.rs` を新規作成
- `docs/schemas/knowledge.schema.json` を拡張して `api.json` のスキーマを正式定義

**情報の流れ:**

```
Source (.ls)
  → Parser  → AST + Metadata (:doc, :params, :returns, :example)
  → TypeInfer → 型シグネチャ (Int -> Int -> Int)
  → api_doc.rs → api.json
```

**修正対象:** `docs/schemas/knowledge.schema.json`, `crates/lsharp-driver/src/main.rs`, 新規 `crates/lsharp-driver/src/api_doc.rs`

---

### A-3. stdlib 全モジュールのメタデータ整備

**完成後の使い方:**

```bash
# stdlib の API リファレンスを生成
$ lsharp doc --json stdlib/
  Generated docs/stdlib-api.json

# AI が stdlib を問い合わせる
$ lsharp info stdlib:List
  Module: List
  Types:
    List a = Cons a (List a) | Nil
  Functions:
    length : List a -> Int          — リストの長さを返す
    head   : List a -> a -> a       — 先頭要素を返す (デフォルト値付き)
    tail   : List a -> List a       — 先頭を除いた残り
    map    : (a -> b) -> List a -> List b  — 各要素に関数を適用
    filter : (a -> Bool) -> List a -> List a — 条件を満たす要素を抽出
    fold   : (b -> a -> b) -> b -> List a -> b — 畳み込み
    ...
```

**現状と変更点:**

現在の stdlib は日本語コメントのみでメタデータなし:

```lisp
;; 現状 (Core.ls)
;; 絶対値
(defn abs [x] (if (< x 0) (- 0 x) x))
```

これを以下のように変更:

```lisp
;; 完成後 (Core.ls)
(defn abs
  :doc "整数の絶対値を返す"
  :params [(x "対象の整数")]
  :returns "x の絶対値 (非負整数)"
  :example (abs (- 0 5))
  [x]
  (if (< x 0) (- 0 x) x))
```

**対象:** 11 モジュール × 平均 10 関数 = 約 110 関数にメタデータ付与

**修正対象:** `stdlib/*.ls` 全 11 ファイル

---

### A-4. 言語リファレンス (ユーザー向け)

**完成後のドキュメント構成:**

```
docs/guides/
  quick-start.md       # 5 分で始める L#
  language-reference.md # 構文・型・モジュール完全リファレンス
  ai-guide.md          # AI エージェント向け利用ガイド
```

**quick-start.md の内容イメージ:**

```markdown
# 5 分で始める L#

## Hello World
  (defn main [] (print 42))

## 関数定義
  (defn fib [n]
    (if (<= n 1)
      n
      (+ (fib (- n 1)) (fib (- n 2)))))

## 型を使う
  (type (Option a) (Some a) None)

  (defn unwrap [opt default]
    (match opt
      [(Some x) x]
      [None default]))

## レコード
  (type Point (record (: x Int) (: y Int)))
  (let [p {Point x 10 y 20}]
    (print (Point.x p)))

## モジュール
  ;; Utils.ls
  (module Utils)
  (defn helper [x] (+ x 100))

  ;; main.ls
  (import Utils)
  (defn main [] (print (helper 42)))

## ビルドと実行
  $ lsharp compile main.ls -o app.wasm
  $ wasmtime app.wasm
```

**ai-guide.md の内容イメージ:**

```markdown
# AI エージェント向け L# ガイド

## L# コードを書くときの手順

1. `llms.txt` を読んで構文を確認する
2. 使いたいパッケージの `api.json` を読む (または `lsharp info <pkg>`)
3. `(import Module)` で必要なモジュールをインポート
4. 型推論が全自動なので型注釈は省略可能

## よくあるエラーと対処

| エラー | 原因 | 対処 |
|--------|------|------|
| undefined symbol `foo` | import が足りない | `(import Module)` を追加 |
| type mismatch: Int vs String | 型不一致 | 引数の型を確認 |
| cyclic dependency | 循環 import | モジュール構成を見直す |

## イディオム集

;; Option の安全なアンラップ
(match opt
  [(Some x) (process x)]
  [None default-value])

;; リスト処理のパイプライン
(let [result (filter is-positive (map double xs))]
  (fold + 0 result))

;; レコードの更新
(let [updated {(original) | x 10}]
  updated)
```

**実装方針:** book/ の既存章 (ch01-ch16) から抽出・要約して人間+AI が読める簡潔な形にまとめる

---

### A-5. ドキュメントサイト生成

**完成後の使い方:**

```bash
$ lsharp doc-site --output _site/
  Generating site...
    Language reference ... ok
    Quick start guide ... ok
    AI guide ... ok
    Stdlib API reference (11 modules) ... ok
    Book chapters (16) ... ok
    llms.txt ... ok
  Site generated: _site/
  Open _site/index.html to preview
```

**生成されるサイト構造:**

```
_site/
  index.html              # トップページ
  llms.txt                # AI 向けメタデータ (サイトルート)
  guides/
    quick-start.html      # チュートリアル
    language-reference.html
    ai-guide.html
  api/
    index.html            # stdlib 一覧
    Core.html             # Core モジュール API
    List.html             # List モジュール API
    ...
    stdlib.json           # 機械可読 stdlib API
  book/
    ch01-introduction.html
    ...
```

**実装方針:**

- 既存の `HtmlTemplate.ls` / `HtmlLayout.ls` パイプラインを拡張
- Markdown → HTML 変換 (guides/, book/)
- api.json → HTML 変換 (API リファレンスページ)
- 静的サイト生成なので外部依存は最小限

---

## 3. P12-B: パッケージシステムコア

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

# AI が読むパッケージ説明
[project.ai]
summary = "2D 幾何プリミティブと距離計算"
capabilities = ["点・ベクトル演算", "アフィン変換", "距離計算"]
conventions = "座標は Int。角度はラジアン。"
examples-entry = "examples/basic.ls"

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
[project] に追加: description, license, authors, repository,
                   keywords, lsharp-version
新規セクション:    [project.exports], [project.ai], [dev-dependencies]
```

**実装方針:**

- `crates/lsharp-driver/src/config.rs` の `ProjectConfig` に新フィールドを追加
- 全て `#[serde(default)]` で後方互換を維持 (既存の lsharp.toml はそのまま動く)
- `ExportsConfig`, `AiConfig` を新規 struct として定義
- `validate_config()` に新フィールドのバリデーションを追加

---

### B-2. パッケージディレクトリ規約と lsharp init

**完成後の使い方:**

```bash
$ lsharp init my-lib
  Created my-lib/lsharp.toml
  Created my-lib/src/Main.ls
  Created my-lib/.gitignore

$ tree my-lib/
my-lib/
  lsharp.toml
  src/
    Main.ls
  .gitignore
```

**標準レイアウト規約:**

```
my-package/
  lsharp.toml              # パッケージ定義 (必須)
  llms.txt                 # AI メタデータ (lsharp doc --llms で生成)
  src/                     # ソースコード
    Main.ls                #   エントリポイント (実行可能パッケージの場合)
    MyModule.ls            #   → (module MyModule) として参照可能
    MyModule/              #   → ネストモジュール
      Sub.ls               #     → (import MyModule.Sub) として参照可能
  examples/                # 使用例
  tests/                   # テスト
  docs/
    api.json               # 機械可読 API (lsharp doc --json で生成)
```

**ファイル解決のルール変更:**

```
現状:    (import Utils) → Utils.ls (エントリファイルと同じディレクトリ)
完成後:  (import Utils) → src/Utils.ls (lsharp.toml 存在時は src/ prefix)
                        → .lsharp/packages/utils/src/Utils.ls (依存パッケージ)
                        → <stdlib-path>/Utils.ls (標準ライブラリ)
```

**実装方針:**

- `crates/lsharp-driver/src/init.rs` を新規作成 (`lsharp init` テンプレート生成)
- `ModuleGraph::resolve_module_file()` を拡張して `src/` prefix 探索を追加

---

### B-3. 可視性制御の実装

**完成後の動作:**

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

(defn main []
  (do
    (print (distance a b))           ;; OK: :only に含まれている
    (print (internal-helper 42))))   ;; ERROR: private 関数
;;  ^^^^^^^^^^^^^^^^^^^^^ error: `internal-helper` is not exported from Geometry
```

```
;; lsharp.toml
[project.exports]
modules = ["Geometry"]
```

```lisp
;; 外部パッケージから:
(import Geometry)           ;; OK: exports に含まれている
(import Geometry.Internal)  ;; ERROR: exports に含まれていない
;; ^^^^^^^^^^^^^^^^^^^ error: module `Geometry.Internal` is not exported
```

**3 段階の制御:**

| レベル | 制御対象 | 仕組み |
|--------|----------|--------|
| シンボル | `(import M :only [f])` | f 以外の M のシンボルを参照するとエラー |
| 宣言 | `(private (defn ...))` | 他モジュールから private シンボルを参照するとエラー |
| モジュール | `[project.exports]` | exports に無いモジュールを外部パッケージから import するとエラー |

**実装方針:**

- `crates/lsharp-types/src/infer.rs` の `TypeEnv` に可視性情報を追加
- `inject_external_types()` で `:only` フィルタと `private` フィルタを適用
- `ModuleGraph` でパッケージ境界を認識し、`exports` に基づくモジュールフィルタを適用

---

### B-4. stdlib 自動リンク

**完成後の動作:**

```lisp
;; 設定なしで stdlib が使える
(import List)
(import Core)

(defn main []
  (let [xs (Cons 1 (Cons 2 (Cons 3 Nil)))]
    (print (length xs))))
```

```bash
# 何も設定しなくても stdlib が解決される
$ lsharp compile app.ls -o app.wasm
```

**モジュール解決の優先順位:**

```
1. ローカル src/ (同パッケージ内のモジュール)
2. .lsharp/packages/ (インストール済み依存パッケージ)
3. <compiler-install-dir>/stdlib/ (標準ライブラリ)
4. $LSHARP_STDLIB_PATH (環境変数による上書き)
```

**実装方針:**

- `ModuleGraph::resolve_module_file()` に stdlib パスの探索を追加
- コンパイラバイナリの隣に stdlib/ を配置する規約を定義
- `LSHARP_STDLIB_PATH` 環境変数で上書き可能にする

---

### B-5. 依存関係解決とインストール

**完成後の使い方:**

```bash
# 依存パッケージを追加
$ lsharp add geometry-utils
  Added geometry-utils = "0.2.0" to lsharp.toml

# 全依存パッケージをインストール
$ lsharp install
  Resolving dependencies...
    geometry-utils@0.2.0 (git: https://github.com/user/geometry-utils.git, tag: v0.2.0)
    math-core@1.0.3 (transitive, from geometry-utils)
  Downloading geometry-utils ... ok
  Downloading math-core ... ok
  Lock file written: .lsharp/lock.toml

$ tree .lsharp/
.lsharp/
  lock.toml
  packages/
    geometry-utils-a1b2c3/
      lsharp.toml
      src/
        GeometryUtils.ls
    math-core-d4e5f6/
      lsharp.toml
      src/
        MathCore.ls
```

**lock.toml の中身:**

```toml
[metadata]
lsharp-version = "0.1.0"
generated = "2026-03-27T10:00:00Z"

[[package]]
name = "geometry-utils"
version = "0.2.0"
source = "git+https://github.com/user/geometry-utils.git#tag=v0.2.0"
checksum = "sha256:abc123..."

[[package]]
name = "math-core"
version = "1.0.3"
source = "git+https://github.com/user/math-core.git#tag=v1.0.3"
checksum = "sha256:def456..."
```

**バージョン解決ルール:**

| 記法 | 意味 | 例 |
|------|------|-----|
| `"1.0.0"` | >=1.0.0, <2.0.0 (Cargo 互換) | 1.0.0, 1.5.3 は OK / 2.0.0 は NG |
| `"=1.0.0"` | 完全一致 | 1.0.0 のみ |
| `">=1.2.0"` | 1.2.0 以上 | 1.2.0, 3.0.0 も OK |

**実装方針:**

- `crates/lsharp-driver/src/resolver.rs` を新規作成
- Git clone → tag checkout → `.lsharp/packages/<name>-<hash>/` に配置
- path 依存はシンボリックリンクまたはパスをそのまま使用
- `ModuleGraph` の探索パスに `.lsharp/packages/*/src/` を追加

---

## 4. P12-C: パッケージ配布 & エコシステム

### C-1. パッケージレジストリプロトコル

**Phase 1 (Git-tag ベース):**

レジストリサーバーなし。Git リポジトリ + セマンティックバージョンタグが source of truth:

```bash
# ライブラリ作者
$ git tag v0.1.0
$ git push origin v0.1.0

# 利用者の lsharp.toml
[dependencies.my-geometry]
git = "https://github.com/user/my-geometry.git"
tag = "v0.1.0"
```

**Phase 2 (static HTTP registry):**

Go module proxy / Deno land に近い方式。静的ファイルでメタデータを配布:

```
https://registry.lsharp.dev/
  my-geometry/
    meta.json          # { "versions": ["0.1.0", "0.2.0"], "latest": "0.2.0" }
    0.1.0/
      meta.json        # { "git": "...", "tag": "v0.1.0", "checksum": "..." }
      api.json         # 機械可読 API
      llms.txt         # AI メタデータ
```

```bash
# Phase 2 では名前だけで依存を追加できる
$ lsharp add my-geometry
  Resolved my-geometry@0.2.0 from registry.lsharp.dev
```

---

### C-2. パッケージ公開と検証

**完成後の使い方:**

```bash
$ lsharp publish
  Checking lsharp.toml ... ok
  Checking [project.exports] ... ok (2 modules exported)
  Generating api.json ... ok
  Generating llms.txt ... ok
  Comparing with previous version (0.1.0) ...
    + added function: rotate (Geometry.Vec2)
    ~ changed signature: distance (Point -> Point -> Float, was Int)  ⚠ BREAKING
  ⚠ Breaking change detected in minor version bump.
    Consider releasing as 1.0.0 instead.
  Proceed? [y/N]
```

---

### C-3. パッケージ API diff & 互換性チェック

**完成後の使い方:**

```bash
$ lsharp api-diff v0.1.0 v0.2.0
  Comparing my-geometry@0.1.0 → 0.2.0

  Added:
    + Geometry.Vec2.rotate : Vec2 -> Float -> Vec2

  Changed:
    ~ Geometry.distance : Point -> Point -> Int  →  Point -> Point -> Float
      ⚠ Return type changed (BREAKING)

  Removed:
    - (none)

  Verdict: ⚠ BREAKING — semver requires major version bump
```

---

### C-4. AI パッケージ検索・理解サポート

**完成後の使い方:**

```bash
# パッケージ検索
$ lsharp search "geometry distance"
  my-geometry  0.2.0  — 2D 幾何プリミティブと距離計算
  geo-3d       0.1.0  — 3D 幾何ライブラリ

# パッケージ情報 (AI が読む)
$ lsharp info my-geometry
  Package: my-geometry@0.2.0
  Summary: 2D 幾何プリミティブと距離計算
  Modules: Geometry, Geometry.Vec2

  ## Geometry
  Types:
    Point = record { x: Int, y: Int }
  Functions:
    distance : Point -> Point -> Float
      2 点間のユークリッド距離を計算する
    make-point : Int -> Int -> Point
      座標から Point を生成する

  ## Geometry.Vec2
  Types:
    Vec2 = record { dx: Float, dy: Float }
  Functions:
    rotate : Vec2 -> Float -> Vec2
      ベクトルを回転する (ラジアン)

  ## Usage
    (import Geometry)
    (distance {Point x 0 y 0} {Point x 3 y 4})
```

**AI エージェントのワークフロー:**

```
1. ユーザーから「距離計算がしたい」と指示を受ける
2. `lsharp search "distance"` → my-geometry を発見
3. `lsharp info my-geometry` → API とシグネチャを把握
4. `lsharp add my-geometry` → 依存追加
5. llms.txt で L# の構文を確認しながらコードを生成
6. `lsharp compile` → 動作確認
```

---

## 5. 依存関係と実装順序

```
Phase 12-A (AI ドキュメント基盤) ← Phase 11 と独立して着手可能
│
├─ A-1 llms.txt テンプレート        ← 最初に着手 (他に依存なし)
├─ A-2 api.json 生成               ← A-1 と並行可能
├─ A-3 stdlib メタデータ            ← A-2 完了後 (api.json で検証)
├─ A-4 ガイド文書                   ← A-1 と並行可能
└─ A-5 ドキュメントサイト           ← A-1〜A-4 完了後

Phase 12-B (パッケージコア) ← A-1, A-2 完了後に着手推奨
│
├─ B-1 lsharp.toml 拡張            ← 最初に着手 (config.rs のみ)
├─ B-2 ディレクトリ規約 + init      ← B-1 完了後
├─ B-3 可視性制御                   ← B-1 完了後 (exports 仕様に依存)
├─ B-4 stdlib 自動リンク            ← B-2 完了後 (解決順序の定義に依存)
└─ B-5 依存解決 + install           ← B-1, B-2, B-4 完了後

Phase 12-C (配布エコシステム) ← B-5 完了後に着手推奨
│
├─ C-1 レジストリプロトコル         ← B-5 と並行設計可能
├─ C-2 publish                      ← C-1, A-2 完了後
├─ C-3 API diff                     ← A-2 完了後 (api.json 比較)
└─ C-4 search / info                ← C-1 完了後
```

**最小実用セット (MVP):**

A-1 (llms.txt) + A-2 (api.json) + A-3 (stdlib メタデータ) だけで、
AI が L# の構文を理解し、stdlib を使ったコードを正しく書ける状態になる。
パッケージシステムがなくても、この 3 つで AI フレンドリーの最低ラインを達成できる。

---

## 6. 既存基盤との関係

### 拡張するもの

| 既存 | 拡張内容 |
|------|---------|
| `config.rs` (lsharp.toml) | `[project.exports]`, `[project.ai]`, `[dev-dependencies]` 追加 |
| `module_graph.rs` (ModuleGraph) | 検索パスに `src/`, `.lsharp/packages/`, `stdlib/` を追加 |
| `knowledge.schema.json` | 型シグネチャ・パラメータ docs・AI hints フィールド追加 |
| `ast.rs` (Metadata) | 既存の `:doc` / `:params` / `:returns` をパイプライン全体で伝搬 |
| stdlib/*.ls | 全関数に `:doc` / `:params` / `:returns` メタデータ付与 |
| CLI (main.rs) | `doc --llms`, `doc --json`, `doc-site`, `init`, `install`, `add`, `publish`, `search`, `info`, `api-diff` サブコマンド追加 |

### 新規作成するもの

| ファイル | 役割 |
|---------|------|
| `crates/lsharp-driver/src/llms.rs` | llms.txt テンプレート結合・生成 |
| `crates/lsharp-driver/src/api_doc.rs` | AST + 型情報 → api.json 生成 |
| `crates/lsharp-driver/src/init.rs` | `lsharp init` スキャフォールド |
| `crates/lsharp-driver/src/resolver.rs` | 依存解決・ダウンロード・lock.toml |
| `docs/guides/quick-start.md` | 5 分チュートリアル |
| `docs/guides/language-reference.md` | 言語リファレンス |
| `docs/guides/ai-guide.md` | AI 向けガイド |
| `llms.txt` | リポジトリルートの AI メタデータ |

### 変更しないもの

- Phase 11 の bootstrap / native / GC 関連コード
- 既存の E2E テスト群
- selfhost/ コンパイラモジュール群 (Phase 12 では Rust 側で実装し、将来 selfhost へ移行)

# モジュールシステム -- コードの組織化

## なぜモジュールが必要か

プログラムが大きくなると、すべてのコードを 1 ファイルに書くことは現実的でなくなる。モジュールシステムは以下の問題を解決する:

1. **名前空間の分離**: 異なるモジュールで同じ名前を使える
2. **カプセル化**: 内部実装を隠蔽し、公開 API だけを提供する
3. **依存関係の明示**: どのモジュールが何に依存しているかが明確になる

## 設計判断

### モジュール = ファイル

L# では 1 ファイルが 1 モジュールに対応する (F#/OCaml 寄りの設計)。ファイルパスからモジュール名が決まる:

```
src/geometry.ls  → Geometry モジュール
src/math/vec2.ls → Math.Vec2 モジュール
```

### 名前空間区切り

ドット `.` を使用する。これは多くの言語で馴染みがある:

```lisp
Math.Vec2.add
Geometry.Point
```

### 可視性

デフォルト公開、`(private ...)` で非公開にする方式を採用:

```lisp
(module Geometry)

;; 公開 (デフォルト)
(defn distance [p1 p2] ...)

;; 非公開
(private
  (defn helper [x] ...))
```

## 構文

### モジュール宣言

```lisp
;; ファイル先頭で宣言
(module Math.Vec2)
```

### インポート

4つのインポート形式を用意する:

```lisp
;; 完全修飾アクセス
(import Math.Vec2)
;; Math.Vec2.add で参照

;; モジュールエイリアス
(import Math.Vec2 :as V)
;; V.add で参照

;; 選択的インポート
(import Math.Vec2 :only [add sub])
;; add, sub が直接参照可能

;; 全公開 (F# の open に相当)
(import Math.Vec2 :open)
;; 全てのエクスポートが直接参照可能
```

### 使用例

```lisp
;; ファイル: src/geometry.ls
(module Geometry)

(type Point
  (record
    (: x Float)
    (: y Float)))

(defn distance [(: p1 Point) (: p2 Point)] : Float
  (let [dx (- (Point.x p1) (Point.x p2))
        dy (- (Point.y p1) (Point.y p2))]
    (sqrt (+ (* dx dx) (* dy dy)))))

;; ファイル: src/main.ls
(module Main)

(import Geometry :open)

(defn main [] : Unit
  (let [p1 {Point x 0.0 y 0.0}
        p2 {Point x 3.0 y 4.0}]
    (print (distance p1 p2))))
```

## 実装の詳細

### モジュールグラフ

複数のファイルをコンパイルするには、モジュール間の依存関係を解析し、正しい順序で処理する必要がある。`crates/lsharp-ir/src/module_graph.rs` にモジュールグラフの実装がある:

1. **依存グラフ構築**: 各ファイルの `import` 宣言からモジュール間の依存を抽出
2. **循環依存検出**: グラフにサイクルがあればコンパイルエラー
3. **トポロジカルソート**: 依存先が先にコンパイルされるよう、コンパイル順序を決定

### モジュール環境 (ModuleEnv)

型推論器に `ModuleEnv` が追加されている (`crates/lsharp-types/src/infer.rs`)。各モジュールの型環境を分離し、インポート情報に基づいて名前解決を行う:

- **完全修飾アクセス**: `Math.Vec2.add` → `Math.Vec2` モジュールから `add` を検索
- **エイリアス**: `:as V` → `V.add` で参照可能
- **選択的インポート**: `:only [add sub]` → `add`, `sub` のみ直接参照
- **全公開**: `:open` → 全エクスポートを直接参照

可視性制御は `(private ...)` で包まれた宣言を `privates` リストに記録し、他モジュールからのアクセスを禁止する。

### IR リンクと Wasm 出力

複数のモジュールは IR レベルで結合される (`link_modules`)。各モジュールの関数インデックスと GC 型インデックスをリベース (再配置) し、import 関数の重複を除去した上で、最終的に**単一の Wasm モジュール**にフラット化される。

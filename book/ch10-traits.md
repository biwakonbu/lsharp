# トレイト -- アドホック多相とインタフェース

> **実装状態**: トレイト定義・impl の構文解析と型チェック、デフォルト実装のフォールバックは実装済み。辞書パスイングによる IR 変換と WasmGC vtable コード生成は未実装。

## 多相性の2つの形

第 4 章で見た HM 型推論は**パラメトリック多相** (parametric polymorphism) を提供する。`(defn id [x] x)` は**任意の**型に対して同じように動作する。

しかし、型ごとに**異なる**動作をしたい場合がある。たとえば「値を文字列に変換する」操作は、`Int` と `Point` で処理が異なる。これを**アドホック多相** (ad hoc polymorphism) と呼ぶ。

## トレイトとは

トレイトは「ある型が満たすべきインタフェース」を定義する。Rust のトレイト、Haskell の型クラス、Swift のプロトコルに相当する:

```lisp
;; Show トレイト: 値を文字列表示できる型の集合
(trait (Show a)
  (defn show [(: self a)] : String))

;; Eq トレイト: 等値比較できる型の集合
(trait (Eq a)
  (defn eq [(: self a) (: other a)] : Bool)
  ;; デフォルト実装
  (defn ne [(: self a) (: other a)] : Bool
    (not (eq self other))))
```

## トレイト実装

特定の型に対してトレイトを実装する:

```lisp
(impl (Show Point)
  (defn show [(: self Point)] : String
    (str "Point(" (Point.x self) ", " (Point.y self) ")")))

(impl (Eq Point)
  (defn eq [(: self Point) (: other Point)] : Bool
    (and (== (Point.x self) (Point.x other))
         (== (Point.y self) (Point.y other)))))
```

## トレイト制約

関数が「Show を実装した任意の型」を受け取れるように制約を指定する:

```lisp
;; 単一制約
(defn to-string [(: x a)] : String
  :where [(Show a)]
  (show x))

;; 複数制約
(defn compare-and-show [(: x a) (: y a)] : String
  :where [(Eq a) (Show a)]
  (if (eq x y) (show x) "not equal"))
```

`:where` をメタデータキーワードとして使用する。パラメータリストと本体の間に自然に配置できる。

## WasmGC 表現: 辞書パスイング

トレイトの呼び出しは**辞書パスイング (dictionary passing)** で実現する。各トレイト実装は「辞書」として構造体に格納され、関数の追加引数として渡される:

```wasm
;; Show トレイトの辞書型
(type $Show_dict (struct
  (field $show (ref $show_func_type))))
(type $show_func_type (func (param (ref eq)) (result (ref $String))))

;; Point 用 Show 辞書インスタンス
(global $show_Point_dict (ref $Show_dict)
  (struct.new $Show_dict (ref.func $show_Point)))

;; to-string(dict, x)
;; where dict : (ref $Show_dict)
```

静的ディスパッチが可能な場合は**単相化 (monomorphization)** で最適化する。呼び出し時に具体型が確定していれば、辞書の間接呼び出しを直接呼び出しに変換できる。

## デフォルト実装

`Eq` トレイトの `ne` メソッドのように、デフォルト実装を持つメソッドは `impl` ブロックで省略できる。型推論器は `default_impls` キャッシュを保持し、impl に明示的な実装がなければデフォルト実装にフォールバックする。

```lisp
(impl (Eq Point)
  ;; eq のみ実装すれば、ne はデフォルト実装が使われる
  (defn eq [(: self Point) (: other Point)] : Bool
    (and (== (Point.x self) (Point.x other))
         (== (Point.y self) (Point.y other)))))
```

## Orphan Rule

トレイトの実装は**orphan rule** に従う。ある型に対するトレイトの実装は、型またはトレイトが定義されたモジュールでのみ許可される。これにより、異なるモジュールで同じ型に対する矛盾した実装が作られることを防ぐ。

## Associated Types (将来拡張)

将来的には associated types (関連型) もサポートする予定:

```lisp
(trait (Collection c)
  (type-assoc Item)
  (defn get [(: self c) (: idx Int)] : (Option Item)))
```

associated types により、トレイトが「出力の型」も指定できるようになる。

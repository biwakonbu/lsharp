# レコード型 -- 構造化データと WasmGC

> **実装状態**: レコード型の構文解析、型推論、IR 降位は実装済み。WasmGC コード生成は MVP として i64 フォールバックで動作する。本格的な WasmGC struct 出力は今後の課題。

## レコード型の必要性

これまでの L# はプリミティブ型 (`Int`, `Float`, `String`, `Bool`) と ADT しか扱えなかった。実用的なプログラムには、名前付きフィールドを持つ構造化データが不可欠である。

```lisp
;; 目標: レコード型の定義と使用
(type Point
  (record
    (: x Float)
    (: y Float)))

(defn distance [(: p1 Point) (: p2 Point)] : Float
  (let [dx (- (Point.x p1) (Point.x p2))
        dy (- (Point.y p1) (Point.y p2))]
    (sqrt (+ (* dx dx) (* dy dy)))))
```

## 設計判断: 公称型 vs 構造型

レコード型の設計では「公称型 (nominal typing)」と「構造型 (structural typing)」の選択がある。

**構造型**: 同じフィールド構成なら同じ型として扱う (TypeScript の方式)

**公称型**: 型名が異なれば別の型として扱う (Rust, Java の方式)

L# は**公称型**を採用する:

1. WasmGC の `struct` 型が公称であり、直接マッピング可能
2. ADT と一貫した名前空間管理ができる
3. エラーメッセージが「Point 型が必要です」と明確になる
4. 将来のトレイトシステムとの親和性が高い

## 構文設計

### レコード型定義

```lisp
;; 基本的なレコード型
(type Point
  (record
    (: x Float)
    (: y Float)))

;; 型パラメータ付きレコード
(type (Pair a b)
  (record
    (: first a)
    (: second b)))
```

`(record ...)` を `type` 宣言内のキーワードにすることで、既存の ADT バリアント列との衝突を避ける。

### レコード構築

```lisp
;; 中括弧リテラルで構築
{Point x 1.0 y 2.0}

;; ネストしたレコード
{Line start {Point x 0.0 y 0.0} end {Point x 1.0 y 1.0}}
```

中括弧 `{}` はレクサーに `LBrace`/`RBrace` トークンとして既に定義済みである。

### フィールドアクセス

```lisp
;; 型修飾付きフィールドアクセス
(Point.x point)          ;; => Float
(Pair.first pair)        ;; => a

;; ネストしたアクセス
(Point.x (Line.start line))  ;; => Float
```

`TypeName.field` は型修飾付きアクセサ関数として型環境に登録される。たとえば `Point.x` は `(-> Point Float)` 型の関数になる。型が常に明示されるため、同名フィールドを持つ複数のレコード型が存在しても曖昧さが生じない。

### レコード更新 (Functional Update)

```lisp
;; 一部のフィールドだけ変更した新しいレコードを生成
{point | x 3.0}

;; 複数フィールドの更新
{pair | first 10 second 20}
```

関数型言語ではデータは不変が基本である。既存の値を変更するのではなく、一部を変えた新しい値を作る。

### パターンマッチ

```lisp
(match p
  [{Point x y} (+ x y)])
```

## WasmGC 表現

レコード型は WasmGC の `struct` 型に直接マッピングされる:

```wasm
;; (type Point (record (: x Float) (: y Float)))
(type $Point (struct (field $x f64) (field $y f64)))

;; {Point x 1.0 y 2.0}
f64.const 1.0
f64.const 2.0
struct.new $Point

;; (Point.x point)
local.get $point
struct.get $Point $x

;; {point | x 3.0}
local.get $point
struct.get $Point $y    ;; y を取得
f64.const 3.0           ;; 新しい x
struct.new $Point       ;; 新しい Point を構築
```

## ADT の WasmGC 化

レコード型で WasmGC 基盤を整備する際に、ADT も同時に WasmGC 化する。現在は ADT のバリアントが全て `i64` にフォールバックしているが、WasmGC では部分型 (subtyping) を活用する:

```wasm
;; (type (Option a) (Some a) None)

;; 親型 (abstract)
(type $Option (sub (struct (field $tag i32))))

;; Some バリアント
(type $Option.Some (sub $Option
  (struct (field $tag i32) (field $value (ref eq)))))

;; None バリアント
(type $Option.None (sub $Option
  (struct (field $tag i32))))
```

パターンマッチは `$tag` フィールドで分岐し、`ref.cast` でダウンキャストする。

## 実装の影響範囲

レコード型の実装は全レイヤーに変更を要する:

| レイヤー | 変更内容 |
|----------|----------|
| Lexer/Parser | `record` キーワード、`{}` リテラル、`Dot` アクセス |
| AST | `TypeExpr::Record`, `Expr::RecordLit`, `Expr::FieldAccess` |
| 型推論 | レコード情報の登録、アクセサ関数の型付け |
| IR | `StructNew`, `StructGet`, `StructSet` 命令 |
| Codegen | WasmGC struct 型定義と GC 命令出力 |

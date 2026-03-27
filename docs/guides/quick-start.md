# Quick Start

L# は S 式構文と Hindley-Milner 型推論を組み合わせた関数型言語です。ここでは 5 分で基本的な書き方を一通り確認します。

## 1. Hello World

```lisp
(defn main []
  (do
    (print "hello, lsharp")
    0))
```

`lsharp compile src/Main.ls -o main.wasm` で Wasm を生成できます。

## 2. Fibonacci

```lisp
(defn fib [n]
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))
```

- `defn` で関数を定義する
- `if` は式なので必ず値を返す
- 型注釈がなくても `fib : Int -> Int` と推論される

## 3. ADT とパターンマッチ

```lisp
(type (Option a) (Some a) None)

(defn unwrap [opt default]
  (match opt
    [(Some value) value]
    [None default]))
```

- `type` で代数的データ型を定義する
- `match` で分岐し、各分岐は同じ型を返す

## 4. Record

```lisp
(type Point (record (: x Int) (: y Int)))

(defn move-x [p dx]
  {(p) | x (+ (Point.x p) dx)})
```

- `record` で named field を持つ型を作る
- `Point.x` のようにフィールドアクセスする
- `{(p) | x ...}` でレコード更新できる

## 5. Module

```lisp
;; src/Math/Geometry.ls
(module Math.Geometry)

(defn distance-squared [x y]
  (+ (* x x) (* y y)))
```

```lisp
;; src/Main.ls
(import Math.Geometry)

(defn main []
  (do
    (print (distance-squared 3 4))
    0))
```

- `src/Math/Geometry.ls` は `(import Math.Geometry)` で参照される
- 依存探索順は `src/` → `.lsharp/packages/*/src/` → `stdlib/`

## 次の導線

- 構文全体は `language-reference.md` を参照
- パッケージ構成は `package-layout.md` を参照
- 標準ライブラリ API は `lsharp doc-site --output _site` で HTML 化できる

# 高度な型機能

## 高カインド型 (Higher-Kinded Types)

### 型コンストラクタとカインド

これまでの型はすべて「値の型」、すなわちカインド `*` (スター) を持つ型だった。`Int`, `String`, `(Option Int)` はすべてカインド `*` である。

一方、`Option` 自体は「型を受け取って型を返す」**型コンストラクタ**である。`Option` のカインドは `* -> *`、つまり「型を1つ受け取って型を1つ返す」という意味になる。

高カインド型 (HKT) では、この型コンストラクタ自体をパラメータとして扱える:

```lisp
;; Functor トレイト -- 型コンストラクタに対するトレイト
(trait (Functor f)
  (defn fmap [(: func (-> a b)) (: fa (f a))] : (f b)))
```

ここで `f` は `* -> *` カインドの型変数である。`Option`, `List`, `Result String` など、型パラメータを1つ取る型コンストラクタなら何でも `f` に入れられる。

### Functor の実装例

```lisp
(impl (Functor Option)
  (defn fmap [(: func (-> a b)) (: fa (Option a))] : (Option b)
    (match fa
      [(Some x) (Some (func x))]
      [None None])))
```

HKT は**モナド**のような抽象の基盤となる。

## GADT (一般化代数的データ型)

### 通常の ADT の限界

通常の ADT ではバリアントの型パラメータを制約できない:

```lisp
;; 通常の ADT: 全バリアントが (Expr a) 型
(type (Expr a)
  (IntLit Int)
  (BoolLit Bool)
  (Add (Expr a) (Expr a)))
```

`(Add (IntLit 1) (BoolLit true))` が型エラーにならない問題がある。

### GADT の構文

GADT では各バリアントの**戻り値型**を個別に指定できる:

```lisp
(type (Expr a)
  :gadt
  [(IntLit Int)                        : (Expr Int)]
  [(BoolLit Bool)                      : (Expr Bool)]
  [(Add (Expr Int) (Expr Int))         : (Expr Int)]
  [(If (Expr Bool) (Expr a) (Expr a))  : (Expr a)])
```

これにより `IntLit` は常に `(Expr Int)` を返し、`BoolLit` は `(Expr Bool)` を返す。`(Add (IntLit 1) (BoolLit true))` は型エラーとなる。

### 型推論への影響

GADT のパターンマッチでは**型の絞り込み (type refinement)** が発生する。`(Expr a)` に対してパターン `(IntLit n)` がマッチした場合、そのブランチ内では `a = Int` が判明する。

これを実現するには、Algorithm W からバイディレクショナル型チェックへの移行が必要になる。

## Computation Expressions (F# 風)

### モナディック計算の問題

モナドを使ったコードは `bind` の連鎖が読みにくい:

```lisp
(bind (Some 1) (fn [x]
  (bind (Some 2) (fn [y]
    (return (+ x y))))))
```

### Computation Expressions の構文

F# の computation expression にインスパイアされた構文糖衣:

```lisp
;; ビルダー定義
(computation-builder option
  (defn bind [(: x (Option a)) (: f (-> a (Option b)))] : (Option b)
    (match x
      [(Some v) (f v)]
      [None None]))
  (defn return [(: x a)] : (Option a)
    (Some x)))

;; 使用
(option!
  (let! [x (Some 1)]
  (let! [y (Some 2)]
  (return (+ x y)))))
;; => (Some 3)
```

`let!` は `bind` に脱糖 (desugar) される。コンパイラが自動的に以下のコードに変換する:

```lisp
(bind (Some 1) (fn [x]
  (bind (Some 2) (fn [y]
    (return (+ x y))))))
```

### 依存関係

Computation Expressions はトレイト (Phase 5) と HKT (Phase 6a) の両方に依存する。`bind` や `return` の型をトレイトで抽象化し、任意のモナドに対して `!` 構文を使えるようにするためである。

## 実装の優先順位

これらの高度な機能は Phase 6 として計画されている:

| 機能 | 依存関係 | 難易度 |
|------|----------|--------|
| HKT | Phase 5 (トレイト) | 中〜高 |
| GADT | 独立 (型チェッカーの大改修) | 高 |
| Computation Expressions | Phase 5 + 6a | 中 |
| ネストモジュール | Phase 4 (モジュール) | 低 |

段階的に追加していくことで、各機能の正しさを確認しながら型システムを成長させる。

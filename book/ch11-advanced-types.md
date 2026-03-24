# 高度な型機能

本章では L# の型システムの高度な機能——高カインド型 (HKT)、GADT、Computation Expressions——を解説する。これらは第 4 章の Hindley-Milner 型推論と第 10 章のトレイトの上に構築される拡張機能である。

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

### Kind の実装

L# の Kind は `crates/lsharp-types/src/types.rs` に定義されている:

```rust
/// 型のカインド (型の型)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// 具体型のカインド: *
    Star,
    /// 型コンストラクタのカインド: k1 -> k2
    Arrow(Box<Kind>, Box<Kind>),
}
```

ユーティリティメソッドでよく使うカインドを生成できる:

```rust
impl Kind {
    /// * -> * (1引数の型コンストラクタ)
    pub fn unary() -> Self {
        Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))
    }

    /// * -> * -> * (2引数の型コンストラクタ)
    pub fn binary() -> Self {
        Kind::Arrow(
            Box::new(Kind::Star),
            Box::new(Kind::Arrow(
                Box::new(Kind::Star),
                Box::new(Kind::Star),
            )),
        )
    }
}
```

`Int`, `String`, `Bool` のカインドは `Star`。`Option`, `List` のカインドは `Arrow(Star, Star)` (= `* -> *`)。`Result`, `Map` のカインドは `Arrow(Star, Arrow(Star, Star))` (= `* -> * -> *`)。

### Kind 互換性チェック

トレイト実装時に、実装型のカインドがトレイトの要求するカインドと一致するかを検証する:

```rust
fn kinds_compatible(trait_kind: &Kind, type_kind: &Kind) -> bool {
    match (trait_kind, type_kind) {
        (Kind::Star, Kind::Star) => true,
        (Kind::Arrow(_, _), Kind::Arrow(_, _)) => trait_kind == type_kind,
        _ => false,
    }
}
```

不一致の場合、`TypeError::KindMismatch` が報告される:

```
エラー: Kind の不一致: Int は * ですが、
  トレイト Functor は * -> * を要求します
```

### Functor の実装例

```lisp
(impl (Functor Option)
  (defn fmap [(: func (-> a b)) (: fa (Option a))] : (Option b)
    (match fa
      [(Some x) (Some (func x))]
      [None None])))
```

HKT は**モナド**のような抽象の基盤となる。`Monad` トレイトは `Functor` を前提とし、`bind` と `return` の操作を定義する。

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

この定義では `(Add (IntLit 1) (BoolLit true))` が型エラーにならない。`IntLit` も `BoolLit` も同じ `(Expr a)` 型として扱われるため、`Add` の引数として両方が許容される。

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

これにより `IntLit` は常に `(Expr Int)` を返し、`BoolLit` は `(Expr Bool)` を返す。`(Add (IntLit 1) (BoolLit true))` は型エラーとなる——`Add` の第2引数は `(Expr Int)` を要求するが、`(BoolLit true)` は `(Expr Bool)` だからである。

### GADT の応用: 型安全なインタプリタ

GADT の典型的な応用は型安全なインタプリタである:

```lisp
(defn eval [(: expr (Expr a))] : a
  (match expr
    [(IntLit n) n]
    [(BoolLit b) b]
    [(Add x y) (+ (eval x) (eval y))]
    [(If cond then else)
      (if (eval cond) (eval then) (eval else))]))
```

各ブランチで `a` の型が自動的に絞り込まれる:

- `IntLit n` にマッチした場合 → `a = Int`、`n : Int` を返す
- `BoolLit b` にマッチした場合 → `a = Bool`、`b : Bool` を返す
- `Add x y` にマッチした場合 → `a = Int`、`(+ (eval x) (eval y))` は `Int` を返す

### 型推論への影響

GADT のパターンマッチでは**型の絞り込み (type refinement)** が発生する。`(Expr a)` に対してパターン `(IntLit n)` がマッチした場合、そのブランチ内では `a = Int` が判明する。

これを実現するには、通常の Algorithm W では不十分で、**バイディレクショナル型チェック**への拡張が必要になる。バイディレクショナル型チェックでは:

1. **推論モード** (synthesis): 式の型をボトムアップに推論する (従来の Algorithm W)
2. **チェックモード** (checking): 期待される型をトップダウンに伝搬して検証する

GADT のパターンマッチでは、チェックモードがバリアントの戻り値型から型等式を生成し、ブランチ内の推論環境に追加の制約を導入する。

### 型の絞り込みの仕組み

```
match expr with                    -- expr : (Expr a)
  (IntLit n) ->                    -- GADT が (IntLit Int) : (Expr Int) と宣言
                                   -- よって a = Int が判明
    n                              -- n : Int, 返り値 : Int = a (整合)
  (BoolLit b) ->                   -- GADT が (BoolLit Bool) : (Expr Bool) と宣言
                                   -- よって a = Bool が判明
    b                              -- b : Bool, 返り値 : Bool = a (整合)
```

この「ブランチごとの型変数束縛」は、通常の HM 推論の単一化とは異なり、ブランチの局所スコープでのみ有効な等式制約である。

## Computation Expressions (F# 風)

### モナディック計算の問題

モナドを使ったコードは `bind` の連鎖が読みにくい:

```lisp
(bind (Some 1) (fn [x]
  (bind (Some 2) (fn [y]
    (return (+ x y))))))
```

ネストが深くなるにつれて読みにくさが増す。これは「コールバック地獄」と本質的に同じ問題である。

### Computation Expressions の構文

F# の computation expression にインスパイアされた構文糖衣で、この問題を解決する:

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

### 脱糖 (Desugaring)

`let!` は `bind` に、`return` は `return` にそれぞれ脱糖される。コンパイラが自動的に以下のコードに変換する:

```lisp
;; 脱糖前
(option!
  (let! [x (Some 1)]
  (let! [y (Some 2)]
  (return (+ x y)))))

;; 脱糖後
(bind (Some 1) (fn [x]
  (bind (Some 2) (fn [y]
    (return (+ x y))))))
```

IR 降位 (`crates/lsharp-ir/src/lower/expr.rs`) では、この脱糖が段階的に処理される:

1. `let! [x expr] rest` → `bind` 関数の呼び出しに変換。`expr` を第1引数、`(fn [x] rest)` を第2引数とする
2. `do! expr` → `bind` 関数の呼び出しに変換。結果は束縛されず破棄される
3. `return expr` → ビルダーの `return` 関数の呼び出しに変換

### Computation Expressions の応用

Option 以外にも、様々なモナドに対して computation expression を定義できる:

```lisp
;; Result モナド (エラー処理)
(computation-builder result
  (defn bind [(: x (Result a e)) (: f (-> a (Result b e)))] : (Result b e)
    (match x
      [(Ok v) (f v)]
      [(Err e) (Err e)]))
  (defn return [(: x a)] : (Result a e)
    (Ok x)))

;; 使用例
(result!
  (let! [config (read-config "app.toml")]
  (let! [db (connect (Config.db-url config))]
  (return db))))
```

### 依存関係

Computation Expressions はトレイトと HKT の両方に依存する。`bind` や `return` の型をトレイトで抽象化し、任意のモナドに対して `!` 構文を使えるようにするためである。

将来的には `Monad` トレイトを定義し、全てのモナドに対して統一的な computation expression を提供する:

```lisp
(trait (Monad m)
  (defn bind [(: ma (m a)) (: f (-> a (m b)))] : (m b))
  (defn return [(: x a)] : (m a)))
```

## 各機能の依存関係と実装順序

これらの高度な機能は互いに依存関係を持つ:

```
トレイト (Phase 5)
    |
    +---> HKT (Phase 6a)
    |        |
    |        +---> Functor/Monad トレイト
    |
    +---> Computation Expressions (Phase 6c)
              |
              +---> Monad トレイト + HKT が前提

GADT (Phase 6b) -- 独立 (型チェッカーの大改修が必要)
```

| 機能 | 依存関係 | 難易度 | 状態 |
|------|----------|--------|------|
| HKT | トレイト | 中〜高 | 構文解析・Kind チェック実装済み |
| GADT | 独立 (型チェッカーの改修) | 高 | 構文解析実装済み |
| Computation Expressions | トレイト + HKT | 中 | 構文解析・脱糖実装済み |

段階的に追加していくことで、各機能の正しさを確認しながら型システムを成長させる。全ての機能の構文解析は既に完了しており、型推論と IR 降位の拡張が今後の主要な作業となる。

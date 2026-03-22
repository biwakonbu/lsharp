# 型推論 -- プログラムの安全性を自動検証する

## 型推論とは何か

型推論 (type inference) は、プログラマが型を書かなくても、コンパイラがプログラムの型を自動的に決定する仕組みである。

```lisp
(defn add [x y] (+ x y))
```

このコードには型注釈がない。しかし L# コンパイラは `+` が `(Int, Int) -> Int` であることを知っているので、`x` と `y` は `Int` であり、`add` の型は `(Int, Int) -> Int` だと推論できる。

```bash
$ cargo run -- check examples/fib.ls
fib : (Int) -> Int
main : () -> Unit
```

L# は **Hindley-Milner (HM) 型推論**を採用している。これは ML, OCaml, Haskell, F# で使われている型推論アルゴリズムで、以下の特徴を持つ:

- **完全な型推論**: 型注釈が一切なくても、推論可能な型は必ず見つかる
- **最も一般的な型**: 推論される型は常に最も一般的 (principal type) である
- **多相性**: `let` 束縛で多相型が自動的に推論される

## 型の表現

L# の型は 4 種類に分類される (`crates/lsharp-types/src/types.rs`):

```rust
pub enum Type {
    Con(String),              // 具体型: Int, String, Bool
    Var(TypeVarId),           // 型変数: 推論中の未知の型
    Fun(Vec<Type>, Box<Type>), // 関数型: (Int, Int) -> Bool
    App(String, Vec<Type>),    // 型適用: (Option Int)
}
```

この 4 種類は後続の章で拡張される機能にも対応する。レコード型は `Con("Point")` のような具体型として型環境に登録され、フィールド情報は `RecordInfo` として別途管理される (第 7 章)。型エイリアスは推論時に透過的に展開されるため `Type` enum 自体は変わらない (第 8 章)。

### 型変数

型推論の核心は**型変数 (type variable)** である。型変数は「まだわかっていない型」を表す。

```
add の推論過程:
  x : t0       (t0 は新しい型変数)
  y : t1       (t1 は新しい型変数)
  (+ x y)     → + は (Int, Int) -> Int
               → t0 = Int, t1 = Int と判明
  結果: add : (Int, Int) -> Int
```

型変数は `TypeVarId` (単なる整数) で識別され、`TypeVarGen` が一意な ID を生成する:

```rust
pub struct TypeVarGen {
    next_id: TypeVarId,
}

impl TypeVarGen {
    pub fn fresh(&mut self) -> Type {
        let id = self.next_id;
        self.next_id += 1;
        Type::Var(id)
    }
}
```

## 型代入 (Substitution)

型推論が「`t0 は Int だ」と判明したとき、この情報を記録するのが**型代入 (substitution)** である:

```rust
pub struct Substitution {
    map: BTreeMap<TypeVarId, Type>,
}
```

型代入は `{ t0 → Int, t1 → String }` のようなマッピングで、型に適用すると型変数を具体型に置き換える:

```rust
impl Type {
    pub fn apply_subst(&self, subst: &Substitution) -> Type {
        match self {
            Type::Var(id) => {
                if let Some(ty) = subst.get(*id) {
                    ty.apply_subst(subst)  // 推移的に適用
                } else {
                    Type::Var(*id)
                }
            }
            Type::Fun(params, ret) => Type::Fun(
                params.iter().map(|p| p.apply_subst(subst)).collect(),
                Box::new(ret.apply_subst(subst)),
            ),
            Type::Con(name) => Type::Con(name.clone()),
            // ...
        }
    }
}
```

注目すべきは、置換の適用が**推移的**であること。`{ t0 → t1, t1 → Int }` を `t0` に適用すると、`t0 → t1 → Int` と追跡して最終的に `Int` が得られる。

## 統合 (Unification)

型推論の心臓部が**統合 (unification)** である。2つの型を「等しくする」ための代入を見つける:

```rust
fn unify(&mut self, t1: &Type, t2: &Type, span: Span)
    -> Result<Substitution, TypeError>
{
    match (t1, t2) {
        // 同じ具体型 → 成功 (代入不要)
        (Type::Con(a), Type::Con(b)) if a == b =>
            Ok(Substitution::new()),

        // 型変数 → その型に束縛
        (Type::Var(id), ty) | (ty, Type::Var(id)) =>
            self.bind_var(*id, ty, span),

        // 関数型 → 引数と戻り値をそれぞれ統合
        (Type::Fun(p1, r1), Type::Fun(p2, r2)) => {
            // 引数の数が一致することを確認
            // 各引数を統合し、代入を累積
            // 戻り値型も統合
        }

        // 型適用 → 型名が一致し、引数をそれぞれ統合
        (Type::App(n1, a1), Type::App(n2, a2)) if n1 == n2 => { ... }

        // それ以外 → 型エラー
        _ => Err(TypeError::Mismatch { expected: t1, found: t2 })
    }
}
```

### occurs check -- 無限型の防止

型変数を型に束縛する際、**occurs check** を行う:

```rust
fn bind_var(&self, var: TypeVarId, ty: &Type, span: Span)
    -> Result<Substitution, TypeError>
{
    // 同一変数への束縛は無視
    if let Type::Var(id) = ty {
        if *id == var { return Ok(Substitution::new()); }
    }

    // occurs check: t0 = (t0) -> Int のような無限型を防止
    if ty.free_vars().contains(&var) {
        return Err(TypeError::InfiniteType { var, ty: ty.clone(), span });
    }

    let mut subst = Substitution::new();
    subst.insert(var, ty.clone());
    Ok(subst)
}
```

`t0 = (t0) -> Int` は `t0 = ((t0) -> Int) -> Int = ...` と無限に展開されてしまう。occurs check はこのような無限型を検出して拒絶する。

## Algorithm W -- 式の型推論

HM 型推論の具体的なアルゴリズムは **Algorithm W** と呼ばれる。各式に対して「代入と推論された型」の組を返す:

```rust
fn infer_expr(&mut self, env: &TypeEnv, expr: &Expr)
    -> Result<(Substitution, Type), TypeError>
```

### リテラルと変数

最も単純なケース:

```rust
// リテラル: 型は自明
Expr::Lit(_, Literal::Int(_))  => Ok((empty_subst, Type::int()))
Expr::Lit(_, Literal::Bool(_)) => Ok((empty_subst, Type::bool()))

// 変数: 型環境から取得してインスタンス化
Expr::Var(span, name) => {
    let scheme = env.get(name)?;  // 型スキームを取得
    let ty = self.instantiate(scheme);  // 新しい型変数で具体化
    Ok((empty_subst, ty))
}
```

### if 式

`(if cond then else)` の推論:

```rust
Expr::If(span, cond, then, else_) => {
    // 1. 条件式は Bool でなければならない
    let (s1, cond_ty) = self.infer_expr(env, cond)?;
    let s_cond = self.unify(&cond_ty, &Type::bool(), span)?;

    // 2. then 節と else 節を推論
    let (s2, then_ty) = self.infer_expr(&env, then)?;
    let (s3, else_ty) = self.infer_expr(&env, else_)?;

    // 3. 両方の分岐は同じ型でなければならない
    let s_branch = self.unify(&then_ty, &else_ty, span)?;

    Ok((composed_subst, final_type))
}
```

3つの制約がある: (1) 条件は `Bool`、(2,3) 両分岐は同じ型。これらを統合で強制する。

### 関数適用

`(f arg1 arg2 ...)` の推論が最も面白い:

```rust
Expr::App(span, func, args) => {
    // 1. 関数の型を推論
    let (s1, func_ty) = self.infer_expr(env, func)?;

    // 2. 各引数の型を推論
    let mut arg_types = Vec::new();
    for arg in args {
        let (s, arg_ty) = self.infer_expr(&env, arg)?;
        arg_types.push(arg_ty);
    }

    // 3. 戻り値型の型変数を生成
    let ret_ty = self.var_gen.fresh();

    // 4. 関数の型と「(arg_types) -> ret_ty」を統合
    let expected = Type::Fun(arg_types, Box::new(ret_ty.clone()));
    let s_unify = self.unify(&func_ty, &expected, span)?;

    Ok((final_subst, ret_ty.apply_subst(&s_unify)))
}
```

ポイントは手順 4 である。関数の推論された型と「引数型から戻り値型への関数型」を統合することで、型変数が具体化される。

### let 式と多相性

`(let [x 10 y (+ x 1)] body)`:

```rust
Expr::Let(_, bindings, body) => {
    for (pat, val) in bindings {
        let (s1, val_ty) = self.infer_expr(&env, val)?;

        // let 多相: 値の型を汎化
        let scheme = self.generalize(&env, &val_ty);
        env.insert(pat_name, scheme);
    }

    self.infer_expr(&env, body)
}
```

**let 多相 (let polymorphism)** は HM 型推論の重要な機能である。`let` で束縛された値は**汎化 (generalization)** される:

```rust
fn generalize(&self, env: &TypeEnv, ty: &Type) -> TypeScheme {
    let env_vars = env.free_vars();      // 環境中の自由型変数
    let ty_vars = ty.free_vars();         // 型中の自由型変数
    // 環境にない型変数を全称量化する
    let vars = ty_vars.filter(|v| !env_vars.contains(v));
    TypeScheme { vars, ty }
}
```

汎化により `(defn id [x] x)` の型は単なる `t0 -> t0` ではなく `forall a. a -> a` になる。使用するたびに `a` が別の型に**インスタンス化 (instantiation)** されるので、`(id 42)` も `(id true)` も型安全に使える:

```rust
fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
    let mut subst = Substitution::new();
    for &var in &scheme.vars {
        subst.insert(var, self.var_gen.fresh());  // 新しい型変数に置換
    }
    scheme.ty.apply_subst(&subst)
}
```

## 再帰関数の型推論

再帰関数は特別な処理が必要である。関数本体の中で自分自身を呼び出すので、関数の型を知る前に関数の型が必要になる。

L# はこれを**仮登録**で解決する:

```rust
fn infer_defn(&mut self, env: &TypeEnv, name: &str, params: &[Param],
              body: &Expr, ...) -> Result<(Substitution, Type), TypeError>
{
    // 1. 関数自身を型変数として仮登録
    let self_ty = self.var_gen.fresh();
    env.insert(name, TypeScheme::mono(self_ty.clone()));

    // 2. パラメータの型変数を生成
    let param_types = params.map(|_| self.var_gen.fresh());

    // 3. 本体を型推論 (再帰呼び出しは self_ty を使う)
    let (subst, body_type) = self.infer_expr(&env, body)?;

    // 4. 関数型を構築
    let func_type = Type::Fun(param_types, Box::new(body_type));

    // 5. 仮登録した型と実際の関数型を統合
    let s_self = self.unify(&self_ty, &func_type, span)?;

    Ok((subst, func_type))
}
```

`fib` の例:

1. `fib : t0` として仮登録
2. 本体で `(fib (- n 1))` が呼ばれると、`t0` が `(Int) -> Int` と統合される
3. 最終的に `fib : (Int) -> Int` が確定する

## ADT とパターンマッチの型推論

### コンストラクタの登録

ADT 定義 `(type (Option a) (Some a) None)` を処理すると、コンストラクタが型環境に登録される:

```
Some : forall a. a -> (Option a)
None : forall a. (Option a)
```

`Some` は引数を取る関数型、`None` は引数なしの値として登録される。

### パターンマッチの推論

```lisp
(match opt
  [(Some x) x]
  [None 0])
```

各パターンの型推論:

1. `(Some x)` パターン → `Some` は `a -> (Option a)` なので、パターン全体は `(Option a)` 型。`x` は `a` 型に束縛される
2. `None` パターン → `(Option a)` 型
3. 両パターンの型が `opt` の型と統合される
4. 各腕の本体の型が統合される: `x : a` と `0 : Int` → `a = Int`
5. 最終結果: `(Option Int) -> Int`

## 型環境 (TypeEnv)

型環境は変数名から型スキームへのマッピングである:

```rust
pub struct TypeEnv {
    bindings: BTreeMap<String, TypeScheme>,
}
```

型推論器は起動時に**組み込み環境**を構築する:

```rust
fn builtin_env(&mut self) -> TypeEnv {
    let mut env = TypeEnv::new();

    // 算術: (Int, Int) -> Int
    for op in ["+", "-", "*", "/", "%"] {
        env.insert(op, TypeScheme::mono(Type::Fun(
            vec![Type::int(), Type::int()],
            Box::new(Type::int()),
        )));
    }

    // 比較: (Int, Int) -> Bool
    for op in ["<", ">", "<=", ">=", "==", "!="] { ... }

    // print: forall a. a -> Unit
    env.insert("print", TypeScheme {
        vars: vec![a],
        ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::unit())),
    });

    env
}
```

`print` は**多相的**に定義されている。任意の型を受け取れるので、`(print 42)` も `(print "hello")` も型エラーにならない。

## 型注釈

型推論が完全であるとはいえ、可読性やドキュメントのために型注釈を書くことができる:

```lisp
(defn add [(: x Int) (: y Int)] : Int
  (+ x y))
```

型注釈は**制約**として機能する。推論された型と注釈された型を統合し、矛盾があれば型エラーになる:

```rust
Expr::Ann(span, expr, type_expr) => {
    let (s1, inferred) = self.infer_expr(env, expr)?;
    let annotated = self.resolve_type_expr(type_expr, &[]);
    let s2 = self.unify(&inferred, &annotated, span)?;
    Ok((s1.compose(&s2), annotated))
}
```

## まとめ

HM 型推論の仕組みを整理する:

| 概念 | 役割 |
|------|------|
| 型変数 | 未知の型を表す。推論中に具体化される |
| 型代入 | 型変数から型へのマッピング。推論結果を蓄積する |
| 統合 | 2つの型を等しくする代入を見つける |
| 汎化 | 型変数を全称量化して多相型を作る |
| インスタンス化 | 多相型を使うたびに新しい型変数で具体化する |
| occurs check | 無限型を検出して拒絶する |

これらの機構が組み合わさることで、型注釈なしでも安全な型チェックが可能になる。

次章では、型チェック済みの AST を中間表現 (IR) に変換する**IR 降位 (lowering)** を見ていく。

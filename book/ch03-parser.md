# 構文解析 -- トークンから木構造を作る

## 構文解析とは何か

前章で字句解析器がソースコードをトークン列に変換した。しかしトークン列は「文字の並び」に過ぎず、プログラムの**構造**を表現していない。構文解析 (parsing) は、このトークン列を**抽象構文木 (Abstract Syntax Tree, AST)** に変換する。

たとえば `(+ (* 2 3) 4)` というコードは、次の木構造になる:

```
    App(+)
   /      \
 App(*)    4
 /   \
2     3
```

この木構造があれば「まず `2 * 3` を計算し、その結果に `4` を足す」という実行順序が明確になる。

## S 式の利点 -- パーサーが簡単

L# は S 式ベースの構文を採用している。これはパーサーの実装において大きな利点がある。

C や Python のような言語では、演算子の**優先順位 (precedence)** と**結合性 (associativity)** を処理するために複雑なパーサーが必要になる。たとえば `2 + 3 * 4` が `2 + (3 * 4)` と解釈されるのか `(2 + 3) * 4` と解釈されるのかは、パーサーが優先順位規則を知っていなければならない。

S 式ではこの問題が存在しない。構造は括弧で**明示的に**表現される:

```lisp
(+ 2 (* 3 4))    ;; 2 + (3 * 4)
(* (+ 2 3) 4)    ;; (2 + 3) * 4
```

括弧がそのまま木構造に対応するため、優先順位の規則は不要である。

## AST の設計

L# の AST は 3 つの主要な型で構成される (`crates/lsharp-syntax/src/ast.rs`):

### 式 (Expr)

プログラムの計算を表現する:

```rust
pub enum Expr {
    Lit(Span, Literal),                           // リテラル: 42, "hello"
    Var(Span, String),                             // 変数参照: x, add
    If(Span, Box<Expr>, Box<Expr>, Box<Expr>),     // if 式
    Let(Span, Vec<(Pattern, Expr)>, Box<Expr>),    // let 束縛
    Lambda(Span, Vec<Param>, Box<Expr>),            // 無名関数
    App(Span, Box<Expr>, Vec<Expr>),               // 関数適用
    Match(Span, Box<Expr>, Vec<MatchArm>),         // パターンマッチ
    Do(Span, Vec<Expr>),                           // 逐次実行
    Ann(Span, Box<Expr>, TypeExpr),                // 型注釈

    // レコード型 (第 7 章で詳述)
    RecordLit(Span, String, Vec<(String, Expr)>),  // {Point x 1.0 y 2.0}
    FieldAccess(Span, Box<Expr>, String),           // (Point.x p)
    RecordUpdate(Span, Box<Expr>, Vec<(String, Expr)>), // {p | x 3.0}
}
```

すべての AST ノードに `Span` が付与されている。これにより、型エラーなどの後段のエラーもソースコードの正確な位置を指し示せる。

### パターン (Pattern)

`let` 束縛や `match` で使用する:

```rust
pub enum Pattern {
    Wildcard(Span),                           // _
    Var(Span, String),                         // 変数束縛
    Lit(Span, Literal),                        // リテラルパターン
    Constructor(Span, String, Vec<Pattern>),   // コンストラクタパターン
    RecordPat(Span, String, Vec<(String, Pattern)>), // {Point x y} (第 7 章)
}
```

### 宣言 (Decl)

トップレベルの定義:

```rust
pub enum Decl {
    Defn {                            // 関数定義
        span: Span,
        name: String,
        params: Vec<Param>,
        return_ty: Option<TypeExpr>,
        body: Expr,
        where_clauses: Vec<WhereClause>,  // トレイト制約 (第 10 章)
        metadata: Option<Metadata>,        // 構造化メタデータ
    },
    TypeDef {                         // ADT 定義
        span: Span,
        name: String,
        type_params: Vec<String>,
        variants: Vec<Variant>,
        metadata: Option<Metadata>,
    },
    RecordDef { .. },                 // レコード型定義 (第 7 章)
    TypeAlias { .. },                 // 型エイリアス (第 8 章)
    TypeConstrained { .. },           // 制約付き型 (第 8 章)
    ModuleDecl { .. },                // モジュール宣言 (第 9 章)
    ImportDecl { .. },                // インポート (第 9 章)
    TraitDef { .. },                  // トレイト定義 (第 10 章)
    ImplDef { .. },                   // トレイト実装 (第 10 章)
    Private { .. },                   // 非公開宣言 (第 9 章)
}
```

`Decl` はコンパイラの進化に伴い多くのバリアントを持つようになった。各バリアントの詳細は対応する章で解説する。初期実装では `Defn` と `TypeDef` のみだったが、型システムの拡張で大幅に増えている。

## パーサーの実装

L# のパーサーは**再帰下降パーサー (recursive descent parser)** として実装されている (`crates/lsharp-syntax/src/parser.rs`)。

### 再帰下降パーサーとは

再帰下降パーサーは、文法規則ごとに1つの関数を用意する方式である。関数が互いを再帰的に呼び出すことで、ネストした構造を自然にパースできる。

```rust
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,           // 現在の読み取り位置
}
```

### プログラムのパース

プログラムはトップレベル宣言の列である:

```rust
pub fn parse_program(&mut self) -> Result<Program, ParseError> {
    let mut decls = Vec::new();
    while !self.is_eof() {
        decls.push(self.parse_decl()?);
    }
    Ok(Program { decls })
}
```

### 宣言のパース

すべての宣言は `(` で始まる。その直後のキーワードで宣言の種類を判別する:

```rust
fn parse_decl(&mut self) -> Result<Decl, ParseError> {
    self.expect(TokenKind::LParen)?;  // 開き括弧を消費

    match self.peek_kind() {
        Some(TokenKind::Defn) => self.parse_defn(start_span),
        Some(TokenKind::Type) => self.parse_type_def(start_span),
        // ...
    }
}
```

S 式の構文では、先頭のキーワードを1つ見るだけで何をパースすべきかが決まる。これを **LL(1)** と呼ぶ -- 先読み 1 トークンで判別可能という意味である。

### 関数定義のパース

`(defn name [params] body)` の形式:

```rust
fn parse_defn(&mut self, start_span: Span) -> Result<Decl, ParseError> {
    self.advance();                        // defn を消費
    let name = self.expect_symbol()?;      // 関数名
    let params = self.parse_params()?;     // パラメータリスト

    // オプションの戻り値型注釈 `: RetType`
    let return_ty = if self.check(TokenKind::Colon) {
        self.advance();
        Some(self.parse_type_expr()?)
    } else {
        None
    };

    let body = self.parse_expr()?;         // 本体
    self.expect(TokenKind::RParen)?;       // 閉じ括弧
    Ok(Decl::Defn { name, params, return_ty, body, .. })
}
```

パラメータリストは `[x y]` のように角括弧で囲まれる。型注釈付きの場合は `[(: x Int) (: y Int)]` となる:

```rust
fn parse_param(&mut self) -> Result<Param, ParseError> {
    if self.check(TokenKind::LParen) {
        // (: name Type) 形式
        self.advance();
        self.expect(TokenKind::Colon)?;
        let name = self.expect_symbol()?;
        let ty = self.parse_type_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(Param { name, ty: Some(ty), .. })
    } else {
        // 型注釈なし
        let name = self.expect_symbol()?;
        Ok(Param { name, ty: None, .. })
    }
}
```

### 式のパース

式のパースは、先頭のトークンで分岐する:

```rust
fn parse_expr(&mut self) -> Result<Expr, ParseError> {
    match self.peek_kind() {
        Some(TokenKind::LParen)    => self.parse_list_expr(),  // 括弧式
        Some(TokenKind::Int(n))    => Ok(Expr::Lit(..)),       // 整数リテラル
        Some(TokenKind::Float(n))  => Ok(Expr::Lit(..)),       // 浮動小数点
        Some(TokenKind::String(s)) => Ok(Expr::Lit(..)),       // 文字列
        Some(TokenKind::Bool(b))   => Ok(Expr::Lit(..)),       // 真偽値
        Some(TokenKind::Symbol(s)) => Ok(Expr::Var(..)),       // 変数
        _ => Err(ParseError::Unexpected { .. }),
    }
}
```

括弧で始まる式は、その直後のトークンでさらに分岐する:

```rust
fn parse_list_expr(&mut self) -> Result<Expr, ParseError> {
    self.expect(TokenKind::LParen)?;

    // 空括弧 = Unit 値
    if self.check(TokenKind::RParen) {
        return Ok(Expr::Lit(.., Literal::Unit));
    }

    match self.peek_kind() {
        Some(TokenKind::If)    => self.parse_if(..),      // (if ...)
        Some(TokenKind::Let)   => self.parse_let(..),     // (let ...)
        Some(TokenKind::Fn)    => self.parse_lambda(..),  // (fn ...)
        Some(TokenKind::Match) => self.parse_match(..),   // (match ...)
        Some(TokenKind::Do)    => self.parse_do(..),      // (do ...)
        Some(TokenKind::Colon) => self.parse_ann(..),     // (: ...)
        _                      => self.parse_app(..),     // 関数適用
    }
}
```

### パターンマッチのパース

`(match scrutinee [pat1 body1] [pat2 body2] ...)`:

```rust
fn parse_match(&mut self, start_span: Span) -> Result<Expr, ParseError> {
    self.advance(); // match
    let scrutinee = self.parse_expr()?;  // マッチ対象

    let mut arms = Vec::new();
    while self.check(TokenKind::LBracket) {
        self.advance();                          // [
        let pattern = self.parse_pattern()?;     // パターン
        let body = self.parse_expr()?;           // 本体
        self.expect(TokenKind::RBracket)?;       // ]
        arms.push(MatchArm { pattern, body, .. });
    }

    self.expect(TokenKind::RParen)?;
    Ok(Expr::Match(.., Box::new(scrutinee), arms))
}
```

角括弧 `[]` を使って各腕 (arm) を囲むのは L# の特徴的な構文である。Clojure のベクタリテラルに影響を受けた設計で、丸括弧のネストが深くなりすぎるのを防ぐ。

### パターンの判別規則

パターンのパースでは、大文字始まりか小文字始まりかで意味が変わる:

```rust
fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
    match self.peek_kind() {
        // _ → ワイルドカード
        Some(TokenKind::Symbol(s)) if s == "_" =>
            Ok(Pattern::Wildcard(..)),

        // 大文字始まり → コンストラクタ (引数なし)
        Some(TokenKind::Symbol(s)) if s.starts_with(uppercase) =>
            Ok(Pattern::Constructor(.., name, vec![])),

        // 小文字始まり → 変数束縛
        Some(TokenKind::Symbol(_)) =>
            Ok(Pattern::Var(.., name)),

        // (Constructor arg1 arg2) → 引数付きコンストラクタ
        Some(TokenKind::LParen) => { /* ... */ }

        // リテラルパターン
        Some(TokenKind::Int(_)) => Ok(Pattern::Lit(..)),
        // ...
    }
}
```

大文字/小文字の区別は Haskell や OCaml と同じ慣例である。`Some` はコンストラクタ、`x` は変数束縛を意味する。

## 型式のパース

型の表現も S 式を活用する:

```rust
fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
    match self.peek_kind() {
        // 大文字始まり → 具体型名 (Int, String, ...)
        Some(TokenKind::Symbol(s)) if uppercase =>
            Ok(TypeExpr::Named(..)),

        // 小文字始まり → 型変数 (a, b, ...)
        Some(TokenKind::Symbol(_)) =>
            Ok(TypeExpr::Var(..)),

        Some(TokenKind::LParen) => {
            if self.check(TokenKind::Arrow) {
                // (-> Int Int Bool) → 関数型
            } else {
                // (Option Int) → 型適用
            }
        }
    }
}
```

型の世界でも大文字/小文字の区別が機能する。`Int` は具体的な型名、`a` は型変数 (任意の型を表す) である。

## エラー処理

パーサーは 3 種類のエラーを返す:

```rust
pub enum ParseError {
    Unexpected { expected: String, found: String, span: Span },
    UnexpectedEof { expected: String },
    UnknownForm { name: String, span: Span },
}
```

`Unexpected` は「X を期待したが Y が見つかった」、`UnknownForm` は「認識できないキーワードが式の先頭にある」場合に発生する。

## AST の表示

L# の AST は `Display` トレイトを実装しており、パース結果を人間が読める形で表示できる:

```bash
$ cargo run -- parse examples/fib.ls
(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
(defn main [] (print (fib 10)))
```

入力と出力がほぼ同じ形であることに注目してほしい。S 式では AST の構造とソースコードの見た目が一致するため、パース結果の検証が容易である。

## まとめ

S 式ベースの構文は、パーサーの実装を驚くほど単純にする:

- **優先順位ルール不要**: 括弧が構造を完全に決定する
- **LL(1) パーサー**: 先読み 1 トークンで全て判別できる
- **再帰下降**: 文法規則とコードが 1:1 対応する
- **大文字/小文字規則**: 型名とコンストラクタ (大文字) vs 変数 (小文字)

次章では、この AST に対して**型推論**を行い、プログラムの型安全性を検証する。

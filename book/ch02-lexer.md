# 字句解析 -- ソースコードをトークンに分解する

## 字句解析とは何か

コンパイラが最初に行う仕事は、人間が書いたソースコードの文字列を意味のある単位に分解することである。この処理を**字句解析 (lexical analysis)** と呼び、分解された各単位を**トークン (token)** と呼ぶ。

たとえば、以下の L# コードを考える:

```lisp
(+ 1 2)
```

字句解析器はこの文字列を次の 5 つのトークンに分解する:

| トークン | 種類 |
|----------|------|
| `(` | 左括弧 (LParen) |
| `+` | シンボル (Symbol) |
| `1` | 整数 (Int) |
| `2` | 整数 (Int) |
| `)` | 右括弧 (RParen) |

空白はトークンの区切りとして機能するが、それ自体はトークンにならない。字句解析器はこの「空白の読み飛ばし」も担当する。

## L# のトークン設計

L# のトークンは `TokenKind` 列挙型で定義されている (`crates/lsharp-syntax/src/token.rs`):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 区切り文字
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // リテラル
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // 識別子・シンボル
    Symbol(String),

    // キーワード -- 基本
    Defn, Let, If, Match, Type, Fn, Do, Module, Import,

    // キーワード -- 型システム拡張
    Record,          // レコード型定義
    TypeAlias,       // 型エイリアス
    TypeConstrained, // 制約付き型
    Constraints,     // 制約ブロック
    Trait,           // トレイト定義
    Impl,            // トレイト実装
    Where,           // トレイト制約
    Private,         // 非公開宣言

    // 型注釈・記号
    Colon, // :
    Arrow, // ->
    Pipe,  // | (レコード更新構文用)
    Dot,   // . (フィールドアクセス用)

    // 特殊
    Eof,
}
```

S 式ベースの言語はトークンの種類が少ないという特徴がある。C や Java のような言語では数十種類の演算子トークンが必要だが、L# では `+` や `*` もすべて `Symbol` として扱う。演算子と関数の区別がないのは S 式の大きな利点である。

キーワードは言語の進化に伴い増えていく。基本キーワード (`defn`, `let`, `if` 等) に加えて、型システムの拡張で `record`, `type-alias`, `type-constrained`, `trait`, `impl`, `where`, `private` が追加された。これらは後続の章で詳しく解説する。

### なぜ括弧が 3 種類あるのか

L# では 3 種類の括弧を使い分ける:

- **丸括弧 `()`**: 式の基本構造。関数呼び出し、定義、制御構造
- **角括弧 `[]`**: パラメータリスト、パターンマッチの腕 (Clojure 由来)
- **波括弧 `{}`**: レコードリテラル `{Point x 1.0 y 2.0}` とレコード更新構文 `{p | x 3.0}` で使用

## Span -- ソース位置の追跡

各トークンは「ソースコード中のどこにあるか」を記録する。これは**エラー報告**に不可欠である。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,  // 開始バイト位置
    pub end: usize,    // 終了バイト位置
}
```

たとえば `(+ 1 2)` を字句解析した場合、`+` のトークンには `Span { start: 1, end: 2 }` が付与される。コンパイルエラーが発生したとき、この情報をもとに「ソースコードの何行目、何文字目で問題が起きたか」を正確に報告できる。

## 字句解析器の実装

L# の字句解析器は `Lexer` 構造体として実装されている (`crates/lsharp-syntax/src/lexer.rs`):

```rust
pub struct Lexer<'src> {
    source: &'src str,    // ソースコード全体
    bytes: &'src [u8],    // バイト列 (高速アクセス用)
    pos: usize,           // 現在の読み取り位置
}
```

`Lexer` は 1 文字ずつソースコードを走査し、パターンに応じて適切なトークンを生成する。

### メインループ

字句解析の中心は `next_token` メソッドである:

```rust
fn next_token(&mut self) -> Result<Token, LexError> {
    self.skip_whitespace_and_comments();

    if self.pos >= self.bytes.len() {
        return Ok(Token::new(TokenKind::Eof, Span::new(self.pos, self.pos)));
    }

    let start = self.pos;
    let ch = self.bytes[self.pos] as char;

    match ch {
        '(' => { self.pos += 1; Ok(Token::new(TokenKind::LParen, ...)) }
        ')' => { self.pos += 1; Ok(Token::new(TokenKind::RParen, ...)) }
        '"' => self.lex_string(),
        _ if ch.is_ascii_digit() => self.lex_number(),
        '-' if self.peek_next().is_some_and(|c| c == '>') => {
            // -> (アロー) の特別処理
        }
        '-' if self.peek_next().is_some_and(|c| c.is_ascii_digit()) => {
            // 負数リテラルの処理
        }
        ':' => { self.pos += 1; Ok(Token::new(TokenKind::Colon, ...)) }
        _ if is_symbol_start(ch) => self.lex_symbol(),
        _ => Err(LexError::UnexpectedChar { ch, span: ... }),
    }
}
```

このメソッドは次の手順で動作する:

1. **空白とコメントをスキップ**する
2. 入力が終わっていれば `Eof` トークンを返す
3. 先頭の文字を見て、どの種類のトークンかを**判別 (dispatch)** する
4. 対応する解析関数を呼び出してトークンを生成する

### `-` の曖昧さ

`-` は3つの意味を持ちうる:

1. **アロー `->` の一部**: 次の文字が `>` なら `Arrow` トークン
2. **負数リテラル**: 次の文字が数字なら数値の一部
3. **減算演算子**: それ以外の場合はシンボル

字句解析器は**先読み (lookahead)** で次の文字を確認し、正しいトークンを生成する。この判別順序が重要で、`->` の判定を先に行わないと、`->` が「`-` シンボル」と「`>` シンボル」に分割されてしまう。

### 空白とコメントのスキップ

```rust
fn skip_whitespace_and_comments(&mut self) {
    while self.pos < self.bytes.len() {
        let ch = self.bytes[self.pos] as char;
        if ch.is_ascii_whitespace() {
            self.pos += 1;
        } else if ch == ';' {
            // 行コメント: 行末までスキップ
            while self.pos < self.bytes.len()
                  && self.bytes[self.pos] != b'\n' {
                self.pos += 1;
            }
        } else {
            break;
        }
    }
}
```

L# のコメントは `;` (セミコロン) で始まり、行末まで続く。これは Lisp 系言語の伝統的なコメント構文である。

### 数値リテラルの解析

数値の解析では、整数と浮動小数点数を区別する必要がある:

```rust
fn lex_number(&mut self) -> Result<Token, LexError> {
    let start = self.pos;
    let mut is_float = false;

    // 負号の処理
    if self.bytes[self.pos] == b'-' { self.pos += 1; }

    // 整数部: 連続する数字を読む
    while self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }

    // 小数部: '.' の後に数字が続くなら浮動小数点数
    if self.bytes[self.pos] == b'.'
       && self.bytes[self.pos + 1].is_ascii_digit() {
        is_float = true;
        self.pos += 1;
        while self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }
    }

    let text = &self.source[start..self.pos];
    if is_float {
        text.parse::<f64>().map(|n| Token::new(TokenKind::Float(n), ...))
    } else {
        text.parse::<i64>().map(|n| Token::new(TokenKind::Int(n), ...))
    }
}
```

`3.14` は `Float(3.14)` に、`42` は `Int(42)` になる。小数点の後に数字が続かない場合 (例: `42.add`) は整数として処理し、`.` はシンボルの一部として次のトークンに回す。

### キーワードとシンボルの判別

S 式言語では、キーワードもシンボルも同じ規則で読み取る。読み取った後に文字列の内容でキーワードかどうかを判定する:

```rust
fn lex_symbol(&mut self) -> Result<Token, LexError> {
    let start = self.pos;
    while is_symbol_char(self.bytes[self.pos] as char) {
        self.pos += 1;
    }

    let text = &self.source[start..self.pos];
    let kind = match text {
        "defn"   => TokenKind::Defn,
        "let"    => TokenKind::Let,
        "if"     => TokenKind::If,
        "match"  => TokenKind::Match,
        "type"   => TokenKind::Type,
        "true"   => TokenKind::Bool(true),
        "false"  => TokenKind::Bool(false),
        _        => TokenKind::Symbol(text.to_string()),
    };

    Ok(Token::new(kind, span))
}
```

`defn` や `let` といった予約語はキーワードトークンに変換され、それ以外の文字列はすべて `Symbol` トークンになる。

### シンボル文字の規則

L# のシンボルは通常の識別子に加えて、演算子記号も含む:

```rust
fn is_symbol_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(ch,
            '_' | '+' | '-' | '*' | '/' | '='
            | '<' | '>' | '!' | '?' | '&' | '|'
            | '%' | '^' | '~' | '@')
}

fn is_symbol_char(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit()
        || matches!(ch, '.' | '-')
}
```

これにより `+`, `<=`, `even?` のような記号もシンボルとして読み取れる。S 式では「演算子」と「関数名」に区別がないため、すべてシンボルである。

## エラー処理

字句解析で発生しうるエラーは 3 種類:

```rust
pub enum LexError {
    UnexpectedChar { ch: char, span: Span },      // 認識できない文字
    UnterminatedString { span: Span },              // 閉じ引用符のない文字列
    InvalidNumber { text: String, span: Span },     // 不正な数値
}
```

すべてのエラーに `Span` を持たせることで、エラー箇所をソースコード上で正確に指し示せる。

## まとめ

字句解析は文字列をトークン列に変換する単純な処理だが、いくつかの判断が求められる:

- **先読みによる曖昧性の解消**: `-` の3つの解釈を区別する
- **位置情報の保持**: `Span` でエラー報告を支援する
- **S 式の簡潔さ**: トークン種類が少なく、演算子はすべてシンボル

次章では、このトークン列から木構造 (AST) を構築する**構文解析**を見ていく。

# エラー報告 -- 開発者に優しいコンパイラを作る

## エラーメッセージの重要性

コンパイラのエラーメッセージは、プログラマとコンパイラの対話である。良いエラーメッセージは以下の情報を提供する:

1. **何が**間違っているか
2. **どこで**間違っているか
3. **なぜ**間違っているか
4. (可能であれば) **どう直せばよいか**

歴史的に、C++ コンパイラのテンプレートエラーは数百行に及ぶ解読不能なメッセージで悪名高かった。一方、Elm コンパイラは「エラーメッセージは対話である」という哲学のもと、具体的な修正候補を提示するスタイルで高い評価を受けた。Rust コンパイラもこの流れを継承し、エラーコード (E0308 など)、ラベル付きの矢印表示、`help:` による修正提案を提供している。

L# はこれらの先行事例に学び、以下の設計原則を採用している:

- **全レイヤーに位置情報**: 字句解析から IR 変換まで、全てのエラーに `Span` を付与
- **統一エラー型**: パイプライン全体のエラーを一つの型で扱い、呼び出し元で分岐しやすくする
- **日本語エラーメッセージ**: ユーザー向けメッセージは日本語で提供し、内部的にはコード識別

## Span の実装詳細

エラーの位置情報を担う最も基本的なデータ構造が `Span` である。`crates/lsharp-syntax/src/span.rs` に定義されている:

```rust
/// ソースコード上のバイトオフセット範囲
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 開始バイトオフセット (含む)
    pub start: usize,
    /// 終了バイトオフセット (含まない)
    pub end: usize,
}
```

`Span` は半開区間 `[start, end)` でソースコード上の範囲を表現する。バイトオフセットを使用しているため、UTF-8 文字列に対しても正確な位置を保持できる。`Clone` と `Copy` を derive しているのは、`Span` が全ての AST ノードに付与されるため、コピーコストが低い値型であることが重要だからである。

### 主要メソッド

```rust
impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// 2つの Span を結合して、両方を含む最小の Span を返す
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// ダミーの Span (テスト用)
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}
```

#### `merge` メソッド

`merge` は2つの `Span` を包含する最小範囲を返す。これは、複合式のスパンを構成する際に使われる。たとえば `(+ 1 2)` という式のスパンは、開き括弧の位置と閉じ括弧の位置を `merge` して求められる。パーサーが式を再帰的に構築する際、子ノードのスパンを順次 `merge` していくことで、正確な範囲情報が得られる:

```rust
// パーサーでの使用例 (概念的)
let lhs_span = lhs.span();
let rhs_span = rhs.span();
let full_span = lhs_span.merge(rhs_span);
```

#### `dummy` メソッド

`Span::dummy()` は `start: 0, end: 0` のスパンを返す。これはユニットテストで AST ノードを手動構築する際に使用される。ソースコードを持たないテストでは実際のバイトオフセットが不要なため、ダミー値で代用する。本番コードで `dummy()` が使われることはなく、テスト専用のユーティリティである。

#### Display 実装

```rust
impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}
```

`Display` 実装は `42..50` のようなフォーマットでスパン情報を出力する。エラーメッセージ中で `(42..50)` のように括弧付きで表示されるのは、`TypeError` 等の `#[error]` アトリビュート内で `{span}` として埋め込まれるためである。

### Span とソース位置の関係

`Span` はバイトオフセットのみを保持し、行番号や列番号は保持しない。行番号・列番号への変換はエラー表示時に miette クレートが行う。この設計には以下の利点がある:

- **メモリ効率**: `Span` は `usize` 2つ = 16 バイトで済む (行番号・列番号を持つと 32 バイト)
- **計算コスト**: 行番号の計算はエラー表示時 (= まれ) にのみ必要
- **不変性**: ソースコードの改行位置に依存せず、バイトオフセットだけで範囲を特定できる

## 統一エラー型 LsharpError

L# のコンパイラパイプラインは複数のクレートで構成されており、各フェーズが独自のエラー型を定義している。これらを統一的に扱うのが `crates/lsharp-driver/src/error.rs` の `LsharpError` 列挙型である:

```rust
use thiserror::Error;

/// コンパイラパイプライン全体の統一エラー型
#[derive(Debug, Error)]
pub enum LsharpError {
    /// 字句解析エラー
    #[error(transparent)]
    Lex(#[from] lsharp_syntax::lexer::LexError),

    /// 構文解析エラー
    #[error(transparent)]
    Parse(#[from] lsharp_syntax::parser::ParseError),

    /// 型推論エラー
    #[error(transparent)]
    Type(#[from] lsharp_types::infer::TypeError),

    /// 制約エラー
    #[error(transparent)]
    Constraint(#[from] lsharp_types::constraints::ConstraintError),

    /// IR 変換エラー
    #[error(transparent)]
    Lower(#[from] lsharp_ir::lower::LowerError),

    /// コード生成エラー
    #[error(transparent)]
    Codegen(#[from] lsharp_wasm::codegen::CodegenError),

    /// モジュールグラフエラー
    #[error(transparent)]
    ModuleGraph(#[from] lsharp_ir::module_graph::ModuleGraphError),
}
```

### thiserror による自動変換

`thiserror` クレートの `#[from]` アトリビュートにより、各バリアントに対して `From` トレイトの実装が自動生成される。これにより、パイプライン中で `?` 演算子を使ったエラー伝播が自然に書ける:

```rust
// パイプライン全体の処理 (概念的)
fn compile(source: &str) -> Result<Vec<u8>, LsharpError> {
    let tokens = lex(source)?;          // LexError -> LsharpError::Lex
    let ast = parse(tokens)?;           // ParseError -> LsharpError::Parse
    let typed = infer(&ast)?;           // TypeError -> LsharpError::Type
    let ir = lower(&typed)?;            // LowerError -> LsharpError::Lower
    let wasm = codegen(&ir)?;           // CodegenError -> LsharpError::Codegen
    Ok(wasm)
}
```

`?` 演算子が各エラー型を自動的に `LsharpError` に変換するため、呼び出し元はエラーの発生フェーズを意識せずにパイプラインを構成できる。

### `#[error(transparent)]` の意味

`#[error(transparent)]` は、`LsharpError` の `Display` 実装を内側のエラー型に委譲することを意味する。つまり `LsharpError::Type(err)` を表示すると、`TypeError` 自身の `Display` が使われる。ラッパー型が独自のメッセージを追加しないため、ユーザーには元のエラーメッセージがそのまま表示される。

### 7つのバリアントとパイプライン対応

各バリアントがパイプラインのどの段階に対応するかを整理する:

| バリアント | 発生源 | 発生タイミング |
|-----------|--------|-------------|
| `Lex` | `lsharp_syntax::lexer::LexError` | トークン分割時 |
| `Parse` | `lsharp_syntax::parser::ParseError` | AST 構築時 |
| `Type` | `lsharp_types::infer::TypeError` | 型推論・型チェック時 |
| `Constraint` | `lsharp_types::constraints::ConstraintError` | 値制約検証時 |
| `Lower` | `lsharp_ir::lower::LowerError` | AST -> IR 変換時 |
| `Codegen` | `lsharp_wasm::codegen::CodegenError` | IR -> Wasm 生成時 |
| `ModuleGraph` | `lsharp_ir::module_graph::ModuleGraphError` | モジュール依存解決時 |

パイプラインの前段でエラーが発生した場合、後段の処理は実行されない。字句解析エラーがあればパースは試みられず、型エラーがあれば IR 変換は行われない。この「早期脱出」戦略は実装が単純で、エラーの連鎖 (一つのエラーが後続の偽エラーを大量に生む現象) を防ぐ利点がある。

## エラーの種類

### 字句解析エラー (LexError)

最も低レベルなエラー。`crates/lsharp-syntax/src/lexer.rs` に定義されている:

```rust
pub enum LexError {
    #[error("予期しない文字 '{ch}' ({span})")]
    UnexpectedChar { ch: char, span: Span },

    #[error("閉じられていない文字列リテラル ({span})")]
    UnterminatedString { span: Span },

    #[error("不正な数値リテラル '{text}' ({span})")]
    InvalidNumber { text: String, span: Span },
}
```

3つのバリアントは、字句解析で発生し得るすべてのエラーパターンを網羅している:

```
エラー: 予期しない文字 '#'
  --> example.ls:3:5
   |
 3 |     #invalid
   |     ^ 認識できない文字です
```

### 構文解析エラー (ParseError)

文法に違反する構造。`crates/lsharp-syntax/src/parser.rs` に定義されている:

```rust
pub enum ParseError {
    #[error("予期しないトークン: {found} (期待: {expected}) ({span})")]
    Unexpected { expected: String, found: String, span: Span },

    #[error("予期しない入力終端 (期待: {expected})")]
    UnexpectedEof { expected: String },

    #[error("不明なフォーム: {name} ({span})")]
    UnknownForm { name: String, span: Span },

    #[error("複数のパースエラー: ...")]
    Multiple(Vec<ParseError>),
}
```

注目すべきは `Multiple` バリアントの存在である。パーサーはエラー発生後に次のトップレベル宣言まで回復して解析を続行する機能を持っており、複数のエラーを蓄積して一括報告できる。これは L# パーサーの重要な設計上の特徴であり、後述の「エラーリカバリ」の部分的な実現でもある。

```
エラー: 予期しないトークン: ) (期待: 式)
  --> example.ls:2:15
   |
 2 |   (defn add [])
   |               ^ 関数本体の式が必要です
```

### 型エラー (TypeError)

最も多くの情報を伝えるエラー。次節で詳細に解説する。

```
エラー: 型の不一致: Int と Bool
  --> example.ls:1:20
   |
 1 | (defn bad [] (+ 1 true))
   |                    ^^^^ Bool 型の値
   |
   = ノート: + は (Int, Int) -> Int ですが、
     第2引数に Bool 型の値が渡されました
```

## TypeError の12バリアント

型エラーは L# コンパイラが発する最も多様なエラーであり、12のバリアントを持つ。`crates/lsharp-types/src/infer.rs` に定義されている。各バリアントは `thiserror` の `#[error]` アトリビュートで日本語のエラーメッセージを持ち、全てに `Span` が付与されている。

### 1. Mismatch -- 型の不一致

```rust
#[error("型の不一致: {expected} と {found} ({span})")]
Mismatch {
    expected: Type,
    found: Type,
    span: Span,
}
```

最も頻出する型エラー。Unification (単一化) の過程で、2つの型が一致しないときに発生する。`expected` は文脈から期待される型、`found` は実際に推論された型を保持する。たとえば `(+ 1 true)` では `expected: Int, found: Bool` となる。

### 2. InfiniteType -- 無限型

```rust
#[error("無限型: t{var} は {ty} に出現します ({span})")]
InfiniteType {
    var: TypeVarId,
    ty: Type,
    span: Span,
}
```

Occurs check の失敗。型変数 `t0` を含む型 `(-> t0 t0)` に `t0` を代入しようとすると、`t0 = (-> t0 t0) = (-> (-> t0 t0) (-> t0 t0)) = ...` と無限に展開されてしまう。`(defn f [x] (x x))` のような自己適用で発生する。

### 3. UndefinedVar -- 未定義変数

```rust
#[error("未定義の変数: {name} ({span})")]
UndefinedVar { name: String, span: Span },
```

型環境に存在しない変数を参照した場合。スペルミスや、スコープ外の変数へのアクセスで発生する。

### 4. UndefinedConstructor -- 未定義コンストラクタ

```rust
#[error("未定義のコンストラクタ: {name} ({span})")]
UndefinedConstructor { name: String, span: Span },
```

パターンマッチや式中で、定義されていない ADT コンストラクタを使用した場合に発生する。

### 5. ArityMismatch -- 引数数不一致

```rust
#[error("引数の数が不一致: 期待 {expected}, 実際 {found} ({span})")]
ArityMismatch {
    expected: usize,
    found: usize,
    span: Span,
}
```

関数やコンストラクタに渡す引数の数が合わない場合。`(Some 1 2)` のように、1引数のコンストラクタに2つの引数を渡すと `expected: 1, found: 2` のエラーとなる。

### 6. UndefinedRecord -- 未定義レコード型

```rust
#[error("未定義のレコード型: {name} ({span})")]
UndefinedRecord { name: String, span: Span },
```

レコード構築や分解で、定義されていないレコード型名を使用した場合に発生する。

### 7. UndefinedField -- 未定義フィールド

```rust
#[error("未定義のフィールド: {record_name}.{field_name} ({span})")]
UndefinedField {
    record_name: String,
    field_name: String,
    span: Span,
}
```

レコード型のフィールドアクセスで、存在しないフィールド名を指定した場合。`record_name` と `field_name` の両方を保持することで、「どのレコード型のどのフィールドが問題か」を明確に伝える。

### 8. RecursiveAlias -- 再帰型エイリアス

```rust
#[error("再帰的な型エイリアス: {name} ({span})")]
RecursiveAlias { name: String, span: Span },
```

型エイリアスが自分自身を参照している場合。`(type-alias A A)` のような直接再帰や、`(type-alias A B)` `(type-alias B A)` のような間接再帰で発生する。型エイリアスの展開が無限ループに陥ることを防ぐ。

### 9. UndefinedAlias -- 未定義型エイリアス

```rust
#[error("未定義の型エイリアス: {name} ({span})")]
UndefinedAlias { name: String, span: Span },
```

型注釈で使用された型エイリアス名が定義されていない場合に発生する。

### 10. UndefinedTrait -- 未定義トレイト

```rust
#[error("未定義のトレイト: {name} ({span})")]
UndefinedTrait { name: String, span: Span },
```

`:where` 句や `impl` ブロックで参照されたトレイト名が存在しない場合。

### 11. MissingImpl -- トレイト実装の欠如

```rust
#[error("トレイト {trait_name} の実装が見つかりません: {type_name} ({span})")]
MissingImpl {
    trait_name: String,
    type_name: String,
    span: Span,
}
```

トレイト制約 (`:where [(Show a)]`) を持つ関数に、その制約を満たさない型の値を渡した場合。`trait_name` と `type_name` を保持することで、「何のトレイトが足りないのか」を明確にする。

### 12. MismatchWithAlias -- エイリアス展開を含む型不一致

```rust
#[error("型の不一致: {expected} と {found} (エイリアス '{alias_name}' は {expanded} に展開) ({span})")]
MismatchWithAlias {
    expected: Type,
    found: Type,
    alias_name: String,
    expanded: Type,
    span: Span,
}
```

型エイリアスが関与する型不一致。通常の `Mismatch` と異なり、エイリアス名とその展開結果も含む。ユーザーがエイリアス名で型を書いている場合、展開後の型を表示することで「なぜ不一致なのか」を理解しやすくなる。

### 13. KindMismatch -- Kind の不一致

```rust
#[error("Kind の不一致: {type_name} は {actual_kind} ですが、トレイト {trait_name} は {expected_kind} を要求します ({span})")]
KindMismatch {
    type_name: String,
    trait_name: String,
    expected_kind: Kind,
    actual_kind: Kind,
    span: Span,
}
```

HKT (高カインド型) に関連するエラー。`(impl (Functor Int) ...)` のように、`* -> *` カインドが期待される位置に `*` カインドの型を渡した場合に発生する。Kind の概念については第11章を参照されたい。

## miette クレートによるリッチ表示

L# は **miette** クレートを使ってリッチなエラー表示を行う。miette はソースコードの該当箇所をハイライトし、矢印で問題のある位置を指し示す。

miette の主な特徴:

- **ソーススパンのハイライト**: `Span` の範囲をアンダーラインで表示
- **複数ラベル**: 一つのエラーに複数のソース箇所を紐付け可能
- **関連情報**: `help:` や `note:` で補足情報を追加
- **テーマ**: ターミナルの色に応じて表示を調整

`Span` のバイトオフセットを miette の `SourceSpan` に変換するだけで、行番号・列番号の計算、ソース行の抽出、矢印の配置を自動的に行ってくれる。

## エラーメッセージ設計原則

L# のエラーメッセージ設計は、Rust コンパイラと Elm コンパイラの設計哲学に影響を受けている。

### Elm の「友好的コンパイラ」アプローチ

Elm コンパイラは「コンパイラエラーは対話である」という哲学を持つ。エラーメッセージは常に:

1. 何が起きたかを平易な言葉で説明する
2. 具体的な修正提案を示す
3. 関連するドキュメントへのリンクを提供する

L# はこの思想を取り入れ、エラーメッセージを日本語で記述している。`#[error("型の不一致: {expected} と {found}")]` のように、専門用語を使いつつも文脈がわかる構成にしている。

### Rust コンパイラのエラー構造

Rust コンパイラのエラーは以下の構造を持つ:

```
error[E0308]: mismatched types
 --> src/main.rs:2:14
  |
2 |     let x: i32 = "hello";
  |            ---   ^^^^^^^ expected `i32`, found `&str`
  |            |
  |            expected due to this
```

L# は同様の構造を miette で実現しつつ、以下の点を重視している:

| 設計原則 | L# での実現 |
|---------|-----------|
| エラーの位置を正確に示す | 全バリアントに `Span` を付与 |
| 期待された型と実際の型を両方表示 | `Mismatch { expected, found }` |
| 型エイリアスの展開を見せる | `MismatchWithAlias` バリアント |
| Kind エラーで型名とトレイト名を表示 | `KindMismatch` バリアント |
| フィールドエラーでレコード名を含める | `UndefinedField { record_name, field_name }` |

### L# 固有の工夫

L# のエラー設計で特筆すべき点は、**型エイリアス展開の可視化**である。多くのコンパイラは型エイリアスを展開した後の型のみを表示するため、ユーザーは「なぜ自分の書いた型名と違う型が出てくるのか」に困惑する。L# の `MismatchWithAlias` は、エイリアス名と展開結果の両方を表示することで、この問題を解消している:

```
エラー: 型の不一致: UserId と String
  (エイリアス 'UserId' は Int に展開)
  --> example.ls:5:10
```

## エラーリカバリ

### 現状: パーサーレベルのリカバリ

L# のパーサーは既に部分的なエラーリカバリを実装している。`ParseError::Multiple` バリアントが示すように、パーサーはエラー発生後に次のトップレベル宣言まで回復して解析を続行する。これにより、1回のコンパイルで複数の構文エラーを一括報告できる。

### 将来の展望

#### 1. 型推論のリカバリ

型推論でエラーが発生した式に `Error` 型 (ポイズン型) を割り当てて推論を続行する手法。GCC の `error_mark_node` や TypeScript の `any` 型に相当する:

```rust
// 将来の実装イメージ
enum Type {
    Con(String),
    Var(TypeVarId),
    Fun(Box<Type>, Box<Type>),
    // ...
    Error,  // ポイズン型: エラーが発生した式に割り当て
}
```

`Error` 型は全ての型と単一化可能であるため、一つのエラーが連鎖して偽エラーを生むことを防ぐ。ただし、`Error` 型の伝播範囲を適切に制限しないと、本来報告すべきエラーまで隠蔽してしまう問題がある。

#### 2. エラーに対する修正提案

未定義変数エラーに対して、編集距離 (Levenshtein 距離) に基づく候補提示を行う:

```
エラー: 未定義の変数: pritn (5..10)
  |
  = ヒント: 'print' のことですか?
```

この機能は型環境に登録された全変数名との距離を計算し、閾値以下の候補をリストアップする。Rust コンパイラの `did you mean` 機能と同等のものである。

#### 3. LSP 統合

LSP (Language Server Protocol) サーバーとの統合により、エディタ上でリアルタイムにエラーを表示する。L# は既に `lsharp-lsp` クレートで LSP サーバーの基盤を持っている。`Span` ベースのエラー情報は LSP の `Diagnostic` に直接マッピング可能であり、`Span` のバイトオフセットを行番号・列番号に変換するだけでエディタ上にエラーを表示できる。

#### 4. エラーコード体系

将来的には、Rust の `E0308` のようなエラーコード体系を導入し、各エラーに固有の識別子を付与する。エラーコードは以下の用途に活用できる:

- **ドキュメント参照**: `--explain E0001` でエラーの詳細な説明と対処法を表示
- **フィルタリング**: 特定のエラーを警告に降格、または抑制
- **統計**: プロジェクト内で頻出するエラーパターンの分析

## まとめ

エラー報告は「正しいプログラムをコンパイルする」ことと同等に重要な機能である。L# は `Span` による位置追跡、`LsharpError` による統一的なエラー管理、`TypeError` の12バリアントによる詳細な型エラー報告を通じて、開発者にとって親切なコンパイラを目指している。miette による視覚的なエラー表示と、将来のエラーリカバリ・修正提案機能により、エラーメッセージの品質を継続的に向上させていく。

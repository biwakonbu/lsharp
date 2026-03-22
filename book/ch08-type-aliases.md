# 型エイリアスと制約付き型 -- 型に意味と制約を付ける

## 型エイリアスとは

型エイリアスは既存の型に別名を与える機能である。新しい型を定義するのではなく、既存の型への「透過的な」参照を作る。

```lisp
;; 単純な型エイリアス
(type-alias Name String)

;; パラメトリック型エイリアス
(type-alias (Map k v) (HashMap k v))

;; 関数型のエイリアス
(type-alias (Predicate a) (-> a Bool))

;; レコードの短縮名
(type-alias Vec2 Point)
```

## 設計判断

### type-alias を type と分ける理由

`type` は既に ADT 定義に使われている。エイリアスと ADT の構文が混在すると、パーサーが「これはバリアント定義か、それともエイリアスのターゲット型か」を判別しにくくなる。

```lisp
(type Name String)        ;; ADT? エイリアス? 曖昧
(type-alias Name String)  ;; 明確にエイリアス
```

別キーワードにすることで、パーサーが先頭トークンだけで判別でき、LL(1) の性質を維持できる。

### 透過的 (transparent) セマンティクス

型エイリアスは型推論時に**完全に展開**される。つまり `Name` と `String` は完全に同じ型として扱われる:

```lisp
(type-alias Name String)

(defn greet [(: name Name)] : String
  name)  ;; OK: Name = String なので型が一致する
```

### 非再帰制約

エイリアスの再帰的定義は禁止する:

```lisp
;; エラー: 再帰的エイリアス
(type-alias Recursive (Option Recursive))
```

再帰的なデータ構造が必要な場合は ADT を使う。

## エラーメッセージでの扱い

型エイリアスは推論時に展開されるが、エラーメッセージではエイリアス名を保持して表示する。ユーザーが `Name` と書いたのに、エラーで `String` と表示されると混乱を招くためである。

```
型エラー: Name 型が必要ですが、Int 型が見つかりました
    (Name は String のエイリアスです)
```

## 実装の影響範囲 (型エイリアス)

型エイリアスは型推論段階で完全に展開されるため、IR やコード生成には影響がない:

| レイヤー | 変更内容 |
|----------|----------|
| Lexer | `TypeAlias` キーワード追加 |
| Parser | `type-alias` 宣言のパース |
| AST | `Decl::TypeAlias { name, params, target }` |
| 型推論 | エイリアス登録と展開 |
| IR/Codegen | 変更なし |

---

## 制約付き型 (Constrained Types)

型エイリアスが「型に名前を付ける」だけの機能だったのに対し、**制約付き型**は「型に値の制約を宣言する」機能である。L# の制約付き型はユニークな設計を持つ -- ユーザーは制約を宣言するだけで、コンパイラが自動的にテストを生成・実行する。

### 構文

```lisp
;; 数値の範囲制約
(type-constrained Age Int
  :constraints [(>= 0) (<= 150)])

;; 文字列パターン制約
(type-constrained Email String
  :constraints [(min-length 3)
                (max-length 254)
                (matches "^[^@]+@[^@]+\\.[^@]+$")])

;; 列挙制約
(type-constrained Color String
  :constraints [(one-of "red" "green" "blue")])

;; 任意の述語制約
(type-constrained EvenNumber Int
  :constraints [(satisfies even?)])
```

### 組み込み制約述語

| 述語 | 対象 | 例 |
|------|------|-----|
| `>=`, `<=`, `>`, `<` | 数値 | `(>= 0)` |
| `range` | 数値 | `(range 0 150)` |
| `matches` | 文字列 | `(matches "^[a-z]+$")` |
| `min-length`, `max-length` | 文字列 | `(min-length 1)` |
| `one-of` | 任意 | `(one-of "red" "green" "blue")` |
| `satisfies` | 任意 | `(satisfies even?)` |

### コンパイラ自動テスト生成

ユーザーはテストを書かない。コンパイラが制約から自動的にテストを生成・実行する:

```
制約の種類           コンパイラの自動検証
──────────────────────────────────────────────────
(>= N), (<= N)      境界値 N, N-1, N+1 + ランダム値
(matches regex)      正規表現から合致/非合致文字列を自動生成
(min-length N)       長さ N, N-1 の文字列生成
(one-of ...)         全列挙 + 範囲外値
(satisfies f)        ランダム入力で反例探索 (QuickCheck 方式)
```

### 値の構築と検証

```lisp
;; リテラル -- コンパイル時検証
(let [age : Age 25])            ;; OK
(let [age : Age -1])            ;; コンパイルエラー: >= 0 に違反
(let [email : Email "a@b.com"]) ;; OK
(let [email : Email "invalid"]) ;; コンパイルエラー: パターン不一致

;; 変数 -- ランタイム検証 (スマートコンストラクタ経由)
(Age.new user-input)            ;; => (Result Age ConstraintError)
(Email.new raw-string)          ;; => (Result Email ConstraintError)

;; 内部値へのアクセス
(Age.value age)                 ;; => Int
(Email.value email)             ;; => String
```

コンパイラが自動生成するもの:

- `Age.new : (-> Int (Result Age ConstraintError))` -- スマートコンストラクタ
- `Age.value : (-> Age Int)` -- 内部値アクセサ
- `Age.valid? : (-> Int Bool)` -- バリデーション関数

### 制約の階層関係

制約付き型は別の制約付き型を基底にできる。暗黙の変換は禁止される:

```lisp
;; Age: 0..150 の整数
(type-constrained Age Int
  :constraints [(>= 0) (<= 150)])

;; AdultAge: Age の部分状態 (18..150)
(type-constrained AdultAge Age
  :constraints [(>= 18)])
;; Age の制約 (0..150) を自動継承 + (>= 18) を追加
;; 結果の有効範囲: 18..150
```

コンパイラが自動生成する型変換関数:

```lisp
;; 緩い方向 (AdultAge -> Age): 常に安全
(Age.from-adult a)              ;; => Age (常に成功)

;; 厳しい方向 (Age -> AdultAge): 失敗の可能性
(AdultAge.from-age a)           ;; => (Result AdultAge ConstraintError)
```

### コンパイル設定: `lsharp.toml`

```toml
[constraints]
# 自動テストのランダム入力生成回数 (デフォルト: 100)
random-test-count = 100

# satisfies のランダム反例探索回数 (デフォルト: 1000)
satisfies-search-count = 1000
```

### コンパイル時の流れ

```
[1] 制約 DSL を解析
      |
[2] 制約の階層関係を解決 (基底型の制約を継承)
      |
[3] 制約の種類を判定
      |
      +-- 組み込み述語 (>=, matches, etc.)
      |     -> 自動テスト生成 (境界値 + ランダム)
      |
      +-- satisfies (任意関数)
            -> 反例探索 (ランダム入力で矛盾を検索)
      |
[4] 自動テスト実行 (コンパイルの一部)
      |   失敗 -> コンパイルエラー + 反例を表示
      v
[5] 通過 -> リテラル/定数式に対する制約チェック
      |
[6] スマートコンストラクタ + 型変換関数の自動生成
```

### AST 表現

制約付き型は `Decl::TypeConstrained` と `Constraint` enum で表現される:

```rust
pub enum Decl {
    TypeConstrained {
        span: Span,
        name: String,
        base_type: TypeExpr,
        constraints: Vec<Constraint>,
    },
    // ...
}

pub enum Constraint {
    Gte(Expr),           // (>= N)
    Lte(Expr),           // (<= N)
    Range(Expr, Expr),   // (range N M)
    Matches(String),     // (matches "regex")
    MinLength(Expr),     // (min-length N)
    MaxLength(Expr),     // (max-length N)
    OneOf(Vec<Expr>),    // (one-of v1 v2 ...)
    Satisfies(String),   // (satisfies fn-name)
}
```

## 実装の影響範囲 (制約付き型)

| レイヤー | 変更内容 |
|----------|----------|
| Lexer | `TypeConstrained`, `Constraints` キーワード追加 |
| Parser | `type-constrained` 宣言と各制約述語のパース |
| AST | `Decl::TypeConstrained`, `Constraint` enum |
| 型推論 | 制約検証、リテラルの制約チェック |
| 新規 | `crates/lsharp-types/src/constraints.rs` -- 制約評価エンジン |
| IR | スマートコンストラクタのランタイム検証コード生成 |

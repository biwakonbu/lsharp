# L# 型システム機能ロードマップ

## 概要

L# は S 式構文の関数型言語で、WebAssembly にコンパイルする。
本ドキュメントでは型システムの全体設計と段階的な実装計画を定義する。

### 設計方針

- **ベース**: F# の型システムを参考に、S 式構文に最適化した独自設計
- **重視**: 表現力 -- 型で表現できる範囲を最大化する
- **ターゲット**: WasmGC を活用したリッチなランタイム表現
- **型推論**: Hindley-Milner (Algorithm W) をベースに段階的に拡張
- **AI 協調**: コンパイラ・テスト・git が協調し、AI が正しく実装を進められるコードベースを言語レベルでサポート

### 現状 (実装済み)

| 機能 | 状態 | 構文例 |
|------|------|--------|
| プリミティブ型 | 実装済み | `Int`, `Float`, `String`, `Bool`, `Unit` |
| ADT (代数的データ型) | 実装済み | `(type (Option a) (Some a) None)` |
| 関数型 | 実装済み | `(-> Int Int Bool)` |
| HM 型推論 | 実装済み | Let 多相、汎化、統合 |
| 型注釈 | 実装済み | `(: x Int)`, `(defn f [] : Int ...)` |
| パターンマッチ | 実装済み | `(match x [(Some v) v] [None 0])` |

---

## Phase 0: プロジェクト基盤 (Project Foundation)

### git リポジトリの必須化

L# コンパイラはプロジェクトが git リポジトリであることを前提とする。
ドキュメント鮮度追跡・知識ベース・変更検知の全てが git に依存するため。

```
$ lsharp build
error[PROJ001]: git リポジトリが見つかりません。

  L# はドキュメント追跡と知識管理に git を使用します。
  以下のコマンドでリポジトリを初期化してください:

    git init
    git add .
    git commit -m "Initial commit"
```

`lsharp init` コマンドで git 初期化も含めたプロジェクトセットアップを提供する:

```
$ lsharp init my-project
  [1/4] ディレクトリ作成: my-project/
  [2/4] git リポジトリ初期化: git init
  [3/4] プロジェクト設定: lsharp.toml
  [4/4] 初期コミット: git commit -m "lsharp init"
```

---

## Phase 1: レコード型 (Record Types)

### 設計判断

**公称型 (Nominal Typing)** を採用する。

理由:
1. WasmGC の `struct` 型が公称であり、直接マッピング可能
2. ADT と一貫した名前空間管理ができる
3. エラーメッセージが明確になる
4. 将来のモジュール・トレイトシステムとの親和性が高い

### 構文

#### レコード型定義

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

#### レコード構築

```lisp
;; 中括弧リテラルで構築
{Point x 1.0 y 2.0}

;; ネストしたレコード
{Line start {Point x 0.0 y 0.0} end {Point x 1.0 y 1.0}}
```

中括弧 `{}` はレクサーに `LBrace`/`RBrace` トークンとして既に定義済み。

#### フィールドアクセス

```lisp
;; 型修飾付きフィールドアクセス
(Point.x point)          ;; => Float
(Pair.first pair)        ;; => a

;; ネストしたアクセス
(Point.x (Line.start line))  ;; => Float

;; 同名フィールドでも曖昧さなし
(Point.x p)    ;; Point の x
(Color.x c)    ;; Color の x
```

`TypeName.field` はシンボルとして字句解析し、型修飾付きアクセサ関数として型環境に登録する。
例: `Point.x` は `(-> Point Float)` 型のアクセサ関数。
型が常に明示されるため、同名フィールドを持つ複数のレコード型が存在しても曖昧さが生じない。

#### レコード更新 (Functional Update)

```lisp
;; 一部のフィールドだけ変更した新しいレコードを生成
{point | x 3.0}

;; 複数フィールドの更新
{pair | first 10 second 20}
```

#### パターンマッチ

```lisp
(match p
  [{Point x y} (+ x y)])

;; 部分パターン (一部のフィールドのみマッチ)
(match p
  [{Point x _} x])
```

### WasmGC 表現

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

;; {point | x 3.0} -- 全フィールドをコピーして x だけ差し替え
local.get $point
struct.get $Point $y    ;; y を取得
f64.const 3.0           ;; 新しい x
                        ;; スタック: [new_y, new_x]
struct.new $Point       ;; 新しい Point を構築
```

### ADT の WasmGC 化 (同時整備)

現在 ADT は全て i64 にフォールバックしている。レコードで WasmGC 基盤を整備する際に ADT も同時に WasmGC 化する。

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

;; パターンマッチ: $tag フィールドで分岐 + ref.cast でダウンキャスト
```

### 実装対象

| ファイル | 変更内容 |
|----------|----------|
| `crates/lsharp-syntax/src/token.rs` | `Record` トークン追加 |
| `crates/lsharp-syntax/src/ast.rs` | `TypeExpr::Record`, `Expr::RecordLit`, `Expr::FieldAccess(type_name, field)`, `Expr::RecordUpdate`, `Pattern::RecordPat` |
| `crates/lsharp-syntax/src/parser.rs` | レコード構文のパース |
| `crates/lsharp-types/src/types.rs` | `RecordInfo` 定義、型レジストリ拡張 |
| `crates/lsharp-types/src/infer.rs` | レコードリテラル・フィールドアクセス・更新の型推論 |
| `crates/lsharp-ir/src/lib.rs` | `IrType::Ref(u32)`, `StructNew`, `StructGet`, `StructSet` 命令 |
| `crates/lsharp-ir/src/lower.rs` | レコードの IR 降位 |
| `crates/lsharp-wasm/src/codegen.rs` | WasmGC struct 型定義と GC 命令出力 |

---

## Phase 2: 型エイリアスと制約付き型 (Type Aliases & Constrained Types)

### 2a. 型エイリアス

#### 構文

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

`type-alias` を `type` とは別キーワードにする。
`type` は既に ADT 定義に使われており、エイリアスと ADT の曖昧性を避けるため。

#### セマンティクス

- **透過的 (transparent)**: 型推論時に完全に展開される
- **非再帰**: 再帰的エイリアスは禁止。再帰型は ADT で表現する
- **エラーメッセージ**: エイリアス名を保持して表示し、ユーザーの意図を尊重する
- **IR/codegen への影響なし**: 型推論段階で完全展開されるため

### 2b. 制約付き型 (Constrained Types)

#### 設計判断

型に値の制約を宣言し、コンパイラが自動的にテストを生成・実行する。
テスト実装者の質に依存しない -- ユーザーは制約を宣言するだけ。

#### 構文

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

#### 組み込み制約述語

| 述語 | 対象 | 例 |
|------|------|-----|
| `>=`, `<=`, `>`, `<` | 数値 | `(>= 0)` |
| `range` | 数値 | `(range 0 150)` |
| `matches` | 文字列 | `(matches "^[a-z]+$")` |
| `min-length`, `max-length` | 文字列 | `(min-length 1)` |
| `one-of` | 任意 | `(one-of "red" "green" "blue")` |
| `satisfies` | 任意 | `(satisfies even?)` |

#### コンパイラ自動テスト生成

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

#### 値の構築と検証

```lisp
;; リテラル -- コンパイル時検証
(let [age : Age 25])            ;; OK
(let [age : Age -1])            ;; コンパイルエラー: >= 0 に違反
(let [email : Email "a@b.com"]) ;; OK
(let [email : Email "invalid"]) ;; コンパイルエラー: パターン不一致

;; 定数式 -- コンパイル時検証
(let [age : Age (+ 10 20)])     ;; OK (30 に畳み込み可能)

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

#### 制約の階層関係 (Constraint Hierarchy)

制約付き型は別の制約付き型を基底にできる。暗黙の変換は禁止。
基底型を指定することで、型の包含関係が明示される。

```lisp
;; Age: 0..150 の整数
(type-constrained Age Int
  :constraints [(>= 0) (<= 150)])

;; AdultAge: Age の部分状態 (18..150)
;; 基底が Int ではなく Age -> 関係性が型で明示される
(type-constrained AdultAge Age
  :constraints [(>= 18)])
;; Age の制約 (0..150) を自動継承 + (>= 18) を追加
;; 結果の有効範囲: 18..150
```

型変換のルール:

```lisp
;; 緩い方向 (AdultAge -> Age): 制約が弱まる -> 常に安全
;; コンパイラが包含関係を判定し、Result 不要の変換を自動生成
(Age.from-adult a)              ;; => Age (常に成功)

;; 厳しい方向 (Age -> AdultAge): 制約が強まる -> 失敗の可能性
(AdultAge.from-age a)           ;; => (Result AdultAge ConstraintError)

;; 暗黙の変換は禁止 -- 常に明示的な変換が必要
(defn greet [(: age Age)] : String ...)
(let [a : AdultAge 25])
(greet a)                       ;; コンパイルエラー: Age と AdultAge は異なる型
(greet (Age.from-adult a))      ;; OK: 明示的に変換
```

コンパイラが自動生成する変換関数:

```
制約の包含関係             生成される変換関数
────────────────────────────────────────────────────
AdultAge の範囲 ⊆ Age の範囲
  -> Age.from-adult : (-> AdultAge Age)          -- 常に成功
  -> AdultAge.from-age : (-> Age (Result ...))   -- 失敗の可能性
```

#### コンパイル設定: `lsharp.toml`

```toml
[constraints]
# 自動テストのランダム入力生成回数 (デフォルト: 100)
random-test-count = 100

# satisfies のランダム反例探索回数 (デフォルト: 1000)
satisfies-search-count = 1000
```

#### コンパイル時の流れ

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
      |   (インクリメンタル: 制約が変わっていなければスキップ)
      v
[5] 通過 -> 制約の正しさをコンパイラが信頼
      |
[6] リテラル/定数式に対する制約チェック
      |
[7] スマートコンストラクタ + 型変換関数の自動生成
```

### 実装対象

| ファイル | 変更内容 |
|----------|----------|
| `crates/lsharp-syntax/src/token.rs` | `TypeAlias`, `TypeConstrained`, `Constraints` キーワード |
| `crates/lsharp-syntax/src/ast.rs` | `Decl::TypeAlias`, `Decl::TypeConstrained { constraints }` |
| `crates/lsharp-syntax/src/parser.rs` | `type-alias`, `type-constrained` のパース |
| `crates/lsharp-types/src/infer.rs` | エイリアス展開、制約検証、リテラルの制約チェック |
| 新規: `crates/lsharp-types/src/constraints.rs` | 制約 DSL 評価器、自動テスト生成エンジン |
| `crates/lsharp-ir/src/lower.rs` | スマートコンストラクタのランタイム検証コード生成 |

---

## Phase 3: 構造化メタデータとドキュメント追跡 (Structured Metadata & Doc Tracking)

### 設計思想

コード・ドキュメント・テスト・仕様の全てが連動した状態をコンパイラが検出する。
コメントを含む全ての自然言語記述を追跡し、実装との乖離を防ぐ。

責務の分担:
- **コンパイラ**: 変更を検知する (機械的に検出可能なもの)
- **AI**: 自然言語の整合性を判断する (意味的な乖離)
- **git**: 「確認済み」を記録する (署名)

### 3a. 構造化メタデータ

#### 構文

```lisp
(defn calculate-tax
  [(: income Income) (: rate TaxRate)] : Money

  :doc "所得と税率から課税額を計算する"

  :params {income "課税対象の所得額"
           rate   "適用する税率"}

  :returns "課税額。常に 0 以上で income を超えない"

  :invariant [(>= result 0)
              (<= result (Income.value income))]

  :rationale "税率は年度ごとに変わるため引数で受け取る"

  :see-also [apply-deduction calculate-net-income]

  :example [(assert-eq (calculate-tax (Income.new 1000) (TaxRate.new 0.1))
                       (Money.new 100))]

  :since "0.2.0"

  ;; 実装
  (Money.new (* (Income.value income) (TaxRate.value rate))))
```

```lisp
;; 型定義にも
(type-constrained TaxRate Float
  :doc "税率 (0% - 100%)"
  :rationale "負の税率や 100% 超は法律上存在しない"
  :constraints [(>= 0.0) (<= 1.0)])
```

```lisp
;; ADT に状態遷移を記述
(type PaymentStatus
  :doc "決済の状態遷移"
  :rationale "Refunded は Completed からのみ遷移可能"
  :transitions {Pending    [Completed Failed]
                Completed  [Refunded]
                Failed     []
                Refunded   []}
  (Pending)
  (Completed)
  (Failed)
  (Refunded))
```

#### コンパイラによる機械的検証

| チェック | 対象 | 種類 |
|---------|------|------|
| `:params` のキーが引数リストと一致するか | 構造 | **エラー** |
| `:params` に全引数の説明があるか | 網羅性 | **警告** |
| `:see-also` の参照先が存在するか | 参照 | **エラー** |
| `:invariant` がテストで検証されるか | 意味 | **エラー** |
| `:example` が実行して通るか | 意味 | **エラー** |
| `:doc` 内の識別子が存在するか | 参照 | **警告** |

### 3b. ドキュメント鮮度追跡

#### コンパイラが追跡する依存関係

コンパイラはコメント (`;;`) を最も近い定義に紐付け、変更を追跡する:

```lisp
;; 注文の合計金額を計算する
;; 税込み価格にクーポン割引を適用した後の金額を返す
(defn calc-total [(: order Order) (: coupon (Option Coupon))] : Money
  ...)
```

内部的に構築される紐付け:

```
Comment["注文の合計金額を...クーポン割引を適用"]
  +-- attached_to: calc-total
  +-- comment_hash: "x1y2z3"
  +-- code_hash: "a1b2c3"    <- この関数の AST ハッシュ
```

#### 変更検知ルール

```
シグネチャ変更 (引数追加/削除/型変更):
  :doc 未更新            -> 警告
  :params 不一致         -> エラー
  コメント未更新          -> 警告

実装本体変更:
  :invariant が壊れる    -> エラー (自動テストで検証)
  :example が壊れる      -> エラー (自動実行で検証)
  コメント未更新          -> 警告 (AI レビューに委譲)

参照先の削除/リネーム:
  :see-also の参照切れ    -> エラー
  :doc 内の識別子が不在   -> 警告
```

#### 鮮度管理ファイル: `.lsharp-doc-status`

コンパイラがビルド時に毎回再生成するキャッシュファイル。
git で追跡しない (`.gitignore` に追加)。マージコンフリクトの心配が不要。

確認済みの状態は git commit のトレイラー (`Doc-Reviewed-By`, `Doc-Review-Status`) に
記録されており、コンパイラは `git log` から確認履歴を復元する。

```json
{
  "calc-total": {
    "sig_hash": "a1b2c3",
    "body_hash": "d4e5f6",
    "doc_hash": "g7h8i9",
    "last_reviewed_commit": "abc1234",
    "status": "stale",
    "comments": [
      {
        "lines": [12, 13],
        "hash": "x1y2z3",
        "status": "needs_review"
      }
    ]
  }
}
```

### 3c. AI エージェント向けチェックポイント

`lsharp review` コマンドは AI エージェント (Claude Code, Codex 等) が消費する
チェックポイント情報を出力する。エージェントが何を確認すべきかをコンパイラが指示する。

#### `lsharp review` コマンド

```
$ lsharp review

doc-review:
  - file: src/order.ls
    function: calc-total
    status: stale
    reason: signature_changed
    check_points:
      - "引数 coupon が削除されています。コメント (line 12-13) がクーポンに言及していないか確認してください"
      - ":doc の内容が現在のシグネチャと一致するか確認してください"
    diff: |
      - (defn calc-total [(: order Order) (: coupon (Option Coupon))] : Money
      + (defn calc-total [(: order Order)] : Money
    comments:
      - lines: [12, 13]
        content: |
          ;; 注文の合計金額を計算する
          ;; 税込み価格にクーポン割引を適用した後の金額を返す
```

AI エージェントはこの出力を読み、コメントやドキュメントの更新を判断・実行する。

#### git commit での署名

```
$ git commit

  pre-commit hook: lsharp doc-check
  未確認のドキュメントがあります:
    src/order.ls:12-13 (calc-total)

  以下のいずれかを実行してください:
    lsharp review          チェックポイントを確認
    lsharp doc-ack         手動で確認済みにする
    --skip-doc-review      今回はスキップ
```

コミットメッセージに確認済みのトレイラーが自動付与される:

```
refactor: calc-total からクーポン処理を分離

Doc-Reviewed-By: human
Doc-Review-Status: updated
```

### 3d. 知識ベース出力 (`--emit knowledge`)

コンパイラが AI 向けの機械可読な知識ベースを出力する:

```
$ lsharp --emit knowledge src/
```

出力例:

```json
{
  "types": {
    "Age": {
      "base": "Int",
      "constraints": [">= 0", "<= 150"],
      "rationale": "...",
      "used_by": ["User", "Registration"],
      "constructed_by": ["parse-age", "User.new"]
    }
  },
  "functions": {
    "calculate-tax": {
      "signature": "(Income, TaxRate) -> Money",
      "invariants": ["result >= 0", "result <= income"],
      "rationale": "...",
      "depends_on": ["TaxRate", "Income", "Money"],
      "called_by": ["process-payment"]
    }
  }
}
```

### 警告レベルの設定: `lsharp.toml`

```toml
[doc-review]
# 構造化メタデータ (:doc, :params, :invariant)
# コンパイラが機械的に検証できる -> 常にエラー
structured = "error"

# コメント (;;)
# AI レビュー + コミット署名で解消 -> 警告
comments = "warn"

# コミット時の強制レベル
# "block" = 未確認があればコミット不可
# "warn"  = 警告のみ
# "skip"  = チェックしない
pre-commit = "block"
```

### 2 種類の記述の区別

```lisp
;; これは普通のコメント
;; コンパイラが追跡する (変更検知 + AI レビュー)
;; ただし構造化メタデータほど厳密ではない

(defn foo [(: x Int)] : Int
  :doc "これは構造化メタデータ"
  :invariant [(> result 0)]
  ;; ↑ コンパイラとの契約。実装と乖離したらエラー
  ...)
```

| レイヤー | 検出方法 | 強制力 |
|---------|---------|--------|
| `:params`, `:invariant`, `:example` | コンパイラが機械的に検証 | **エラー** |
| `:doc`, `:rationale`, `:see-also` | AST ハッシュで変更検知 | **警告** -> AI レビューで解消 |
| `;;` コメント | 近接コードの変更検知 | **警告** -> AI レビュー + コミット署名 |
| git commit | pre-commit hook | **ブロック** (設定可能) |

### 実装対象

| ファイル | 変更内容 |
|----------|----------|
| `crates/lsharp-syntax/src/ast.rs` | `Metadata` 構造体 (doc, params, invariant, rationale, see_also, example, since) |
| `crates/lsharp-syntax/src/parser.rs` | メタデータキーワードのパース |
| 新規: `crates/lsharp-docs/` | ドキュメント追跡クレート |
| `crates/lsharp-docs/src/tracker.rs` | コメント紐付け、AST ハッシュ計算、鮮度管理 |
| `crates/lsharp-docs/src/knowledge.rs` | `--emit knowledge` の JSON 出力 |
| `crates/lsharp-docs/src/review.rs` | `lsharp review` の AI 連携 |
| 新規: `.lsharp-doc-status` | 鮮度管理キャッシュ (ビルド時再生成、git 非追跡) |

---

## Phase 4: モジュールシステム / 名前空間

### 設計判断

- **モジュール = ファイル**: 1:1 対応 (F#/OCaml 寄り)
- **名前空間区切り**: ドット `.` を使用
- **可視性**: デフォルト公開、`(private ...)` で非公開
- **ネストモジュール**: Phase 7 以降に延期

### 構文

#### モジュール宣言

```lisp
;; ファイル先頭で宣言
(module Math.Vec2)
```

#### インポート

```lisp
;; 完全修飾アクセス
(import Math.Vec2)
;; Math.Vec2.add で参照

;; モジュールエイリアス
(import Math.Vec2 :as V)
;; V.add で参照

;; 選択的インポート
(import Math.Vec2 :only [add sub])
;; add, sub が直接参照可能

;; 全公開 (F# の open に相当)
(import Math.Vec2 :open)
;; 全てのエクスポートが直接参照可能
```

#### 使用例

```lisp
;; ファイル: src/geometry.ls
(module Geometry)

(type Point
  (record
    (: x Float)
    (: y Float)))

(defn distance [(: p1 Point) (: p2 Point)] : Float
  :doc "2 点間のユークリッド距離を計算する"
  :params {p1 "始点" p2 "終点"}
  :invariant [(>= result 0.0)]
  (let [dx (- (Point.x p1) (Point.x p2))
        dy (- (Point.y p1) (Point.y p2))]
    (sqrt (+ (* dx dx) (* dy dy)))))

;; ファイル: src/main.ls
(module Main)

(import Geometry :open)

(defn main [] : Unit
  (let [p1 {Point x 0.0 y 0.0}
        p2 {Point x 3.0 y 4.0}]
    (print (distance p1 p2))))
```

### 実装対象

| ファイル | 変更内容 |
|----------|----------|
| `crates/lsharp-syntax/src/token.rs` | `Module`, `Import` は既存 (未使用) |
| `crates/lsharp-syntax/src/ast.rs` | `Decl::ModuleDecl`, `Decl::ImportDecl` |
| `crates/lsharp-syntax/src/parser.rs` | モジュール/インポート宣言のパース |
| `crates/lsharp-types/src/infer.rs` | `ModuleEnv` 追加、名前解決フェーズ |
| `crates/lsharp-ir/src/lower.rs` | 複数モジュールのリンク |
| `crates/lsharp-wasm/src/codegen.rs` | 単一 Wasm モジュールへのフラット化 |
| 新規: ドライバー層 | モジュールグラフ構築・循環依存検出 |

---

## Phase 5: トレイト (Traits)

### 設計判断

**Rust 風トレイト** を採用する。

理由:
1. **orphan rule** でモジュールシステムとの整合性を保証
2. WasmGC の `funcref` で vtable を表現可能
3. **associated types** が型レベルプログラミングの基盤になる
4. デフォルト実装でボイラープレートを削減

### 構文

#### トレイト定義

```lisp
;; 基本的なトレイト
(trait (Show a)
  (defn show [(: self a)] : String))

;; デフォルト実装付き
(trait (Eq a)
  (defn eq [(: self a) (: other a)] : Bool)
  (defn ne [(: self a) (: other a)] : Bool
    (not (eq self other))))
```

#### トレイト実装

```lisp
(impl (Show Point)
  (defn show [(: self Point)] : String
    (str "Point(" (Point.x self) ", " (Point.y self) ")")))

(impl (Eq Point)
  (defn eq [(: self Point) (: other Point)] : Bool
    (and (eq (Point.x self) (Point.x other))
         (eq (Point.y self) (Point.y other)))))
```

#### トレイト制約

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

`:where` をメタデータキーワードとして使用。パラメータリストと本体の間に自然に配置できる。

#### Associated Types (将来拡張)

```lisp
(trait (Collection c)
  (type-assoc Item)
  (defn get [(: self c) (: idx Int)] : (Option Item)))
```

### WasmGC 表現 (辞書パスイング)

```wasm
;; Show トレイトの辞書型
(type $Show_dict (struct
  (field $show (ref $show_func_type))))
(type $show_func_type (func (param (ref eq)) (result (ref $String))))

;; Point 用 Show 辞書インスタンス
(global $show_Point_dict (ref $Show_dict)
  (struct.new $Show_dict (ref.func $show_Point)))

;; トレイト制約付き関数は辞書引数を追加で受け取る
;; to-string(dict, x) where dict : (ref $Show_dict)
```

静的ディスパッチが可能な場合は monomorphization (単相化) で最適化する。

### 実装対象

| ファイル | 変更内容 |
|----------|----------|
| `crates/lsharp-syntax/src/token.rs` | `Trait`, `Impl`, `Where` キーワード |
| `crates/lsharp-syntax/src/ast.rs` | `Decl::TraitDef`, `Decl::ImplDef`, 制約情報 |
| `crates/lsharp-types/src/types.rs` | `TypeScheme` に constraints 追加 |
| `crates/lsharp-types/src/infer.rs` | トレイト解決、辞書パスイング変換 |
| `crates/lsharp-ir/src/lower.rs` | 辞書引数の追加、vtable 構築 |
| `crates/lsharp-wasm/src/codegen.rs` | `funcref` と `call_ref` 命令 |

---

## Phase 6: 高度な型機能 (将来)

### 6a. 高カインド型 (Higher-Kinded Types)

```lisp
;; Functor トレイト -- 型コンストラクタに対するトレイト
(trait (Functor f)
  (defn fmap [(: func (-> a b)) (: fa (f a))] : (f b)))

(impl (Functor Option)
  (defn fmap [(: func (-> a b)) (: fa (Option a))] : (Option b)
    (match fa
      [(Some x) (Some (func x))]
      [None None])))
```

**依存**: Phase 5 (トレイト)
**影響**: `Type` に kind (種) の概念を追加。kind 推論の実装。

### 6b. GADT (一般化代数的データ型)

```lisp
(type (Expr a)
  :gadt
  [(IntLit Int)                        : (Expr Int)]
  [(BoolLit Bool)                      : (Expr Bool)]
  [(Add (Expr Int) (Expr Int))         : (Expr Int)]
  [(If (Expr Bool) (Expr a) (Expr a))  : (Expr a)])
```

**依存**: 独立 (ただし型チェッカーの大改修が必要)
**影響**: Algorithm W からバイディレクショナル型チェックへの移行。パターンマッチでの型の絞り込み (type refinement)。

### 6c. Computation Expressions (F# 風)

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

**依存**: Phase 5 (トレイト) + 6a (HKT)
**影響**: `let!` の構文糖衣脱糖パスを追加。

### 6d. ネストモジュール

```lisp
(module Outer
  (module Inner
    (defn helper [] : Int 42))
  (defn main [] : Int
    (Inner.helper)))
```

**依存**: Phase 4 (モジュール)

---

## 横断的課題: WasmGC 基盤整備

### WasmGC の現状 (2025 年時点)

WasmGC は全主要ブラウザ・ランタイムで安定サポート済み:
- Chrome v119+, Firefox v120+, Safari v18.2+
- wasmtime, wasmer v6.0+ でフルサポート
- wasm-encoder (Rust) で GC 型 API がフルサポート済み (`StructType`, `ArrayType`, `SubType`)

### Phase 1 で必要な WasmGC 基盤

1. **IrType の拡張**: `Ref(TypeIdx)` を追加し、GC 参照型を表現
2. **IR 命令の追加**: `struct.new`, `struct.get`, `struct.set`, `ref.cast` 等
3. **codegen の拡張**: GC 型定義セクションの出力
4. **wasm-encoder 対応**: GC 拡張のサポート確認 (バージョンアップが必要な可能性)

### String の WasmGC 表現

WasmGC array で UTF-8 バイト列として表現する。線形メモリとの混在を避け、全てを GC 上で統一する。

```wasm
;; String 型
(type $String (array (mut i8)))

;; 文字列リテラル "hello"
(array.new_data $String $hello_data 0 5)

;; 長さ取得
(array.len (local.get $str))

;; 文字アクセス
(array.get $String (local.get $str) (i32.const 0))
```

### 正規表現エンジン

`(matches ...)` 制約のランタイム検証用に、正規表現エンジンを L# ランタイムライブラリとして Wasm に組み込む。
外部依存なしでポータブルに動作する。

初期実装はシンプルなサブセットから始める:
- Phase 1: リテラルマッチ、文字クラス `[a-z]`、`*`, `+`, `?`
- Phase 2: `^`, `$`, `|`, グルーピング `()`
- Phase 3: 後方参照、先読み等の高度な機能 (必要に応じて)

---

## 実装順序

```
Phase 0: プロジェクト基盤 (git 必須化、lsharp init)
    |
Phase 1: レコード型 + WasmGC 基盤 + ADT WasmGC 化
    |
Phase 2: 型エイリアス + 制約付き型 (自動テスト生成)
    |
Phase 3: 構造化メタデータ + ドキュメント追跡 + AI レビュー統合
    |
Phase 4: モジュールシステム
    |
Phase 5: トレイト
    |
Phase 6: HKT / GADT / Computation Expressions / ネストモジュール
```

## 検証方法

各フェーズの検証:

1. `examples/` に各機能のサンプル `.ls` ファイルを追加
2. `crates/lsharp-types/src/infer.rs` のユニットテストで型推論を検証
3. `crates/lsharp-ir/tests/` のスナップショットテストで IR 出力を検証
4. `crates/lsharp-wasm/tests/` で Wasm 出力を検証
5. wasmtime/wasmer で実行テスト (WasmGC 対応ランタイムが必要)

# Language Reference

L# の利用者向けリファレンスです。実装詳細ではなく、日常的に使う構文と型システムに絞って整理しています。

## Core Syntax

- 関数定義: `(defn name [params] body)`
- ラムダ: `(fn [params] body)`
- let 束縛: `(let [x 1 y 2] (+ x y))`
- 条件分岐: `(if cond then else)`
- 逐次実行: `(do expr1 expr2 result)`
- パターンマッチ: `(match expr [pattern body] ...)`

## Type System

- プリミティブ型: `Int`, `Float`, `String`, `Bool`, `Unit`
- 関数型: `Int -> Int -> Int`
- 型推論: Hindley-Milner
- 多相型: `(Option a)`, `(Result a e)`

## Constrained Types

`type-constrained` は値の意味的な制約を型定義に載せます。

```lisp
(type-constrained Email String
  :constraints [(min-length 3)
                (matches "^[^@]+@[^@]+\\.[^@]+$")])
```

| constraint | target | example |
|---|---|---|
| `>=` / `<=` / `range` | `Int` | `(range 1 65535)` |
| `one-of` | `Int` | `(one-of [1 2 3])` |
| `min-length` / `max-length` | `String` | `(min-length 3)` |
| `matches` | `String` | `(matches "^\\w{3}\\d{3}$")` |
| `satisfies` | runtime predicate | `(satisfies even?)` |

`matches` は L# 内蔵の regex engine で評価します。Rust の `regex` crate ではなく、
後方参照と先読みを含む L# 側の semantics を保つための実装です。

| syntax | meaning |
|---|---|
| literal, `.`, `^`, `$` | literal match, any char, start/end anchors |
| `*`, `+`, `?`, `{n}`, `{n,m}`, `{n,}` | quantifiers |
| `*?`, `+?`, `??`, `{n,m}?` | non-greedy suffix accepted; boolean match language is unchanged |
| `[abc]`, `[a-z]`, `[^a-z]` | character classes and negated classes |
| `\d`, `\w`, `\s`, `\D`, `\W`, `\S` | digit, word, whitespace shorthand classes and negations |
| `(abc)`, `(?:abc)`, `a|b` | capturing group, non-capturing group, alternation |
| `\1` ... `\9` | backreferences to capturing groups |
| `(?=...)`, `(?!...)` | positive and negative lookahead |
| `\p{L}`, `\p{N}`, `\P{L}`, `\P{N}` | Unicode letter/number classes and negations |

## Data Definitions

### ADT

```lisp
(type (Option a) (Some a) None)
```

### Record

```lisp
(type Point (record (: x Int) (: y Int)))
```

## Modules

- ファイルパスが import 名へ対応する
- `src/Foo/Bar.ls` は `(import Foo.Bar)` で参照する
- package 境界では `[project.exports]` が公開モジュールを制御する
- `private` でモジュール内シンボルを非公開化できる

## Metadata

関数にはドキュメント用メタデータを付けられます。

```lisp
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(abs (- 0 5))]
  (if (< x 0) (- 0 x) x))
```

- `:doc` は説明
- `:params` はパラメータ説明
- `:returns` は戻り値説明
- `:example` はサンプル式

## Stdlib Modules

- `Core`
- `List`
- `Map`
- `Set`
- `Vector`
- `String`
- `Char`
- `IO`
- `Json`
- `Path`
- `Debug`

## MCP Tools

- `lsharp_hover`: 型と `:doc` を取得
- `lsharp_completion`: 補完候補を取得
- `lsharp_check`: 型チェック
- `lsharp_package_api`: package API 一覧を取得
- `lsharp_stdlib_api`: stdlib API 一覧を取得

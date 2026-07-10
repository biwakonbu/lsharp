# Examples Matrix

このページは `examples/*.ls` の対応表です。サンプルの正本は tracked な `.ls` ファイルであり、`examples/*.wasm` は `.gitignore` 対象の生成物です。

## Status

- **実行サンプル**: `lsharp compile` の入力として使う通常のサンプル。
- **metadata サンプル**: `lsharp test` / `lsharp doc` の入力として使うサンプル。通常の executable entrypoint ではない。
- **型チェックのみ / stub main**: 言語機能の構文・型チェックを示すが、実行本体は `print 42` などの stub。runtime-ready と断定しない。

| example | status | demonstrates | related docs | notes |
|---------|--------|--------------|--------------|-------|
| [`computation.ls`](../../examples/computation.ls) | 型チェックのみ / stub main | computation builder, `let!` / `return` 系の将来導線 | [Language Reference](./language-reference.md), [高度な型機能](../../book/ch11-advanced-types.md) | MVP 段階では builder 登録のみ。Wasm 実行は GC 型サポート後の予定。 |
| [`constrained.ls`](../../examples/constrained.ls) | 実行サンプル | `type-constrained`, numeric constraints, `range`, `one-of` | [Language Reference](./language-reference.md), [型エイリアスと制約付き型](../../book/ch08-type-aliases.md) | 制約付き型の書き方を確認する入口。 |
| [`factorial.ls`](../../examples/factorial.ls) | 実行サンプル | recursion, arithmetic, `do`, repeated `print` | [Quick Start](./quick-start.md), [Language Reference](./language-reference.md) | 再帰と複数出力の最小例。 |
| [`fib.ls`](../../examples/fib.ls) | 実行サンプル | recursion, `if`, arithmetic | [Quick Start](./quick-start.md), [型推論](../../book/ch04-type-inference.md) | README / book でも使う代表的な compile smoke input。 |
| [`gadt.ls`](../../examples/gadt.ls) | 型チェックのみ / stub main | GADT-oriented ADT, pattern matching, recursive evaluator shape | [Language Reference](./language-reference.md), [高度な型機能](../../book/ch11-advanced-types.md) | GC struct 型の runtime support が揃うまでは `main` は stub。 |
| [`hello.ls`](../../examples/hello.ls) | 実行サンプル | minimal `main`, `print` | [Quick Start](./quick-start.md) | 最小の compile/run 確認用。 |
| [`hkt.ls`](../../examples/hkt.ls) | 型チェックのみ / stub main | higher-kinded type shape, `trait (Functor f)` | [Language Reference](./language-reference.md), [高度な型機能](../../book/ch11-advanced-types.md) | HKT を使う抽象は runtime-ready と断定しない。 |
| [`metadata.ls`](../../examples/metadata.ls) | metadata サンプル | `:doc`, `:params`, `:returns`, `:example`, `:invariant` | [Language Reference](./language-reference.md), [テスト戦略](../../book/ch13-testing.md) | `lsharp test examples/metadata.ls` / `lsharp doc examples/metadata.ls -o metadata.html` 用。 |
| [`module.ls`](../../examples/module.ls) | 実行サンプル | `(module ...)`, module-local functions | [Package Layout](./package-layout.md), [モジュールシステム](../../book/ch09-modules.md) | 単一ファイル module の最小例。 |
| [`nested-module.ls`](../../examples/nested-module.ls) | 実行サンプル | module declaration, helper functions, nested-style naming | [Package Layout](./package-layout.md), [モジュールシステム](../../book/ch09-modules.md) | module / helper function grouping の例。 |
| [`record.ls`](../../examples/record.ls) | 実行サンプル | record type, record literal, field access | [Language Reference](./language-reference.md), [レコード型](../../book/ch07-record-types.md) | `Point.x` field access の最小例。 |
| [`trait-where.ls`](../../examples/trait-where.ls) | 実行サンプル | trait declaration, `:where` constrained function | [Language Reference](./language-reference.md), [トレイト](../../book/ch10-traits.md) | 静的 trait constraint の表面構文を示す。 |
| [`trait.ls`](../../examples/trait.ls) | 実行サンプル | trait declaration, method signature | [Language Reference](./language-reference.md), [トレイト](../../book/ch10-traits.md) | trait 定義の最小例。 |
| [`type-alias.ls`](../../examples/type-alias.ls) | 実行サンプル | `type-alias`, typed parameters, typed return | [Language Reference](./language-reference.md), [型エイリアスと制約付き型](../../book/ch08-type-aliases.md) | alias を public API の読みやすさに使う例。 |
| [`types.ls`](../../examples/types.ls) | 実行サンプル | ADT, polymorphic type constructors, pattern matching | [Language Reference](./language-reference.md), [型推論](../../book/ch04-type-inference.md) | `Option` / `Result` と `match` の組み合わせ。 |

## Update Rule

新しい `.ls` サンプルを追加した場合は、この表にも同じ commit で 1 行追加してください。実行を伴わないサンプルは、理由を `status` と `notes` の両方で明示します。

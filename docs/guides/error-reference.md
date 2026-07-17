# Error Reference

このページは L# の `LS####` エラーコード体系の利用者向け正本です。MCP の `lsharp_errors` tool も同じ driver 側 table を参照します。

現時点の scope は error code reference と MCP lookup の統一です。lexer/parser/macro/type/lowering/codegen の下層エラーは stable code/span まで接続済みで、LSP の syntax/type diagnostics と MCP `lsharp_check` も code と source range を forwarding します。CodegenError は source span を持たないため `span = None` とし、CLI の全診断経路、LSP incremental/module/codegen diagnostics の range/code forwarding は I-02 / imp-02 の残タスクとして扱います。

## Code Ranges

- `LS0001` - `LS0003`: lexer
- `LS0101` - `LS0104`: parser
- `LS0201`: macro expansion
- `LS1001` - `LS1013`: type checker
- `LS2001` - `LS2006`: metadata contract validation and migration
- `LS3001` - `LS3104`: lowering and module graph
- `LS4001`: codegen

## Legacy Codes

旧 MCP code の `E0001` - `E0005` は 1 release の互換 alias として扱います。

| legacy | current |
|--------|---------|
| `E0001` | `LS1001` |
| `E0002` | `LS1002` |
| `E0003` | `LS1002` |
| `E0004` | `LS1004` |
| `E0005` | `LS1003` |

## Reference

| code | name | summary | fix |
|------|------|---------|-----|
| `LS0001` | unexpected-character | lexer が未知の文字を検出しました | 該当文字を削除するか、文字列 literal 内へ移動してください。 |
| `LS0002` | unterminated-string | 文字列 literal が閉じていません | 文字列 literal の末尾に `"` を追加してください。 |
| `LS0003` | invalid-number | 数値 literal の形式が不正です | 整数または小数の literal 表記を確認してください。 |
| `LS0101` | unexpected-token | parser が予期しない token を検出しました | 括弧の対応、form 名、引数の並びを確認してください。 |
| `LS0102` | unexpected-eof | source が途中で終了しました | 閉じ括弧、vector、metadata form の終端を追加してください。 |
| `LS0103` | unknown-form | 未知の special form です | `defn`, `type`, `module`, `import`, `trait`, `impl` などの form 名を確認してください。 |
| `LS0104` | multiple-parse-errors | 複数の parse error が発生しました | 先頭の parse error から順に修正してください。 |
| `LS0201` | macro-expansion-error | マクロ展開に失敗しました | macro 定義と呼び出し引数、quote / unquote の構造を見直してください。 |
| `LS1001` | undefined-variable | 未定義の識別子です | 定義、import、module path、綴りを確認してください。 |
| `LS1002` | type-mismatch | 型が一致していません | if 条件、then/else、関数引数、戻り値の型を揃えてください。 |
| `LS1003` | infinite-type | 無限型が発生しました | 再帰的自己参照や関数適用の形を見直してください。 |
| `LS1004` | arity-mismatch | 関数引数の個数または型が一致していません | 呼び出し引数の数と型を関数定義に合わせてください。 |
| `LS1005` | undefined-constructor | 未定義の constructor です | 型定義、import、constructor 名を確認してください。 |
| `LS1006` | undefined-record | 未定義の record 型です | `type Name (record ...)` の定義と import を確認してください。 |
| `LS1007` | undefined-field | 未定義の record field です | field 名と record 型定義を確認してください。 |
| `LS1008` | recursive-alias | 再帰的な type alias です | 再帰が必要な場合は ADT を使い、alias の循環を外してください。 |
| `LS1009` | undefined-alias | 未定義の type alias です | alias 定義と import を確認してください。 |
| `LS1010` | undefined-trait | 未定義の trait です | trait 定義、import、trait 名を確認してください。 |
| `LS1011` | missing-impl | 必要な trait impl が見つかりません | 対象型の impl を追加するか、constraint を見直してください。 |
| `LS1012` | alias-type-mismatch | type alias 展開後の型が一致していません | alias の target 型と利用箇所の型を確認してください。 |
| `LS1013` | kind-mismatch | kind が一致していません | 型引数の数と高カインド型の使い方を確認してください。 |
| `LS2001` | legacy-example-migration | legacy `:example` に canonical migration 候補があります | report の disposition を確認し、docs-only `:example` または `:assert` へ明示的に移行してください。 |
| `LS2002` | legacy-invariant-migration | legacy `:invariant` に canonical migration 候補があります | binder、generator、postcondition を明示した `:property` へ移行してください。 |
| `LS2003` | ambiguous-legacy-contract | legacy contract の意味を安全に自動判定できません | 参照 scope と意図を確認し、`:example`、`:assert`、`:case`、`:property` のいずれかを明示してください。 |
| `LS2004` | empty-executable-contract | executable contract に検査対象がありません | `:assert` に Bool predicate を少なくとも 1 件追加してください。 |
| `LS2005` | vacuous-contract | executable contract が実装の挙動を検査していません | 実装結果または入力に依存する Bool predicate を指定してください。 |
| `LS2006` | empty-case-contract | canonical case に検査対象がありません | `:case` に actual / expected の組を少なくとも 1 件追加してください。 |
| `LS3001` | unsupported-lowering | lowering が未対応の構文です | 対応済みの言語機能へ書き換えるか、該当機能の lowering 実装を追加してください。 |
| `LS3002` | undefined-lowered-function | lowering 後に未定義関数が残っています | module import、関数定義、stdlib 参照を確認してください。 |
| `LS3101` | cyclic-module-dependency | module dependency に循環があります | module 境界を分割するか、循環 import をなくしてください。 |
| `LS3102` | module-not-found | module が見つかりません | file path、module 名、package install 状態を確認してください。 |
| `LS3103` | module-not-exported | module が package から export されていません | `lsharp.toml` の exports または import 先を確認してください。 |
| `LS3104` | duplicate-module | module が重複しています | source file の配置と module declaration を整理してください。 |
| `LS4001` | codegen-error | codegen が失敗しました | 直前の type / lowering diagnostics と対象 backend の既知制限を確認してください。 |

## MCP Lookup

```json
{
  "tool": "lsharp_errors",
  "arguments": { "error_code": "LS1001" }
}
```

legacy alias も lookup できます。

```json
{
  "tool": "lsharp_errors",
  "arguments": { "error_code": "E0001" }
}
```

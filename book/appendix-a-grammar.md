# 付録A: L# 文法リファレンス {.unnumbered}

## 表記規則 {.unnumbered}

本付録では拡張 BNF (EBNF) 記法を用いる:

- `|` は選択を表す
- `*` は 0 回以上の繰り返しを表す
- `+` は 1 回以上の繰り返しを表す
- `?` は省略可能な要素を表す
- `'...'` はリテラル文字列を表す

## プログラム構造 {.unnumbered}

```
program     = decl*
decl        = func_decl | type_decl | trait_decl | impl_decl
            | module_decl | import_decl | type_alias
            | type_constrained | computation_builder
            | private_decl | expr
```

## 宣言 {.unnumbered}

```
func_decl   = '(' 'defn' SYMBOL '[' param* ']' type_ann? where_clause? body ')'
param       = SYMBOL | '(' ':' SYMBOL type_expr ')'
type_ann    = ':' type_expr
where_clause = ':where' '[' constraint* ']'
constraint  = '(' SYMBOL SYMBOL ')'
body        = expr

type_decl   = '(' 'type' type_head variant+ ')'
            | '(' 'type' type_head '(' 'record' field+ ')' ')'
            | '(' 'type' type_head ':gadt' gadt_variant+ ')'
type_head   = SYMBOL | '(' SYMBOL SYMBOL+ ')'
variant     = '(' SYMBOL type_expr* ')'  |  SYMBOL
gadt_variant = '[' '(' SYMBOL type_expr* ')' ':' type_expr ']'
field       = '(' ':' SYMBOL type_expr ')'

trait_decl  = '(' 'trait' '(' SYMBOL SYMBOL ')' method+ ')'
method      = '(' 'defn' SYMBOL '[' param* ']' type_ann? body? ')'

impl_decl   = '(' 'impl' '(' SYMBOL SYMBOL ')' method+ ')'

module_decl = '(' 'module' module_path ')'
module_path = SYMBOL ('.' SYMBOL)*

import_decl = '(' 'import' module_path import_mod? ')'
import_mod  = ':as' SYMBOL
            | ':only' '[' SYMBOL+ ']'
            | ':open'

type_alias  = '(' 'type-alias' type_head type_expr ')'

type_constrained = '(' 'type-constrained' SYMBOL type_expr
                     ':constraints' '[' constraint_pred+ ']' ')'
constraint_pred  = '(' pred_op literal+ ')'
pred_op    = '>=' | '<=' | 'range' | 'one-of'

computation_builder = '(' 'computation-builder' SYMBOL method+ ')'

private_decl = '(' 'private' decl+ ')'
```

## 式 {.unnumbered}

```
expr        = literal | SYMBOL | qualified_name
            | if_expr | let_expr | lambda | app
            | match_expr | do_expr | record_lit
            | record_update | field_access
            | computation_expr

literal     = INT | FLOAT | STRING | BOOL | '(' ')'

qualified_name = SYMBOL '.' SYMBOL ('.' SYMBOL)*

if_expr     = '(' 'if' expr expr expr ')'
let_expr    = '(' 'let' '[' binding+ ']' expr ')'
binding     = SYMBOL expr
lambda      = '(' 'fn' '[' param* ']' type_ann? expr ')'
app         = '(' expr expr* ')'
match_expr  = '(' 'match' expr match_arm+ ')'
match_arm   = '[' pattern when_guard? expr ']'
when_guard  = ':when' expr
do_expr     = '(' 'do' expr+ ')'

record_lit  = '{' SYMBOL (SYMBOL expr)+ '}'
record_update = '{' expr '|' (SYMBOL expr)+ '}'
field_access = '(' qualified_name expr ')'

computation_expr = '(' SYMBOL '!' comp_step+ ')'
comp_step   = '(' 'let!' '[' SYMBOL expr ']' comp_step ')'
            | '(' 'do!' expr ')' comp_step
            | '(' 'return' expr ')'
            | expr
```

## パターン {.unnumbered}

```
pattern     = '_'
            | SYMBOL
            | literal
            | '(' SYMBOL pattern* ')'
            | '{' SYMBOL (SYMBOL+ )? '}'
```

## 型式 {.unnumbered}

```
type_expr   = SYMBOL
            | '(' SYMBOL type_expr+ ')'
            | '(' '->' type_expr+ type_expr ')'
            | '(' 'record' field+ ')'
```

## トークン {.unnumbered}

```
INT         = '-'? [0-9]+
FLOAT       = '-'? [0-9]+ '.' [0-9]+
STRING      = '"' char* '"'
BOOL        = 'true' | 'false'
SYMBOL      = [a-zA-Z_!?+\-*/<>=] [a-zA-Z0-9_!?+\-*/<>=]*
COMMENT     = ';' [^\n]*
```

## キーワード一覧 {.unnumbered}

| キーワード | 用途 |
|-----------|------|
| `defn` | 関数定義 |
| `let` | 変数束縛 |
| `if` | 条件分岐 |
| `match` | パターンマッチ |
| `type` | 型定義 (ADT / レコード / GADT) |
| `fn` | ラムダ式 |
| `do` | 逐次実行 |
| `module` | モジュール宣言 |
| `import` | モジュールインポート |
| `record` | レコード型定義 |
| `trait` | トレイト定義 |
| `impl` | トレイト実装 |
| `where` | トレイト制約 |
| `type-alias` | 型エイリアス |
| `type-constrained` | 制約付き型 |
| `private` | 非公開宣言 |
| `computation-builder` | 計算式ビルダー定義 |

## 組み込み演算子 {.unnumbered}

| 演算子 | 型 | 説明 |
|--------|------|------|
| `+` | `(Int, Int) -> Int` | 加算 |
| `-` | `(Int, Int) -> Int` | 減算 |
| `*` | `(Int, Int) -> Int` | 乗算 |
| `/` | `(Int, Int) -> Int` | 除算 |
| `%` | `(Int, Int) -> Int` | 剰余 |
| `<` | `(Int, Int) -> Bool` | 小なり |
| `<=` | `(Int, Int) -> Bool` | 以下 |
| `>` | `(Int, Int) -> Bool` | 大なり |
| `>=` | `(Int, Int) -> Bool` | 以上 |
| `==` | `(Int, Int) -> Bool` | 等値 |
| `!=` | `(Int, Int) -> Bool` | 非等値 |
| `and` | `(Bool, Bool) -> Bool` | 論理積 |
| `or` | `(Bool, Bool) -> Bool` | 論理和 |
| `not` | `(Bool) -> Bool` | 論理否定 |

## 組み込み関数 {.unnumbered}

| 関数 | 型 | 説明 |
|------|------|------|
| `print` | `(Int) -> Unit` | 整数を標準出力に表示 |
| `string-length` | `(String) -> Int` | 文字列長 |
| `string-concat` | `(String, String) -> String` | 文字列結合 |
| `char-at` | `(String, Int) -> Int` | 指定位置の文字コード |
| `substring` | `(String, Int, Int) -> String` | 部分文字列 |
| `string-eq` | `(String, String) -> Bool` | 文字列等値比較 |
| `int-to-string` | `(Int) -> String` | 整数を文字列に変換 |
| `string-print` | `(String) -> Unit` | 文字列を標準出力に表示 |
| `ref-new` | `(a) -> (Ref a)` | 参照セルの作成 |
| `ref-get` | `(Ref a) -> a` | 参照セルの値取得 |
| `ref-set` | `(Ref a, a) -> Unit` | 参照セルの値設定 |
| `vector-new` | `(Int) -> (Vector a)` | ベクター作成 |
| `vector-push` | `(Vector a, a) -> Unit` | 要素追加 |
| `vector-get` | `(Vector a, Int) -> a` | 要素取得 |
| `vector-set` | `(Vector a, Int, a) -> Unit` | 要素設定 |
| `map-new` | `() -> (Map k v)` | マップ作成 |
| `map-insert` | `(Map k v, k, v) -> Unit` | 要素挿入 |
| `map-get` | `(Map k v, k) -> v` | 要素取得 |
| `map-contains` | `(Map k v, k) -> Bool` | キー存在確認 |
| `map-remove` | `(Map k v, k) -> Unit` | 要素削除 |


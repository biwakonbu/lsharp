# Stdlib Guide

L# の stdlib は `stdlib/*.ls` が正本です。公開 docs site は stdlib source の metadata から API ページと `api/stdlib.json` を生成します。

## Finding APIs

```bash
lsharp doc-site -o _site
```

生成後は `_site/api/stdlib.json` と `_site/api/*.html` を確認します。AI / MCP から探す場合は `lsharp_stdlib_api` を使います。

## Modules

- `Core`: 基本演算、比較、制御補助
- `List`: list 操作
- `Vector`: indexed sequence 操作
- `Map`: key/value collection
- `Set`: set collection
- `String`: text 操作
- `Char`: character 操作
- `IO`: file / IO helper
- `Path`: path helper
- `Json`: JSON 値と変換
- `Debug`: debug 出力や確認用 helper

## Usage Pattern

まず language feature の書き方を [Language Reference](./language-reference.md) で確認し、stdlib の関数名と metadata は generated API で確認します。

```lisp
(import List)
(import String)

(defn main []
  0)
```

module import の解決順序は `src/`、`.lsharp/packages/*/src/`、`stdlib/` です。package layout の詳細は [Package Layout](./package-layout.md) を参照します。

## Documentation Ownership

- stdlib API の正本は `stdlib/*.ls` の `:doc` metadata です。
- docs site のページ一覧は `docs/site.toml` が正本です。
- language guide や AI skill は stdlib API の詳細を複製せず、generated API へ誘導します。

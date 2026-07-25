# Intent validation

`lsharp test` は implementation conformance（実装が contract を満たすか）を実行し、
`lsharp validate` は intent/evidence graph の追跡可能性と矛盾を確認します。この二つは
別軸なので、`validate` は単一の `verified` フラグを生成しません。

## JSON manifest を検証する

現在の入力境界は [`intent-graph.schema.json`](../schemas/intent-graph.schema.json) に
定義した `schema_version: 1` JSON manifest です。node、evidence、typed edge を記述し、
次のように実行します。

```bash
lsharp validate intent-graph.json
lsharp validate intent-graph.json --format json
```

text と JSON は同じ facts を返します。

| status | 意味 | exit code |
|--------|------|-----------|
| `pass` | trace が閉じ、open question と contradiction がない | `0` |
| `fail` | contradiction が観測された | `1` |
| `unknown` | trace gap、open question、または独立 review 不足がある | `2` |

未知の field、重複 ID、存在しない node/evidence の参照、未対応 schema version は入力
エラーとして非ゼロで終了します。欠落を pass と解釈しないため、CI やレビュー自動化では
`unknown` を別のアクションとして扱えます。

## 現在の境界

この slice は Rust の manifest parser と CLI を graph model へ接続したものです。
L# source syntax、selfhost/native の report parity、EmbeddedCli/MCP、Mac/Linux の
artifact/runtime evidence は後続の M2-03 task として残ります。

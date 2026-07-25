# Intent validation

`lsharp test` は implementation conformance（実装が contract を満たすか）を実行し、
`lsharp validate` は intent/evidence graph の追跡可能性と矛盾を確認します。この二つは
別軸なので、`validate` は単一の `verified` フラグを生成しません。

## JSON manifest を検証する

JSON manifest の入力境界は [`intent-graph.schema.json`](../schemas/intent-graph.schema.json) に
定義した `schema_version: 1` です。node、evidence、typed edge を記述し、次のように実行します。

```bash
lsharp validate intent-graph.json
lsharp validate intent-graph.json --format json
```

project の `lsharp.toml` に manifest を登録すると、入力 path を省略して同じ検証を実行できます。

```toml
[validation]
manifest = "docs/intent-graph.json"
```

```bash
lsharp validate
lsharp validate --format json
```

設定から解決する path は project-relative に限定されます。絶対 path、`..` を含む path、
存在しない path、project root 外を指す symlink は診断として拒否します。設定がない場合は
暗黙の既定 manifest を探さず、明示 path または `[validation].manifest` が必要です。
project の下位 directory から実行した場合も、祖先の `lsharp.toml` を探索します。

text と JSON は同じ facts を返します。

| status | 意味 | exit code |
|--------|------|-----------|
| `pass` | trace が閉じ、open question と contradiction がない | `0` |
| `fail` | contradiction が観測された | `1` |
| `unknown` | trace gap、open question、または独立 review 不足がある | `2` |

未知の field、重複 ID、存在しない node/evidence の参照、未対応 schema version は入力
エラーとして非ゼロで終了します。欠落を pass と解釈しないため、CI やレビュー自動化では
`unknown` を別のアクションとして扱えます。

## L# source から検証する

source metadata を直接 graph へ投影する場合は、明示的に `--source` を指定します。

```bash
lsharp validate --source src/Checkout.ls
lsharp validate --source src/Checkout.ls --format json
lsharp validate --source src/Checkout.ls \
  --emit-manifest target/intent-graph.json --format json
```

source は parse 後に `:intent` / `:claim` / `:assumption` / `:open-question` node と
`:motivates` / `:constrained-by` / `:tested-by` edge へ変換されます。`:evidence` は required
provenance/sampling fields を持つ record として登録され、`:supports` / `:contradicts` は登録済み
evidence にだけ接続されます。record がない evidence edge は入力エラーとして拒否されます。
Contract の executable definition や selfhost/native parity がまだ接続されていない source は、
欠落を補完せず `unknown`（exit code `2`）を返します。
parse error、duplicate node、typed endpoint mismatch、orphan edge は入力エラーとして
report とは別に非ゼロ終了します。`--source` と positional JSON manifest path は同時に指定
できません。

## Source の intent node

source から node identity を持たせる場合は、宣言 metadata に stable ID と本文を明示します。

```lisp
(defn cancel []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
  true)
```

`:assumption` と `:open-question` も同じ形式で記述できます。wire ID の kind と directive
kind が一致しない、本文が空、同じ ID が重複する入力は fail-closed です。node 間の trace を
source に明示する場合は、次の edge metadata を使います。

```lisp
(defn cancel []
  :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
  :constrained-by "claim:checkout/cancel-rejects-shipped" "assumption:checkout/state-authoritative"
  :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
  true)
```

観測 evidence は required provenance/sampling fields を省略せず named metadata で登録します。

```lisp
(defn cancel []
  :evidence "evidence:checkout/cancel-observation"
    :subject "claim:checkout/cancel-rejects-shipped"
    :method "case" :outcome "pass"
    :runner "cargo-test" :target "aarch64-apple-darwin"
    :source-commit "0123456789abcdef" :artifact-digest "sha256:abc123"
    :cases 1 :seed 42 :generator "checkout-cancel-fixture"
    :shrinks [8 3 1] :coverage [("negative" 2) ("positive" 1)]
    :producer "lsharp-test" :tool-version "0.2.0"
    :timestamp "2026-07-25T00:00:00Z" :independence "same-author"
  :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
  true)
```

Rust source adapter は全 node を先に登録し、`motivates` / `constrained-by` の typed endpoint
kind と存在を検査してから graph edge を追加します。`tested-by` は Claim→Contract の typed
edge として claim trace gap を閉じます。`evidence` record は全 required fields を canonical
`Evidence` へ投影し、`supports` / `contradicts` は evidence registry closure を検査します。
未登録 evidence は `EvidenceRegistryRequired` として返し、黙って無視しません。source の optional
shrinks/coverage は canonical `SamplingPlan` と manifest へ投影されますが、selfhost/native 実行と
generator/shrink policy の parity は後続境界です。

`--emit-manifest <output.json>` を指定すると、graph 構築後の version 1 manifest を明示 path へ atomic/durable に保存します。
report は従来どおり stdout へ出し、`unknown` (exit code `2`) でも graph が構築できれば manifest を残します。
既存の出力は symlink を追従せず destination 自体を置換し、parse/adapter error では manifest を作りません。
出力先の親 directory は暗黙には作成しません。

## 現在の境界

この slice は Rust の manifest parser/CLI と source node/edge registry を graph model へ接続し、
project config から安全に入力を発見するものです。`validate --source` は source parser → graph →
report までを Rust CLI で実行できます。required-field evidence record を含む source edge は実行でき、
evidence registry 未接続の source edge は明示的に拒否します。Rust MCP の `lsharp_validate` は
`source` / `file` に加えて `manifest`（JSON object/string）/ `manifest_file` を受け取り、同じ
version 1 parser と fact-oriented report を返します。`include_manifest: true` を指定すると、同じ graph から
canonical manifest を inline で返します（filesystem write は行いません）。manifest emission、selfhost/native の report
parity、EmbeddedCli、Mac/Linux の artifact/runtime evidence は後続の M2-03 task として残ります。

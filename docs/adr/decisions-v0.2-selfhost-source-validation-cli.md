# ADR: v0.2 selfhost source validation CLI の最初の slice

- Status: Accepted (partial)
- Date: 2026-07-25
- Scope: EC-M2-03 selfhost `validate --source`

## Context

Rust の `lsharp validate --source` は source adapter を通じて graph と report を構築できる。
一方、selfhost `App.Cli` は `validate` command と source metadata の JSON report をまだ公開して
いなかった。Rust-free の日常開発経路へ移すには、まず同じ exit code と trace gap の最小契約を
selfhost 側へ固定する必要がある。

## Decision

`App.Cli` に次の最小 surface を追加する。

```text
lsharp validate --source <source.ls> --format json
```

- `defn` の ordered metadata に保持された `intent` / `claim` / `motivates` / `tested-by` を
  集計し、未接続 claim を `trace-gap.claim-without-test` として JSON report へ投影する。
- contract/evidence registry が未接続の source は従来どおり `unknown` / 終了コード `2` とする。
  registered contradictory evidence は canonical report の `fail`、終了コード `1`、
  `independent_reviews` / `contradicting_observations` へ投影する。
- `validate` の option は `--source` と `--format json` の組み合わせだけを受理し、未対応形式は
  compile error として拒否する。
- source graph が構築できた場合は、`--emit-manifest <output.json>` を受理し、report stdoutとは別の
  fileへ version 1 manifestを出力する。serializer は nodes/evidence/edges の順序、typed IDの
  namespace/key、evidence execution/provenance、sampling shrinks/coverageをRustのmanifest wire shape
  に合わせる。manifest writeが失敗した場合は reportを成功扱いにしない。

## Evidence and boundary

`selfhost_cli_validation_contract` は command、option、report code の source contract を RED から
GREEN へ確認した。Rust-host actual Wasm の `test_e2e_selfhost_cli_validate_source_json_reports_trace_gap`
は同じ fixture、argv、filesystem、exit code、JSON reportを検証する。既存の関数間 root lease helperを
明示的な shape checkへ切り出し、`App.Cli` の `run-check-program` と test runner の caller rootを
focused ledger/static contractで balanceした。変更後の actual Wasm testは `1 passed`（291.84s）で、
同じ fixtureの argv/filesystem、unknown exit `2`、JSON status/trace gapを確認した。

typed signature metadata、nested module/private/impl traversal、parse/duplicate/orphan diagnostic、
evidence の全 report/status parity、EmbeddedCli/MCP、native stage0 と Mac Apple Silicon / Linux x86_64 の
current-source parity は未完了である。`source-evidence-graph-from-program` を使う registry/edge と
`--emit-manifest` の initial CLI projection は Rust-host actual Wasm で確認済みだが、selfhost/native
stage0、durable atomic write、CLIの全 fail-closed diagnostic、release provenanceは残件である。

追加の actual Wasm evidenceとして `test_e2e_selfhost_cli_validate_source_json_reports_contradicting_evidence`
が `1 passed`（280.40s）となり、registered `review` / `independent-review` / `contradicted` record と
`:contradicts` edgeを `fail`、exit `1`、`independent_reviews=1`、`contradicting_observations=1` へ投影する。

`test_e2e_selfhost_cli_validate_source_emits_manifest` は同じ source fixtureで `--emit-manifest` の
相対 output path、report stdoutとの分離、version 1 manifestの node/evidence/edge wire shape、sampling
と provenance、unknown exit `2` を確認した。`test_e2e_selfhost_evidence_manifest_serializer_matches_version_one_shape`
は軽量 selfhost bundleで serializer 単体を確認した。これらは Rust-host actual Wasm の verified sliceで
あり、`test_e2e_selfhost_cli_validate_source_does_not_emit_manifest_for_graph_error` は未登録 evidence
edgeで exit `1` と manifest未生成を確認した。native stage0/current-source target parityの証拠には
拡大解釈しない。

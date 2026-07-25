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
- contract/evidence registry が未接続のため report status は `unknown`、終了コードは `2` に固定する。
- `validate` の option は `--source` と `--format json` の組み合わせだけを受理し、未対応形式は
  compile error として拒否する。

## Evidence and boundary

`selfhost_cli_validation_contract` は command、option、report code の source contract を RED から
GREEN へ確認した。Rust-host actual Wasm の `test_e2e_selfhost_cli_validate_source_json_reports_trace_gap`
は同じ fixture、argv、filesystem、exit code、JSON reportを検証するが、現在の lowering ledger が
既存 `typeinfer-builtin-root-value` の関数間 root lease を `ImbalancedExit` として検出するため、
実行は明示的に ignore している。この blocker を解消してから actual Wasm の GREEN とする。

typed signature metadata、nested module/private/impl traversal、parse/duplicate/orphan diagnostic、
evidence/contract registry、`--emit-manifest`、EmbeddedCli/MCP、native stage0 と Mac Apple Silicon /
Linux x86_64 の current-source parity は未完了である。

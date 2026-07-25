# ADR: source validation の version 1 manifest emission

- Status: Accepted (partial)
- Date: 2026-07-25
- Scope: EC-M2-02 / EC-M2-03
- Related: `decisions-v0.2-validation-source-cli.md`, `decisions-v0.2-source-evidence-record.md`

## Context

`IntentGraph` には deterministic な version 1 manifest serializer があるが、
`lsharp validate --source` は report だけを stdout へ出力していた。そのため source の node、
typed edge、evidence の provenance/sampling を後続の validator や review artifact へ保存する
公開経路がなく、report と manifest を同じ stream に混ぜる実装も避ける必要があった。

## Decision

`validate` に明示的な output path を受ける option を追加する。

```text
lsharp validate --source src/Checkout.ls \
  --emit-manifest target/intent-graph.json --format json
```

- `--emit-manifest OUTPUT` は source mode と JSON manifest input mode の両方で利用できる。
- graph の parse/adapter が成功した時点で、既存の `IntentGraph::to_manifest_json_string()` を使って
  version 1 manifest を OUTPUT へ書く。親 directory は暗黙に作成しない。
- OUTPUT は同じ親 directory の一時ファイルへ `create_new` で書き、file `sync_all` 後に `rename` で
  置換する。Unix では rename 後に親 directory も `sync_all` し、失敗時は一時ファイルを回収する。
  既存の symlink は追従せず、destination 自体を置換する。
- report は従来どおり stdout の text/JSON へ出し、manifest JSON を stdout に混ぜない。
- report status の exit code (`pass=0`, `fail=1`, `unknown=2`) は維持する。manifest の書き込み失敗や
  parse/adapter error は report status と混ぜず、driver I/O/input error として返す。
- graph 構築前の入力エラーでは OUTPUT を作成しない。

## Consequences

- source evidence の typed node/edge、execution identity、sampling、provenance を deterministic な
  review artifact として保存できる。
- `unknown` report でも graph が構築できれば manifest を残せるため、欠落を pass として隠さない。
- Rust CLI の manifest 書き込みは crash-consistent な atomic/durable file boundary を持つ。ただし
  source provenance、release packaging、selfhost/native writer の完了を意味しない。
- source/native parity、manifest parser/report の selfhost 実装、EmbeddedCli/MCP、Mac/Linux の
  artifact/runtime evidence はこの Rust CLI slice の範囲外で、TODO の `[~]` に残る。

## Evidence

- `crates/lsharp-driver/tests/validate_cli.rs`
  - `validate_source_emits_manifest_without_mixing_report_stdout`
  - `validate_source_does_not_emit_manifest_for_adapter_errors`
  - `validate_manifest_input_can_emit_normalized_manifest`
- `crates/lsharp-driver/src/atomic_write.rs`
  - `atomic_write_replaces_destination_without_following_symlink`
  - `validate_manifest_emit_replaces_symlink_without_following_target`
- `cargo test -p lsharp-driver --test validate_cli`
- `cargo test -p lsharp-driver --bin lsharp atomic_write`
- `cargo clippy -p lsharp-driver --bin lsharp -- -D warnings`
- changed-file `rustfmt --edition 2024 --check`
- `bash scripts/audit_docs.sh`

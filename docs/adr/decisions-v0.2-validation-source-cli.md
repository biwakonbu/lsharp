# ADR: `lsharp validate --source` の入力境界

- Status: Accepted (partial)
- Date: 2026-07-25
- Scope: EC-M2-03

## Context

`lsharp validate` は version 1 JSON manifest を入力にして intent/evidence graph を
検証していた。一方、L# source の `:intent` / `:claim` / `:assumption` /
`:open-question` と `:motivates` / `:constrained-by` は Rust adapter まで実装済みで、
CLI から同じ判定 model を実行できなかった。

source 側にはまだ `tested-by`、contract、evidence、selfhost/native parity がないため、
source を入力できることと validation が `pass` になることを同一視してはいけない。

## Decision

次の明示的な source mode を追加する。

```text
lsharp validate --source <source.ls> [--format text|json]
```

- source は `lsharp_syntax::parse` で parse し、
  `validation_source::source_program_to_intent_graph` で `IntentGraph` へ投影する。
- report の text/JSON shape と exit code は JSON manifest mode と共有する。
  `pass=0`、`fail=1`、`unknown=2` とする。
- source に contract/evidence がない場合は欠落を補完せず、`unknown` と trace gap を返す。
- parse error、duplicate node、typed endpoint mismatch、orphan edge は report に混ぜず、
  診断として非ゼロ終了する。
- `--source` と positional manifest path は同時指定を拒否する。
- positional manifest と `lsharp.toml` の `[validation].manifest` 解決は従来どおり維持する。

## Evidence

- `cargo test -p lsharp-driver --test validate_cli validate_source`
- `cargo test -p lsharp-driver --test validate_cli`
- source graph の node/edge adapter tests:
  `crates/lsharp-syntax/tests/intent_edges.rs`,
  `crates/lsharp-types/tests/validation_source.rs`

## Remaining boundary

この ADR は Rust CLI の source input wiring だけを決定する。`tested-by`/evidence の source
投影、manifest emission、selfhost/native parity、EmbeddedCli/MCP、対応 target の
artifact/runtime evidence は EC-M2 の後続タスクであり、完了扱いにしない。

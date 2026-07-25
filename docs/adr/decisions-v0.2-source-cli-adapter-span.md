# ADR: v0.2 `validate --source` adapter diagnostic span forwarding

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: Rust `lsharp validate --source` source-graph adapter boundary
- Related: `EC-M2-03`, `docs/adr/decisions-v0.2-validation-source-cli.md`,
  `docs/adr/decisions-v0.2-source-cli-parser-diagnostic.md`

## Context

`validate --source` は parser error については stable code と fail-closed output を持つ一方、
source adapter error はエラーメッセージだけを stderr に出していた。duplicate、edge endpoint、
evidence field のように adapter が既に保持している directive span が CLI の source contextへ
届かないため、入力を修正する場所を行単位で特定できなかった。

## Decision

- `SourceGraphError::source_span()` は adapter error が保持する primary directive span を返す。
  duplicate は重複した declaration、edge/evidence error は現在の directive を primary とする。
- `cmd_validate_source` は span がある adapter error を `miette::NamedSource` と
  `LabeledSpan` へ接続し、stderr に source directive の抜粋と label を表示する。
- span を持たない `Node`、`Graph`、`EdgeId`、`KindMismatch` は既存のメッセージ表示を維持する。
- adapter error は report/manifest generation より前に返し、manifest を作らない fail-closed
  境界を維持する。

## Evidence

- `validate_source_does_not_emit_manifest_for_adapter_errors` は未登録 `supports` edge の
  source directive を stderr で確認し、adapter error 前に manifest が作られないことを検証する。
- `cargo test -p lsharp-driver --test validate_cli`（23 tests）
- `cargo test -p lsharp-types --test validation_source`（21 tests）
- `cargo check --workspace`
- `cargo clippy -p lsharp-driver -p lsharp-types --all-targets -- -D warnings`

## Boundary

この判断は Rust source adapter が既に保持する span の CLI forwarding に限定する。stable
source-adapter error code taxonomy、span を持たない graph-only error、field-level span、
selfhost/native stage0、EmbeddedCli/MCP、Mac Apple Silicon / Linux x86_64 artifact/runtime、
EC-M2-03 aggregate の完了を意味しない。

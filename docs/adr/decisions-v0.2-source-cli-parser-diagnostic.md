# ADR: v0.2 `validate --source` parser diagnostic code forwarding

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: Rust `lsharp validate --source` parser-input boundary
- Related: `EC-M2-03`, `docs/adr/decisions-v0.2-validation-source-cli.md`,
  `docs/guides/error-reference.md`

## Context

`validate --source` は source parse error を path と表示文字列だけで stderr に返していた。
parser 自体が持つ stable `LS0101`〜`LS0104` code が CLI 境界で失われるため、JSON report を
生成できない入力 error を automation が分類できず、`--emit-manifest` の fail-closed 境界も
code 単位で検証できなかった。

## Decision

- `cmd_validate_source` は parser error の `ParseError::code()` を `[LS####]` prefix として
  stderr の driver diagnostic に forward する。
- parse error は report/manifest generation より前に返し、stdout を空のまま、manifest を作らない。
- source adapter error の stable code taxonomy と source span forwarding は別の validation
  diagnostic sliceとして扱い、この変更で完了扱いにしない。

## Evidence

- `validate_source_forwards_parser_code_and_does_not_emit_manifest` は RecordDef metadata の
  malformed source を `LS0101` として返し、空 stdout・manifest未生成を検証する。
- `cargo test -p lsharp-driver --test validate_cli`（23 tests）
- `cargo check --workspace`
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`

## Boundary

この判断は Rust parser error の `validate --source` CLI forwarding に限定する。source graph
adapter error の code/span taxonomy、selfhost/native stage0 parity、EmbeddedCli/MCP、両 target
artifact/runtime evidence、EC-M2-03 aggregate の完了は意味しない。

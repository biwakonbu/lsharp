# ADR: EC-M3 EmbeddedCli manifest emission

- Status: Accepted (verified Rust-host actual Wasm slice)
- Date: 2026-07-27
- Scope: `selfhost/src/App/EmbeddedCli.ls` and actual EmbeddedCli argv E2E
- Supersedes: the filesystem external-boundary decision in `docs/adr/decisions-ec-m3-embedded-validation-status.md`

## Context

`App.Cli` already projected the source evidence graph through the canonical version 1
`validation-source-manifest-json` serializer and wrote it separately from the validation report.
`EmbeddedCli` accepted `--emit-manifest` for option compatibility but rejected it with an
external-boundary diagnostic, so the two selfhost entrypoints exposed different observable
contracts.

## Decision

- Keep `--emit-manifest <path>` in the EmbeddedCli `validate --source` option surface.
- Reuse `validation-source-manifest-json` and the existing `write-file` builtin to emit the
  version 1 manifest before rendering the report.
- Keep report stdout as exactly one JSON line and derive the process exit from the same validation
  status: `pass=0`, `fail=1`, `unknown=2`.
- Do not add private review fields or a second manifest schema. Atomic/durable replacement,
  write-error cleanup, source provenance, and native-stage0 behavior remain separate follow-up
  boundaries.

## Evidence

- RED: `cargo test -p lsharp-wasm --test e2e selfhost_cli_actual_main_args::test_e2e_selfhost_embedded_cli_validate_source_emits_manifest -- --nocapture`
  failed because `intent-graph.json` did not exist after the EmbeddedCli external-boundary path
  (`0 passed; 1 failed`, 256.70s).
- GREEN: the same command passed (`1 passed`, 255.11s).
- The E2E asserts `unknown` report / exit `2`, one stdout JSON line, absence of the old
  `external-boundary:embedded-cli-manifest-output` diagnostic, and manifest fields
  `schema_version=1`, claim node identity, and empty evidence/edge arrays.

## Consequences and open boundaries

EmbeddedCli and App.Cli now share the Rust-host actual Wasm source/report/manifest wiring for this
slice. This does not close native stage0 producer/parser parity, atomic/durable output semantics,
write/provenance failure handling, MCP server parity, or the Mac Apple Silicon / Linux x86_64
artifact/runtime matrix. `TODO.md` therefore keeps `EC-M2-03` and `EC-M3` aggregate work as `[~]`.

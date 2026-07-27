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
  source provenance, and native-stage0 behavior remain separate follow-up boundaries. The
  EmbeddedCli write-error path is fail-closed as recorded below.

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

## Follow-up decision: manifest write failure

When `write-file` returns a negative result for `--emit-manifest`, `EmbeddedCli` now emits the
stable diagnostic `source validation manifest write failed`, exits `1`, and skips both the
validation report and manifest artifact. The report is rendered only after the manifest write
has succeeded, so a filesystem failure cannot be mistaken for a validation result.

Evidence: RED `test_e2e_selfhost_embedded_cli_validate_source_rejects_manifest_write_failure`
failed against the previous implementation (`exit 2` and report output, `0 passed; 1 failed`,
252.02s). GREEN with the fail-closed branch passed (`1 passed`, 254.32s), asserting the diagnostic,
exit `1`, absence of `"status"`, and absence of `missing/intent-graph.json`.

This is a Rust-host actual Wasm EmbeddedCli boundary only. Native stage0 atomic/durable
replacement, source/release provenance, and the two-target artifact/runtime matrix remain open.

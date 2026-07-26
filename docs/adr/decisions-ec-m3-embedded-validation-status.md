# ADR: EC-M3 EmbeddedCli validation status and external boundary

- Status: Accepted (verified Rust-host slice)
- Date: 2026-07-27
- Scope: `selfhost/src/App/EmbeddedCli.ls` and actual EmbeddedCli argv E2E

## Context

M2 connected EmbeddedCli source validation and report generation, but its status projection
treated every non-contradictory source as `unknown` and always returned exit `2`. The Rust
validation model requires `pass` when there are no trace gaps or open questions and at least
one independent review. The M3 plan also requires un-migrated filesystem capabilities to be
an explicit external boundary; EmbeddedCli must not silently add a manifest writer.

## Decision

- Derive one status code in EmbeddedCli and use it for both report and process exit:
  - contradiction: `fail`, exit `1`
  - trace gap, open question, or no independent review: `unknown`, exit `2`
  - otherwise: `pass`, exit `0`
- Count intent/claim trace gaps before rendering the report so the status cannot diverge from
  the emitted `trace_gaps` array.
- Keep `--emit-manifest` in the option parser for contract compatibility, but return the stable
  `external-boundary:embedded-cli-manifest-output` diagnostic, exit `1`, and no report/file from
  EmbeddedCli. Filesystem manifest emission remains an `App.Cli`/host boundary until its
  atomic/durable and provenance contract is implemented for native targets.

The existing `selfhost_evidence_registry` test continues to cover the direct selfhost adapter's
canonical manifest value/bytes. It is not treated as EmbeddedCli filesystem evidence.

## Evidence

1. RED: `test_e2e_selfhost_embedded_cli_validate_source_reports_pass` failed before the change
   with `status=unknown` and exit `2` for a complete graph with one independent review.
2. GREEN: actual EmbeddedCli argv E2E passed for `pass=0`, `fail=1`, and `unknown=2` (the
   status cases were run on the same fixture family) and for the manifest external boundary:
   stable diagnostic, exit `1`, no report, and no file.
3. The direct canonical serializer parity remains covered by
   `test_e2e_selfhost_evidence_manifest_matches_rust_canonical_value`.

## Consequences

EmbeddedCli's Rust-host actual Wasm surface now agrees with the Rust validation status rules and
does not turn an un-migrated filesystem operation into an ambiguous success. Native stage0,
atomic/durable manifest emission, MCP, provenance, and Mac/Linux artifact evidence remain open
follow-up boundaries in M3.

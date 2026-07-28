# ADR: v0.2 native validation source node required fields

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-01`、`docs/adr/decisions-v0.2-source-intent-id-policy.md`

## Context

Source intent nodes must carry an explicit stable wire ID and non-empty description. The Rust syntax
oracle rejects a node whose ID or text operand is omitted with `LS0101`, while native source-file smoke
did not cover either required-field parser boundary.

## Decision

- Add one fixture omitting the `:intent` ID and one fixture omitting the `:claim` text.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:1`, empty stdout, and no manifest file.
- Never derive an ID from function name, module, span, declaration order, or hash; keep omission as a
  parser-level failure distinct from typed kind mismatch and duplicate identity.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  missing-node-ID/text fixture variables and contracts were absent from the inner smoke.
- GREEN: Rust syntax oracle test `intent_metadata_requires_wire_id_and_non_empty_text` passed, and the
  native source-file provenance test returned `Linux native stage0 source-file provenance tests: OK`
  under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a parser/native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-01` and
M2/M3 aggregate `[~]` until actual stage0 replay covers the same fixtures.

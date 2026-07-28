# ADR: v0.2 native validation malformed evidence required-field rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-registry.md`

## Context

An evidence record requires the stable ID, subject, method, outcome, runner, target, source commit,
artifact digest, generator, producer, tool version, timestamp, and independence fields. The Rust syntax
oracle already rejects a record that stops after `:subject`, but native source-file smoke did not cover
that parser boundary.

## Decision

- Add a `:evidence "evidence:checkout/malformed"` fixture with only the `:subject` field.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:1`, empty stdout, and no manifest file.
- Keep required-field arity rejection at parser-level code `1`, distinct from evidence registry closure
  (`code 6`) and graph/report validation statuses.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the malformed evidence fixture variables and contract were absent from the inner smoke.
- GREEN: existing Rust syntax oracle test
  `evidence_record_metadata_requires_all_named_fields` passed, and the native source-file provenance
  test returned `Linux native stage0 source-file provenance tests: OK` under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a parser/native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.

# ADR: v0.3 offline provider-input identity preparer

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: explicit release evidence identity producer boundary
- Related: [`decisions-v0.3-release-identity-gate.md`](decisions-v0.3-release-identity-gate.md),
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

The native release verifier already checks a canonical `review_evidence_identity`, but callers still
had to hand-build the JSON object. That made it easy to omit the artifact or provider snapshot digest,
change field order, or accidentally rely on an implicit environment. The provider itself remains an
external concern; the repository needs a deterministic boundary for the explicit bytes it receives.

## Decision

Add [`prepare-review-evidence-identity.py`](../../scripts/ci/prepare-review-evidence-identity.py) as an
offline producer with these rules:

- `subject_digest`, `source_commit`, `artifact`, and `now` are required explicit inputs.
- The artifact digest is computed from the named file bytes.
- `--trust-store` and `--review-lifecycle` are all-or-none. When present, each digest is computed from
  the named snapshot bytes; when absent, both identity fields are `null` and remain unverified.
- The output key order is exactly `subject_digest`, `source_commit`, `artifact_digest`,
  `trust_store_digest`, `lifecycle_digest`, `now`, matching the release verifier.
- `now` uses the shared [`review_identity_timestamp.py`](../../scripts/ci/review_identity_timestamp.py)
  parser. It rejects year zero, out-of-range clock fields, nonexistent month days, and non-leap
  February 29 while accepting valid leap days, matching Rust's `validate_canonical_timestamp`.
- `--output` uses a same-directory temporary file, `fsync`, and `os.replace`; a missing parent or write
  failure produces no success output.
- The helper does not read network, environment, current checkout, or an implicit trust root.

## Evidence

RED was a contract test that invoked the missing helper and failed, followed by calendar-boundary tests
that demonstrated both Python boundaries accepted invalid timestamps. GREEN is the combined 11-test
producer/verifier suite, including a valid leap day and invalid calendar/clock values. The producer output
is passed back through the verifier with an artifact and `--require-provider-input`, proving the two
boundaries agree on field order and digest bytes.

## Boundary

This closes only the offline producer boundary. Provider API retrieval/authentication, selfhost/native
MCP parity, current-source stage0 provenance, and Mac Apple Silicon/Linux x86_64 packaged runtime gates
remain open in `EC-M3-05`.

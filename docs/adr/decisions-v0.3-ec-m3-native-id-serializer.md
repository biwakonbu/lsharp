# ADR: EC-M3 canonical manifest ID serializer

- Date: 2026-07-31
- Status: Accepted (verified focused slice)
- Scope: `EC-M3-01` / `M3-05-N9`

## Context

The native `validate --source --emit-manifest` path produced a manifest whose edge IDs were
correct, but the `namespace` and `key` fields on `nodes`, `evidence`, and evidence subjects were
empty. The Rust canonical fixture therefore failed even though the source graph, spans, text, and
edge relationships were present. The failure was specific to the native serializer path: each ID
part was extracted through separate nested temporary expressions.

## Decision

`selfhost/src/Tools/Validation/Evidence.ls` owns one `validation-source-id-fields` helper that
parses a wire ID once and returns the ordered `namespace`/`key` JSON fields. Node, subject, and
evidence serializers reuse that helper; `validation-source-id-json` wraps the same fields for the
standalone ID projection. This keeps the observable field order and canonical bytes unchanged while
avoiding repeated native temporary values whose lifetime differed from the Rust oracle path.

The change is intentionally limited to the serializer. No additional `root_push`/`root_pop` calls
were added to `App.Cli`, the graph builder, or the source node collector because those experiments
did not repair the canonical bytes and would expand the ownership surface without evidence.

## Evidence

- RED: the Mac current-source source-file smoke reached `EC-M3-01` and failed only because native
  node/evidence/subject `key` fields were empty; edge IDs and spans matched the canonical fixture.
- Focused contract: `test_native_validation_manifest_serializer_reuses_stable_id_fields` checks
  that node, subject, and evidence serializers all use the shared helper.
- Native preflight with the existing Mac stage0 and the working-tree serializer produced the
  expected validation exit (`2`) and byte-identical
  `tests/fixtures/validation/ec-m3-canonical-manifest.json` (`cmp_rc=0`).

This is a verified serializer slice, not completion of N9. A fresh current-source stage0 producer,
packaged artifact/runtime, provider input, rollback anchor, and Linux x86_64 evidence remain in
`TODO.md` as `[~]` requirements.

## Consequences

Canonical manifest output no longer depends on repeated nested ID substring expressions in the
native path. The helper is shared by every manifest ID projection, so future field-order or ID
format changes have one implementation point and one focused contract to update.

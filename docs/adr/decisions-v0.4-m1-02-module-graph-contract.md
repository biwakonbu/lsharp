# ADR: v0.4 M1-02 module graph and cache evidence contract

## Status

Accepted as the next-version design boundary (2026-08-01). This ADR does not
claim V4-M1-02, `LEGACY-MODULE-01`, or `LEGACY-COMP-01` complete. Existing
SCC/cache implementation slices remain partial until the required native and
two-target evidence is attached.

## Context

The existing `imp-04-module-system-strengthening` design already records Rust
implementation work for deterministic SCC grouping, multi-file inference, and
process-local/cache keys. The next version needs one observable contract that
can be exercised by `check → compile → build`, compared between Rust and native,
and carried into the V4 fixture/evidence system. Without that boundary, a
focused SCC unit test or a warm Rust cache can be mistaken for module graph
completion.

## Decision

### 1. Graph identity and deterministic plan

- The graph root is the canonical entry source path. Logical module names,
  canonical paths, and source fingerprints are recorded together; a module
  name alone is never a cache identity.
- Resolution emits a deterministic plan with `entry`, `modules`, `edges`,
  `sccs`, and `order`. Module names, edge lists, SCC members, and ties in the
  dependency order are sorted before serialization. The plan is an observable
  report input, not an implementation-only debug print.
- Each edge records the import form (`import`, `open`, `qualified`, or
  `only`), the resolved target, and the visibility boundary. A missing target,
  ambiguous target, or private symbol crossing the boundary is an explicit
  failure; the resolver must not silently fall back to a formatter-special
  case, host module, or network/provider lookup.

### 2. SCC and inference boundary

- SCC groups are computed after resolution and before type inference. A
  singleton SCC follows the existing single-module path; a cyclic SCC is
  inferred as one group and then revalidated against per-module visibility.
- The observable contract is the same for fresh and cached execution: graph
  plan, diagnostics/spans, type surface, lowered IR, Wasm bytes, and runtime
  output must be equal for the same source identity.
- Existing `LS3101`–`LS3104` module-graph diagnostic assignments remain stable.
  New diagnostics must use a new code and carry a source span wherever the
  originating syntax/source has a span. Missing spans are a fail-closed
  evidence boundary, not permission to synthesize a location.

### 3. Cache identity and invalidation

- A module cache entry is valid only when its own source fingerprint, canonical
  entry-root scope, target/backend identity, and direct dependency surface key
  match. The dependency key is computed from sorted dependency identities and
  exported type surfaces, not map iteration order.
- A private implementation-only change may preserve a dependent hit; an
  exported type, import edge, source path, target, backend, or schema-version
  change must invalidate the affected downstream SCC. A cache miss must fall
  back to fresh compile, never to stale artifact bytes.
- CLI cache opt-in, LSP session cache, and future disk artifact cache are
  separate boundaries. Evidence must name which one ran; a process-local LSP
  hit does not prove CLI or native persistence.

### 4. V4 fixture and evidence shape

V4-M1-02 fixtures should cover at least these observable cases:

| Fixture class | Required observation |
|---|---|
| missing module | stable `LS3102`-class diagnostic, source span, non-zero check, no artifact |
| import cycle / cyclic SCC | deterministic cycle/SCC plan and explicit diagnostic or accepted SCC result |
| private export / `only` | visibility rejection or accepted restricted surface, with stable span |
| qualified/open import | resolved edge form and deterministic module order |
| record/ADT cross-module | type surface, IR, Wasm, runtime parity |
| dependency surface change | cold/warm cache distinction and downstream invalidation |

Each fixture is selected by the V4-M1-01 batch producers and compared through
`semantic_fixture_diff.py`. The evidence index additionally records the
command (`check`, `compile`, or `build`), target, source commit, report paths,
artifact digest, runtime result, and negative gates. Pending artifact/runtime,
stale source commit, or one-target-only results remain `[~]`.

## TDD and gate order

1. RED: add one minimal fixture and assert graph plan/diagnostic, exit code,
   artifact boundary, and cache observation before changing implementation.
2. GREEN: implement one graph/SCC/cache slice while preserving the existing
   single-module path; run focused Rust tests and the corresponding report
   producer with fallback/network disabled.
3. Differential: compare Rust and native reports for the same fixture IDs and
   source commit. A report header, IR snapshot, or warm-cache counter alone is
   insufficient.
4. Target gate: run Mac Apple Silicon and Linux x86_64 current-source native
   programs, Wasm validation, and standalone runtime where the fixture claims
   those boundaries. Reuse one task-owned VM replay and its lock.
5. Evidence: update the V4 evidence index and ADR only after the scope of every
   requirement matches the evidence. Keep partial parity as `[~]`.

## Consequences

- Parser/type/import work can proceed independently while sharing a stable
  report shape and failure vocabulary.
- Existing Rust focused slices remain useful but cannot silently close the
  native, artifact, runtime, or two-target requirements.
- The V4 fixture matrix may grow with these cases without changing the report
  schema; new fixture IDs still require explicit expected diagnostics and
  runtime boundaries.

## Evidence and remaining work

- Existing `imp-04` Rust SCC/cache slices are design/partial implementation
  evidence only; they are not reused as current native evidence.
- V4-M1-01 matrix, report producers, evidence audit, schema, and runbook are
  the prerequisite tooling.
- Remaining work is the fixture REDs, native Mac/Linux execution, Wasm/runtime
  parity, cache invalidation evidence, and cleanup audit. V4-M1-02 stays `[~]`.

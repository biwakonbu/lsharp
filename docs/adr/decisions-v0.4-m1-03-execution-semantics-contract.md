# ADR: v0.4 M1-03 execution semantics and memory ABI contract

## Status

Accepted as the next-version design boundary (2026-08-01). This ADR does not
claim V4-M1-03, `LEGACY-LANG-01`, `LEGACY-LANG-02`, `LEGACY-IO-01`,
`LEGACY-RUNTIME-01`, or `LEGACY-ROOT-01` complete. Existing focused slices
remain partial until their artifact, runtime, and two-target evidence is
attached.

## Context

L# already has useful Rust and selfhost slices for records, ordinary ADTs,
patterns, WasmGC allocation, bounded file/argv I/O, and root-lifetime
diagnostics. Those slices currently prove different layers and use different
observations. A passing lowering snapshot or a Rust-host runtime therefore
cannot prove that the same value representation and resource boundary survives
through lowering, code generation, and a native stage0 runtime.

V4 needs one contract that makes the observable semantics explicit while
preserving the existing Rust oracle/native differential boundary. The contract
must also distinguish a successful value result from memory/resource behavior:
an implementation that prints the expected value but leaks a root, changes an
import/ftable shape, or silently falls back to a host helper is not equivalent.

## Decision

### 1. Value representation is an end-to-end observation

For every supported representation, the fixture report records the same source
identity and the following ordered observations:

`typed surface → lowered IR → source/ftable/import shape → Wasm digest/size →
runtime stdout/stderr/exit`.

The first slice covers:

- record literals, updates, and field access;
- ordinary ADT constructors and nested pattern matches;
- constructor refinement and the explicit unsupported boundary for GADT/HKT
  cases that are not implemented;
- Map creation/lookup/update at a size that crosses the current bounded scan;
- closure capture, including a closure that calls an allocating helper.

The report must identify the representation and backend policy. Equal stdout is
not sufficient when the IR, imports, artifact bytes, or exit code differ.

### 2. Root, allocator, and GC behavior is part of the contract

Allocation-sensitive fixtures record a resource observation in addition to the
value result:

- root creation, use, and release are associated with a stable root-lifetime
  event or ledger entry;
- free-list growth and object-table growth are reported separately;
- a limit boundary returns the stable resource diagnostic (currently
  `LS4002` where that existing contract applies), a non-zero exit, and no
  falsely successful artifact/runtime result;
- a closure or helper that allocates while another value is live must retain
  the live value across the call;
- sentinel and free-list entries are distinguished by an explicit state, not a
  coincidental payload value.

Metrics may be unavailable in an early producer. In that case the report is
`pending`, never an empty successful metric. A value-only runtime report cannot
close the GC/root requirement.

### 3. Linear-memory and I/O ABI are explicit boundaries

The ABI fixture records source/ftable/import names, memory offsets or handles
where they are observable, and the process boundary for:

- argv order and absent/empty argv;
- file and stdin reads, including the existing 4096-byte boundary;
- short reads, invalid descriptors, and permission/close failures;
- stdout/stderr bytes and the exact exit code;
- unsupported component-sidecar or Preview2 behavior as an explicit external
  boundary rather than an implicit fallback.

The same fixture must be safe to run with fallback and network disabled. An
external helper or provider may be used only when the fixture declares its
input as an explicit, hashed snapshot; it must not be discovered from the
environment. Runtime file snapshots are UTF-8 content keyed by normalized
project-relative paths. The producer materializes them only in its
task-owned runtime directory, preopens that directory explicitly for
Wasmtime, and fails closed instead of overwriting an existing file or
traversing a symlink. Runtime stdin snapshots are also explicit UTF-8 content;
the producer passes them as the child process stdin and never inherits an
operator or host-process stream.

### 4. Failure and unsupported-feature policy

- Unsupported representation, backend, ABI, or resource mode returns a stable
  diagnostic or explicit external-boundary status before emitting a misleading
  artifact.
- Diagnostics retain their code, source span, stderr, and exit code. Missing
  source spans are a fail-closed evidence result; a synthetic span cannot turn
  an incomplete report into a pass.
- Rust, host `lsharp`, `cargo`, `rustc`, and embedded selfhost fallback are
  oracle/bootstrap boundaries only. A native success that invokes one of them
  is recorded as fallback and cannot pass this contract.
- The same source commit, target, backend policy, and fixture ID are required
  for Rust/native comparison. Pending artifact, runtime, metric, or target
  evidence remains `[~]`.

## Fixture and task decomposition

The following task IDs are the smallest units that can be RED-tested and
verified independently. They are scheduling units, not completion claims.

| Task | RED observable contract | Required GREEN/gates |
|---|---|---|
| `V4-M1-03-R1` value representation | nested record/ADT/pattern and Map fixtures expose typed surface, IR, ftable/import, Wasm, and runtime result | Rust differential, native stage0 check/compile, Wasm validation/runtime, both targets where supported |
| `V4-M1-03-R2` closure and allocation | captured value survives an allocating helper; failure has stable code/exit and no stale result | focused root-lifetime test, Rust oracle, native no-fallback run, runtime output plus resource observation |
| `V4-M1-03-R3` GC/free-list | object/root/free-list growth and limit boundary are explicit, including `LS4002` where applicable | allocator/GC focused tests, negative limit fixture, Wasm/runtime metrics, Mac/Linux evidence |
| `V4-M1-03-R4` linear-memory ABI | argv, file, stdin, short-read/fd-error, and 4096-byte fixtures preserve bytes and exit semantics | source/ftable/import report, Rust/native differential, standalone Wasm runtime, target matrix |
| `V4-M1-03-R5` evidence projection | value, ABI, resource, fallback, and cleanup observations are projected into the V4 report/index without scope loss | schema/producer/audit tests, artifact digest, runtime result, negative no-fallback/network gates |

Each task adds its minimal fixture before implementation changes. Fixtures are
selected through the V4-M1-01 batch producers; the report schema may gain an
optional `resource`/`abi` observation only through a versioned schema change.
Until that change and its focused tests exist, a fixture must declare those
observations as pending rather than overloading `stdout` or a debug log.

## TDD and gate order

1. **RED:** add one smallest fixture and assert the expected value/result,
   diagnostic/span, exit code, artifact boundary, and resource/ABI observation.
2. **GREEN:** change one representation or boundary while preserving the
   existing single-value and single-file paths. Run focused Rust tests first.
3. **Differential:** run the Rust oracle and native-stage0 producer with the
   same fixture ID, source commit, target, and fallback/network prohibition.
4. **Runtime:** validate Wasm, execute the standalone artifact, and compare
   stdout/stderr/exit plus resource/ABI observations. A summary/header or IR
   snapshot alone is insufficient.
5. **Targets and cleanup:** run Mac Apple Silicon and Linux x86_64 current-source
   gates where the fixture claims support. Reuse one task-owned VM lock and
   remove only task-owned artifacts, processes, and temporary directories.
6. **Evidence:** update the evidence index, ADR, and active TODO only after the
   requirement scope matches every observation. Keep partial parity as `[~]`.

## Consequences

- Runtime behavior, memory safety, ABI shape, and value semantics can be
  reviewed together without requiring a single monolithic implementation.
- Existing record/ADT/GC/I/O work remains reusable as focused evidence, but it
  cannot close the native, artifact, runtime, or two-target boundary by itself.
- The V4-M1-01 matrix remains the selection mechanism; this ADR adds the
  required observations and task order rather than creating a parallel runner.
- Metrics and ABI fields become versioned public evidence. Producers must fail
  closed when they cannot observe them instead of fabricating success.

## Current verified slice (2026-08-01)

- `valid/nested-record-pattern` is the first `V4-M1-03-R1` fixture. It keeps
  nested record variable patterns on the supported path and declares the
  AST/type/IR/ftable/import/Wasm/runtime/report observations in the V4 matrix.
- The matrix RED→GREEN contract is covered by
  `python3 scripts/ci/test-semantic-fixture-matrix.py` (13 tests). The Rust
  oracle producer, with fixture implementation commit
  `4790bb3e647d03b2ccfa883bc502e40d2385865f`
  and target-declared `aarch64-apple-darwin`, observed exit `0`, stdout
  `41\n1\n7\n`, Wasm size `6822`, and digest
  `sha256:370c8ea8332a147ab5614c4062421c3dcad2957c0004d022678c51f2e762e7a`.
  `wasm-tools validate` also passed. The current Rust report observes the
  artifact/runtime fields; ftable/import byte observations remain pending.
- A literal record-field pattern currently returns explicit `LS3001` unsupported
  representation in the Rust compiler. The dedicated
  `invalid/record-field-pattern-literal` fixture now fixes that boundary at
  line 8, columns 19–21, exit `1`, with no artifact or runtime. The Rust report
  producer accepts both the existing `(start..end)` form and the compiler's
  multiline `Span { start: …, end: … }` form; missing spans still fail closed.
- `valid/map-collections` extends `V4-M1-03-R1` with three map inserts and
  deterministic size/membership observations (`3\n1\n0\n`, exit `0`). The Rust
  oracle at target-declared `aarch64-apple-darwin` observed a valid Wasm
  artifact of 7,318 bytes with digest
  `sha256:a1630630ca3e9fcde823ed3532d5c51a146dcb0d7b4ad9d4171980674a69345c`;
  Wasmtime 43.0.0 validation and standalone execution passed.
- `valid/closure-allocation` is the first `V4-M1-03-R2` fixture. It captures the
  heap string `keep!` in a closure, runs an allocating `churn` helper 256 times,
  and then calls the closure through `apply`; the expected runtime is `5\n`,
  exit `0`, with no diagnostics. The Rust oracle at target-declared
  `aarch64-apple-darwin` observed a valid Wasm artifact of 7,148 bytes with
  digest `sha256:5713540aa1993830c2629aeaa4d5f24ce6bdaed0eb5422dd51201939a311e91f`;
  Wasmtime 43.0.0 validation and standalone execution passed.
- `valid/free-list-growth` is the first `V4-M1-03-R3` fixture. It performs 4,097
  unrooted `__alloc 8` calls, crossing the current initial free-list capacity,
  and returns the completed allocation count (`4097\n`, exit `0`). The Rust
  oracle at target-declared `aarch64-apple-darwin` observed a valid Wasm
  artifact of 6,557 bytes with digest
  `sha256:184ca6b1c66604b13b5e78560a06fac99e7d28f5673360d6282712ab7c138bff`;
  Wasmtime 43.0.0 validation and standalone execution passed.
- `valid/argv-program-only` is the first `V4-M1-03-R4` fixture. With no user
  arguments, WASI still exposes the deterministic program-name entry, so
  `command-line-args` returns `1` (`1\n`, exit `0`). The Rust oracle at
  target-declared `aarch64-apple-darwin` observed a valid Wasm artifact of
  6,498 bytes with digest
  `sha256:5b6f2251feac0697d5c22f849a43cf15209e959320ef978c5806b312c0c6ab51`;
  Wasmtime 43.0.0 validation and standalone execution passed.
- `valid/io-read-file` extends `V4-M1-03-R4` with an explicit runtime input
  snapshot: the matrix projects `{\"input.txt\": \"payload\"}` and rejects
  unsafe paths or non-string content. The Rust oracle at source commit
  `708bc6f5775f69a849cd8b683778eb1963928361` and target-declared
  `aarch64-apple-darwin` observed a valid Wasm artifact of 6,583 bytes with
  digest
  `sha256:843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48`;
  Wasmtime 43.0.0 with the task-owned directory preopened returned `payload`,
  exit `0`, and empty stderr. `wasm-tools 1.245.1 validate` passed. The
  producer refuses to overwrite an existing input and the report remains
  task-local until a final-main evidence run.
- `valid/io-read-stdin` extends `V4-M1-03-R4` with the same explicit boundary
  for stdin: the matrix projects a UTF-8 `runtime_stdin` snapshot and both
  report producers pass it as child stdin without inheriting a host stream.
  The Rust oracle at implementation commit
  `dd459fd10bf9aa391a9b7b7bd971226a5dea1eb5` and target-declared
  `aarch64-apple-darwin` observed a valid Wasm artifact of 6,498 bytes with
  digest
  `sha256:2d96798a5befcf678b898ab375462cba4095668fd81b3e2cac3377867e0abe72`;
  Wasmtime 43.0.0 returned `payload`, exit `0`, and empty stderr, and
  `wasm-tools 1.245.1 validate` passed. Native/Linux parity remains pending;
  the report is task-local until a final-main evidence run.
- This is Rust-oracle evidence only. Native stage0 execution, Linux x86_64,
  ftable/import byte parity, root/resource telemetry, limit-boundary diagnostics,
  file/stdin/short-read ABI coverage, and the two-target completion audit remain
  pending, so R1, R2, R3, R4, and V4-M1-03 stay `[~]`. The report is
  task-local and is not attached to the evidence index; regenerate it at the
  final main SHA before attaching it.

## Evidence and remaining work

- The V4-M1-01 matrix, Rust/native report producers, evidence schema, audit, and
  runbook are prerequisites and are already present as verified tooling slices.
- This ADR supplies design and task decomposition only. No V4-M1-03 fixture,
  resource/ABI schema extension, native target run, Wasm/runtime parity, or
  two-target completion evidence is claimed here.
- V4-M1-03 remains `[~]` until `R1`–`R5` have requirement-scoped evidence and
  the completion audit can distinguish value success from resource/ABI success.

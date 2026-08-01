# ADR: v0.4 M1-01 Mac Rust-oracle valid fixture batch

## Status

Accepted for the verified partial evidence slice (2026-08-01, source commit
`ed72cb59987dfb8523886f775ab9170ecc436cc6`). This ADR does not complete
V4-M1-01, native stage0 parity, or either-target completion gate.

This is the historical valid-only run. The current-source full-matrix result
is recorded in [`Mac Rust-oracle current-source full batch ADR`](decisions-v0.4-m1-01-rust-oracle-current-source-full-batch.md).

## Context

The semantic fixture matrix and producer contract were previously tested only
with synthetic reports. The next-version evidence loop needs at least one
current-source Rust-oracle execution that produces real Wasm bytes and runtime
output. The first run also exposed a stale manifest expectation: `examples/module.ls`
computes `3 * 4 + 5`, so its output is `17`, not the declared `41`.

## Decision

- Treat the current source commit, an explicitly built `lsharp` compiler, and
  an explicitly selected Wasmtime as the only inputs to the Rust-oracle lane.
- Record the 14 valid matrix fixtures as a Mac Apple Silicon (`aarch64-apple-darwin`)
  partial evidence batch. Every fixture must produce a regular Wasm artifact,
  validate with `wasm-tools`, and run with exit code `0` and the manifest's
  expected stdout/stderr.
- Correct `valid/module-import`'s expected stdout to `17\n`; the RED contract
  test prevents the source fixture and matrix expectation from drifting again.
- Keep invalid coverage, native-stage0 execution, Linux x86_64 execution, and
  Rust/native differential comparison as explicit pending boundaries.

## Evidence

Compiler: `lsharp 0.1.0`, built with
`CARGO_TARGET_DIR=.../lsharp-v4-m1-01-rust-oracle-smoke/target cargo build --release -p lsharp-driver --bin lsharp`.
Runtime: `wasmtime 43.0.0`; producer: `scripts/ci/semantic_fixture_rust_report.py`.
All reports used source commit
`ed72cb59987dfb8523886f775ab9170ecc436cc6`, target
`aarch64-apple-darwin`, task-owned work directories, and the producer's
`LSHARP_DISABLE_EMBEDDED_COMPONENT=1` fallback guard.

| fixture | Wasm bytes | SHA-256 | stdout | exit |
|---|---:|---|---|---:|
| `valid/adt-pattern` | 6860 | `18bde1b73df395f54b74cae04c5986b6169a3e31d60f44b056051ad8740f6ed8` | `42\n0\n` | 0 |
| `valid/argv-program-only` | 6498 | `5b6f2251feac0697d5c22f849a43cf15209e959320ef978c5806b312c0c6ab51` | `1\n` | 0 |
| `valid/closure-allocation` | 7148 | `5713540aa1993830c2629aeaa4d5f24ce6bdaed0eb5422dd51201939a311e91f` | `5\n` | 0 |
| `valid/free-list-growth` | 6557 | `184ca6b1c66604b13b5e78560a06fac99e7d28f5673360d6282712ab7c138bff` | `4097\n` | 0 |
| `valid/io-read-file` | 6583 | `843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48` | `payload` | 0 |
| `valid/io-read-file-empty` | 6583 | `843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48` | `` | 0 |
| `valid/io-read-file-missing` | 6583 | `843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48` | `` | 0 |
| `valid/io-read-stdin` | 6498 | `2d96798a5befcf678b898ab375462cba4095668fd81b3e2cac3377867e0abe72` | `payload` | 0 |
| `valid/map-collections` | 7318 | `a1630630ca3e9fcde823ed3532d5c51a146dcb0d7b4ad9d4171980674a69345c` | `3\n1\n0\n` | 0 |
| `valid/module-import` | 6548 | `ea4316dee98dd1e856cb76bc8f548031a92dd71ccf71dfa0950b036e7b8cb613` | `17\n` | 0 |
| `valid/nested-record-pattern` | 6822 | `370c8ea8332a147ab5614c4062421c3dcad2957c0004d022678c51f2e762e7a8` | `41\n1\n7\n` | 0 |
| `valid/record-accessor` | 6639 | `f67cf8f154a0fa39b040f985421f7871708abdad72500deae8d76157aa767107` | `10\n` | 0 |
| `valid/recursive-runtime` | 6543 | `281bd213afee3e7687490eb2de1605573c450a5a53e9a8f3a4ca4652dbed0017` | `55\n` | 0 |
| `valid/syntax-basic` | 6498 | `9c7b6a778439dff5abc70db7c67f5359536894c4329b5abc1fff2c57f5213811` | `42\n` | 0 |

`invalid/record-field-pattern-literal` was also executed through the Rust
producer and returned `LS3001`, line 8 columns 19–21, exit `1`, with no
artifact/runtime. The remaining invalid fixtures intentionally stay pending
when their compiler output lacks an explicit code or byte span.

Commands:

- `python3 scripts/ci/test-semantic-fixture-matrix.py` — 20 tests, including
  the module-import source/expectation RED/GREEN contract.
- `python3 scripts/ci/semantic_fixture_rust_report.py` — 14 valid fixtures and
  the explicit invalid fixture report.
- `/Users/biwakonbu/.cargo/bin/wasm-tools validate` — all 14 Wasm artifacts.

## Consequences

The Mac Rust-oracle lane now has real current-source artifact/runtime evidence
for every valid matrix fixture, and the module-import expectation is no longer
false. This is still not native-stage0 evidence: the corresponding native
report, Linux target, invalid full set, and differential/aggregate gates remain
`[~]` in the milestone and TODO.

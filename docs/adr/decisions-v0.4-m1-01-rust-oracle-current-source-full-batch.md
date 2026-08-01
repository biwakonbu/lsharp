# ADR: v0.4 M1-01 Mac Rust-oracle current-source full batch

## Status

Accepted for the verified Mac Rust-oracle full-matrix slice (2026-08-01,
source commit `8af9af3c30b8260700ca6b7b05030a56c42805e3`). The earlier valid and
invalid batch ADRs remain as historical run records; this ADR is the current
source projection. It does not complete V4-M1-01, native stage0 parity, Linux
x86_64 execution, Rust/native differential, or two-target aggregate evidence.

## Context

The valid and invalid Rust-oracle runs had previously been recorded at
different source commits (`ed72cb...` and `6943f4...`). That was useful partial
evidence but could not be treated as one current-source fixture matrix. The
next-version contract requires one deterministic fixture selection, explicit
compiler/Wasmtime paths, no embedded fallback, and an evidence record whose
source commit matches the checkout used to produce every observation.

## Decision

- Run all 19 fixtures in `scripts/ci/semantic-fixture-matrix.json` in one
  deterministic Rust-oracle batch at the current checkout's source commit.
- Keep `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` and explicit absolute compiler and
  Wasmtime paths as producer boundaries.
- Require every valid fixture to produce a regular Wasm artifact, pass
  `wasm-tools validate`, and match the manifest's runtime exit/stdout/stderr.
- Require every invalid fixture to expose the expected `LS####` code and source
  span, exit `1`, no artifact, and no runtime execution. Missing fields remain
  fail-closed rather than synthetic.
- Treat this as Mac Rust-oracle evidence only. Native stage0, Linux x86_64,
  differential, and aggregate gates remain separate completion boundaries.

## Evidence

Compiler: `lsharp 0.1.0`, built with
`cargo build --release -p lsharp-driver --bin lsharp` in a task-owned target
directory. Runtime: Wasmtime `43.0.0`; Wasm validation:
`/Users/biwakonbu/.cargo/bin/wasm-tools validate`. Producer:
`scripts/ci/semantic_fixture_rust_report.py`, target
`aarch64-apple-darwin`, source commit
`8af9af3c30b8260700ca6b7b05030a56c42805e3`.

### Invalid fixtures

| fixture | diagnostic | source span | exit | artifact | runtime |
|---|---|---|---:|---|---|
| `invalid/lexer-unexpected-character` | `LS0001` | line 1 columns 1–2 | 1 | not-applicable | not-run |
| `invalid/module-not-found` | `LS3102` | line 1 columns 1–23 | 1 | not-applicable | not-run |
| `invalid/parser-unexpected-eof` | `LS0102` | line 1 columns 1–14 | 1 | not-applicable | not-run |
| `invalid/record-field-pattern-literal` | `LS3001` | line 8 columns 19–21 | 1 | not-applicable | not-run |
| `invalid/type-undefined-value` | `LS1001` | line 1 columns 16–29 | 1 | not-applicable | not-run |

### Valid fixtures

All 14 artifacts passed `wasm-tools validate` and Wasmtime exited `0`.

| fixture | Wasm bytes | SHA-256 | stdout |
|---|---:|---|---|
| `valid/adt-pattern` | 6860 | `18bde1b73df395f54b74cae04c5986b6169a3e31d60f44b056051ad8740f6ed8` | `42\n0\n` |
| `valid/argv-program-only` | 6498 | `5b6f2251feac0697d5c22f849a43cf15209e959320ef978c5806b312c0c6ab51` | `1\n` |
| `valid/closure-allocation` | 7148 | `5713540aa1993830c2629aeaa4d5f24ce6bdaed0eb5422dd51201939a311e91f` | `5\n` |
| `valid/free-list-growth` | 6557 | `184ca6b1c66604b13b5e78560a06fac99e7d28f5673360d6282712ab7c138bff` | `4097\n` |
| `valid/io-read-file` | 6583 | `843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48` | `payload` |
| `valid/io-read-file-empty` | 6583 | `843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48` | empty |
| `valid/io-read-file-missing` | 6583 | `843524e4a13a230bfdf184c0392ab6a2eda9a422fb16c9d6eb48875f7267fb48` | empty |
| `valid/io-read-stdin` | 6498 | `2d96798a5befcf678b898ab375462cba4095668fd81b3e2cac3377867e0abe72` | `payload` |
| `valid/map-collections` | 7318 | `a1630630ca3e9fcde823ed3532d5c51a146dcb0d7b4ad9d4171980674a69345c` | `3\n1\n0\n` |
| `valid/module-import` | 6548 | `ea4316dee98dd1e856cb76bc8f548031a92dd71ccf71dfa0950b036e7b8cb613` | `17\n` |
| `valid/nested-record-pattern` | 6822 | `370c8ea8332a147ab5614c4062421c3dcad2957c0004d022678c51f2e762e7a8` | `41\n1\n7\n` |
| `valid/record-accessor` | 6639 | `f67cf8f154a0fa39b040f985421f7871708abdad72500deae8d76157aa767107` | `10\n` |
| `valid/recursive-runtime` | 6543 | `281bd213afee3e7687490eb2de1605573c450a5a53e9a8f3a4ca4652dbed0017` | `55\n` |
| `valid/syntax-basic` | 6498 | `9c7b6a778439dff5abc70db7c67f5359536894c4329b5abc1fff2c57f5213811` | `42\n` |

The producer report contained exactly the 19 manifest fixture IDs, with exact
expected diagnostics and runtime values. The durable batch command is the
runbook command in
[`v4-m1-semantic-fixture-evidence.md`](../development/operations/v4-m1-semantic-fixture-evidence.md)
with all 19 fixture IDs selected, `--target aarch64-apple-darwin`, and
`--source-commit 8af9af3c30b8260700ca6b7b05030a56c42805e3`.

## Consequences

V4-M1-01 now has one current-source Mac Rust-oracle report covering the full
fixture matrix: 14 valid artifact/runtime observations and 5 invalid
diagnostic observations. This is still a verified partial slice; native stage0,
Linux x86_64, Rust/native differential, and two-target aggregate evidence stay
`[~]` and must not be promoted by this Mac-only result.

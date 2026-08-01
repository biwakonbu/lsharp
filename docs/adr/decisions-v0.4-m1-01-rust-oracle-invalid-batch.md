# ADR: v0.4 M1-01 Mac Rust-oracle invalid fixture batch

## Status

Accepted for the verified partial evidence slice (2026-08-01, source commit
`3b6039fcd3f91e5d5c266aaeaa2f87af7c349948`). This ADR does not complete
V4-M1-01, native stage0 parity, or either-target completion gate.

## Context

The invalid half of the semantic fixture matrix must not turn missing compiler
diagnostics into synthetic evidence. The Rust producer requires both an
`LS####` code and a source byte span; the remaining invalid fixtures therefore
need to be classified individually rather than hidden behind one batch result.

## Decision

- Run every invalid fixture in its own task-owned work directory and report
  only an explicit compiler code/span pair.
- Treat `invalid/record-field-pattern-literal` and
  `invalid/type-undefined-value` as observed Rust-oracle partial evidence.
- Keep `invalid/lexer-unexpected-character`, `invalid/module-not-found`, and
  `invalid/parser-unexpected-eof` pending. The producer refuses them because
  the current Rust diagnostics omit a code or a byte span; no synthetic value
  is permitted.
- Preserve the matrix's expected codes/spans for the three pending fixtures so
  the missing diagnostic parity remains an actionable implementation task.

## Evidence

Compiler: `lsharp 0.1.0`, Wasmtime: `43.0.0`, producer:
`scripts/ci/semantic_fixture_rust_report.py`, target:
`aarch64-apple-darwin`, source commit:
`3b6039fcd3f91e5d5c266aaeaa2f87af7c349948`.

| fixture | result | observed diagnostic | exit |
|---|---|---|---:|
| `invalid/lexer-unexpected-character` | pending: code missing | — | — |
| `invalid/module-not-found` | pending: span missing | — | — |
| `invalid/parser-unexpected-eof` | pending: code missing | — | — |
| `invalid/record-field-pattern-literal` | observed | `LS3001`, line 8 columns 19–21 | 1 |
| `invalid/type-undefined-value` | observed | `LS1001`, line 1 columns 16–29 | 1 |

Both observed fixtures produced no Wasm artifact and no runtime execution, and
their code/span/exit values exactly matched the matrix expectations. The three
refused reports returned non-zero producer status with explicit messages:
`Rust oracle diagnostic code is missing` or
`Rust oracle diagnostic span is missing`.

Commands:

- `CARGO_TARGET_DIR=.../lsharp-v4-m1-01-rust-oracle-invalid/target cargo build --release -p lsharp-driver --bin lsharp`
- `python3 scripts/ci/semantic_fixture_rust_report.py` once per invalid fixture
- a direct report/manifest comparison for the two observed fixtures
- `python3 scripts/ci/test-semantic-fixture-rust-report.py` — 11 contract tests

## Consequences

Two of five invalid fixtures now have current-source Mac Rust-oracle evidence,
while the three missing-diagnostic cases are visible as a concrete parser/
diagnostic parity gap. V4-M1-01 remains `[~]`; native stage0, Linux x86_64,
full invalid parity, and Rust/native differential evidence remain pending.

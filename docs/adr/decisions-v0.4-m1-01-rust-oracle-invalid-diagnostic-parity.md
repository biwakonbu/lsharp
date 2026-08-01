# ADR: v0.4 M1-01 Rust-oracle invalid diagnostic parity

## Status

Accepted for the verified Mac Rust-oracle diagnostic slice (2026-08-01,
source commit `6943f488a213e63b5612eeabefe106357c922427`). This ADR does not
complete V4-M1-01, native stage0 parity, Linux x86_64 execution, or the
Rust/native differential and aggregate gates.

The earlier invalid-batch classification in
[`Mac Rust-oracle invalid batch ADR`](decisions-v0.4-m1-01-rust-oracle-invalid-batch.md)
is historical; this ADR records the subsequent diagnostic-parity fix and
current-source observation.

## Context

The invalid half of the semantic fixture matrix is evidence only when the
compiler exposes both an `LS####` diagnostic code and a source byte span. The
formatter previously discarded parser/lexer codes, parser EOF errors did not
carry a span, and missing-module errors stopped at the module-graph boundary
without the import declaration span. The Rust report producer therefore
correctly rejected three fixtures even though their expected diagnostics were
already fixed in the matrix.

## Decision

- Preserve the parser/lexer `LS####` code when `format_source` crosses the
  compile boundary.
- Give `UnexpectedEof` a normalized span. EOF is placed immediately after the
  last meaningful token so trailing whitespace/newlines do not move the
  diagnostic outside the source expression; recovering declarations use their
  start as a fallback span.
- Keep the existing `ModuleNotFound` API and add a source-aware
  `ModuleNotFoundAt` variant. The module resolver obtains the matching import
  declaration span from the source and exposes the same `LS3102` code.
- Cover the contract at the compile boundary with lexer, parser-EOF, and
  missing-module regression tests. The report producer remains fail-closed;
  no diagnostic value is synthesized when code or span is absent.

## Evidence

Implementation and tests were committed at
`6943f488a213e63b5612eeabefe106357c922427` and pushed to `origin/main`.
The Mac Apple Silicon Rust-oracle producer was then run with the compiler at
`target/release/lsharp`, Wasmtime `43.0.0`, target `aarch64-apple-darwin`, and
the five invalid fixture IDs in deterministic order.

| fixture | diagnostic | source span | exit | artifact | runtime |
|---|---|---|---:|---|---|
| `invalid/lexer-unexpected-character` | `LS0001` | line 1 columns 1–2 | 1 | not-applicable | not-run |
| `invalid/module-not-found` | `LS3102` | line 1 columns 1–23 | 1 | not-applicable | not-run |
| `invalid/parser-unexpected-eof` | `LS0102` | line 1 columns 1–14 | 1 | not-applicable | not-run |
| `invalid/record-field-pattern-literal` | `LS3001` | line 8 columns 19–21 | 1 | not-applicable | not-run |
| `invalid/type-undefined-value` | `LS1001` | line 1 columns 16–29 | 1 | not-applicable | not-run |

The durable reproduction command is:

```bash
python3 scripts/ci/semantic_fixture_rust_report.py \
  --manifest scripts/ci/semantic-fixture-matrix.json \
  --root "$ROOT" \
  --fixture-id invalid/lexer-unexpected-character \
  --fixture-id invalid/module-not-found \
  --fixture-id invalid/parser-unexpected-eof \
  --fixture-id invalid/record-field-pattern-literal \
  --fixture-id invalid/type-undefined-value \
  --target aarch64-apple-darwin \
  --source-commit 6943f488a213e63b5612eeabefe106357c922427 \
  --compiler "$ROOT/target/release/lsharp" \
  --wasmtime /Users/biwakonbu/.wasmtime/bin/wasmtime \
  --work-dir "$ROOT/ci-artifacts/v4-m1-01/6943f488a213e63b5612eeabefe106357c922427/aarch64-apple-darwin/oracle-work" \
  --runtime-dir "$ROOT/ci-artifacts/v4-m1-01/6943f488a213e63b5612eeabefe106357c922427/aarch64-apple-darwin/runtime" \
  --output "$ROOT/ci-artifacts/v4-m1-01/6943f488a213e63b5612eeabefe106357c922427/aarch64-apple-darwin/oracle-invalid.json"
```

## Consequences

All five invalid fixtures now have current-source Mac Rust-oracle
code/span/exit/no-artifact evidence. V4-M1-01 remains `[~]`: native stage0,
Linux x86_64, Rust/native differential, and two-target aggregate evidence are
still pending.

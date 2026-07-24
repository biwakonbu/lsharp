# WASI runtime recursion stack limit validation

- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 D-3 / Rust-oracle WASI runtime

## Contract

The E2E fixture uses a non-tail recursive function so each recursive call consumes Wasm call stack. With Wasmtime `max_wasm_stack = 64 KiB`:

- recursion depths 0, 32, and 128 complete and print the input depth;
- recursion depth 100,000 fails before producing a result;
- the failure is reported as a runtime trap containing both `stack` and `trap`.

The fixture intentionally avoids tail recursion because tail-recursive source can be lowered into a loop and would not exercise the Wasm call-stack boundary.

## Reproduction

```bash
bash scripts/ci/test-runtime-recursion-limits-contract.sh
scripts/ci/test-runtime-recursion-limits.sh
```

The test helper creates a dedicated Wasmtime engine for the fixture; the normal public WASI runner stack configuration is not changed.

## Scope and remaining evidence

This is a Rust/WASI runtime limit slice on the current host. It does not prove the exact maximum frame count, Linux x86_64 native stage0 behavior, Component/HTTP parity, or a product-level configurable recursion limit. Those remain separate evidence requirements.

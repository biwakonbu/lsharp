# ADR: WasmGC descriptor metadata-hash の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs` / `wasi:filesystem/types@0.2.3`

## Context

`descriptor.metadata-hash` は filesystem object metadata の 128-bit hash を
`metadata-hash-value { lower: u64, upper: u64 }` として返す。synthetic import の接続だけでは、
result discriminant、record payload の canonical offsets、同一 descriptor での安定性を証明できない。

## Decision

実際の `wasmgc-cli-fs` Component から `source.txt` を read-only descriptor として開き、
`descriptor.metadata-hash` を二回呼び出す。二つの success result の `lower` / `upper` payload が
一致することを guest linear memory で確認し、descriptor と preopen descriptor を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_reads_stable_descriptor_metadata_hash_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture`
- Expected boundary: both `metadata-hash` calls return success, the 128-bit payload is stable, host bytes
  remain `hello`, stdout is empty, and exit code is 0.

## Residual risk

これは descriptor metadata-hash の verified partial slice である。`metadata-hash-at`、
descriptor-stat variants、remaining filesystem operations、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate
completion とは扱わない。

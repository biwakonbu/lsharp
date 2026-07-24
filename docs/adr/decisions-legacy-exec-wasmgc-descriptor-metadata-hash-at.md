# ADR: WasmGC descriptor metadata-hash-at の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs` / `wasi:filesystem/types@0.2.3`

## Context

`descriptor.metadata-hash-at` は directory descriptor と相対 path から filesystem object metadata の
128-bit hash を返す。synthetic import の接続だけでは、path-flags、string lowering、result record の
canonical offsets、同一 path での安定性を証明できない。

## Decision

実際の `wasmgc-cli-fs` Component から named preopen の directory descriptor に path-flags `0` と
`source.txt` を渡し、`descriptor.metadata-hash-at` を二回呼び出す。二つの success result の
`lower` / `upper` payload が一致することを guest linear memory で確認し、directory descriptor を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_reads_stable_metadata_hash_at_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture`
- Expected boundary: both calls return success, the 128-bit payload is stable, host bytes remain `hello`,
  stdout is empty, and exit code is 0.

## Residual risk

これは descriptor metadata-hash-at の verified partial slice である。`stat-at`、`set-times-at`、
remaining filesystem operations、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64
native evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

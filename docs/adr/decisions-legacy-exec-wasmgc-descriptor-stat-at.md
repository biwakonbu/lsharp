# ADR: WasmGC descriptor stat-at の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs` / `wasi:filesystem/types@0.2.3`

## Context

`descriptor.stat-at` は directory descriptor と相対 path から `descriptor-stat` record を返す。
synthetic import の接続だけでは、path-flags、string lowering、record payload の canonical offsets、
file type/size の実値を証明できない。

## Decision

実際の `wasmgc-cli-fs` Component から named preopen の directory descriptor に path-flags `0` と
`source.txt` を渡し、`descriptor.stat-at` の success result を読む。record の regular-file type
`6` と size `5` を guest linear memory で確認し、directory descriptor を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_stats_file_at_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture`
- Expected boundary: `stat-at` succeeds with type 6 and size 5, host bytes remain `hello`, stdout is empty,
  and exit code is 0.

## Residual risk

これは descriptor stat-at の verified partial slice である。timestamp fields、`set-times-at`、
remaining filesystem operations、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64
native evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

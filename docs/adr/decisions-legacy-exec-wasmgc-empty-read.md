# ADR: WasmGC input-stream empty source read の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`input-stream.read` は non-blocking で、空の入力源からの読み取りは stream error ではなく
success の空 list で表現される。empty source を failure と誤分類しないことを、regular-file EOF
とは別の fixture で確認する必要がある。

## Decision

空の `input.txt` を read-only named preopen から開き、`read-via-stream(0)` で input stream を取得する。
`read(1)` の result は success、list length は 0 であることを確認し、marker `Z` を stdout に渡す。
最後に input stream、descriptor、preopen を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_reads_empty_input_stream_as_empty_success`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_reads_empty_input_stream_as_empty_success -- --nocapture`
- Expected boundary: empty source の `read(1)` は success + empty list、stdout is `Z`、exit code is 0。

## Residual risk

これは empty source の verified partial slice である。stream error/closed、複数回の partial read、
poll readiness、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native evidence、
native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

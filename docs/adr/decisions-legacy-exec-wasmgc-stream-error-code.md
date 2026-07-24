# ADR: WasmGC stream-error last-operation-failed の filesystem error-code downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` /
  `wasi:filesystem/types@0.2.3`

## Context

`input-stream.blocking-read` が返す `stream-error::last-operation-failed` の error resource と、
filesystem-specific な `error-code` への downcast が WasmGC Component の実 resource table 上で未検証だった。
EOF/closed とは異なる stream failure を、成功扱いや曖昧な診断にせず固定する必要がある。

## Decision

read-only preopen の descriptor から offset `u64::MAX` の input stream を作り、
`blocking-read(1)` を呼ぶ。outer result が error、`stream-error` が
`last-operation-failed` であることを確認し、その error resource を borrowed
`filesystem-error-code` に渡す。結果は `Some(error-code::invalid)` でなければ失敗とし、成功時は marker `E`、
`wasi:cli/run` exit code `0` とする。error、input stream、descriptor、preopen は同じ実行内で drop する。

## Evidence

- `crates/lsharp-wasm/tests/wasmgc_probe.rs`
  - `wasm_gc_component_cli_fs_runner_maps_stream_failure_to_filesystem_error_code`
  - `emit_component_cli_stream_failure_probe_module`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_maps_stream_failure_to_filesystem_error_code -- --nocapture`

## Residual risk

write/flush 由来の `last-operation-failed`、他の OS error mapping、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64 の native gate、native/selfhost parity は未検証である。

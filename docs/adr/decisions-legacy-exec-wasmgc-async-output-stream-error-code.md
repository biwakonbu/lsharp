# ADR: WasmGC 非同期 output-stream write 後の filesystem error-code downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` /
  `wasi:filesystem/types@0.2.3`

## Context

output stream の `blocking-write-and-flush` による同期 failure downcast に続き、non-blocking
`write` が開始した filesystem I/O の失敗が、完了待ちの `blocking-flush` を通って
`stream-error::last-operation-failed` として観測できるか未検証だった。pending operation の状態遷移と
error resource の lifecycle を固定する必要がある。

## Decision

read-write preopen の `output.txt` に offset `u64::MAX` の output stream を作り、`check-write` の
permit を確認して `write(1)` を開始する。続く `blocking-flush` が outer error、
`stream-error::last-operation-failed` であることを確認し、その error resource を borrowed
`filesystem-error-code` に渡す。結果は `Some(error-code::invalid)` でなければ失敗とし、成功時は marker `A`、
`wasi:cli/run` exit code `0` とする。error、output stream、descriptor、preopen は同じ実行内で drop する。

## Evidence

- `crates/lsharp-wasm/tests/wasmgc_probe.rs`
  - `wasm_gc_component_cli_fs_runner_maps_async_output_stream_failure_to_filesystem_error_code`
  - `emit_component_cli_async_output_stream_failure_probe_module`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_maps_async_output_stream_failure_to_filesystem_error_code -- --nocapture`

## Residual risk

pending write 後の `check-write` / non-blocking `flush` projection、他の OS error mapping、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64 の native gate、native/selfhost parity は未検証である。

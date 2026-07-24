# ADR: WasmGC output-stream last-operation-failed の filesystem error-code downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` /
  `wasi:filesystem/types@0.2.3`

## Context

input stream の failure downcast に続き、filesystem descriptor から作った output stream の
`blocking-write-and-flush` が返す `stream-error::last-operation-failed` と、filesystem-specific な
`error-code` への downcast が未検証だった。出力側でも invalid filesystem operation を成功扱いにせず、
error resource の lifecycle を含む契約を固定する必要がある。

## Decision

read-write preopen の `output.txt` に offset `u64::MAX` の output stream を作り、
`blocking-write-and-flush` を呼ぶ。outer result が error、`stream-error` が
`last-operation-failed` であることを確認し、その error resource を borrowed
`filesystem-error-code` に渡す。結果は `Some(error-code::invalid)` でなければ失敗とし、成功時は marker `O`、
`wasi:cli/run` exit code `0` とする。error、output stream、descriptor、preopen は同じ実行内で drop する。

## Evidence

- `crates/lsharp-wasm/tests/wasmgc_probe.rs`
  - `wasm_gc_component_cli_fs_runner_maps_output_stream_failure_to_filesystem_error_code`
  - `emit_component_cli_output_stream_failure_probe_module`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_maps_output_stream_failure_to_filesystem_error_code -- --nocapture`

## Residual risk

非同期 `write` 後の `flush`/`check-write` error、他の OS error mapping、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64 の native gate、native/selfhost parity は未検証である。

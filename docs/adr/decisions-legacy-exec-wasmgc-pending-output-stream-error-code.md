# ADR: WasmGC pending output-stream failure の filesystem error-code downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` / `wasi:filesystem/types@0.2.3`

## Context

`output-stream.write` が開始した非同期 filesystem I/O の失敗は、`blocking-flush` 経由では
検証済みだった。一方、`output-stream.subscribe` で pollable を取得して完了を待ち、続く
`check-write` で pending failure を観測する経路は未検証だった。

## Decision

read-write preopen の `output.txt` に offset `u64::MAX` の output stream を作り、
`check-write` の permit を確認して 1 byte の `write` を開始する。その後
`output-stream.subscribe` → `pollable.block` → `check-write` を実行し、次を契約とする。

- outer result は error である
- `stream-error` は `last-operation-failed`（discriminant `0`）である
- `result<u64, stream-error>` の canonical ABI payload alignment に従い、stream-error case は
  result pointer `+8`、error handle は `+12` にある
- error を borrowed `filesystem-error-code` に渡すと `Some(error-code::invalid)`（discriminant `12`）
  が返る
- marker `C`、`wasi:cli/run` exit code `0` を確認し、error/pollable/output stream/descriptor/preopen
  を drop する

## Evidence

- Test: `wasm_gc_component_cli_fs_runner_maps_pending_output_stream_failure_to_filesystem_error_code`
- Probe: `emit_component_cli_pending_output_stream_failure_probe_module`
- Gate: `cargo test -p lsharp-wasm --test wasmgc_probe` focused test passed

## Residual risk

non-blocking `flush` 後の failure projection、他の OS error mapping、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64 native gate、selfhost parity は未完了である。

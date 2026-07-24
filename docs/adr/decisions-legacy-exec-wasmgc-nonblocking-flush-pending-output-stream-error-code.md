# ADR: WasmGC non-blocking flush 後の pending output-stream failure downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` / `wasi:filesystem/types@0.2.3`

## Context

pending `output-stream.write` の failure は、`blocking-flush` と
`subscribe` → `pollable.block` → `check-write` の各経路で確認済みだった。一方、non-blocking
`flush` は pending write 中に即時 success を返し、後続の readiness boundary で failure を投影する
契約が未検証だった。

## Decision

read-write preopen の `output.txt` に offset `u64::MAX` の output stream を作り、
`check-write` の permit を確認して 1 byte の `write` を開始する。直後に non-blocking `flush` を
呼び、outer result が success であることを確認する。その後
`output-stream.subscribe` → `pollable.block` → `check-write` を実行し、次を契約とする。

- `flush` は pending I/O を待たずに success result を返す
- 後続 `check-write` は outer error を返す
- `stream-error` は `last-operation-failed`（discriminant `0`）である
- `result<u64, stream-error>` の canonical ABI payload alignment に従い、stream-error case は
  result pointer `+8`、error handle は `+12` にある
- error を borrowed `filesystem-error-code` に渡すと `Some(error-code::invalid)`（discriminant `12`）
  が返る
- marker `F`、`wasi:cli/run` exit code `0` を確認し、error/pollable/output stream/descriptor/preopen
  を drop する

## Evidence

- Test: `wasm_gc_component_cli_fs_runner_maps_nonblocking_flush_pending_output_stream_failure_to_filesystem_error_code`
- Probe: `emit_component_cli_nonblocking_flush_pending_output_stream_failure_probe_module`
- Gate: focused `cargo test -p lsharp-wasm --test wasmgc_probe` passed

## Residual risk

他の OS error mapping、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
gate、selfhost parity は未完了である。

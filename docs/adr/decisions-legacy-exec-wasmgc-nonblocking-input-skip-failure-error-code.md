# ADR: WasmGC non-blocking input-stream skip failure downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` / `wasi:filesystem/types@0.2.3`

## Context

`input-stream.skip` の成功 lifecycle は検証済みだったが、pending filesystem read が完了した後の
non-blocking skip failure と filesystem error downcast は未検証だった。

## Decision

read-only preopen の `input.txt` に offset `u64::MAX` の input stream を作り、
`input-stream.subscribe` → `pollable.block` で pending I/O を完了させる。その後
`input-stream.skip(1)` を呼び、次を契約とする。

- skip は outer error を返し、`stream-error::last-operation-failed`（discriminant `0`）になる
- `result<u64, stream-error>` の canonical ABI は stream-error case `+8`、error handle `+12` である
- error を borrowed `filesystem-error-code` に渡すと `Some(error-code::invalid)`（discriminant `12`）
  が返る
- marker `S`、`wasi:cli/run` exit code `0` を確認し、error/pollable/input stream/descriptor/preopen
  を drop する

## Evidence

- Test: `wasm_gc_component_cli_fs_runner_maps_nonblocking_input_skip_failure_to_filesystem_error_code`
- Probe: `emit_component_cli_nonblocking_input_skip_failure_probe_module`
- Gate: focused `cargo test -p lsharp-wasm --test wasmgc_probe` passed

## Residual risk

他の OS error mapping、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
gate、selfhost parity は未完了である。

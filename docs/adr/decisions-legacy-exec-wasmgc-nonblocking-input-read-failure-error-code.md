# ADR: WasmGC non-blocking input-stream read failure downcast

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` / `wasi:filesystem/types@0.2.3`

## Context

`input-stream.blocking-read` の filesystem failure は検証済みだったが、pending read を
non-blocking `input-stream.read` で観測する経路と、failure resource の downcast は未検証だった。
なお、vendored `wasi:io/streams@0.2.3` には `input-stream.check-read` が存在しないため、WIT を
拡張して API を作らない。

## Decision

read-only preopen の `input.txt` に offset `u64::MAX` の input stream を作り、
`input-stream.subscribe` → `pollable.block` で pending I/O を完了させる。その後
`input-stream.read(1)` を呼び、次を契約とする。

- read は outer error を返し、`stream-error::last-operation-failed`（discriminant `0`）になる
- `result<list<u8>, stream-error>` の canonical ABI は result case `+0`、stream-error case `+4`、
  error handle `+8` である
- error を borrowed `filesystem-error-code` に渡すと `Some(error-code::invalid)`（discriminant `12`）
  が返る
- marker `R`、`wasi:cli/run` exit code `0` を確認し、error/pollable/input stream/descriptor/preopen
  を drop する

## Evidence

- Test: `wasm_gc_component_cli_fs_runner_maps_nonblocking_input_stream_failure_to_filesystem_error_code`
- Probe: `emit_component_cli_nonblocking_input_stream_failure_probe_module`
- Gate: focused `cargo test -p lsharp-wasm --test wasmgc_probe` passed

## Residual risk

他の OS error mapping、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
gate、selfhost parity は未完了である。

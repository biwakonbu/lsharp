# ADR: WasmGC input-stream read / blocking-read の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`input-stream.read` は non-blocking で、要求長より少ない bytes、または一時的に空の list を
返し得る。`blocking-read` は少なくとも 1 byte が利用可能になるまで待つが、read と同じく
要求長以下の list を返すため、non-blocking read の結果を固定長成功として扱えない。

## Decision

実際の `wasmgc-cli-fs-streams` Component から read-only named preopen の `input.txt` (`hello`) を
開き、`read-via-stream(0)` で input stream を取得する。まず `read(0)` が success かつ空 list を
返すことを確認する。次に `read(5)` の結果 list が要求長を超えないことを確認して bytes を stdout
へ渡し、`5 - first_read_len` を `blocking-read` に渡して残りの bytes を stdout へ渡す。最後に
input stream、descriptor、preopen を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof -- --nocapture`
- Expected boundary: `read(0)` は空 list、`read(5)` と残量の `blocking-read` は success、stdout is `hello`、
  exit code is 0。

## Residual risk

これは input-stream read の verified partial slice である。stream error/closed、EOF・empty source、
複数回の partial read、poll readiness、Wasm artifact/runtime differential、Mac Apple Silicon/Linux
x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

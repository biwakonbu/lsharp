# ADR: WasmGC input-stream skip / blocking-skip の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`input-stream.skip` は non-blocking read と同じく、要求長より少ない bytes を返し得る。一方
`blocking-skip` は少なくとも 1 byte が利用可能になるまで待つため、両者を固定長の成功値だけで
扱うと、non-blocking partial result を誤って failure と判定する。

## Decision

実際の `wasmgc-cli-fs-streams` Component から read-only named preopen の `input.txt` (`hello!`) を
開き、`read-via-stream(0)` で input stream を取得する。`skip(2)` の success result を受け取り、
返された count が 2 以下であることを確認する。残りの bytes を `blocking-skip(2-count)` で消費し、
success count が要求値と一致することを確認する。最後に `blocking-read(4)` で `llo!` を stdout に
渡し、input stream、descriptor、preopen を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_skips_input_stream_then_reads_remaining_bytes`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_skips_input_stream_then_reads_remaining_bytes -- --nocapture`
- Expected boundary: non-blocking `skip` と `blocking-skip` が success になり、stdout is `llo!`、
  exit code is 0。

## Residual risk

これは input-stream skip の verified partial slice である。stream error/resource failure、EOF/zero-length
skip、poll readiness、`read` の non-blocking data contract、Wasm artifact/runtime differential、Mac
Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate
completion とは扱わない。

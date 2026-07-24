# ADR: WasmGC output-stream check-write/write/flush の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`output-stream.write` は `check-write` が返した permit 以下の長さだけを受け付ける。便利な
`blocking-write-and-flush` だけでは、readiness の permit、直接 write、非 blocking flush、blocking
flush の境界が同じ resource table で接続されていることを証明できない。

## Decision

実際の `wasmgc-cli-fs-streams` Component から read-write named preopen に `checked.txt` を
create+truncate/write で開き、`write-via-stream(0)` で output stream を取得する。stream の
`check-write` が success かつ正の permit を返すことを確認し、その permit 以下の `hello` を直接
`write` する。続けて `flush` と `blocking-flush` の success result を確認し、output stream、file
descriptor、preopen を drop する。実行後の host artifact が `hello` であることを確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_checks_writes_and_flushes_stream_then_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_checks_writes_and_flushes_stream_then_drops_resources -- --nocapture`
- Expected boundary: positive `check-write` permit、direct `write`、`flush`、`blocking-flush` が成功し、
  `checked.txt` is `hello`、stdout is empty、exit code is 0。

## Residual risk

これは output-stream readiness/write/flush の verified partial slice である。`write-zeroes` の
check-write precondition、stream error/resource failure、`subscribe`/poll readiness、`splice`、input
stream の残る operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

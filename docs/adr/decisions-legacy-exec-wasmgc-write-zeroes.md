# ADR: WasmGC output-stream write-zeroes の check-write contract 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`output-stream.write-zeroes` は bytes の payload を渡さない convenience operation だが、通常の
`write` と同じ `check-write` permit の precondition を持つ。blocking zero-fill の検証だけでは、
caller が permit を取得してから直接 zero-fill を行う contract を証明できない。

## Decision

実際の `wasmgc-cli-fs-streams` Component から read-write named preopen に `direct-zeroes.bin` を
create+truncate/write で開き、`write-via-stream(0)` で output stream を取得する。stream の
`check-write` が 4 bytes 以上の success permit を返すことを確認し、`write-zeroes(4)` を直接呼ぶ。
続けて `blocking-flush` の success result を確認し、output stream、file descriptor、preopen を drop
する。実行後の host artifact が 4 個の zero byte であることを確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_writes_zeroes_after_check_write_then_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_writes_zeroes_after_check_write_then_drops_resources -- --nocapture`
- Expected boundary: `check-write >= 4`、direct `write-zeroes(4)`、`blocking-flush` が成功し、
  `direct-zeroes.bin` is `[0, 0, 0, 0]`、stdout is empty、exit code is 0。

## Residual risk

これは output-stream direct zero-fill の verified partial slice である。stream error/resource
failure、zero-length write、`subscribe`/poll readiness、`splice`、input stream の残る operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost
parity は別途必要であり、aggregate completion とは扱わない。

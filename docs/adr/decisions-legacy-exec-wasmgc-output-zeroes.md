# ADR: WasmGC output-stream blocking-write-zeroes の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`output-stream.blocking-write-zeroes-and-flush` は output stream の readiness、zero-byte payload、
flush、stream resource lifecycle を一つの operation で閉じる。通常の bytes write の検証だけでは、
zero-fill と flush 後の host artifact を証明できない。

## Decision

実際の `wasmgc-cli-fs-streams` Component から read-write named preopen に `zeros.bin` を create+
truncate/write で開き、`write-via-stream(0)` で output stream を取得する。stream に
`blocking-write-zeroes-and-flush(3)` を呼び、success result を確認して output stream、file descriptor、
preopen を drop する。実行後の host artifact が 3 個の zero byte であることを確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_writes_zeroes_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_writes_zeroes_and_drops_resources -- --nocapture`
- Expected boundary: zeroes write/flush succeeds, `zeros.bin` is `[0, 0, 0]`, stdout is empty, and exit code is 0.

## Residual risk

これは output-stream zero-fill の verified partial slice である。`check-write` 前提の直接 `write-zeroes`、
separate `flush` / `blocking-flush`、stream error/resource failure、remaining streams operations、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は
別途必要であり、aggregate completion とは扱わない。

# ADR: WasmGC descriptor advise の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs` / `wasi:filesystem/types@0.2.3`

## Context

`descriptor.advise` は open file descriptor に offset、length、`advice` enum を渡して file
access pattern を通知する。filesystem descriptor の mutation や timestamp と異なり host bytes を
変更しないため、actual Component の enum canonical ABI、file descriptor boundary、result/drop lifecycle
を個別に検証する必要がある。

## Decision

実際の `wasmgc-cli-fs` Component から read-only named preopen の `input.txt` descriptor を開き、
offset `0`、length `5`、`advice::normal` で `descriptor.advise` を呼ぶ。success result を確認し、
file と preopen の descriptor を drop する。実行後に host bytes `hello` を再読して advisory call が
artifact を変更しないことも確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_advises_descriptor_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_advises_descriptor_and_drops_resources -- --nocapture`
- Expected boundary: `advise(normal)` succeeds for the opened regular file, host bytes remain `hello`,
  stdout is empty, and exit code is 0.

## Residual risk

これは descriptor advise の verified partial slice である。other `advice` enum variants、range/error
behavior、remaining filesystem operations、Wasm artifact/runtime differential、Mac Apple Silicon/Linux
x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

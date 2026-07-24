# ADR: WasmGC poll list の複数 ready index 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/poll@0.2.3`

## Context

`wasi:io/poll.poll` は一つ以上の ready pollable の index を result list に返す。これまでの検証は
一要素 list の index `0` に限られており、複数 child pollable の borrowed list と result projection が
未固定だった。

## Decision

空の `input.txt` から input stream を作成し、`input-stream.subscribe` を二回呼ぶ。二つの pollable を
それぞれ `block` / `ready` で確認して list `[pollable0, pollable1]` を `poll` に渡し、result list length
`2`、index `0` と `1` を guest 側で検証する。marker `P`、exit code `0`、全 resource drop を同じ actual
Component 実行で確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_polls_multiple_input_stream_pollables_as_ready`
- Focused gate:
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_polls_multiple_input_stream_pollables_as_ready -- --nocapture`
- Expected boundary: stdout `P`、exit code `0`、result indices `[0, 1]`。

## Residual risk

これは poll list の複数 ready index verified partial slice である。異なる input source、
`last-operation-failed` payload、filesystem error-code downcast、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate
completion とは扱わない。

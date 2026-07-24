# ADR: WasmGC input-stream pollable empty/EOF readiness の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3` / `wasi:io/poll@0.2.3`

## Context

`input-stream.subscribe` の child `pollable` は、bytes が読める場合だけでなく、stream の他端が
closed/EOF になった場合も ready になる。空入力を「ready ではない」と扱うと、`block` が返らず
EOF を検出できない。

## Decision

空の `input.txt` を read-only named preopen から開き、`read-via-stream(0)` → `input-stream.subscribe`
を呼ぶ。`pollable.block` 後に `pollable.ready` が true であることを確認し、marker `R` を stdout に
渡す。pollable、input stream、descriptor、preopen を drop する。同じ helper を非空 `hello` fixture
でも実行し、ready path と resource lifecycle が入力有無で共通であることを確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Tests: `wasm_gc_component_cli_fs_runner_polls_empty_input_stream_as_ready`,
  `wasm_gc_component_cli_fs_runner_subscribes_and_polls_input_stream`
- Focused gates:
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_polls_empty_input_stream_as_ready -- --nocapture`
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_subscribes_and_polls_input_stream -- --nocapture`
- Expected boundary: empty/non-empty input の `pollable.block` 後 `ready == true`、stdout is `R`、exit code is 0。

## Residual risk

これは input-stream pollable の empty/EOF readiness verified partial slice である。stream error/closed、
poll list の empty input、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

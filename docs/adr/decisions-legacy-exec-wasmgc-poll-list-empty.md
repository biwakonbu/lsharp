# ADR: WasmGC poll list empty/EOF readiness の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/poll@0.2.3`

## Context

`wasi:io/poll.poll` は borrowed pollable list のうち ready な index を返す。input stream が EOF/empty
でも pollable は ready になるため、データ bytes がないことを list の空結果や failure と誤分類してはならない。

## Decision

空の `input.txt` を read-only named preopen から開き、`read-via-stream(0)` → `input-stream.subscribe`
→ `pollable.block` → `pollable.ready` を実行する。pollable handle を一要素 list として `poll` に渡し、
result list length `1`、ready index `0` を確認する。marker `P` を stdout に渡し、pollable、input stream、
descriptor、preopen を drop する。同じ helper を非空 `hello` fixtureでも実行する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Tests: `wasm_gc_component_cli_fs_runner_polls_empty_input_stream_list_as_ready`,
  `wasm_gc_component_cli_fs_runner_polls_subscribed_input_stream_list`
- Focused gates:
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_polls_empty_input_stream_list_as_ready -- --nocapture`
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_polls_subscribed_input_stream_list -- --nocapture`
- Expected boundary: non-empty/empty input の poll result は `[0]`、stdout is `P`、exit code is 0。

## Residual risk

これは poll list の empty/EOF readiness verified partial slice である。stream error/closed、empty list
input の trap contract、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

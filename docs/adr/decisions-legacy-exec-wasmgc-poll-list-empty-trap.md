# ADR: WasmGC poll list empty input の actual Component trap 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/poll@0.2.3`

## Context

`wasi:io/poll.poll` は空の borrowed pollable list を受け取った場合に trap する契約である。
空の input stream が ready になることとは別に、list 自体が空の入力を成功や空の result list として
扱わない境界を固定する必要がある。

## Decision

空の `input.txt` から input stream と pollable を作成し、長さ `0` の pollable list を `poll` に渡す。
actual Component の実行は正常な `ExecutionOutput` を返さず、`wasi:cli/run` の error として失敗することを
受け入れる。runner が trap を成功 exit code や stdout marker に変換しないことも同じテストで確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_traps_on_empty_poll_list`
- Focused gate:
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_traps_on_empty_poll_list -- --nocapture`
- Expected boundary: empty poll list は error となり、error text に `poll` を含む。

## Residual risk

これは poll list の empty input trap verified partial slice である。stream error/closed、filesystem
error-code downcast、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native evidence、
native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

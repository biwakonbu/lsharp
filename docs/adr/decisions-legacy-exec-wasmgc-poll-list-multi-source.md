# ADR: WasmGC poll list の異なる input source 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/poll@0.2.3`

## Context

これまでの `wasi:io/poll.poll` 検証は、同じ input stream から派生した複数 pollable の ready index
projection に限られていた。異なるファイルを source とする input stream が同じ borrowed pollable
list に入り、source 順の index を返す契約は未固定だった。

## Decision

同じ read-only preopen 配下に空の `source-a.txt` と `source-b.txt` を用意する。それぞれを
別 descriptor の `read-via-stream` で input stream にし、`input-stream.subscribe` から二つの pollable
を作る。二つを `[pollable0, pollable1]` の順で `poll` に渡し、両方を `block` / `ready` で確認した上で
result list length `2`、index `0` と `1` を guest 側で検証する。二つの pollable、stream、descriptor、
preopen の drop を先に完了してから marker `P` を stdout に渡し、exit code `0` を同じ actual Component
実行で確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_polls_multiple_input_sources_as_ready`
- Focused gate:
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_polls_multiple_input_sources_as_ready -- --nocapture`
- Expected boundary: stdout `P`、exit code `0`、result indices `[0, 1]`。

## Residual risk

これは異なる input source の複数 ready index verified partial slice である。`last-operation-failed` の
payload、filesystem error-code downcast、異なる ready/error 状態の projection、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は別途必要であり、
aggregate completion とは扱わない。

# ADR: WasmGC blocking-read EOF の `stream-error::closed` 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

non-blocking `input-stream.read` は regular-file EOF で success の空 list を返す。一方、
`blocking-read` は少なくとも一 byte が読めるまで待つ API であり、EOF に到達した場合は
`stream-error::closed` を返す。両者を同じ empty-list success として扱うと、guest 側の制御フローが
壊れる。

## Decision

空の `input.txt` から input stream を作成し、`blocking-read(1)` を actual Component から呼ぶ。
outer result の error case と `stream-error` closed discriminant `1` を guest 側で確認し、marker `C`
を stdout に渡す。resource drop と正常な `wasi:cli/run` exit 0 も同じ実行で確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_blocking_reads_empty_input_stream_reports_closed`
- Focused gate:
  - `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_blocking_reads_empty_input_stream_reports_closed -- --nocapture`
- Expected boundary: stdout `C`, exit code `0`; non-blocking empty read の marker `Z` と区別する。

## Residual risk

これは blocking-read EOF/closed の verified partial slice である。`last-operation-failed` payload、
filesystem error-code downcast、複数回の partial blocking read、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate
completion とは扱わない。

# ADR: WasmGC descriptor set-times の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs` / `wasi:filesystem/types@0.2.3`

## Context

`descriptor.set-times` は open descriptor と二つの `new-timestamp` variant から file/directory の
時刻を更新する。`set-times-at` だけの検証では、descriptor-level の canonical variant ABI、
write-enabled open descriptor、result/drop boundary を証明できない。

## Decision

実際の `wasmgc-cli-fs` Component から read-write named preopen の `input.txt` descriptor を開き、
access/modify timestamp をともに `no-change` として `descriptor.set-times` を呼ぶ。success result
を確認し、file と preopen の descriptor を drop する。`no-change` は host file bytes を変更しないため、
実行後に `hello` を再読して host artifact の非変質も確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_sets_descriptor_times_without_changing_no_change_values`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_sets_descriptor_times_without_changing_no_change_values -- --nocapture`
- Expected boundary: `set-times` succeeds with both `new-timestamp::no-change` variants, host bytes remain
  `hello`, stdout is empty, and exit code is 0.

## Residual risk

これは descriptor set-times の verified partial slice である。`now` / explicit `timestamp` payload、
directory descriptor mutation、remaining filesystem operations、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate
completion とは扱わない。

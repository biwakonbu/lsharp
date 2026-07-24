# ADR: WasmGC input-stream EOF read の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`input-stream.read` は non-blocking で、入力が尽きたときは stream error ではなく success の
空 list を返す。read-to-EOF の境界を result discriminant と list length の両方で確認しないと、
EOF を stream failure と誤分類する可能性がある。

## Decision

`input.txt` (`hello`) を read-only named preopen から開き、`read-via-stream(0)` で input stream を
取得する。`read(0)`、`read(5)`、残量の `blocking-read` で `hello` を stdout に渡した直後、同じ
stream に `read(1)` を呼ぶ。EOF 呼び出しの result は success、list length は 0 であることを確認し、
marker `E` を stdout に追加する。最後に input stream、descriptor、preopen を drop する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof -- --nocapture`
- Expected boundary: EOF `read(1)` は success + empty list、stdout is `helloE`、exit code is 0。

## Residual risk

これは regular-file EOF の verified partial slice である。stream error/closed、empty source、複数回の
partial read、poll readiness、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64
native evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

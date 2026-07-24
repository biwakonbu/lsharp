# ADR: WasmGC output-stream splice / blocking-splice の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs-streams` / `wasi:io/streams@0.2.3`

## Context

`output-stream.splice` は output の `check-write`、input の read、output の write を一つの
borrowed-resource operation にまとめる。non-blocking operation は要求長より少ない bytes を返せる
ため、direct splice の success と blocking completion を同じ resource table で確認する必要がある。

## Decision

実際の `wasmgc-cli-fs-streams` Component から read-write named preopen の `input.txt` を input
descriptor として開き、`spliced.txt` を output descriptor として create+truncate/write で開く。
それぞれ `read-via-stream(0)` と `write-via-stream(0)` で input/output stream を取得し、
`output-stream.splice(5)` の success result を確認する。続けて `blocking-splice(5)` を呼び、direct
splice が少ない bytes を返す場合も含めて completion の success result を確認する。input/output
stream、descriptor、preopen を drop し、実行後の host artifact が `hello` であることを確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_splices_input_into_output_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_splices_input_into_output_and_drops_resources -- --nocapture`
- Expected boundary: direct `splice` と `blocking-splice` が success になり、`spliced.txt` is `hello`、
  stdout is empty、exit code is 0。

## Residual risk

これは output/input splice の verified partial slice である。exact transferred-byte count、stream
error/resource failure、zero-length splice、poll readiness、input stream の read/skip 残り operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native evidence、native/selfhost
parity は別途必要であり、aggregate completion とは扱わない。

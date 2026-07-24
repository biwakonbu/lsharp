# ADR: WasmGC CLI の write/append stream lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem descriptor、output-stream resource

## Context

Stage 2t/2u で named preopen、read stream、direct read/EOF を検証したが、descriptor から得た
`output-stream` の write、flush、append、drop を同じ Component resource boundary で確認していなかった。

## Decision

- `wasmgc-cli-fs-streams` world の `wasi:io/streams@0.2.3` を明示的に使い、
  `descriptor.write-via-stream` と `descriptor.append-via-stream` を別の operation として検証する。
- 最初の descriptor は create+truncate/write flags で開き、offset 0 の write stream へ `hello` を
  `output-stream.blocking-write-and-flush` で書く。
- descriptor を閉じた後、同じ named preopen から既存ファイルを再度開き、append stream へ `!` を
  blocking write-and-flush する。各 output-stream、descriptor、preopen は success/error path で
  resource-drop する。
- Component の exit 0 だけで成功扱いにせず、host filesystem の最終 bytes `hello!` を確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_writes_and_appends_streams_then_drops_resources` は二つの named
  read-write preopen を渡した actual Component で、write-via-stream、append-via-stream、flush、
  output-stream/descriptor drop、exit 0、host bytes `hello!` を確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 46 tests passed。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、docs audit も成功した。

## Consequences

- write/append stream の guest resource lifecycle と host artifact boundary が verified partial slice に
  なった。
- direct `write`/`stat`、read-directory、close-after-error、directory-entry stream、pollable、artifact/
  runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了であり、aggregate
  task は `[~]` のまま維持する。

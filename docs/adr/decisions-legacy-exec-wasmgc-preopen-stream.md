# ADR: WasmGC CLI の名前付き preopen と descriptor read stream lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem、I/O stream resource

## Context

Stage 2s では host path を guest の `"."` に一つだけ公開し、preopen rights と
`descriptor.open-at` の境界を固定した。descriptor を guest-visible な名前で公開する API と、
filesystem/types が返す input-stream を同じ Component resource table で read/drop する actual
Component evidence はまだなかった。

## Decision

- `Preview2Preopen` は host path、guest path、directory/file rights を一つの capability として表し、
  `...with_preview2_stdout_and_preopens` API が preopen table を順序通り構築する。
- 既存の `Option<&Path>` runner API は guest path `"."`、read-write rights の互換 wrapper として残す。
- `wasmgc-cli-fs-streams` world は `wasi:filesystem/preopens`、`wasi:filesystem/types`、
  `wasi:io/streams` を明示的に import し、filesystem descriptor と stream resource の境界を暗黙の
  fallback にしない。
- descriptor は `open-at` で得た read stream を `blocking-read` で消費し、stream と descriptor を
  resource-drop してから command result を返す。失敗は `wasi:cli/run` の exit/result boundary で観測する。

## Evidence

- `wasm_gc_component_cli_fs_runner_reads_named_preopen_stream_and_drops_resources` は guest path
  `data` の read-only preopen から `input.txt` の `hello` を実際に読み、custom stdout に出力し、
  input-stream/descriptor の drop 後に exit 0 を返す。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 44 tests passed。
- 新 API の不存在を先に確認する compile RED を通し、実装後の focused Component GREEN を確認した。

## Consequences

- 複数 preopen と guest-visible path alias を runner で明示的に構成できる。
- descriptor direct `read`/`write`/`stat`、write/append stream、close-after-error、directory-entry
  stream、pollable、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost
  parity は未完了であり、aggregate task は `[~]` のまま維持する。

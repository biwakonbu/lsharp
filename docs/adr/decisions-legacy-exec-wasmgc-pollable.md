# ADR: WasmGC CLI の input-stream pollable lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 `wasi:io/streams` / `wasi:io/poll`

## Context

Stage 2z までに descriptor と stream の read/write、directory、type/flags、drop を検証したが、
non-blocking input-stream の readiness を `pollable` resource へ接続する経路は未検証だった。

## Decision

- 既存の `wasmgc-cli-fs-streams` world と二つの read-only named preopen を使い、
  `descriptor.read-via-stream` から input-stream を作成する。
- `input-stream.subscribe` で child pollable を作成し、`pollable.block` でデータ準備を待った後、
  `pollable.ready` が true を返すことを確認する。subscribe 直後の non-blocking readiness は常に
  true と仮定しない。
- pollable、input-stream、descriptor、両 preopen を drop し、Component exit 0、stdout empty、
  host bytes unchanged (`hello`) を同じ実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_subscribes_and_polls_input_stream` は actual Component で
  subscribe、block、ready、child resource drop、exit 0、host bytes を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 51 tests passed（Stage 2aa を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- input-stream の pollable subscribe/block/ready/drop と Component resource ownership が verified
  partial slice になった。
- `wasi:io/poll.poll` の複数 pollable list API、残る descriptor operation、artifact/runtime differential、
  Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了であり、aggregate task は `[~]` のまま
  維持する。

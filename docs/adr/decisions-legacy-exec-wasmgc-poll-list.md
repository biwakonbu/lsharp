# ADR: WasmGC CLI の poll list lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 `wasi:io/streams` / `wasi:io/poll`

## Context

Stage 2aa で単一 pollable の subscribe/block/ready を、Stage 2ab で descriptor sync-data を検証したが、
複数の borrowed pollable を `wasi:io/poll.poll` へ渡し、ready index list を受け取る canonical ABI は
未検証だった。

## Decision

- 既存の `wasmgc-cli-fs-streams` world と二つの read-only named preopen を使い、
  `descriptor.read-via-stream` から input-stream、`input-stream.subscribe` から pollable を作る。
- `pollable.block` / `pollable.ready` の後、pollable handle の linear-memory list を `wasi:io/poll.poll`
  へ渡し、返却 `list<u32>` の length `1` と ready index `0` を確認する。
- pollable、input-stream、descriptor、二つの preopen を drop し、Component exit 0、stdout empty、
  host bytes unchanged (`hello`) を同じ実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_polls_subscribed_input_stream_list` は actual Component で borrowed
  pollable list、ready index、resource drop、exit 0、host bytes を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 53 tests passed（Stage 2ac を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- `wasi:io/poll.poll` の list input/output canonical ABI と resource lifecycle が verified partial slice
  になった。
- 残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost
  parity は未完了であり、aggregate task は `[~]` のまま維持する。

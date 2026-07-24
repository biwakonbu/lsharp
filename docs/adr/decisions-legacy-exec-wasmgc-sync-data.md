# ADR: WasmGC CLI の descriptor sync-data lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem descriptor

## Context

Stage 2aa までに named preopen、stream pollable、descriptor read/write/stat、directory、type/flags、
drop を検証したが、descriptor の data synchronization operation を actual Component から実行する
経路は未検証だった。

## Decision

- 既存の `wasmgc-cli-fs` world と二つの read-only named preopen を使い、最初の preopen から
  `input.txt` を `descriptor.open-at` で開く。
- `descriptor.sync-data` の `result<_, error-code>` が success になることを確認する。read-only
  descriptor でも POSIX-compatible host implementation が同期操作を成功扱いにする境界を採用する。
- descriptor と二つの preopen を drop し、Component exit 0、stdout empty、host bytes unchanged
  (`hello`) を同じ実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_syncs_descriptor_data_and_drops_resources` は actual Component で
  preopen table、open-at、sync-data、resource drop、exit 0、host bytes を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 52 tests passed（Stage 2ab を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- descriptor sync-data の result/error/drop boundary が verified partial slice になった。
- `wasi:io/poll.poll` の複数 pollable list API、残る descriptor operation、artifact/runtime differential、
  Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了であり、aggregate task は `[~]` のまま
  維持する。

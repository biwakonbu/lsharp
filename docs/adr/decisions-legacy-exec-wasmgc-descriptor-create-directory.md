# ADR: WasmGC CLI の descriptor create-directory-at lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 `wasi:filesystem/types`

## Context

Stage 2ae で `descriptor.set-size` の host artifact mutation を検証したが、preopen descriptor に
対する path mutation と directory artifact の生成は未検証だった。

## Decision

- `wasmgc-cli-fs` world と二つの read-write named preopen を使い、最初の preopen descriptor に
  relative path `created` を渡す。
- `descriptor.create-directory-at` の `result<_, error-code>` canonical discriminant が success
  になること、host 側に `created/` directory が生成されることを確認する。
- preopen descriptor を drop し、Component exit 0、stdout empty、host directory artifact を同じ
  実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_creates_directory_and_drops_resources` は actual Component で
  path mutation、success result、resource drop、exit 0、host directory を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は Stage 2af を含む 56 tests
  passed。component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff
  check、docs audit も成功した。

## Consequences

- `descriptor.create-directory-at` の actual Component mutation/host artifact/drop boundary が
  verified partial slice になる。
- 残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

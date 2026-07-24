# ADR: WasmGC CLI の descriptor remove-directory-at lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 `wasi:filesystem/types`

## Context

Stage 2af で `descriptor.create-directory-at` の path mutation と host directory creation を検証したが、
同じ preopen descriptor から directory artifact を安全に削除する境界は未検証だった。

## Decision

- `wasmgc-cli-fs` world と二つの read-write named preopen を使い、最初の preopen に
  `to-remove/` fixture directory を公開する。
- `descriptor.remove-directory-at` に relative path `to-remove` を渡し、`result<_, error-code>`
  canonical discriminant が success になること、host 側の directory が消えることを確認する。
- preopen descriptor を drop し、Component exit 0、stdout empty、削除済み host artifact を同じ
  実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_removes_directory_and_drops_resources` は actual Component で
  path mutation、success result、resource drop、exit 0、host directory deletion を確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は Stage 2ag を含む 57 tests
  passed。component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff
  check、docs audit も成功した。

## Consequences

- `descriptor.remove-directory-at` の actual Component mutation/host artifact/drop boundary が
  verified partial slice になる。
- 残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

# ADR: WasmGC CLI の descriptor set-size lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 `wasi:filesystem/types`

## Context

Stage 2ad で `descriptor.sync` の result/drop boundary を検証したが、write-enabled descriptor の
file-size mutation と実ファイルへの artifact 反映は未検証だった。

## Decision

- `wasmgc-cli-fs` world と二つの read-write named preopen を使い、最初の preopen から
  `input.txt` を descriptor flags `write` で開く。
- `descriptor.set-size(7)` の `result<_, error-code>` canonical discriminant が success になること、
  host file bytes が `hello` から `hello\0\0` に拡張されることを確認する。
- descriptor と preopen を drop し、Component exit 0、stdout empty、host artifact を同じ実行で
  確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_sets_descriptor_size_and_drops_resources` は actual Component で
  write-enabled descriptor、set-size success、resource drop、exit 0、host bytes を一つの実行で
  確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は Stage 2ae を含む 55 tests
  passed。component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff
  check、docs audit も成功した。

## Consequences

- `descriptor.set-size` の actual Component mutation/host artifact/drop boundary が verified partial
  slice になる。
- 残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

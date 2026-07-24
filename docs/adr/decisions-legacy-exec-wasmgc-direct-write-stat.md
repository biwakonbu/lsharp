# ADR: WasmGC CLI の descriptor direct write/stat lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem descriptor

## Context

Stage 2v では output-stream の write、flush、append、drop と host bytes を検証したが、WASI
filesystem descriptor の direct `write` と `stat` が同じ Component ABI と実ファイルへ接続することは
未検証だった。

## Decision

- 既存の `wasmgc-cli-fs` world（`wasi:filesystem/types`）で direct operation を検証し、stream-only
  world に不要な import を追加しない。
- `descriptor.write(buffer, offset)` を canonical `list<u8>` 入力と
  `result<filesize, error-code>` の戻り値として扱い、guest の static `hello` を offset 0 に書く。
- 書込結果の filesize `5` と、続く `descriptor.stat` の `descriptor-type=regular-file` / `size=5`
  を Component 内で確認する。descriptor-stat の timestamp は platform-dependent のため契約に含めない。
- 二つの named read-write preopen を渡し、descriptor と preopen を success/error path の双方で
  `[resource-drop]descriptor` により解放する。Component の exit 0 だけで成功扱いにせず、host file
  bytes が `hello` であることを確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_writes_descriptor_directly_and_stats_file` は actual Component で
  preopen table、open-at、direct write、write length、stat type/size、resource drop、exit 0、host
  bytes を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 47 tests passed（Stage 2w を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- descriptor direct write/stat の canonical ABI と host artifact boundary が verified partial slice に
  なった。
- read-directory、close-after-error、directory-entry stream、pollable、artifact/runtime differential、
  Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了であり、aggregate task は `[~]` のまま
  維持する。

# ADR: WasmGC CLI の read-directory と directory-entry stream lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem directory descriptor

## Context

Stage 2x までに named preopen、descriptor read/write/stat、stream write/append、error/drop を検証したが、
directory descriptor から directory-entry stream を取得し、entry と end-of-stream を Component の
canonical ABI で lift する経路は未検証だった。

## Decision

- 既存の `wasmgc-cli-fs` world を使い、二つの read-only named preopen を順序付きで渡す。
- 最初の preopen directory に `descriptor.read-directory` を呼び、
  `[method]directory-entry-stream.read-directory-entry` を二回実行する。
- 一回目は `option<directory-entry>` の `some` と regular-file `input.txt` を確認し、entry name の
  guest string を custom stdout へ渡す。二回目は `none` を end-of-stream として確認する。
- directory-entry stream と preopen descriptor は success/error path で `[resource-drop]` を呼び、
  Component exit 0 と stdout `input.txt` を同じ実行で確認する。`.` / `..` は契約に含めない。

## Evidence

- `wasm_gc_component_cli_fs_runner_reads_directory_entries_and_drops_stream` は actual Component で
  preopen table、read-directory、entry some/type/name、entry none、stream/descriptor drop、exit 0 を
  一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 49 tests passed（Stage 2y を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- directory-entry stream の canonical option/string ABI と end-of-stream/drop boundary が verified
  partial slice になった。
- pollable、残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

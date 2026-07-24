# ADR: WasmGC CLI の direct write error と descriptor drop lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem descriptor error path

## Context

Stage 2w で direct `write` と `stat` の成功 path を検証したが、descriptor が read-only のときに
direct write が error result へなり、エラー後の descriptor/preopen resource を安全に解放できることは
未検証だった。

## Decision

- 既存の `wasmgc-cli-fs` world で、host の `input.txt` を read-only descriptor flags で開く。
- `descriptor.write(buffer, offset)` の canonical result discriminant が error であることを確認し、
  error payload の platform-specific code 値には依存しない。
- write error 後に `[resource-drop]descriptor` を descriptor と preopen の双方へ実行し、`wasi:cli/run`
  の exit 0 と host bytes 不変を同じ actual Component で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_drops_descriptor_after_direct_write_error` は二つの named
  read-write preopen を渡した actual Component で、read-only descriptor、direct write error、descriptor/
  preopen drop、exit 0、host bytes `seed` を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 48 tests passed（Stage 2x を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- direct write の error discriminant と resource-drop ordering が verified partial slice になった。
- read-directory、directory-entry stream、pollable、artifact/runtime differential、Mac Apple Silicon/
  Linux x86_64、native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

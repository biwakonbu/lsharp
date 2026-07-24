# ADR: WasmGC CLI の descriptor direct read と EOF lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem descriptor

## Context

Stage 2t では `descriptor.read-via-stream` と `input-stream.blocking-read` の resource lifecycle を
検証したが、WASI filesystem の direct `descriptor.read` が返す `list<u8>` と EOF bool の ABI、
descriptor drop までを実際の Component で検証していなかった。

## Decision

- direct read は既存の `wasmgc-cli-fs` world（`wasi:filesystem/types`）で検証し、stream-only world
  に不要な import を追加しない。
- `descriptor.read(length, offset)` の result を canonical ABI の
  `result<tuple<list<u8>, bool>, error-code>` として扱い、guest 側の `cabi_realloc` で返却 list を
  linear memory に受ける。
- EOF bool は「要求長に満たなかった」ではなく、実際の read が 0 byte になった時に true になる
  contract として固定する。従って fixture は offset 0/length 5 の `hello` read と、offset 5/length 1
  の empty+EOF read を別々に実行する。
- 成功後は opened descriptor を `[resource-drop]descriptor` で解放してから `wasi:cli/run` の成功
  result を返す。direct read の error path も descriptor drop 後に非成功 result へ収束させる。

## Evidence

- `wasm_gc_component_cli_fs_runner_reads_descriptor_directly_and_reports_eof` は二つの named preopen
  を渡した actual Component で、`descriptor.open-at`、direct `descriptor.read`、empty+EOF read、
  stdout `hello`、descriptor drop、exit 0 を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 45 tests passed で、focused
  direct-read test も含めて probe 全件を通す。
- EOF を exact-length read の bool と誤って固定した RED を修正し、0-byte probe の runtime GREEN へ
  到達した。

## Consequences

- stream resource を作らない direct read の bytes/EOF/drop boundary が actual Component evidence に
  なった。
- direct `write`、`stat`、read-directory、write/append stream、close-after-error、pollable、artifact/
  runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了であり、aggregate
  task は `[~]` のまま維持する。

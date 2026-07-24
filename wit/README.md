# L# WIT World Definitions

WASI Preview2 / Component Model への移行に向けた WIT (WebAssembly Interface Types) 定義。

## ファイル構成

| ファイル | 説明 |
|---------|------|
| `lsharp-compiler.wit` | CLI コンパイラ component world (wasi:cli + wasi:filesystem) |
| `lsharp-http-handler.wit` | HTTP handler component world (wasi:http/incoming-handler) |
| `lsharp-wasmgc-output.wit` | WasmGC `print-string` の `list<u8>` output interface と `wasmgc-output` / `wasmgc-cli` / `wasmgc-cli-fs` / `wasmgc-cli-fs-streams` world |
| `lsharp-core.wit` | 共有インターフェース (compiler, host-fs, host-process) |
| `deps/http.wit` | `lsharp-http-handler.wit` が参照する vendored `wasi:http` package |

## WASI Preview1 → Preview2 マッピング

既存の 9 WASI Preview1 imports と Component Model interface の対応:

| Preview1 import | Preview2 interface |
|----------------|-------------------|
| `fd_write` | `wasi:io/streams` (stdout/stderr) |
| `fd_read` | `wasi:io/streams` (stdin) |
| `proc_exit` | `wasi:cli/exit` |
| `args_get` | `wasi:cli/environment` |
| `args_sizes_get` | `wasi:cli/environment` |
| `path_open` | `wasi:filesystem/types` |
| `fd_close` | `wasi:filesystem/types` |
| `fd_seek` | `wasi:filesystem/types` |
| `fd_filestat_get` | `wasi:filesystem/types` |

## バージョニング

- パッケージバージョンは `0.1.0` (L# 本体に追従)
- WASI interface は実装上 `@0.2.3` (wasmtime-wasi 29 系の stable Preview2 WIT set) を使用
- WasmGC の `wasmgc-cli` world は custom stdout import、明示的な `wasi:cli/exit@0.2.3` import、
  `wasi:cli/run@0.2.3` export を持つ。core module へ未宣言の WASI capability を暗黙に追加しない
- `wasmgc-cli-fs` は `wasmgc-cli` に `wasi:filesystem/preopens@0.2.3` / `types@0.2.3` を明示的に
  加えた検証用 world であり、preopen がない場合や rights が read-only の場合に filesystem access
  を成功扱いにしない。`descriptor.open-at`、direct `descriptor.read` の bytes/EOF/drop boundary、
  direct `descriptor.write` / `descriptor.stat` の host artifact boundary、read-only write error 後の
  descriptor/preopen drop、`read-directory` / `directory-entry-stream` の some/none/drop lifecycle、
  `descriptor.get-type` / `descriptor.get-flags` の type/flags/drop lifecycle、`descriptor.sync-data`
  / `descriptor.sync` の success/drop lifecycle、write-enabled `descriptor.set-size` の host artifact
  lifecycle、`descriptor.create-directory-at` の host directory artifact/drop lifecycle、
  `descriptor.remove-directory-at` の host directory deletion/drop lifecycle、
  `descriptor.unlink-file-at` の host file deletion/drop lifecycle、
  `descriptor.rename-at` の host file rename/drop lifecycle、
  `descriptor.symlink-at` の host symbolic-link artifact/drop lifecycle、
  `descriptor.readlink-at` の string target/drop lifecycle、
  `descriptor.link-at` の host hard-link artifact/drop lifecycle、
  `descriptor.is-same-object` の bool result/drop lifecycle、
  `descriptor.metadata-hash` の 128-bit record/drop lifecycle、
  `descriptor.metadata-hash-at` の 128-bit record/drop lifecycle、
  `descriptor.stat-at` の type/size record/drop lifecycle、
  `descriptor.set-times-at` の no-change result/drop lifecycle、
  `descriptor.set-times` の no-change result/drop lifecycle、
  `descriptor.advise` の normal result/drop lifecycle もこの world で検証する
- `wasmgc-cli-fs-streams` は `wasmgc-cli-fs` に `wasi:io/streams@0.2.3` を明示的に加えた world であり、
  descriptor の `read-via-stream` / `write-via-stream` / `append-via-stream` と input/output-stream の
  resource lifecycle、`input-stream.subscribe` から `wasi:io/poll` の `pollable.block` /
  `pollable.ready`（非空入力と EOF/empty input の readiness）、`wasi:io/poll.poll` の borrowed pollable list/ready index（非空/EOF input）、空 list 入力の trap、resource-drop までを
  同じ Component resource table で検証する。`output-stream.check-write` / `write` / `flush` /
  `blocking-flush` の permit/write/flush/host artifact/drop lifecycle と、
  `output-stream.write-zeroes` の check-write/zero-fill/blocking-flush contract、さらに
  `output-stream.splice` / `blocking-splice` の borrowed input/output transfer contract と、
  `input-stream.skip` / `blocking-skip` の partial/remaining read contract と、
  `input-stream.read` / `blocking-read` の空 list・partial read・remaining bytes・EOF/empty-source contract（blocking-read の EOF は `stream-error::closed`）と、
  `output-stream.blocking-write-zeroes-and-flush` の zero-fill/flush/host artifact/drop lifecycle も
  この world で検証する。暗黙の別 resource table に分離しない

## 関連タスク

- P13-A: Dual-mode WASI runner + Preview2 codegen
- P13-B: WIT definitions + Guest component boundary
- P13-C: Single binary distribution

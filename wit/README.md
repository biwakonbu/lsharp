# L# WIT World Definitions

WASI Preview2 / Component Model への移行に向けた WIT (WebAssembly Interface Types) 定義。

## ファイル構成

| ファイル | 説明 |
|---------|------|
| `lsharp-compiler.wit` | CLI コンパイラ component world (wasi:cli + wasi:filesystem) |
| `lsharp-http-handler.wit` | HTTP handler component world (wasi:http/incoming-handler) |
| `lsharp-wasmgc-output.wit` | WasmGC `print-string` の `list<u8>` output interface と `wasmgc-output` / `wasmgc-cli` world |
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

## 関連タスク

- P13-A: Dual-mode WASI runner + Preview2 codegen
- P13-B: WIT definitions + Guest component boundary
- P13-C: Single binary distribution

# ADR: WasmGC component output の fd_write handler 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: canonical output bytes を WASI `fd_write` semantics へ渡す host boundary

## Context

Stage 2m で、WasmGC packed array は exported linear memory を経由して canonical output import
へ渡せるようになった。次の境界は、その bytes を stdout の WASI `fd_write` へ接続することだが、
実 `WasiP1Ctx` や Preview2 stream を先に混ぜると、partial write、errno、flush/exit ordering の
契約が曖昧になる。

## Decision

- `run_wasm_wasmgc_component_output_to_writer` を canonical output の `std::io::Write` adapter
  とし、`write_all` による partial write 再試行、WriteZero/write error の fail-closed、main の
  exit code 取得後の flush を固定する。
- `run_wasm_wasmgc_component_output_to_fd_write` は stdout fd と
  `Fn(fd, bytes) -> Result<nwritten, errno>` handler を受け取る。handler が返す byte 数を検証し、
  partial は再試行、zero/over-report/errno は error として終了する。
- handler は実 WASI context の所有権を持たず、WASI Preview1/Preview2 の fd table、rights、
  stream 実装は呼び出し側が後段で接続する。canonical import 以外の暗黙 fallback はしない。

## Evidence

- `wasm_gc_component_output_writer_retries_partial_writes_until_chunk_is_consumed`
- `wasm_gc_component_output_writer_propagates_write_error`
- `wasm_gc_component_output_writer_rejects_write_zero`
- `wasm_gc_component_output_writer_flushes_after_nonzero_exit`
- `wasm_gc_component_output_fd_write_retries_partial_writes`
- `wasm_gc_component_output_fd_write_propagates_errno`
- `wasm_gc_component_output_fd_write_rejects_zero_and_overreported_counts`

`cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 35 tests passed。

## Consequences

- WASI `fd_write` の observable semantics を、actual WASI context と独立した RED/GREEN slice として
  再利用できる。
- これは handler contract の verified partial slice であり、実 `WasiP1Ctx`/Preview2 stream への
  接続、fd rights/table、Component actual instantiate/runner、Mac/Linux native evidence、selfhost
  parity は `LEGACY-WASMGC-COMP-IO-01` / `-RUN-01` に残る。

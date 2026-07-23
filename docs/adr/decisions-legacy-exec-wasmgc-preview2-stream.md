# ADR: WasmGC output Component の Preview2 stdout stream 接続

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: custom `wasmgc-output` world と実 WASI Preview2 context/stdout resource の接続

## Context

Stage 2o で custom `wasmgc-output` world の Component instantiate と `list<u8>` host callback を
固定した。単純な `Vec<u8>` sink だけでは、Preview2 の `WasiCtx`、`ResourceTable`、stdout stream
の ownership と write/flush 契約が検証できない。一方、custom world はまだ `wasi:cli/run` を
import/export していないため、既存の generic CLI Component runner と混ぜると未接続の surface を
完了扱いにしやすい。

## Decision

- `run_wasm_wasmgc_component_output_component_with_preview2_stdout` は `WasiCtxBuilder` で stdin、
  argv、preopened directory、stdout `MemoryOutputPipe` を構成し、`ResourceTable` と `WasiView`
  を持つ state を Component store に渡す。
- 同じ Component linker に `wasmtime_wasi::add_to_linker_sync` と custom
  `lsharp:wasmgc-output/stdout@0.1.0` host interface を登録する。custom callback は
  `WasiImpl`/`wasi:cli/stdout.get-stdout` から resource を取得し、`check-write` → permit 分割
  `write` → `flush` → resource 解放の順序で bytes を stdout stream へ渡す。
- custom world の `main: s64` export を直接呼び出す。`wasi:cli/run` の import/export、fd table/rights
  の公開契約、Component artifact の supported target parity はこの ADR の完了条件に含めず、
  TODO の残境界へ保持する。WASI linker の登録だけで未使用 interface を成功扱いにしない。

## Evidence

- `wasm_gc_component_output_component_runner_connects_preview2_stdout_stream` は componentize 後の
  actual bytes を Preview2 linker/context、stdout resource、UTF-8 output、s64 exit code まで実行
  する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 36 tests passed。
- `cargo test -p lsharp-wasm --lib wasi_runner::tests -- --nocapture` は 20 tests passed。

## Consequences

- custom output Component の bytes が、単なる memory sink ではなく実 WASI Preview2 context の
  stdout stream に到達する verified partial slice になった。
- stream resource は callback ごとに取得・解放し、permit を超える一回書きを行わない。
- custom world の `wasi:cli/run` 接続、fd table/rights、Mac Apple Silicon/Linux x86_64 artifact/runtime、
  native/selfhost parity は `LEGACY-WASMGC-COMP-IO-01` / `-RUN-01` に残る。

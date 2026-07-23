# ADR: WasmGC CLI Component の exit/result parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli` world の `wasi:cli/exit` import と Preview2 CLI runner

## Context

Stage 2q で WasmGC CLI Component は `wasi:cli/run` の `bool`/`result` return を 0/1 exit code へ
decode できるようになった。しかし Preview2 の `wasi:cli/exit` は通常の trap ではなく
`wasmtime_wasi::I32Exit` を返すため、runner がこれを一般 error として扱うと `proc_exit` 相当の
status code が失われる。

## Decision

- `wit/lsharp-wasmgc-output.wit` の `wasmgc-cli` world は custom stdout import と
  `wasi:cli/exit@0.2.3` import、`wasi:cli/run@0.2.3` export を明示する。core module へ未宣言の
  WASI capability を追加する暗黙 fallback は行わない。
- `run_wasm_wasmgc_component_cli_with_preview2_stdout` は `wasi:cli/run` の call error を共有
  `wasi_runner::extract_i32_exit` で検査し、`I32Exit` の status code を `ExecutionOutput.exit_code`
  として返す。その他の trap は従来通り error として返す。
- 正常 return は `false` / `result<_, _>` を 0、`true` / failed result を 1 として decode し、
  `wasi:cli/exit` と `wasi:cli/run` の失敗を同じ observable exit-code 契約へ揃える。

## Evidence

- `wasm_gc_component_cli_runner_maps_wasi_cli_exit_to_exit_status` は actual component の
  `wasi:cli/exit` host callを実行し、status 1 を exit code 1 として取得する。
- `wasm_gc_component_cli_runner_maps_failed_wasi_cli_run_result_to_exit_status` は failed
  `wasi:cli/run` result を exit code 1 として取得する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 42 tests passed。

## Consequences

- `proc_exit` 相当の non-zero status と command の failed result が、Preview2 WasmGC CLI runner
  で trap/error に埋もれず観測できる。
- fd table/rights、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、この ADR は aggregate completion を宣言しない。

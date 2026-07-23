# ADR: WasmGC custom CLI world の `wasi:cli/run` 接続

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-output` package の command world、canonical run export、Preview2 runner

## Context

Stage 2p で custom `wasmgc-output` Component の bytes を実 WASI Preview2 stdout stream へ渡せる
ようになった。ただし world が `main: s64` のままだと既存の command Component runner と同じ
`wasi:cli/run` 境界を通らない。core module に未使用の WASI imports を足して成功扱いにするのは、
capability と entry point の契約を曖昧にする。

## Decision

- `wit/lsharp-wasmgc-output.wit` の同一 package/version に `wasmgc-cli` world を追加し、Stage 2q
  の baseline では custom `stdout` import と `wasi:cli/run@0.2.3` export を宣言する。既存の
  `wasmgc-output` world と interface/version を重複させず、後続の exit capability は
  [CLI exit/result parity ADR](decisions-legacy-exec-wasmgc-cli-exit.md) で明示的に追加する。
- `emit_wasm_wasmgc_component_cli` は WasmGC の exported `main: () -> i64` を呼び出す
  `wasi:cli/run@0.2.3#run: () -> i32` core wrapper を追加する。wrapper は main の値を drop し、
  成功時に `0` を返す。WASI capability import は生成しない。
- `run_wasm_wasmgc_component_cli_with_preview2_stdout` は nested `wasi:cli/run` export を解決し、
  Stage 2p の `WasiCtx`/stdout resource bridge を使って command Component を実行する。戻り値は
  Wasmtime の `bool`/`result` representation を 0/1 exit code へ decode する。
- core に canonical run export がない場合は componentize を明示的に拒否し、`wasi:cli/run` を
  `main` export の別名として暗黙に推測しない。

## Evidence

- `wasm_gc_component_output_cli_world_rejects_core_without_wasi_cli_run_export` は run export 欠落を
  RED に固定する。
- `wasm_gc_component_output_cli_world_accepts_canonical_run_export` と
  `wasm_gc_component_output_cli_backend_emits_canonical_run_export` は WIT componentize/validation
  を確認する。
- `wasm_gc_component_cli_runner_executes_wasi_cli_run_with_preview2_stdout` は generated WasmGC core
  を actual Component 化し、Preview2 stdout stream を通って `wasi:cli/run` を実行する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 40 tests passed。

## Consequences

- custom WasmGC output が `main` 直呼びだけでなく、既存 Preview2 command の `wasi:cli/run` entry
  point へ到達する verified partial slice になった。
- fd table/rights、proc-exit/error result の完全 parity、Mac Apple Silicon/Linux x86_64 artifact/runtime、
  native/selfhost parity は `LEGACY-WASMGC-COMP-IO-01` / `-RUN-01` に残る。

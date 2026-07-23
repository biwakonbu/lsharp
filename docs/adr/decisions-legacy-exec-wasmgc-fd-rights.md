# ADR: WasmGC CLI Component の preopen/rights 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component と WASI Preview2 filesystem preopen

## Context

Stage 2r までの WasmGC CLI runner は `dir` を受け取ると directory/file の全 rights で
preopen していたが、filesystem capability の宣言、preopen がない場合の fail-closed、read-only
rights の拒否を guest-visible Component で確認していなかった。fd_write handler の抽象契約だけでは、
実 `ResourceTable` と descriptor rights の境界を証明できない。

## Decision

- `wasmgc-cli` は stdout/exit/run の最小 world とし、filesystem capability は `wasmgc-cli-fs` の
  `wasi:filesystem/preopens@0.2.3` / `types@0.2.3` import へ明示的に分離する。
- `Preview2PreopenRights` は directory/file rights を値として保持し、`read_only()` と
  `read_write()` を提供する。既存 runner API は read-write default の互換 wrapper とし、rights
  指定 API は `WasiCtxBuilder.preopened_dir` に同じ値を渡す。
- `dir == None` のときは preopen を追加しない。Component が `get-directories` で得られる
  descriptor table を前提にしてはならず、明示的な失敗 result を返す契約とする。
- rights で許可されない `descriptor.open-at(create, write)` は Wasm trap へ曖昧に変換せず、
  `wasi:cli/run` failed result の exit code 1 として観測する。

## Evidence

- `wasm_gc_component_cli_fs_runner_enforces_preopen_rights` は actual Component の
  `get-directories` / `descriptor.open-at` を実行し、preopen なし=1、read-only=1、read-write=0
  と file creation を固定する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 43 tests passed。

## Consequences

- filesystem capability が world と runner API の両方で明示され、preopen/rights の失敗が
  exit/result contract として再現できる。
- descriptor の全 operation、stream read/write、resource drop/lifecycle、Wasm artifact/runtime
  differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

# ADR: WasmGC Component artifact の保存・再読込・Preview2 runtime

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-output` Component / serialized `.component.wasm` artifact / Preview2 stdout host

## Context

これまでの WasmGC Component 検証は、core bytes を componentize した後に同じプロセス内の
in-memory Component を validation・instantiate・実行する境界だった。これは Component の
生成と host runtime の契約を確認するが、配布・保存した成果物を後から再利用できることまでは
証明しない。artifact の保存時に bytes が変質しないこと、再読込後にも Component validation と
Preview2 resource/runtime が同じ意味論を保つことを一つの契約にする必要がある。

## Decision

`wasmgc-output` Component の round-trip を次の順序で固定する。

1. core module を `componentize_core_module` で実際の Component bytes に変換する。
2. bytes を一時ディレクトリの `output.component.wasm` へ保存し、同じ path から再読込する。
3. 再読込 bytes が生成時 bytes と一致することを確認し、`wasmparser::Validator` と
   `wasmtime::component::Component` の双方で検証する。
4. in-memory bytes と再読込 artifact を同じ Preview2 stdout host runtime で実行し、stdout と
   exit code が一致することを確認する。probe は `AB` / `37` を観測値とする。
5. helper は成功・失敗のいずれでも一時ディレクトリを cleanup し、テスト後に artifact を残さない。

## Evidence

- Test: `wasm_gc_component_output_artifact_round_trip_preserves_preview2_runtime`
- Helper: `persist_and_reload_wasmgc_component_artifact`
- Probe: `emit_component_output_probe_module`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_output_artifact_round_trip_preserves_preview2_runtime -- --nocapture`

## Residual risk

これは serialized Component の byte-preservation・validation・同一 host runtime 実行を閉じた
verified partial slice であり、compiler が生成する release artifact、Mac Apple Silicon / Linux
x86_64 の native stage0、selfhost parity、複数 target の配布・provenance・rollback は未完了である。
Rust driver の in-memory 成功やこの単一 probe を、`LEGACY-EXEC-01` または全公開 surface の Rust-free
完了へ拡大解釈しない。

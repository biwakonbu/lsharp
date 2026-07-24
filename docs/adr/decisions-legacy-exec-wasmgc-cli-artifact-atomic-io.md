# ADR: WasmGC CLI Component artifact の atomic I/O

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli` Component / `Main.component.wasm` / Preview2 `wasi:cli/run`

## Context

Stage 2bo では、`wasmgc-output` Component の bytes を test helper で一時 file に保存し、再読込後の
validation と host runtime parity を確認した。しかし CLI world の実成果物を保存する production
boundary はなく、直接 `fs::write` すると途中 bytes が公開される可能性があった。CLI Component を
配布可能な artifact として扱うには、書込み失敗時に既存成果物を壊さず、一時 file を残さず、再読込後
も同じ `wasi:cli/run` semantics を保つ契約が必要である。

## Decision

`lsharp_wasm::component_adapter` に次の artifact I/O API を追加する。

- `write_component_artifact(path, bytes)` は同じ親 directory の process-unique temporary path に bytes
  を書き、成功した場合だけ `rename` で destination を置換する。書込みまたは置換が失敗した場合は
  temporary path を cleanup し、destination の既存 bytes は変更しない。
- `read_component_artifact(path)` は destination bytes を明示的に再読込する。validation/runtime の
  責務は caller に残し、artifact I/O と host policy を混同しない。
- CLI Component は再読込 bytes を `wasmparser::Validator`、Wasmtime Component validation、同じ
  Preview2 stdout host runtime の順に通し、in-memory と stdout/exit が一致することを要求する。

## Evidence

- Test: `wasm_gc_component_cli_artifact_round_trip_preserves_wasi_cli_run`
- Unit test: `component_adapter::tests::test_component_artifact_round_trip_replaces_without_temp_residue`
- APIs: `write_component_artifact`, `read_component_artifact`
- Observable result: stdout `CL`, exit code `0`, byte equality, no temporary residue

## Residual risk

この ADR は atomic file replacement と一つの CLI Component runtime parity を閉じた verified partial
slice である。artifact manifest/source fingerprint、durable fsync、全 compiler target の release
pipeline、Mac Apple Silicon/Linux x86_64 native stage0、selfhost parity、rollback provenance は未完了で、
`LEGACY-EXEC-01` の完了や配布 readiness を意味しない。

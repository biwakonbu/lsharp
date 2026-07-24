# ADR: cached Wasm artifact の runtime evidence

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/compile.rs::CompileArtifacts::from_cache`
- Related: `decisions-legacy-tooling-artifact-validation.md`, `decisions-legacy-tooling-cli-artifact-cache.md`

## Context

target-aware structural validation は cache bytes が Wasm として読めることを保証するが、cold compile と同じ実行意味論を
返すことまでは保証しない。process 間 cache を次の version の compile path に接続するには、hit を observable にし、runtime
execution の evidence を分離して記録する必要がある。

## Decision

- `CompileArtifacts::from_cache` を追加し、cache hit は `true`、fresh compile / `emit_ir` / Native は `false` とする。
- cross-session cache hit の Preview1 Wasm を `lsharp_wasm::wasi_runner::run_wasm_wasi` で実行し、cold/hit の stdout parity と
  source change 後の fresh runtime output を test contract に固定する。
- この slice は Rust host runtime evidence に限定する。component world call、native stage0、Linux x86_64、selfhost compiler、
  external `LSHARP_PATH` runtime は別 gate として残す。

## Evidence

- RED: `CompileArtifacts::from_cache` 未実装時に cache hit/miss observable test が compile error となった。
- GREEN: `cargo test -p lsharp-tooling test_compile_session_opt_in_artifact_cache_reuses_across_sessions -- --nocapture --test-threads=1`
  (`1 passed; 0 failed`)。
- `cargo test -p lsharp-tooling test_compile -- --nocapture --test-threads=1` (`51 passed; 0 failed`)。
- runtime output: cold/hit `7\n`、source change fresh `8\n`。

## Consequences

cache hit が structural validation と runtime execution の二段階で検証できる。`from_cache` は host API の observable metadata として
追加されたが、CLI success line は互換性のため変更していない。`LEGACY-MODULE-01` と native 2 target evidence は未完了のまま残る。

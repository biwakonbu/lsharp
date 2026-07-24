# ADR: CompileSession からの opt-in artifact cache 接続

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/compile.rs::CompileSession::with_artifact_cache`
- Related: `decisions-legacy-tooling-compile-key.md`, `decisions-legacy-tooling-compile-artifact-cache.md`

## Context

`ArtifactCache` の安全な store/load 境界だけでは、process 間の compile 時間短縮にはつながらない。一方、既定の
`CompileSession::new` や公開 `compile_file_with_backend_and_cache` に暗黙接続すると、利用者の意図しない disk write、
Native executable の mode 欠落、runtime validation 前の bytes 再利用が起きる。

## Decision

- `CompileSession::with_artifact_cache(root)` を明示した session だけ artifact persistence を有効にする。
- Wasm target（Preview1 / Component / WebWasm / WasmGC）では key を作成し、hit なら output path へ atomic write して
  parser/type/lowering/codegen を省略する。miss は既存 pipeline を通し、成功した bytes を `ArtifactCache` へ保存する。
- `CompileSession::new`、`compile_file_with_backend_and_cache` の既存 API、`emit_ir`、Native target は cache を使わない。
- source formatting 後に key を計算する。imported module を含む key は `CompileCacheKey` が graph 全体から作るため、source 変更は
  fresh compile へ戻る。
- cache hit の Wasm runtime validation、CLI flag/env、cache eviction、Native executable の mode/ABI、selfhost/native stage0 は
  後続 task として残す。

## Evidence

- RED: `CompileSession::with_artifact_cache` 未実装時に cross-session reuse test が compile error となった。
- GREEN: `cargo test -p lsharp-tooling test_compile_session_opt_in_artifact_cache_reuses_across_sessions -- --nocapture`
  (`1 passed; 0 failed`)。
- `cargo test -p lsharp-tooling test_compile -- --nocapture` (`50 passed; 0 failed`)。
- `cargo clippy -p lsharp-tooling --lib --tests -- -D warnings` と targeted rustfmt が成功した。

## Consequences

同一明示 root を渡した別 session は Wasm artifact を再利用できる。既定 CLI、Native target、selfhost/native gate の成功経路は
変更していないため、`LEGACY-MODULE-01` は未完了のまま残る。

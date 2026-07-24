# ADR: tooling と driver の compile session cache 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/compile.rs`, `crates/lsharp-driver/src/main.rs`
- Related: `decisions-legacy-tooling-cache-api.md`, `decisions-legacy-module-canonical-boundary.md`

## Context

`compile_file_with_backend_and_cache(..., cache)` は caller-owned cache の API を提供していたが、公開されている
通常の compile wrapper と driver の default path は、毎回一時 `CompilationCache` を生成していた。LSP 以外の
host/session caller が同じ process で複数回 compile する場合にも、cache の所有期間を明示できる境界が必要だった。

## Decision

- `lsharp_tooling::compile::CompileSession` が `CompilationCache` の所有権と session lifetime を持つ。
- `CompileSession::compile_file_with_backend` は既存の `*_and_cache` pipeline へ委譲し、entry root の scope isolation と
  backend/target の既存契約を維持する。
- 互換 API `compile_file_with_backend` は新しい session を一つ作る薄い wrapper として残す。
- driver の default `compile` / `build` path と embedded component fallback は、それぞれ明示的な session を通る。
- session は process 内の境界に限り、process 間 disk persistence、selfhost/native stage0 parity、cache artifact の
  versioning は後続 task とする。

## Evidence

- RED: `test_compile_session_reuses_default_cache_for_multi_file_compile` は `CompileSession` 未実装時に
  `use of undeclared type CompileSession` で失敗した。
- GREEN: 同テストは cold compile 後に 2 module cache、warm compile の Wasm bytes parity、cache scope 維持を確認する。
- `cargo test -p lsharp-driver --bin lsharp` は `108 passed; 0 failed`。
- `cargo clippy -p lsharp-tooling -p lsharp-driver --all-targets -- -D warnings`、対象 2 file の rustfmt check、
  `git diff --check` が成功した。
- `cargo test -p lsharp-tooling --lib` は今回の compile/session tests を含むが、既存 metadata の
  `LS2005` vacuous failure 2 件（`test_run_metadata_tests_executes_bool_property_binder`、
  `test_run_metadata_tests_rejects_bool_property_above_two_cases`）で crate 全体は未完了。compile 差分とは無関係である。

## Consequences

同一 process の tooling/driver host session は cache lifetime を一つの型で表現できる。単発 CLI の process 終了後に
cache は残らないため、CLI 再実行の高速化や disk persistence はまだ提供しない。`LEGACY-MODULE-01` の aggregate 完了、
依存 SCC key の全公開 surface、selfhost/native stage0 parity は active task のまま残る。

# ADR: CLI の明示 artifact cache root

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-driver/src/main.rs::Command::{Compile,Build}`
- Related: `decisions-legacy-tooling-compile-session-artifact.md`, `decisions-legacy-tooling-artifact-validation.md`

## Context

`CompileSession::with_artifact_cache` は root を明示した API caller から利用できるが、CLI が暗黙の project directory や
user home に cache を作ると、生成物の場所と cleanup 責務が不明確になる。embedded component guest や外部 `LSHARP_PATH`
compiler に host-only cache flag を渡すと、未対応 surface を曖昧に成功させる危険もある。

## Decision

- `compile` / `build` に `--artifact-cache-dir <path>` を追加する。指定時だけ Rust host がその path を
  `CompileSession::with_artifact_cache` へ渡し、未指定時は `CompileSession::new` のまま cache directory を作らない。
- relative path は command の current working directory 基準とする。cache root の作成と cleanup は明示指定した caller の責務とする。
- embedded component delegation の argv 判定ではこの flag を host-only として拒否し、Rust host path へ残す。既存の external
  `LSHARP_PATH` delegation は別 compiler の責務として変更しない。
- `--artifact-cache-dir` は `emit_ir`、Native executable、runtime execution、eviction、default cache location を暗黙に有効化しない。

## Evidence

- RED: `test_cli_compile_artifact_cache_dir_is_explicit` は flag 未実装時に `Command::Compile` field compile error となった。
- GREEN: `cargo test -p lsharp-driver test_cli_compile_artifact_cache_dir_is_explicit -- --nocapture` (`1 passed; 0 failed`)。
- `test_should_delegate_to_embedded_component_args_rejects_rust_only_compile_build_flags` で embedded delegation が flag を拒否することを確認した。
- manual host smoke: `target/debug/lsharp compile examples/fib.ls --target wasi-preview1 --output <tmp>/...wasm --artifact-cache-dir <tmp>/...`
  を 2 回実行し、cache entry 1 件と 5375-byte output を確認後、生成物を回収した。

## Consequences

CLI から明示 root の process 間 Wasm cache を選べるが、default cache location や `LSHARP_PATH` の cross-compiler contract は
未決定のまま残る。`LEGACY-MODULE-01` の native 2 target / selfhost 完了条件は満たさない。

# ADR: CLI の明示 artifact cache entry limit

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-driver/src/main.rs::Command::{Compile,Build}`
- Related: `decisions-legacy-tooling-cli-artifact-cache.md`, `decisions-legacy-tooling-cache-maintenance.md`

## Context

`ArtifactCache::trim_to_entries` は caller が明示的に呼ぶ maintenance API だが、CLI の process 間 cache root を長期間使う場合に
entry 数を制限する host boundary がない。compile path が既定値や implicit cleanup を導入すると、既存の compile/build の stdout、
embedded component delegation、selfhost/native の責務を変えてしまう。

## Decision

- `compile` / `build` に `--artifact-cache-max-entries <N>` を追加する。
- `--artifact-cache-max-entries` は `--artifact-cache-dir <path>` と併用した場合だけ受理し、単独指定は stable CLI error とする。
- compile/build が成功した後にだけ、明示 root の `ArtifactCache::trim_to_entries(N)` を呼ぶ。compile failure では cache maintenance を実行しない。
- embedded component delegation の argv 判定では entry limit を host-only flag として拒否し、Rust host boundary に残す。未指定時の既定 cache root、
  Native/emit_ir の暗黙 persistence、mtime/LRU、byte budget、自動 eviction は導入しない。

## Evidence

- RED: `artifact_cache_max_entries` field と `validate_artifact_cache_options` 未実装時に CLI test が compile error となった。
- GREEN: `cargo test -p lsharp-driver test_cli_compile_artifact_cache -- --nocapture --test-threads=1` (`3 passed; 0 failed`)。
- `cargo test -p lsharp-driver test_should_delegate_to_embedded_component_args_rejects_rust_only_compile_build_flags -- --nocapture --test-threads=1`
  (`1 passed; 0 failed`)。
- `cargo test -p lsharp-driver test_maintain_artifact_cache -- --nocapture --test-threads=1` (`1 passed; 0 failed`) で明示 root の 2 entry を
  `N=1` に trim する host helper を確認した。
- `cargo test -p lsharp-driver --bin lsharp -- --nocapture --test-threads=1` (`112 passed; 0 failed`) と
  `cargo clippy -p lsharp-driver --bin lsharp --tests -- -D warnings` が成功した。
- CLI manual smoke で `fib` (`5375` bytes) と `factorial` (`5388` bytes) を同じ cache root に compile し、各実行後の artifact count が
  `1`、最終 count も `1` になることを確認した。entry limit 単独指定は exit code `1` と
  `--artifact-cache-dir` 併用要求を返した。
- 対象 `main.rs` の rustfmt check、`git diff --check`、`bash scripts/audit_docs.sh` は成功した。

## Consequences

CLI caller が cache disk growth の上限を明示できる。entry limit は key 数だけを制限し、保存 bytes の総量や recency は扱わないため、
より高度な eviction は別 task/ADR とする。`LEGACY-MODULE-01`、native 2 target、selfhost persistence は未完了のまま残る。

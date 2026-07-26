# ADR: driver artifact cache option helper 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/main.rs`, `crates/lsharp-driver/src/artifact_cache_options.rs`, `crates/lsharp-driver/src/main_tests.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/main.rs` は CLI dispatch、embedded component delegation、package/install、API diff に加えて、
artifact cache の environment option、numeric limit parser、cache maintenance を保持していた。cache option
boundary は compile/build wiring から独立しており、親の変更範囲を広げていた。

## Decision

- artifact cache の constants、environment/CLI precedence parser、validation、trim maintenance helper を
  `artifact_cache_options.rs` へ移動する。
- 親では `include!("artifact_cache_options.rs")` を使い、既存 private names と CLI call sites を同じ module
  namespace に維持する。
- cache root の空値診断、limit の parse、zero 値、`--artifact-cache-dir` 併用要件、trim semantics は変更しない。
- zero limit parsing contract を main test suite で明示する。

## Evidence

- RED: `include!("artifact_cache_options.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_resolve_artifact_cache_limits_accepts_zero_values` と既存 artifact cache tests が pass。
- `lsharp-driver` unit 152 件が pass。`default_path_delegation` の既知 7 failures は origin/main の embedded
  component/selfhost default-path boundary として再現し、今回の helper 移動とは独立に分類した。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象
  Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2591 行から 2480 行、`artifact_cache_options.rs` は 112 行となった。

## Consequences

artifact cache option/maintenance boundary を CLI dispatch から独立してレビューできる。private API、CLI
wiring、cache behavior は維持される。default-path integration blocker、driver の追加 production 分割、
selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。

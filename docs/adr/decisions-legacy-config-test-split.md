# ADR: `lsharp-driver/config.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/config.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`config.rs` は `lsharp.toml` の設定 schema、default、validation、読み込み error を実装し、
同じファイルに TOML parsing、dependency variant、validation、error display を確認する 21 件の
回帰テストを保持していた。test-only fixture を分離すると、設定 production API と TOML fixture
の ownership/review 境界を明確にできる。

## Decision

- `Config` / `DependencySpec` / `ProjectConfig` / `ConstraintsConfig` / `DocReviewConfig` の
  schema、default、validation、load error semantics は変更しない。
- `#[cfg(test)] mod tests` の 21 件を
  `crates/lsharp-driver/src/config_tests.rs` へ移動する。
- `config.rs` は `#[cfg(test)] #[path = "config_tests.rs"] mod tests;` で既存の
  `config::tests` namespace を維持する。
- TOML field name、default value、validation message、legacy `load_config` fallback contract
  は同一コミットで変更しない。

## Evidence

- 分離前後の `config::tests` focused gate: 21 passed。
- `config.rs` は 607 行から 291 行へ、`config_tests.rs` は 316 行となった。
- driver unit package lane: 132 passed。`cargo test -p lsharp-driver` の
  `tests/default_path_delegation.rs` は既存の embedded component / selfhost artifact default-path
  failure boundary で green にならず、`test_driver_component_compile_absolute_input_uses_host_artifact_fallback`
  では stack overflow も再現した。今回の config 分離は driver command と integration fixture を
  変更していない。
- `cargo test -p lsharp-driver --doc` は binary-only package のため `no library targets found`
  となり、doc-test は適用対象外。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、Rust 2024 rustfmt、
  `git diff --check`、`bash scripts/audit_docs.sh` は pass。

## Consequences

設定 schema/loader production と TOML fixture の ownership/review 境界が明確になり、21 件の
回帰テストを単独で再実行できる。driver の他の大規模 file、production の責務分割、I-01 / I-08
aggregate は未完了であるため、TODO の partial slice を維持する。

# ADR: `lsharp-tooling/fmt.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-tooling/src/fmt.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`fmt.rs` は source-aware formatter の検証、差分判定、`fmt` サブコマンドの file I/O を担う
production code と、roundtrip、output、check、parse error、diagnostic code を確認する 6 件の
回帰テストを同じファイルに保持していた。test-only fixture を分離すると、tooling formatter
production の変更と CLI fixture の ownership/review 境界を明確にできる。

## Decision

- `format_source`、`check_format`、`cmd_fmt` と formatter/diagnostic semantics は変更しない。
- `#[cfg(test)] mod tests` の 6 件を `crates/lsharp-tooling/src/fmt_tests.rs` へ移動する。
- `fmt.rs` は `#[cfg(test)] #[path = "fmt_tests.rs"] mod tests;` で既存の `fmt::tests` namespace を維持する。
- CLI fmt surface、source-aware formatting、driver I/O error code 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `fmt::tests` focused gate: 6 passed。
- `cargo test -p lsharp-tooling --doc`: 0 passed、0 failed。
- `cargo test -p lsharp-tooling` は 130 passed / 2 failed。失敗は既知の metadata property `LS2005` vacuous failure（`test_run_metadata_tests_executes_bool_property_binder`、`test_run_metadata_tests_rejects_bool_property_above_two_cases`）で、fmt 差分とは無関係である。
- `cargo clippy -p lsharp-tooling --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `fmt.rs` は 151 行から 76 行へ、`fmt_tests.rs` は 75 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

tooling formatter production と CLI fixture の ownership/review 境界が明確になり、6 件の回帰
テストを単独で再実行できる。metadata property failure boundary、formatter production の追加責務
分割、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。

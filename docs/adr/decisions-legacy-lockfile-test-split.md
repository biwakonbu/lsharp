# ADR: `lockfile.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/lockfile.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`lockfile.rs` は lockfile の生成・TOML 読み書きという production code と、Path/Version/Git 依存および round-trip の 5 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、lockfile production の変更と依存解決 fixture の ownership/review 境界を明確にできる。

## Decision

- `Lockfile`、`LockEntry`、生成・読み書き API、TOML の出力形式と semantics は変更しない。
- `#[cfg(test)] mod tests` の 5 件を `crates/lsharp-driver/src/lockfile_tests.rs` へ移動する。
- `lockfile.rs` は `#[cfg(test)] #[path = "lockfile_tests.rs"] mod tests;` で既存の `lockfile::tests` namespace を維持する。
- lockfile runtime、Config/DependencySpec、CLI の install 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `lockfile::tests` focused gate: 5 passed。
- `cargo test -p lsharp-driver` の unit suite: 132 passed。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `lockfile.rs` は 276 行から 143 行へ、`lockfile_tests.rs` は 133 行となった。
- `cargo test -p lsharp-driver` の `default_path_delegation` は 34 passed / 12 failed。失敗は embedded component / selfhost artifact の既存 failure boundary（selfhost summary の期待差、Preview1 runtime 出力差、`build-wasm-bytes-wasi` 未定義）で、test-only 分離差分とは無関係であることを確認した。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

lockfile production と依存 fixture の ownership/review 境界が明確になり、5 件の回帰テストを単独で再実行できる。driver の embedded component integration、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。

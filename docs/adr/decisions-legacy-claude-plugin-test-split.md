# ADR: `lsharp-driver/claude_plugin.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/claude_plugin.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`claude_plugin.rs` は Claude の MCP 設定・skill 配布を行う production code と、設定の
既存値保持、初期設定生成、driver I/O 診断コード、skill template の workflow/SSOT 契約を
確認する 5 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、
plugin installation production と CLI fixture の ownership/review 境界を明確にできる。

## Decision

- `language_guide_markdown`、`cmd_claude_plugin`、設定/skill installation、JSON object helper と
  driver I/O / diagnostic semantics は変更しない。
- `#[cfg(test)] mod tests` の 5 件を
  `crates/lsharp-driver/src/claude_plugin_tests.rs` へ移動する。
- `claude_plugin.rs` は `#[cfg(test)] #[path = "claude_plugin_tests.rs"] mod tests;` で既存の
  `claude_plugin::tests` namespace を維持する。
- Claude plugin CLI surface、`settings.json` merge、skill template contract は同一コミットで
 変更しない。

## Evidence

- 分離前後の `claude_plugin::tests` focused gate: 5 passed。
- `claude_plugin.rs` は 215 行から 110 行へ、`claude_plugin_tests.rs` は 126 行となった。
- `cargo test -p lsharp-driver` の unit lane は 132 passed。`default_path_delegation` は 34 passed / 12 failed で、失敗は embedded component / selfhost artifact の既知 failure boundary（selfhost summary の期待差、Preview1 runtime 出力差、`build-wasm-bytes-wasi` 未定義）に集中し、test-only 分離差分とは無関係である。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`: pass。

## Consequences

Claude plugin installation production と CLI fixture の ownership/review 境界が明確になり、
5 件の回帰テストを単独で再実行できる。driver の他の大規模ファイル、production の追加責務
分割、embedded component / selfhost artifact の failure boundary、I-01 / I-08 aggregate は
未完了であるため、TODO の partial slice を維持する。

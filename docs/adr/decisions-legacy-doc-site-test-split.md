# ADR: `doc_site.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/doc_site.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`doc_site.rs` は公開ドキュメントサイトの生成 API と 8 件の回帰テストを同じファイルに保持し、840 行になっていた。サイト生成の production 差分と、manifest・生成物を確認する test-only 差分を分離すると、後続の docs site 改修を小さい範囲でレビューできる。

## Decision

- 公開 API、サイト生成の意味論、manifest、fixture は変更しない。
- `#[cfg(test)] mod tests` の 8 件を `crates/lsharp-driver/src/doc_site/tests.rs` へ移動する。
- `doc_site.rs` は `#[cfg(test)] mod tests;` で既存の `doc_site::tests` namespace を維持する。
- production の責務分割、driver の他コマンド分割、I-01 / I-08 aggregate は別タスクとして残す。

## Evidence

- 分離前後の `doc_site::tests` focused gate: 8 passed。
- 分離後の `cargo test -p lsharp-driver --bin lsharp`: 132 passed。
- `doc_site.rs` は 840 行から 575 行へ、`doc_site/tests.rs` は 266 行となった。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-driver --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`、`bash scripts/audit_docs.sh`: pass。
- `cargo test -p lsharp-driver` の integration lane は `default_path_delegation` の 12 件で失敗した。失敗は embedded component / selfhost artifact の既存境界に集中し、本変更はそのテスト・production path を変更していないため、この ADR の test split とは別の未解決 failure boundary として記録する。分離前の full integration baseline は採取していないため、厳密な「既存」判定ではなく、今回の差分外として扱う。

## Consequences

doc site の production API と test-only fixture の ownership/review 境界が明確になり、8 件の回帰テストを単独で再実行できる。`main.rs`、`wasi.rs`、`infer.rs`、`ir/lib.rs` など他の大規模 production file と I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。

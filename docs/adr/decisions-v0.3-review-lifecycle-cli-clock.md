# ADR: `lsharp validate` の review lifecycle clock 評価

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `crates/lsharp-driver/tests/review_input_cli.rs`
- Related: `EC-M3-02`, `EC-M3-03`

## Context

Review の署名と lifecycle event を `lsharp validate` へ明示入力できるようになったが、公開 CLI の
回帰テストには「将来の lifecycle transition を `--review-now` より前に適用しない」という契約が
なかった。これが崩れると、まだ有効時刻に達していない `active` transition を先取りして、同じ
attestation を誤って `verified` と報告する。

## Decision

- `validate` は指定された `--review-now` を lifecycle reducer の評価時刻として使う。
- `effective_at` が評価時刻より未来の event は現在の review state に反映しない。
- 署名済み attestation と future transition を同じ fixture に固定し、transition 前は
  `unverified`、transition 到達後は `verified` を JSON report へ投影する。
- provider snapshot、selfhost/native parity、Mac/Linux artifact/runtime はこの CLI 回帰 slice の
  責務外とし、別の未完了境界として扱う。

## Evidence

- RED: `validate_does_not_apply_future_lifecycle_transition_before_review_now` を追加し、
  `2026-08-01T12:00:00Z` と `2026-08-02T00:00:00Z` の observable state を固定した。
- GREEN: `cargo test -p lsharp-driver --test review_input_cli validate_does_not_apply_future_lifecycle_transition_before_review_now -- --exact` が pass。
- Regression: `cargo test -p lsharp-driver --test review_input_cli` は 18件 pass。

## Boundary

これは Rust driver の公開 `validate` が explicit clock を尊重する verified partial slice である。
selfhost/native の同一 CLI 経路、provider adapter、report/MCP projection、Mac/Linux artifact/runtime、
EC-M3-02/03 の aggregate completion は未完了のため `TODO.md` の `[~]` を維持する。

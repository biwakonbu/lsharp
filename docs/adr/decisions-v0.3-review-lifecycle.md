# ADR: v0.3 review lifecycle の append-only reducer

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types` の review lifecycle event と deterministic reducer
- Related: `EC-M3-02`、[`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-attestation-canonical-bytes.md`](decisions-v0.3-review-attestation-canonical-bytes.md)

## Context

review の signature が検証できても、後から supersede/revoke された review を
`independent-review` として数えると、過去の validation report と現在の trust state が
食い違う。provider snapshot の取得は外部境界として残しつつ、L# 内では event の順序と
許可された遷移を deterministic に再生する必要がある。

## Decision

- lifecycle event は typed `ReviewId`、1-based `sequence`、`state`、`effective_at`、
  optional `reason_digest` を持つ。
- `effective_at` は attestation と共有する `YYYY-MM-DDTHH:MM:SSZ` の strict UTC timestamp
  parser を通り、形式不正・存在しない日付・秒範囲外を registry へ入れない（詳細は
  [`decisions-v0.3-review-lifecycle-effective-timestamp.md`](decisions-v0.3-review-lifecycle-effective-timestamp.md)）。
- 初期 state は `proposed` または `active` に限定する。
- 許可する遷移は `proposed → active`、`active → superseded`、`active → revoked` のみとする。
- `superseded` / `revoked` は terminal state とし、後続 event で `active` へ戻さない。
- 同じ review ID の event は sequence を単調増加させ、同一 sequence・巻き戻しを拒否する。
- registry の公開 view は review ID の lexical order、各 review 内の sequence order で flatten する。
  declaration order や provider response order を結果へ持ち込まない。
- event の provider 取得、signature verification、clock/expiry 判定はこの reducer の外部入力境界とする。

## Evidence

- RED: `crates/lsharp-types/tests/review_lifecycle.rs` を先に追加し、未公開の lifecycle
  module import が解決できないことを確認した。
- GREEN: `cargo test -p lsharp-types --test review_lifecycle`（4 passed）。
- Regression: `cargo test -p lsharp-types --lib`（221 passed）、
  `review_attestation`（4 passed）、`intent_ast`（4 passed）、`intent_node_wire`（3 passed）。
- Formatting/contract: 新規 lifecycle files の `rustfmt --check` と `git diff --check` を通過した。

## Boundary

これは in-memory canonical reducer の verified partial slice である。strict `effective_at`
timestamp は別 ADR で追加したが、event の JSON/manifest schema、snapshot file path policy、
signature/key verification、attestation expiry clock の report 接続、M2 report の
`stale_reviews` / `unknown` 投影、CLI/MCP/source/selfhost/native parity、Mac Apple Silicon /
Linux x86_64 artifact/runtime evidence は未完了であり、v0.3 設計文書の後続タスクへ残す。

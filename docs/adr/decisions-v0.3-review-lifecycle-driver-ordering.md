# ADR: v0.3 review lifecycle の CLI/MCP 入力順非依存契約

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp-driver` の `validate` CLI / `lsharp_validate` MCP review input boundary
- Related: [`decisions-v0.3-review-lifecycle-wire-ordering.md`](decisions-v0.3-review-lifecycle-wire-ordering.md)、
  [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  `EC-M3-03`

## Context

review lifecycle wire は provider が返す配列の宣言順を意味論にしてはならない。Rust の
wire parser/reducer は `(review_id, sequence)` で lifecycle を正規化するが、公開入口が
ordered input だけを検証していると、CLI/MCP 間で順序依存の回帰を見逃す。

また lifecycle input は review evidence identity の digest に含まれるため、同じ snapshot を
配列順だけ変えた入力が異なる identity になると、検証済み evidence の再現性を失う。

## Decision

- CLI `validate` と MCP `lsharp_validate` は、同じ lifecycle events を任意の配列順で受けても、
  reducer の現在 state と `review_verifications` を同じ値へ投影する。
- lifecycle component digest は parser の deterministic JSON projection を通じて計算し、入力の
  配列順だけでは `review_evidence_identity.lifecycle_digest` を変えない。
- この契約は revoked など terminal state を含む explicit review context で検証し、legacy の
  review input 省略経路や selfhost/native parity を暗黙に完了扱いしない。

## Evidence

- CLI integration test `validate_projects_out_of_order_lifecycle_as_revoked_with_stable_identity`
  は ordered/reversed の二つの wire を `validate --format json` へ渡し、両方が `revoked` と同一
  lifecycle digest を返すことを確認する。
- MCP test `test_validate_tool_reduces_out_of_order_lifecycle_and_stabilizes_identity` は同じ
  fixture を `lsharp_validate` へ渡し、`review_verifications` と identity digest の一致を確認する。
- `cargo test -p lsharp-driver --test review_input_cli
  validate_projects_out_of_order_lifecycle_as_revoked_with_stable_identity` が 1 passed。
- `cargo test -p lsharp-driver
  test_validate_tool_reduces_out_of_order_lifecycle_and_stabilizes_identity` が 1 passed。
- `git diff --check` は通過した。変更対象ファイル全体の rustfmt check には、今回触れていない
  既存 assertion の整形差分が残っているため、無関係な差分は変更していない。

## Boundary

これは Rust host の CLI/MCP 公開入力境界における verified partial slice である。selfhost/native
stage0 の lifecycle parser、Mac Apple Silicon / Linux x86_64 artifact/runtime parity、provider
snapshot の取得・署名 trust policy、`TODO.md` の EC-M3-03 全要件は未完了であり、該当項目は
`[~]` のまま維持する。

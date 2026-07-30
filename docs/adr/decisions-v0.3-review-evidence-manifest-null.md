# ADR: `review_evidence_identity` の明示 `null` 拒否

- Status: Accepted (verified slice)
- Date: 2026-07-31
- Scope: Rust version 1 intent/evidence graph manifest input と `lsharp validate`
- Related: [`decisions-v0.3-review-evidence-manifest-identity.md`](decisions-v0.3-review-evidence-manifest-identity.md)、
  [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)

## Context

`review_evidence_identity` は manifest に存在しない場合だけ省略でき、存在する場合は
`subject_digest`、`source_commit`、`artifact_digest`、nullable な trust/lifecycle digest、
`now` を持つ object でなければならない。Serde の通常の `Option<T>` は明示された `null` を
`None` として扱うため、壊れた identity を「identity なし」として黙って受理していた。
これは schema の `type: object` と、入力エラー時の no-report/no-manifest 境界に反する。

## Decision

- `review_evidence_identity` に custom deserializer を適用し、field の省略は許可する。
- field が存在する場合は object のみを許可し、明示 `null` は
  `review_evidence_identity must be an object when present` として拒否する。
- 既存の object 内 nullable field（`trust_store_digest` / `lifecycle_digest`）の `null` は互換性の
  ため引き続き許可する。
- parser error は report と manifest を生成しない既存の CLI/MCP fail-closed boundary を再利用する。

## Evidence

- RED: `validation_manifest_review_identity::manifest_identity_rejects_explicit_null` が、修正前に
  明示 `null` を identity なしとして受理することを確認した。
- GREEN: `cargo test -p lsharp-types --test validation_manifest_review_identity`（5 tests）と
  `manifest_input_cli::validate_rejects_null_review_evidence_identity_without_outputs`（1 test）が
  passした。
- object の round-trip、nullable digest の明示、identity conflict、malformed `now` の既存境界は
  同じ focused suite で維持されている。

## Boundary

これは Rust manifest/parser と Rust CLI の入力境界を閉じる verified slice である。selfhost/native
manifest producer parity、current-source と packaged stage0 artifact/runtime、supported 2 target の
release evidence は引き続き未完了であり、M2/M3 aggregate の完了は宣言しない。

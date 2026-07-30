# ADR: v0.3 selfhost review evidence identity projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: selfhost source evidence graph の explicit review evidence identity
- Related: [`decisions-v0.3-review-verification-manifest-projection.md`](decisions-v0.3-review-verification-manifest-projection.md)、
  [`decisions-v0.3-selfhost-source-attestation-report-projection.md`](decisions-v0.3-selfhost-source-attestation-report-projection.md)、
  `EC-M3-05`

## Context

Rust の validation manifest/report/MCP wire は、review verification の入力 identity を
`subject_digest`、`source_commit`、`artifact_digest`、nullable な
`trust_store_digest` / `lifecycle_digest`、`now` の六つの明示 field で保持する。selfhost の
`Tools.Validation.Evidence` は source graph、review、attestation、manifest projection までを
持つが、この identity を保持する境界がなく、Rust と同じ manifest を再現できなかった。

selfhost source は trust store、lifecycle、artifact、clock を推測してはいけない。identity は
caller が明示的に渡した値だけを採用し、検証できない trust/lifecycle は `null` のまま残す
必要がある。

## Decision

- `ReviewIdentity.ls` に explicit identity の constructor、required field、optional digest、UTC
  timestamp の fail-closed validation を置く。内部 record は
  `[subject, source, artifact, trust-or-empty, lifecycle-or-empty, now]` とする。
- evidence graph の既存 `[nodes, edges, registry, reviews, attestations]` shape は壊さず、明示
  identity を attach した場合だけ第六 field として保持する。未指定 graph の manifest shape は
 変えない。
- 同一 identity の再 attach は idempotent に許可する。異なる identity の attach は code `14`
  の診断で拒否し、既存 identity を上書きしない。
- manifest JSON は Rust wire と同じ固定順
  `subject_digest` → `source_commit` → `artifact_digest` → `trust_store_digest` →
  `lifecycle_digest` → `now` で出力する。空の optional digest は省略せず `null` を出力する。
- manifest の top-level field も Rust wire と同じ
  `schema_version` → `nodes` → `evidence` → `reviews?` → `review_evidence_identity?` → `edges`
  の順に出力し、非空の lifecycle digest は文字列のまま保持する。
- この slice は identity の保持・wire projection のみを扱い、trust store/lifecycle による
  `verified` / `stale` / `revoked` 判定、CLI/MCP の option/context wiring、artifact/release gate
  は別タスクとして残す。

## Evidence

- RED: `test_e2e_selfhost_evidence_registry_projects_review_identity_and_rejects_conflict` を
  identity API 未実装の `origin/main` で実行し、`UndefinedVar`（identity constructor）を確認した。
- GREEN: 同テストで valid identity の attach、同値再 attach、競合拒否、不正 timestamp 拒否を
  selfhost Wasm runtime で確認し、manifest の raw JSON field order と nullable `null` を固定した。
- Lifecycle parity: `test_e2e_selfhost_evidence_registry_projects_non_null_identity_in_rust_manifest_order`
  を追加し、trust/lifecycle digest の非NULL projection、同値再 attach、Rust と同じ top-level field
  順を selfhost Wasm runtime で固定した。identity module は 2 tests passed。
- Rust oracle: `cargo test -q -p lsharp-types --test validation_manifest_review_identity`（5 passed）。
- Regression: selfhost evidence registry runtime（57 passed）、source attestation manifest projection
  test、evidence parser duplicate-field contract が通過した。

## Boundary

これは macOS 上の Rust host が生成した selfhost Wasm の verified slice である。native stage0 の
current-source/package provenance、Mac Apple Silicon / Linux x86_64 native parity、selfhost
CLI/MCP の explicit context wiring、外部 trust/lifecycle provider、artifact/release gate は未完了。
したがって `TODO.md` の `EC-M3-05` と関連する selfhost parity 項目は `[~]` のまま維持する。

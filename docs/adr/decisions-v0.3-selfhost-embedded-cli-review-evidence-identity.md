# ADR: v0.3 selfhost EmbeddedCli の review evidence identity wiring

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `selfhost/src/App/EmbeddedCli.ls` の `validate --source` CLI
- Related: [`decisions-v0.3-selfhost-review-evidence-identity.md`](decisions-v0.3-selfhost-review-evidence-identity.md)、
  [`decisions-v0.3-selfhost-source-attestation-report-projection.md`](decisions-v0.3-selfhost-source-attestation-report-projection.md)、
  `EC-M3-05`

## Context

`Tools.Validation.ReviewIdentity` は source evidence graph に explicit identity を保持し、
manifest JSON を Rust wire と同じ順序へ投影できる。しかし selfhost `EmbeddedCli` の
`validate --source` は context option を受け付けず、report には identity がなく、Rust host の
CLI と同じ review evidence provenance を再現できなかった。

identity は caller が明示した subject、source commit、artifact、optional trust/lifecycle、
clock だけから構築し、未指定値や invalid timestamp を推測してはならない。identity を付けても
review verification が未検証なら validation status は `unknown` のまま保持する。

## Decision

- `EmbeddedCli` の validate option parser は次の six flags を保持する。
  `--review-subject-digest`、`--review-source-commit`、`--review-artifact-digest`、
  `--review-trust-store-digest`、`--review-lifecycle-digest`、`--review-now`。
- identity context は subject/source/artifact/now を all-or-none とし、partial または空値を
  report/manifest 生成前に option error として拒否する。trust/lifecycle は省略可能で、値がなければ
  manifest JSON では `null` とする。
- valid context は `source-review-evidence-identity-result` で canonical timestamp を検証し、
  `source-evidence-graph-attach-review-identity` で graph へ attach する。identity validation または
  conflict が失敗した場合は stable source validation error（code `14`）で停止し、report/manifest
  を出力しない。
- JSON report と emitted manifest は同じ `review_evidence_identity` object を出力する。text report は
  `review-evidence-identity: subject=... source=... artifact=... trust-store=... lifecycle=... now=...`
  の deterministic line を追加する。identity がない既存 validate output は変更しない。
- CLI integration の実行経路は review status を変更しない。external trust/lifecycle がない
  `unverified` review は exit `2` (`unknown`) として返す。

## Evidence

- RED: `test_e2e_selfhost_embedded_cli_validate_projects_explicit_review_evidence_identity` を
  option parser 未実装状態で実行し、`--review-subject-digest` の validate option error（exit `1`）を
  確認した。
- GREEN: 同じ Rust-host selfhost Wasm E2E で six flags の valid context を渡し、exit `2` の JSON
  report と emitted manifest の `review_evidence_identity` が同一 object（subject/source/artifact/
  trust/lifecycle/now）になることを確認した。
- Bundle gate: `selfhost_embedded_cli_runtime_bundle` を current source から compile/run し、focused
  E2E が `1 passed`（262.76s）で完了した。
- Optional-null text gate: `test_e2e_selfhost_embedded_cli_validate_text_projects_optional_identity_as_dash`
  を追加し、trust/lifecycle digest を省略した明示 identity が exit `2` の deterministic text report へ
  `trust-store=- lifecycle=-` として投影されることを selfhost Wasm runtime で確認した（1 passed、261.18s）。
- Rust oracle: `cargo test -q -p lsharp-driver --test review_input_cli validate_projects_review_evidence_identity_for_explicit_artifact_context -- --exact --nocapture`
  （1 passed）で同じ optional-null text contract を確認した。
- Rust test binary compile、`git diff --check` は通過した。

## Boundary

これは macOS 上の Rust host が生成した selfhost EmbeddedCli Wasm の verified partial slice である。
native stage0 の current-source/package provenance、Mac Apple Silicon / Linux x86_64 の native parity、
`App.Cli`/MCP wiring、trust/lifecycle provider に
よる `verified`・`stale`・`revoked` 判定、artifact/release gate は未完了である。したがって
`TODO.md` の EC-M3-05 と関連する selfhost parity 項目は `[~]` のまま維持する。

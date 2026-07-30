# ADR: v0.3 review explicit clock の共通 validation boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp-types` の review identity、Rust CLI/MCP の verification context
- Related: [`decisions-v0.3-review-explicit-context.md`](decisions-v0.3-review-explicit-context.md)、
  [`decisions-v0.3-review-evidence-identity.md`](decisions-v0.3-review-evidence-identity.md)、
  [`decisions-v0.3-review-lifecycle-effective-timestamp.md`](decisions-v0.3-review-lifecycle-effective-timestamp.md)

## Context

`review_now` は attestation と lifecycle の検証では strict UTC timestamp として扱っていたが、
verification input が省略された artifact-bearing context では非空文字列であれば受理されていた。
そのため malformed clock が `review_evidence_identity` に投影され、検証不能な入力が status 2 の
report/manifest として残る経路があった。manifest から identity を読み込む境界でも同じ値を
受理し得た。

## Decision

- `lsharp-types` に attestation/lifecycle と共有する public canonical timestamp validator を設ける。
  field ごとの診断を維持し、strict UTC (`...Z`) 以外は受理しない。
- `ReviewVerificationContext::from_options*` は trust/lifecycle/attestation の有無に関係なく、
  明示された `review_now` を context/report/manifest の生成前に検証する。
- `ReviewEvidenceIdentity::new` と manifest input の identity parser も同じ validator を使い、
  malformed `now` を identity として保存しない。
- malformed clock は CLI/MCP とも no-report/no-manifest の input error（status 1）とする。
  canonical clock は従来どおり identity と verification に投影する。

## Evidence

- RED: `validate_rejects_malformed_review_clock_without_verification_inputs` は、検証入力なしの
  malformed clock が status 2/report になる回帰を固定した。
- GREEN: 同 CLI test、MCP の verification-input なし clock test、
  `review_evidence_identity_rejects_malformed_now`、`manifest_identity_rejects_malformed_now` が pass。
- Regression: `lsharp-types` 全体、`lsharp-driver` binary 全体、review-input CLI 全体を pass。

## Boundary and follow-up

これは Rust の model/CLI/MCP 境界を閉じた verified partial slice である。selfhost/native parity、
Mac Apple Silicon / Linux x86_64 の native artifact/runtime、公開 surface 全体の証跡は未完了であり、
`TODO.md` の EC-M3 系項目を `[~]` のまま維持する。

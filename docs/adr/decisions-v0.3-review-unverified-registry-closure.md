# ADR: v0.3 review registry-only input の unverified closure

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: Rust CLI/MCP の review verification facts と manifest projection
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-explicit-state-wiring.md`](decisions-v0.3-review-explicit-state-wiring.md)、
  [`decisions-v0.3-review-evidence-manifest-identity.md`](decisions-v0.3-review-evidence-manifest-identity.md)

## Context

M2 の manifest は review registry だけを持てる。M3 の explicit verification input が渡された
ときに、その registry に attestation がない review を verification facts から落とすと、report
と manifest が「未検証」を記録せず、入力欠落を成功扱いに見せてしまう。これは provider から補完
する理由にはならず、明示 input の不足として `unverified` を保持する必要がある。

## Decision

- CLI/MCP の review verification が明示 context、trust store、または lifecycle snapshot の
  いずれかを受け取った場合、graph の registry review のうち attestation fact がない ID を
  `ReviewVerificationState::Unverified` で補完する。
- 補完後の facts は既存の review-ID canonical sort、duplicate rejection、`Invalid` fail-closed
  projectionを通り、CLI report、MCP output、manifest の同じ projectionへ渡す。
- review verification input/context が全くない既存 invocation では facts を補完せず、M2 の
  legacy report/manifest shape を維持する。
- provider、environment、system clock、manifest 外の attestation は補完に使わない。

## Evidence

- RED: review registry だけの manifestに explicit subject/source/clock を渡した CLI fixtureで、
  `review_verifications` と `reviews[].verification_state` が欠落することを確認した。
- GREEN: CLI 15件と MCP 7件の focused suiteで、registry-only fixtureが `unverified` fact、exit
  `2`、manifest stateを返し、legacy no-input pathの field omissionも維持することを固定した。
- 既存の attestation state、signature error、legacy no-input path は同じ canonical projection
  の focused regression suiteで確認した。

## Boundary

これは EC-M3-03/05 の Rust CLI/MCP verified partial sliceである。source/selfhost/native producer
parity、provider helper boundary、current-source と packaged stage0 の provenance、Mac Apple
Silicon / Linux x86_64 の release artifact/runtime gateは未完了であり、`TODO.md` の EC-M3-05 は
`[~]` のまま維持する。

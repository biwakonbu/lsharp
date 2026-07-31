# ADR: v0.3 stage0 package / release-smoke の provider snapshot wiring

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `package-native-stage0-release.sh` と `release-smoke.sh` の offline identity provider input
- Related: [`decisions-v0.3-native-release-snapshot-wiring.md`](decisions-v0.3-native-release-snapshot-wiring.md)、
  [`decisions-v0.3-provider-snapshot-digest-verification.md`](decisions-v0.3-provider-snapshot-digest-verification.md)、
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

native-only `scripts/release.sh` で検証した trust-store / lifecycle snapshot が、stage0 package と
release smoke へ渡らなければ、同じ archive identity を後段で再検証できない。後段が provider helper
や network を暗黙に呼ぶことなく、caller が明示した raw snapshot bytes を同じ verifier へ渡す必要がある。

## Decision

- `package-native-stage0-release.sh` は `--review-trust-store` と `--review-lifecycle` を受け取り、
  identity file と両方が指定された場合だけ `verify-native-release-identity.py` へ渡す。
- stage0 package は snapshot bytes 自体を archive へコピーしない。archive の identity payload と
  caller が保持する snapshot path を分離し、公開物へ provider data を持ち込まない。
- `release-smoke.sh` は `RELEASE_REVIEW_TRUST_STORE` と `RELEASE_REVIEW_LIFECYCLE` を同時に受け取り、
  native-only archive の `review-evidence-identity.json` を同じ raw bytes で再検証する。片側指定、空
  snapshot、identity payload のない archive への snapshot 指定は fail-closed とする。
- native-only archive の rollback compatibility archive を再帰的に smoke するときは、provider snapshot
  env を明示的に空にして rollback 側へ誤適用しない。rollback archive は native-only identity gate の
  対象外である。
- `native-official-release-local.sh` の multi-target offline propagation は
  [`decisions-v0.3-native-official-multitarget-snapshot-wiring.md`](decisions-v0.3-native-official-multitarget-snapshot-wiring.md)
  へ分離して verified partial とした。provider API/authentication、current source の Mac Apple Silicon /
  Linux x86_64 runtime はこの sliceの外に残す。

## Evidence

- RED: stage0 package test が snapshot optionsを受け付けず失敗し、identity verifierの release-smoke
  marker test が snapshot env wiring 不在で失敗した。
- GREEN: `scripts/ci/test-native-stage0-release-package.sh` が正しい trust/lifecycle snapshot の
  package と改ざん trust snapshot の non-zero / `trust_store_digest` 診断を確認した。
- `scripts/ci/test-release-smoke-provider-snapshots.sh` は native-only archive と rollback fixture を
  実際に展開し、正しい snapshot の recursive smoke 成功と改ざん snapshot の digest mismatch を確認した。
- `python3 -m unittest scripts/ci/test-native-release-identity.py -v`（7 tests）、`bash -n`、
  `py_compile`、`git diff --check` が passした。

## Boundary and follow-up

stage0 package と release-smoke の offline propagation は verified partial slice である。multi-target
orchestrator の propagation は別 ADR で verified partial とした。実 provider取得・認証、current-source
stage0 の provenance、Mac Apple Silicon / Linux x86_64 の packaged runtime evidence は未完了であり、
`EC-M3-05` は `[~]` のまま残す。

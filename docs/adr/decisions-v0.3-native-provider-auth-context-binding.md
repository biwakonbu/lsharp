# ADR: native provider auth-context binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の provider-required identity gate

## Context

`review_evidence_identity` は trust-store と review-lifecycle snapshot の digest を保持できる。
しかし、identity に digest が埋め込まれているだけで、`--require-provider-input` に実際の snapshot
path を渡さない呼び出しも成功できた。この状態は provider の auth-context を current inputへ結び付けて
おらず、digest文字列だけを explicit provider evidence と誤認し得る。

## Decision

`--require-provider-input` を指定した場合は、`--trust-store` と `--review-lifecycle` の両方を必須にする。
path が無い場合は `provider auth context is required` で `UnverifiedIdentity` として終了し、digestが identity
に存在しても成功扱いにしない。path が指定された場合の regular-file、non-empty、digest、lifecycle shape
検証と、provider API取得・署名認証・lifecycleの完全な意味検証は既存の境界を維持する。

## Evidence

- RED: provider digestを持つ identityへ `--require-provider-input` だけを渡す fixtureが、snapshot pathなしで
  exit `0` になった。
- GREEN: 同じ fixtureが exit `2` と `provider auth context` 診断になる focused testを通過した。

## Boundary

これは provider-required release identityに実ファイルの auth-context bindingを要求する verified partial
sliceである。live provider API/auth acquisition、署名・lifecycleの実 semantic verifier、current-source Linux
runtime、両 target packaged/rollback parityは未検証であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま維持する。

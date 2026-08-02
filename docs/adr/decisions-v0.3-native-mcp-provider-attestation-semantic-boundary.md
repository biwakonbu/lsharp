# ADR: v0.3 native MCP provider semantic boundary for attestations

- Status: verified partial (2026-08-02)
- Scope: native MCP `lsharp_validate` provider snapshot postflight
- Related: EC-M3-01〜05, M3-04-N1, M3-05-N2, M3-05-N7, M3-05-N9

## Context

native MCP は trust-store と review-lifecycle snapshotを raw bytes と digestとして受け取り、native側で署名・lifecycle意味検証を行わない。
既存の fail-closed は `review_verifications[]` の semantic stateだけを対象にしており、source-owned `review_attestations[]` に
`verified`、`stale`、`revoked` が含まれていても、receiptが無い場合に通過できた。

## Decision

provider snapshotが指定された場合、native postflightは `review_verifications[]` と `review_attestations[]` の両方について、
`unverified` 以外の semantic stateを拒否する。ただし、receipt bindingで同じ review IDに明示的に束ねられた attestationは既存の
Rust verified handoffとして許可する。snapshotの読み込み、digest、regular-file、receipt projection、Rust側の暗号/lifecycle検証は変更しない。

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k provider_snapshot_semantic_attestation` で、provider snapshot付き
  `review_attestations[].state = verified` が受理された。
- GREEN: 同じ fixtureで source-attestation semantic stateを `provider semantic verification is unavailable` として拒否。
- Related batch: native MCP 96 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、
  source-file evidence harness、official two-target fake gate。

## Remaining boundary

このsliceはnativeが意味検証結果を暗黙に信頼しない境界だけを閉じる。native cryptographic verification、live provider/auth acquisition、
current-source Linux runtime、Mac/Linux packaged provenance/rollback bytes parityは未検証であり、関連TODOは `[~]` のまま維持する。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay、stage regeneration、full buildは起動しない。

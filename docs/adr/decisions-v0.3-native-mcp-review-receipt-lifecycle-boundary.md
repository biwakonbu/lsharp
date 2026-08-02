# ADR: v0.3 native MCP receipt lifecycle semantic boundary

- Status: verified partial (2026-08-02)
- Scope: native MCP `lsharp_validate` explicit receipt/lifecycle inputs
- Related: EC-M3-01〜05, M3-04-N1, M3-05-N2, M3-05-N7, M3-05-N9

## Context

`ReviewVerificationReceipt` の canonical contract は、Rustで検証済みの signature fact と review/provider/key/algorithm、attestation digest、
trust-store digest、verification clockを束ねる。receiptには lifecycle digest がなく、lifecycle identityや意味検証を表さない。
native MCPがreceiptと lifecycle snapshot/digestを同時に受け付けると、receiptの `verified` stateを lifecycle semantic verification済みと
誤って扱う余地がある。

## Decision

receiptと `review_lifecycle` または `review_lifecycle_digest` が同時に指定された場合、native MCPは native実行前に
`native MCP receipt cannot establish lifecycle semantic binding without lifecycle-bound receipt` で拒否する。
explicit trust-store digestだけのreceipt pathは既存の trust-store coherency boundaryで検証して受理する。receipt schemaやcanonical bytesは変更せず、
native cryptographic verifierやlifecycle semantic verifierも追加しない。

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k receipt_lifecycle_snapshot` でreceipt＋lifecycle snapshot/digestがnative実行まで進んだ。
- GREEN: 同じfilterで実snapshotとdigest-only lifecycleの両経路をnative起動前に拒否し、`native.log`を生成しないことを確認。
- Related batch: native MCP 98 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、source-file evidence、official fake two-target gate。

## Remaining boundary

native cryptographic verification、lifecycle semantic verification、live provider/auth acquisition、current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証である。
関連TODOは `[~]` のまま維持する。current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、
Linux replay、stage regeneration、full buildは起動しない。

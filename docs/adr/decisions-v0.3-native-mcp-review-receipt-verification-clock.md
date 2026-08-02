# ADR: v0.3 native MCP receipt verification clock context

- Status: verified partial (2026-08-02)
- Scope: native MCP `lsharp_validate` explicit receipt and review identity
- Related: EC-M3-01〜05, M3-04-N1, M3-05-N2, M3-05-N7, M3-05-N9

## Context

`ReviewVerificationReceipt` はRust側の外部 signature verifierが、明示 trust storeと `verification_now` を使って得た verified factを
nativeへ handoffする。native MCPはreceiptの canonical shapeを検証し、receiptを report/manifestへ投影するが、native暗号検証は行わない。
callerが別の `review_now` を明示してもreceiptの検証時刻を比較しなければ、別のverification contextのverified factを現在のreviewへ再利用できる。

## Decision

receiptと明示 review identityが同時に指定される場合、native MCPはreceiptの `verification_now` と callerの `review_now` を比較する。
不一致は native実行前に `native MCP receipt verification clock mismatch with review context` で拒否し、一致だけを受理する。
receipt schema/canonical bytes、lifecycle digest、native cryptographic verifierは変更・追加しない。receiptなし、または caller identityなしの既存経路は従来の外部境界として維持する。

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k receipt_verification_clock_context_binding` でreceiptの別時刻がnative実行まで進んだ。
- GREEN: 同じfixtureで一致時刻を受理し、不一致時刻をnative起動前に拒否し、`native.log`を生成しないことを確認。
- Related batch: native MCP 99 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、release identity 33 tests、source-file evidence、official fake two-target gate。

## Remaining boundary

これは外部 verifier contextの時刻coherencyであり、native cryptographic verificationやlifecycle semantic verificationの実装ではない。live provider/auth acquisition、current-source Linux runtime、Mac/Linux packaged/rollback parityも未検証で、関連TODOは `[~]` のまま維持する。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay、stage regeneration、full buildは起動しない。

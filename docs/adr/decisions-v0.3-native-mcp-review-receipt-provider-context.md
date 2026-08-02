# ADR: v0.3 native MCP receipt/provider snapshot context

- Status: verified partial (2026-08-02)
- Scope: native MCP `lsharp_validate` explicit receipt input
- Related: EC-M3-01〜05, M3-04-N1, M3-05-N2, M3-05-N7, M3-05-N9

## Context

native MCP は provider snapshotの raw bytesを digest化し、Rustが明示 trust storeで作った verification receiptを別入力として受け取る。
しかし、両方が指定されたとき receiptの `trust_store_digest` と current provider snapshotの digestを比較していなかったため、
別の trust-store contextで得た verified factを現在の入力へ再利用できた。

## Decision

receiptと provider snapshotが同時に指定された場合、native shimは receiptの `trust_store_digest` と current trust-store bytesのSHA-256を
native実行前に比較する。snapshot pathがなく明示 `review_trust_store_digest`だけが渡された場合も同じ値を比較する。不一致は
`native MCP receipt trust-store digest mismatch with provider snapshot` で fail-closed にする。

verification receipt schemaには lifecycle digestが存在しないため、このsliceで lifecycle stateをreceiptへ推論・追加しない。lifecycle意味検証、
live provider/auth acquisition、native cryptographic signature verificationは引き続き外部/Rust boundaryである。

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k receipt_provider_snapshot_context_binding` で異なる trust-store digestのreceiptがnative実行へ進んだ。
- GREEN: 同じfixtureで一致するcontextを受理し、不一致をnative起動前に拒否。
- Related batch: native MCP 97 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、source-file evidence harness、official two-target fake gate。

## Remaining boundary

このsliceはreceiptとtrust-store contextのcoherencyだけを検証する。native cryptographic verification、lifecycle semantic binding、live provider/auth acquisition、
current-source Linux runtime、Mac/Linux packaged provenance/rollback bytes parityは未検証であり、関連TODOは `[~]` のまま維持する。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay、stage regeneration、full buildは起動しない。

# ADR: v0.3 native MCP review-attestation receipt binding

- Status: verified partial (2026-08-02)
- Scope: native MCP `lsharp_validate` report handoff
- Related: EC-M3-01〜05, M3-04-N1, M3-05-N2, M3-05-N7, M3-05-N9

## Context

Rust は明示 trust store で検証した attestation から verification receipt を生成する。native MCP は既に receipt を
`review_verifications[]` と manifest reviewへ投影し、source routeの `review_attestations[]` wire shapeも受け入れていたが、
receipt付き reportの attestation projectionが別の review、provider/key、canonical bytesを指していても通過できた。
この状態では、receiptの `verified` factとsource-owned attestation materialの境界が結び付かない。

## Decision

receiptが指定され、reportに `review_attestations[]` が存在する場合、native postflightで次を fail-closed に検査する。

1. receiptと同じ `review_id` の attestation projectionが一つだけ存在する。
2. projectionの state が `verified` である。
3. provider、key_id、algorithm が receiptと一致する。
4. projectionの canonical bytesをSHA-256した値が receiptの `attestation_digest` と一致する。

これはRust verified receiptの deterministic identity/material handoffであり、native shimがEd25519署名を再検証したり、provider API/authを取得したりする契約ではない。
reportにsource attestationが存在しない従来のreceipt-only routeは変更せず、存在するprojectionだけをreceiptへ束ねる。

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k review_attestation_receipt` で、receipt付き
  reportの attestation 欠落が受理された。
- GREEN: 同じ filterで valid projection 1件と欠落/state/identity/digest不一致 4件が通過。
- Related batch: native MCP 95 tests、`cargo test -q -p lsharp-types --test review_verification_receipt` 4 tests、
  `cargo test -q -p lsharp-driver test_validate_tool_projects_source_attestation_as_unverified` 1 test、
  source-file evidence harness、official two-target fake gate。
- Python syntax、docs audit、`git diff --check` は実装後に実行する。

## Remaining boundary

このsliceはreceipt projectionの identity/material bindingだけを検証する。native cryptographic signature verification、live provider/auth acquisition、
current-source Linux runtime、Mac/Linux packaged provenance/rollback bytes parityは未検証であり、関連TODOは `[~]` のまま維持する。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay、stage regeneration、full buildは起動しない。

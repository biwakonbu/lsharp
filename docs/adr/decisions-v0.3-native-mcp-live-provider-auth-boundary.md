# ADR: v0.3 native MCP live provider/auth external boundary

- Status: verified partial (2026-08-02)
- Scope: native MCP `lsharp_validate` provider inputs
- Related: EC-M3-01〜05, M3-04-N1, M3-05-N2, M3-05-N7, M3-05-N9

## Context

native MCPはprovider snapshotをnetworkへ取りに行かず、明示された trust-store/review-lifecycle regular fileをraw bytesからdigest化して
既存 native commandへ渡す。live provider URLやauth tokenを未知引数として単に処理すると、利用者には取得が可能に見える一方、native側に
provider/auth verifierがない境界が曖昧になる。

## Decision

`provider_url`、`provider_api_url`、`provider_auth_token`、`provider_token`、`auth_token`、`auth_context` のようなlive provider/auth入力は、
native起動前に `live provider/auth acquisition is an external boundary; use explicit offline snapshots` で拒否する。offline snapshot pathは既存の
regular-file/nonempty/digest/semantic fail-closed contractを使い、network helper、auth acquisition、native cryptographic verificationは追加しない。

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k live_provider_auth_inputs_are_external` でprovider URL/tokenが従来の曖昧なunknown-argument診断に留まった。
- GREEN: 同じfixtureで URL/API URL/token/auth token の入力を全てnative起動前に明示拒否し、`native.log`を生成しないことを確認。
- Related batch: native MCP 100 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、release identity 33 tests、source-file evidence、official fake two-target gate。

## Remaining boundary

これはlive provider/auth acquisitionを実装した証拠ではなく、nativeの明示 external boundaryである。native cryptographic verification、live provider/auth実取得、current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証で、関連TODOは `[~]` のまま維持する。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay、stage regeneration、full buildは起動しない。

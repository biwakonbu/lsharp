# ADR: v0.3 selfhost attestation canonical bytes parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: selfhost `Tools.Validation.IntentSource` の attestation canonical bytes
- Related: [`decisions-v0.3-review-attestation-canonical-bytes.md`](decisions-v0.3-review-attestation-canonical-bytes.md)、
  [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  `EC-M3-01`、`EC-M3-04`

## Context

Rust の `ReviewAttestation` は、`lsharp.review-attestation.v1\0` の domain separator と
UTF-8 byte length-prefixed field を署名対象として固定している。selfhost の
`IntentSource.ls` にも同じ producer helper は存在していたが、Rust oracle と実行時の bytes を
比較するテストがなく、Unicode field、optional `expires_at`、`sequence` の境界が parity として
証明されていなかった。

canonical bytes の不一致は署名検証を target ごとに変質させるため、JSON の見た目や manifest
projection の一致だけでは十分ではない。provider authentication や lifecycle の解決はこの
slice の責務ではなく、caller が明示した source record を同じ bytes へ変換する境界だけを固定する。

## Decision

- selfhost `source-review-attestation-canonical-bytes` は Rust と同じ field order、big-endian
  `u64` length prefix、UTF-8 raw bytes、unsigned decimal `sequence` を正本 contract とする。
- optional `expires_at` が空の場合は、field を省略せず zero-length prefix として canonical bytes
  に残す。
- Rust-host Wasm E2E は Unicode を含む全 identity field と `expires_at` 有無の2 fixtureを同じ
  `ReviewAttestation::canonical_bytes()` と比較する。signature bytes は canonical bytes に含めない。
- この parity test は trust store、signature verification、lifecycle reducer、CLI/MCP wiring を
  selfhost 側へ暗黙に追加しない。未接続の境界は `unverified` / `[~]` のまま扱う。

## Evidence

- RED: 新しい parity test の初回実行で、selfhost `print` の自動改行を考慮していない test
  framing が検出され、bytes parser が空行で停止した。fixture の output contract を修正した。
- GREEN: `CARGO_TARGET_DIR=.../tmp/lsharp-m3-next-contract/target cargo test -q -p lsharp-wasm --test e2e 'e2e::selfhost_evidence_registry::attestation::selfhost_attestation_canonical_bytes_match_rust_for_utf8_and_optional_expiry' -- --exact --nocapture`（1 passed）。
- Rust oracle: `cargo test -q -p lsharp-types --test review_attestation`（4 passed）。
- Formatting/docs: 対象 Rust files の `rustfmt --edition 2024 --check`、`git diff --check`、
  `bash scripts/audit_docs.sh`（0 errors, 0 warnings）。

## Boundary

これは macOS 上の Rust host が生成した selfhost Wasm の canonical bytes parity verified slice で
ある。current source commit に一致する native stage0、packaged artifact、Mac Apple Silicon /
Linux x86_64 runtime、署名検証、trust store/lifecycle provider、CLI/MCP の selfhost parity は
未完了であり、`EC-M3-01` / `EC-M3-04` の `[~]` を維持する。

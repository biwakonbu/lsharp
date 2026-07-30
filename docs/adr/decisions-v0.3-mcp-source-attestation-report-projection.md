# ADR: v0.3 MCP source attestation report projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: Rust MCP `lsharp_validate` の `source` / `file` input における source attestation report/manifest projection
- Related: [`decisions-v0.3-source-attestation-report-projection.md`](decisions-v0.3-source-attestation-report-projection.md)、
  [`decisions-v0.3-source-attestation-producer.md`](decisions-v0.3-source-attestation-producer.md)、
  `EC-M3-04`

## Context

Rust source adapter は named-field `:review-attestation` を canonical record へ検証していたが、
MCP の source/file route は graph だけを受け取り、attestation を report へ投影していなかった。
そのため、同じ source を CLI と MCP で検証しても、MCP だけ `review_verifications` と
`manifest.reviews[].verification_state` が欠落し、source に存在する未検証状態を観測できなかった。

## Decision

- MCP の `source` / `file` route は source graph と `SourceReviewAttestation` records を同時に
  受け取り、各 record を `unverified` の `ReviewVerificationFact` として report へ投影する。
- `trust_store`、`review_lifecycle`、subject/source/artifact identity、clock から生成された
  外部 verification fact は、同じ review ID の source fact より優先する。source record 単体を
  `verified` へ昇格させず、外部入力がない場合は fail-open しない。
- report と optional manifest は同じ canonical fact vector を使う。manifest registry に存在する
  review のみ `verification_state` を更新し、registry にない外部 fact は report に残しても
  manifest review を新規作成しない。
- `manifest` / `manifest_file` route の入力契約と、MCP schema の optional verification fields は
  変更しない。source attestation の projection は Rust CLI と同じ report semantics に限定する。

## Evidence

- RED: `test_validate_tool_projects_source_attestation_as_unverified` は、実装前の source/file
  route で `review_verifications` が `null` となることを固定した。
- GREEN: 同テストで MCP source/file input が `unknown`、report の review state が `unverified`、
  `include_manifest` の registry state も `unverified` になることを固定した。
- `test_validate_tool_external_verification_overrides_source_unverified` で、同じ review ID の
  trust/lifecycle/context input が source の暫定 state を `verified` へ置き換えることを固定した。
- Regression: MCP binary focused suite（192 tests）、review input CLI（15 tests）、source
  attestation integration（2 tests）、types source attestation（4 tests）、変更 Rust の rustfmt、
  `cargo clippy -p lsharp-driver --bin lsharp -- -D warnings` を通過した。

## Boundary

これは Rust MCP source/file route の verified partial slice である。selfhost/native MCP、
current-source と packaged stage0 の provenance、canonical bytes の両 target parity、
Mac Apple Silicon / Linux x86_64 の runtime/release gate、`TODO.md` の EC-M3-04 全要件は未完了であり、
項目は `[~]` のまま維持する。

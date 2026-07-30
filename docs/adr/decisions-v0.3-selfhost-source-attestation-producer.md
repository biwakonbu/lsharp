# ADR: v0.3 selfhost source attestation producer parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: selfhost `IntentSource` の named-field `:review-attestation` consumer
- Related: [`decisions-v0.3-selfhost-attestation-canonical-bytes.md`](decisions-v0.3-selfhost-attestation-canonical-bytes.md)、
  [`decisions-v0.3-review-attestation-binding.md`](decisions-v0.3-review-attestation-binding.md)、
  [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  `EC-M3-04`

## Context

Parser の kind 20 (`:review-attestation`) は named fields と span を保持する primitive まで
実装されていたが、selfhost の `IntentSource` consumer が実際に source を parse し、producer
record として取り出す契約は未検証だった。parser の AST 保持や Rust source adapter の成功だけ
では、selfhost 側の field order、`unverified` state、canonical bytes、診断 span の parity は
証明できない。

## Decision

- selfhost source consumer は `review_id`、subject/source/provenance、provider/key、algorithm、
  signature、issued/expires、sequence の11 named fieldsを Rust source adapter と同じ順序で
  `source-review-attestation-record` へ投影する。
- producer 層は signature/trust/lifecycle を暗黙に解決せず、verification state を常に
  `unverified` として保持する。canonical bytes は Rust の length-prefixed UTF-8 contract と
  同じ field values から生成する。
- unknown algorithm などの invalid field は source graph error code `8`、kind `20`、review ID、
  directive span を保持して fail-closed に返す。provider network や trust store はこの層へ
  埋め込まない。

## Evidence

- RED: 新規 source consumer E2E を実装前に追加し、named-field source を selfhost parser と
  `IntentSource` reducer へ通す契約を固定した。
- GREEN: `CARGO_TARGET_DIR=/Users/biwakonbu/github/tmp/lsharp-m3-selfhost-attestation-source/target cargo test -q -p lsharp-wasm --test e2e 'e2e::selfhost_evidence_registry::source_attestation::' -- --nocapture`（2 passed）。
  valid fixture は全 field、`unverified`、span、Rust canonical bytes と一致し、unknown
  algorithm は code `8` / kind `20` / ID / span を返す。
- Rust oracle: `CARGO_TARGET_DIR=/Users/biwakonbu/github/tmp/lsharp-m3-selfhost-attestation-source/target cargo test -q -p lsharp-types --test review_attestation_source`（6 passed）。
- Contract: 対象 Rust files の `rustfmt --edition 2024 --check`、`git diff --check`、
  `bash scripts/audit_docs.sh`（0 errors, 0 warnings）を通す。

## Boundary

これは Rust host が生成・実行する selfhost Wasm における source producer の verified partial
slice である。native source-file smoke、current-source と packaged stage0 の provenance、
trust/signature/lifecycle verification、CLI/MCP wiring、Mac Apple Silicon / Linux x86_64 の
artifact/runtime gate は未完了であり、`EC-M3-04` の `[~]` を維持する。

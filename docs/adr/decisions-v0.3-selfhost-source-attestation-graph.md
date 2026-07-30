# ADR: v0.3 selfhost source attestation graph/manifest projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: selfhost `Tools.Validation.Evidence`、Rust-host Wasm producer/manifest lane
- Related: [`decisions-v0.3-source-attestation-producer.md`](decisions-v0.3-source-attestation-producer.md)、
  [`decisions-v0.3-source-attestation-report-projection.md`](decisions-v0.3-source-attestation-report-projection.md)、
  `EC-M3-04`

## Context

selfhost `IntentSource` は `:review-attestation` の named-field record、canonical bytes、span、
`unverified` state を単体では保持していた。しかし `source-evidence-graph-from-program` が
attestation を graph に渡していなかったため、selfhost の実際の source validation/manifest
経路では record が消え、Rust source adapter の report/manifest projection と観測結果が一致しなかった。

source は trust store、lifecycle、clock を持たないため、selfhost producer がこの record を
`verified` と解釈してはいけない。

## Decision

- selfhost evidence graph に attestation collection を optional な第5 fieldとして保持する。
  既存の node/edge/evidence/review field index は変更せず、旧 graph consumer は空 collection として
  後方互換に扱う。
- `source-evidence-graph-from-program` は nodes、reviews、attestations、evidence registry、edges
  を同じ source program から収集し、attestation validation failure は graph success に隠さない。
- manifest serializer は review ID が一致する source attestation の state を
  `reviews[].verification_state` として投影する。trust/lifecycle input がない source producer の
  state は常に `unverified` とし、attestation がない既存 `:review` は optional field を省略する。
- external trust/lifecycle/context の検証、`verified` への昇格、report の
  `review_verifications` projection は後続の selfhost CLI/native parity boundaryで行う。

## Evidence

- RED: graph-level E2E が未接続状態で `source-evidence-graph-attestations` undefinedとなり、
  manifest E2E は `reviews[0].verification_state` が `null` になることを確認した。
- GREEN: selfhost actual Wasm で graph が attestation ID/state を保持し、source review manifest が
  `verification_state: "unverified"` を投影することを固定した。
- `cargo test -p lsharp-wasm --test e2e selfhost_intent_source_adapter` は 38 tests pass。
- `cargo test -p lsharp-wasm --test e2e selfhost_evidence_registry` は 48 tests pass。
- 仕上げの source 系 focused lane 67 tests、`git diff --check` も passした。

## Boundary and follow-up

これは selfhost graph/manifest producer の verified partial sliceである。selfhost CLI の
JSON/text report facts、MCP、native source-file smoke、current-source と packaged stage0 の
provenance、Mac Apple Silicon / Linux x86_64 runtime parity は未接続であり、`TODO.md` の
`EC-M3-04` / `EC-M3-05` は `[~]` のまま維持する。

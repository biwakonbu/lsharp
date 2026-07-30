# ADR: v0.3 source attestation report projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: Rust `validate --source` の source attestation report/manifest projection
- Related: [`decisions-v0.3-source-attestation-producer.md`](decisions-v0.3-source-attestation-producer.md)、
  [`decisions-v0.3-review-unverified-registry-closure.md`](decisions-v0.3-review-unverified-registry-closure.md)、
  `EC-M3-04`

## Context

Rust source adapter は named-field `:review-attestation` を canonical record へ検証できていたが、
valid record を `source_program_to_intent_graph` の戻り値から捨てていた。そのため
`validate --source` は source に attestation が存在しても report の `review_verifications` と
manifest の `reviews[].verification_state` を生成せず、source の未検証状態を観測できなかった。

source 自体は trust store、lifecycle、current subject/source identity、clock を持たないため、
source record だけを `verified` と解釈してはならない。一方、同じ review ID に対する明示的な
trust/lifecycle input がある場合は、canonical verifier の結果を source の暫定 state より優先する
必要がある。

## Decision

- source adapter は graph と `SourceReviewAttestation` records を同時に返す。既存の graph-only
  API は wrapper として維持し、既存 consumer の挙動を変えない。
- Rust CLI の source validation は、各 source record を `ReviewVerificationState::Unverified`
  の report fact として投影し、同じ ID の source `unverified` fact は外部 verification fact が
  ある場合に置き換える。
- 外部 fact の重複 ID、source record の重複 ID、`Invalid` state は既存の canonical sort/dedup と
  fail-closed projectionへ渡す。source は trust、lifecycle、clock、network/provider から値を補完しない。
- manifest projection は report projection と同じ fact vector を使い、registry に存在する同名
  reviewだけに state を付与する。registry にない外部 factは reportには残るが、manifestへは追加しない。

## Evidence

- RED: source `:review` と valid `:review-attestation` を含む `validate --source --format json`
  で、実装前は `review_verifications` が `null`、manifest state が欠落した。
- GREEN: `crates/lsharp-driver/tests/validate_source_review_attestation.rs` で、外部 input
  なしの source record が `unverified`（exit `2`）として report/manifestへ投影されることを固定した。
- 外部 trust store、lifecycle、subject/source/clock context を同じ review IDへ与えた fixtureで、
  source `unverified` が `verified` に置き換わり、report/manifestの両方が同じ stateを返すことを固定した。

## Boundary

これは Rust CLI の source report/manifest projection に限定した verified partial sliceである。
MCP source/file route の report/manifest projection は
[`decisions-v0.3-mcp-source-attestation-report-projection.md`](decisions-v0.3-mcp-source-attestation-report-projection.md)
で接続した。一方、selfhost/native producer、source-file smoke、canonical bytes の byte-for-byte
target parity、current-source と packaged stage0 の provenance、Mac Apple Silicon / Linux x86_64
runtime/release gate は未完了であり、`TODO.md` の `EC-M3-04` は `[~]` のまま維持する。

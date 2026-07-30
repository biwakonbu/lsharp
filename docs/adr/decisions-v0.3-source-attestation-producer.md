# ADR: v0.3 source attestation producer parity slice

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp-syntax` source metadata、Rust source adapter、selfhost `IntentSource`
- Related: `EC-M3-04`、[`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-attestation-canonical-bytes.md`](decisions-v0.3-review-attestation-canonical-bytes.md)

## Context

v0.3 の attestation model と canonical bytes は Rust 側に存在するが、source syntax が
positional な既存 `:review` と混ざると、named-field の順序・欠落・unknown field が target ごとに
異なる解釈になり得る。selfhost/native producer が malformed attestation を graph/report の成功へ
隠すことも避ける必要がある。一方、source は trust store、lifecycle、implicit clock を保持する境界
ではないため、入力を `verified` へ昇格させてはならない。

## Decision

- 既存の positional `:review`（kind 16、opaque registry）は変更しない。
- 新しい `:review-attestation` は次の named fields だけを受理する。
  `review-id`、`subject-digest`、`source-commit`、`provenance-digest`、`provider`、`key-id`、
  `algorithm`、`signature`、`issued-at`、optional `expires-at`、`sequence`。
- Rust parser は field の重複・unknown field・positional payload・欠落を syntax error とし、
  `ReviewAttestationForm` と directive span を保持する。Rust source adapter は canonical
  `ReviewAttestation` へ投影し、algorithm、base64url signature、ID、canonical UTC timestamp、time window を
  model の fail-closed validation に渡す。
- selfhost parser は同じ payload 順の kind 20 form を生成する。`IntentSource` は Rust と同じ
  canonical bytes（domain separator と big-endian u64 length prefix）を作り、record に source
  span と `unverified` state を保持する。
- `source-graph-from-program` は attestation producer の validation を先に実行する。valid record
  は graph の `reviews` registry へは直接追加せず、Rust CLI の report/manifest projection へ
  source-owned `unverified` fact として渡す（詳細は
  [`decisions-v0.3-source-attestation-report-projection.md`](decisions-v0.3-source-attestation-report-projection.md)）。
  malformed/unsupported algorithm/invalid signature は source validation error として fail-closed
  にする。
- trust store、lifecycle、current-source identity、clock、manifest-side verification は source
  に埋め込まず、後続の EC-M3-04/05 boundary とする。

## Evidence

- RED: named-field form の parser variant、Rust source adapter producer、selfhost IntentSource
  accessors が未実装の状態で、それぞれの focused test が失敗することを確認した。
- GREEN: `cargo test -p lsharp-syntax --test review_attestation_source`（3 tests）、
  `cargo test -p lsharp-types --test review_attestation_source`（4 tests）が passした。invalid calendar
  timestamp と `expires-at <= issued-at` の source span 付き拒否も固定した。
- Rust source adapter は canonical bytes、`unverified`、directive span、unknown algorithm と
  invalid base64url の source error を固定した。
- selfhost actual Wasm E2E は named-field payload/state/span、Rust との canonical bytes
  byte-for-byte parity、algorithm/signature invalid の同一 error code `8` を固定した。
  selfhost parser metadata form の focused test も passした。
- 既存 selfhost IntentSource adapter suite 36 tests、`git diff --check` を通過した。Rust workspace
  の全体 rustfmt check は base branch に既存の未整形差分があるため、今回の完了証拠へ拡大解釈しない。

## Boundary and follow-up

これは source parser と producer の verified partial sliceである。Rust CLI の source
report/manifest projection は別 ADR の verified partial sliceとして接続済みだが、`Evidence`
consumer、selfhost/native producer parity、native source-file smoke、current-source と packaged
stage0 の provenance、Mac Apple Silicon / Linux x86_64 runtime へは未接続である。signature の
暗号学的 verification、trust/lifecycle、EC-M3-05 evidence identity は未完了であり、`TODO.md` の
`EC-M3-04` / `EC-M3-05` を `[~]` のまま維持する。

# ADR: v0.3 native source-file smoke の review attestation projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh` の source validation contract
- Related: [`decisions-v0.3-source-attestation-producer.md`](decisions-v0.3-source-attestation-producer.md)、
  [`decisions-v0.3-selfhost-source-attestation-report-projection.md`](decisions-v0.3-selfhost-source-attestation-report-projection.md)、
  `EC-M3-04`

## Context

Rust source adapter と selfhost `EmbeddedCli` は named-field `:review-attestation` を
`unverified` として report/manifest へ投影できる。一方、native source-file smoke は
review registry と stale/invalid 境界だけを確認しており、native producer が attestation を
落としても検知できなかった。

native stage0 は trust store、lifecycle、current subject/source identity を暗黙に補完しては
ならない。外部 verification input がない source attestation は、Rust/selfhost と同じ
`unverified` fact のまま保持し、validation は `unknown` (exit `2`) とする必要がある。

## Decision

- native source-file smoke に、既存 `:review` と named-field `:review-attestation` を同じ source
  declaration に置く fixtureを追加する。
- `validate --source --format json --emit-manifest` は次を要求する。
  - `review_verifications` が review ID の配列として一件だけ出る。
  - state は `unverified` である。
  - manifest の review record も `unverified` (`verification_state`) を保持する。
  - external trust/lifecycle input がないため exit は `2`、stderr は空である。
- `--format text` は同じ fixtureを
  `review-verification: review:checkout/reviewer-001=unverified` の固定行へ投影する。
- Linux stage0 wrapper は source smoke script 自体を転送するため、fixtureは smoke script 内で
  生成し、別の stale fixture copy 経路を増やさない。

## Evidence

- RED: `scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` に fixture/report/state
  marker を先に要求し、実装前に `VALIDATION_ATTESTATION_SOURCE` 欠落で失敗することを確認した。
- GREEN: fixture、JSON/text assertions、manifest state assertionsを source smokeへ追加し、同じ
  static/provenance harness が `Linux native stage0 source-file provenance tests: OK` で通過した。
- Rust syntax source suite: `cargo test -p lsharp-syntax --test review_attestation_source`
  （3 tests）。
- Rust source adapter suite: `cargo test -p lsharp-types --test review_attestation_source`
  （4 tests）。
- `bash -n`（変更した3 shell script）と `git diff --check` は通過した。

## Boundary

この ADR の GREEN は、source smoke contract と fake Lima/provenance harness の証拠である。
別セッションが Linux x86_64 hostgen を実行中だったため、同じ VM に競合する native stage0
replay は起動していない。従って current-source に一致する packaged stage0 の実 runtime、
Mac Apple Silicon / Linux x86_64 の二 target matrix、trust store/lifecycle による
`verified`/`stale`/`revoked` state、release provenance は未完了であり、`EC-M3-04` は
partial のまま維持する。

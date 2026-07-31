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
review registry と stale/invalid 境界、`review_verifications` だけを確認しており、
native producer が named fields、canonical bytes、span を落としても検知できなかった。

native stage0 は trust store、lifecycle、current subject/source identity を暗黙に補完しては
ならない。外部 verification input がない source attestation は、Rust/selfhost と同じ
`unverified` fact のまま保持し、validation は `unknown` (exit `2`) とする必要がある。

Rust parser、Rust-host selfhost E2E、Mac source-file smoke、Linux source-file smoke が
同じ入力を読むことも必要である。各経路に source を埋め込むと、named-field の順序や
改行による span が drift しても検知できない。

## Decision

- `tests/fixtures/validation/ec-m3-review-attestation-source.ls` を、既存 `:review` と
  named-field `:review-attestation` を同じ declaration に置く canonical fixture とする。
  Rust parser test、Rust-host selfhost E2E、Mac source-file smoke はこのファイルを直接読む。
- Linux stage0 wrapper は同じ fixtureを必須入力として検査し、VM work directoryへコピーしてから
  source smokeを起動する。Mac/Linux の runner が別の source を生成して drift する経路を作らない。
- `validate --source --format json --emit-manifest` は次を要求する。
  - `review_verifications` が review ID の配列として一件だけ出る。
  - state は `unverified` である。
  - manifest の review record も `unverified` (`verification_state`) を保持する。
  - external trust/lifecycle input がないため exit は `2`、stderr は空である。
- `--format text` は同じ fixtureを
  `review-verification: review:checkout/reviewer-001=unverified` の固定行へ投影する。
- JSON report の `review_attestations` は source record がある場合だけ出力し、次の field order を
  Rust wire と固定する: `review_id`, `subject_digest`, `source_commit`, `provenance_digest`,
  `provider`, `key_id`, `algorithm`, `signature`, `issued_at`, `expires_at`, `sequence`, `state`,
  `canonical_bytes`, `span`。`expires-at` 省略時は JSON `null` とし、canonical bytes は UTF-8
  length-prefixed field、span は directive の `start` / `end` を投影する。
- 通常 selfhost CLI と `EmbeddedCli` は共通の `Tools.Validation.Evidence` projection helper を
  使い、attestation のない既存 report shape と manifest shape を変えない。
- named-field attestation の `algorithm`、`signature`、`issued-at`、`expires-at` を壊した
  4 variant は、すべて stable な `source validation error:8`、exit `1`、stdout 空、
  no-report/no-manifest で fail-closed に拒否する。

## Evidence

- RED: `test_native_review_attestation_smoke_uses_shared_current_source_fixture` を先に追加し、
  native smoke が fixture path、copy、期限付き/期限なし report を持たない状態で失敗することを確認した。
- GREEN: canonical fixtureを追加し、Rust parser/Rust-host E2E/Mac smokeが同じファイルを読み、Linux
  wrapperが必須検査とVM copyを行うようにした。Linux fake Lima/provenance harness は
  `Linux native stage0 source-file provenance tests: OK` で通過した。
- RED: `scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` に fixture/report/state
  marker を先に要求し、実装前に `VALIDATION_ATTESTATION_SOURCE` 欠落で失敗することを確認した。
- GREEN: fixture、JSON/text assertions、manifest state assertionsを source smokeへ追加し、同じ
  static/provenance harness が `Linux native stage0 source-file provenance tests: OK` で通過した。
- RED/GREEN: Rust driver の `validate_source_review_attestation` test と native marker を先に
  追加し、report の `review_attestations` が named field order、期限付き/期限なしの nullable
  expiry、canonical bytes、span を持つことを固定した。Rust focused test、selfhost
  `EmbeddedCli` actual Wasm E2E、Linux source-file static/provenance harness が通過した。
- Native runner の入口である `App.Cli` に残っていた `run-validate-source` の閉じ括弧不足を
  最小修正し、actual selfhost CLI の `check --json` bundle gate も通過させた。これにより
  通常 CLI と EmbeddedCli が同じ Evidence helper を構文・bundle compile まで共有する。
- Negative GREEN: valid fixtureから4つの invalid attestation variantを生成し、共通 helperで
  code `8`、exit `1`、stderr diagnostic、stdout/manifestなしを要求する static/provenance
  harness が通過した。
- Rust syntax source suite: `cargo test -p lsharp-syntax --test review_attestation_source`
  （3 tests）。
- Rust source adapter suite: `cargo test -p lsharp-types --test review_attestation_source`
  （4 tests）。
- `bash -n`（変更した3 shell script）と `git diff --check` は通過した。

## Boundary

この ADR の GREEN は、Rust/selfhost report projection と source smoke contract、fake
Lima/provenance harness の evidence である。
別セッションが Linux x86_64 hostgen を実行中だったため、同じ VM に競合する native stage0
replay は起動していない。従って current-source に一致する packaged stage0 の実 runtime、
Mac Apple Silicon / Linux x86_64 の二 target matrix、trust store/lifecycle による
`verified`/`stale`/`revoked` state、release provenance は未完了であり、`EC-M3-04` は
partial のまま維持する。

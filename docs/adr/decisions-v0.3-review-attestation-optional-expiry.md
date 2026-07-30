# ADR: review attestation の `expires-at` 省略を canonical parity へ固定する

- Status: Accepted（EC-M3-04 の verified slice）
- Date: 2026-07-31
- Scope: source `:review-attestation`、Rust canonical model、selfhost producer、native source-file smoke

## Context

review attestation の `expires-at` は任意 field である。省略時に Rust が空 field として
canonical bytes へ含め、selfhost が別の既定値を補うと、同じ attestation が target によって
異なる署名対象になる。また、source producer が省略を malformed と誤認すると、既存の
`unverified` 後方互換境界も崩れる。

## Decision

- `expires-at` の省略は有効な入力として受理する。
- Rust `ReviewAttestation` は `None` を canonical bytes の zero-length field として encode する。
- selfhost `IntentSource` は省略 field を空文字の producer value として保持し、同じ zero-length
  field、`unverified` state、directive span を返す。
- native source-file smoke は、通常の期限付き fixtureに加えて `expires-at` 省略 fixtureを
  `validate --format json/text` で実行し、report/manifest の review state と exit boundary を
  同じ値に固定する。
- 署名検証、trust store、lifecycle、current-source artifact の実行証跡はこの ADR の範囲外であり、
  EC-M3-04/05 の残件として扱う。省略を `verified` に昇格させない。

## Evidence

- `crates/lsharp-types/tests/review_attestation_source.rs`
  - `expires_at() == None`
  - Rust canonical bytes と span の回帰
- `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
  - 同じ fixture の selfhost canonical bytes byte-for-byte 比較
  - `unverified` state と span の確認
- `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
  - JSON/text report、manifest、exit boundary の native source-file contract
- `scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  - static contract と fake Lima/provenance harness の pass

実行した focused gate:

```text
cargo test -p lsharp-types --test review_attestation_source -- --nocapture  # 5 passed
cargo test -p lsharp-wasm --test e2e canonical_bytes_match_rust_without_expiry -- --nocapture  # 1 passed
bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh  # passed
```

この evidence は Rust-host actual Wasm と smoke contract までであり、current source commit に
一致する Mac Apple Silicon / Linux x86_64 packaged stage0 の実 runtime parity を証明しない。

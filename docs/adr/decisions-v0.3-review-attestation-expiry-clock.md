# ADR: v0.3 review attestation の明示 expiry clock

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types` の attestation timestamp validation と deterministic verification clock
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-attestation-binding.md`](decisions-v0.3-review-attestation-binding.md)

## Context

`ReviewAttestation` は `issued_at` / `expires_at` を署名対象へ含めていたが、これまで値の形式と
現在時刻を検証していなかった。そのまま lifecycle と identity が一致すると、期限切れの review
や発行前の review が `verified` へ昇格できる。system clock や provider network を model 内で
取得すると、Rust oracle と native stage0 の結果が実行時刻で変わるため、検証 clock は caller の
明示 snapshot として渡す必要がある。

## Decision

- attestation の `issued_at` と `expires_at`（指定時）は、`YYYY-MM-DDTHH:MM:SSZ` の strict UTC
  形式、実在する Gregorian 日付、秒 `00..59` として constructor/wire 境界で検証する。offset、
  fractional seconds、leap second、存在しない日付は入力エラーとする。
- `expires_at` は `issued_at` より後でなければならない。setter でも同じ境界を維持する。
- `ReviewAttestation::verify_against_at` は caller が渡す `now` を同じ形式で解釈し、
  `issued_at <= now < expires_at`（expiry なしは上限なし）を満たさない場合に `stale` を返す。
  `now == expires_at` は期限切れとして扱う。
- `verify_with_lifecycle` と clock なしの `verify_against` は、期限付き attestation を
  `unverified` として保持し、時刻を知らないまま `verified` に昇格させない。
- system clock、環境変数、network、暗黙の default timestamp は参照しない。署名検証、clock、
  current subject/source/provenance、lifecycle の順に明示入力だけを組み合わせる。
- 既存の `verify_against` は API 互換のため残すが、期限付き attestation を clock なしで
  `verified` にしない。期限なしの既存 record だけは従来どおり lifecycle/identity gate を通せる。

## Evidence

- RED: `crates/lsharp-types/tests/review_signature.rs` に未実装の
  `verify_against_at`、strict timestamp、invalid window、clock なし期限付き API の fixture を
  先に追加し、型/API が未接続で失敗することを確認した。
- GREEN: 同テスト 12件が passし、発行前・期限ちょうど・期限後を `stale`、期限内を `verified`、
  malformed timestamp を constructor/verification error、clock なし期限付き record を
  `unverified` として固定した。
- Regression: `cargo test -p lsharp-types --lib`（221 passed）、review attestation/wire/
  lifecycle/trust-store focused tests、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、
  changed Rust files の `rustfmt --check` を通過した。wire schema の timestamp pattern は
  `python3 -m json.tool docs/schemas/review-provenance-v1.schema.json` で検証した。

## Boundary

これは canonical Rust model の expiry/clock verified partial slice である。manifest/report への
verification state projection、CLI/MCP の clock wiring、source/selfhost/native producer parity、
Mac Apple Silicon/Linux x86_64 artifact/runtime evidence は未完了であり、EC-M3-03〜05 に残す。

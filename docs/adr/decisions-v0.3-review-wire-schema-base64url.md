# ADR: v0.3 review wire canonical base64url schema boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `docs/schemas/review-provenance-v1.schema.json` の attestation signature / trust-store public key
- Related: `EC-M3-01`、[`decisions-v0.3-review-wire.md`](decisions-v0.3-review-wire.md)、
  [`decisions-v0.3-review-signature-verification.md`](decisions-v0.3-review-signature-verification.md)

## Context

Rust の review wire parser は padding なし base64url の alphabet、長さ、末尾の未使用 bit を検証する。
一方、JSON Schema は alphabet と `minLength`（trust key は 43 文字）だけだったため、schema consumer が
`AB` のような末尾 bit 不正の signature や、43 文字でも canonical でない public key を受理できた。
これは schema validation と Rust parser の入力集合が異なり、署名検証前の provider snapshot を target 間で
別の意味へ解釈する余地を残す。

## Decision

- `canonical_base64url` を schema の共通定義として追加する。
- 4 文字単位、余り 2 / 3 の padding なし base64url だけを許可し、余り 2 / 3 の末尾未使用 bit をゼロに
  固定する。
- attestation `signature` は共通定義を参照する。
- trust-store `public_key` は共通定義に加えて 43 文字へ固定する。これは Ed25519 32 bytes の canonical
  base64url 長と一致する。
- 暗号学的署名検証、鍵の実在性、provider 取得、暦日の実在性は既存 Rust runtime boundary の責務として
  維持する。

## Evidence

- RED: schema validator が `signature = "AB"`、`signature = "A"`、末尾 bit 不正の 43 文字 public key を
  受理した。Rust `parse_review_wire` は同じ入力を `InvalidSignatureEncoding` / `InvalidPublicKeyEncoding`
  として拒否した。
- GREEN: `crates/lsharp-driver/tests/review_provenance_schema.rs` に Draft 2020-12 validator と Rust
  parser の同一 fixture を追加し、canonical `AAECAw` / 32-byte key を受理しつつ、不正 tail bit と
  `length mod 4 == 1` を schema/parser の両方で拒否した。
- `cargo test -p lsharp-driver --test review_provenance_schema -- --nocapture`（2 passed）。
- schema の meta-schema validation、`git diff --check`、docs audit は最終 gate で確認する。

## Boundary

これは Rust-host の review wire schema/parser lexical parity に限定した verified partial slice である。
selfhost/native producer、provider/authentication、署名の暗号学的検証、Mac Apple Silicon / Linux x86_64
runtime、EC-M3 aggregate completion は残件であり、TODO の `[~]` を維持する。

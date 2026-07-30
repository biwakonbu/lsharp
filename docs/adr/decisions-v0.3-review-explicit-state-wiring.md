# ADR: v0.3 review explicit input to verification-state wiring

- Status: Accepted (verified partial slice)
- Date: 2026-07-30
- Scope: Rust CLI/MCP の review attestation input、report、manifest state の共通 projection
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-explicit-input-boundary.md`](decisions-v0.3-review-explicit-input-boundary.md)、
  [`decisions-v0.3-review-verification-report-projection.md`](decisions-v0.3-review-verification-report-projection.md)、
  [`decisions-v0.3-review-verification-manifest-projection.md`](decisions-v0.3-review-verification-manifest-projection.md)

## Context

review wire parser、trust/lifecycle model、report fact、manifest の optional state はそれぞれ
利用可能だったが、CLI/MCP は explicit input を preflight で読むだけで attestation を保持せず、
report と manifest に同じ verification state を投影できなかった。入力を捨てて implicit な
`verified` を作らず、signature error と不確実な state を既存の no-report/unknown policy に接続する
必要がある。

## Decision

- `--trust-store` / `trust_store` の version 1 wire に含まれる attestation を `ReviewInputs` が保持し、
  CLI と MCP は同じ `ReviewVerificationFact` を生成する。
- trust store と lifecycle snapshot が両方明示されている場合は
  `ReviewAttestation::verify_with_lifecycle` を使う。trust store だけの場合も署名を検査し、署名が
  valid でも lifecycle がないため `unverified` に留める。trust key 不明や input 欠落も
  `unverified` とし、provider/network/environment/system clock から補完しない。
- known trust key に対する署名破損は verification error として fail-closed にし、CLI は report と
  manifest を出力せず non-zero、MCP は error result として返す。
- fact は review ID の canonical 順へ正規化し、duplicate を拒否する。registry に同じ ID がある
  場合だけ `reviews[].verification_state` を更新し、registry 外の external review ID は report fact
  として保持する。facts が空の既存入力は `IntentGraph::validate()` の後方互換出力を使う。
- report と emitted/inline manifest はこの一つの fact 列から生成し、JSON/text/MCP の順序と state
  名を分岐させない。

## Evidence

- RED: `review_input_cli` に explicit attestation の report/manifest projection を追加し、入力を
  retained しない実装で `review_verifications` が欠けることを確認した。
- GREEN: `cargo test -p lsharp-driver --test review_input_cli`（7 passed）で
  `unverified` projection と known-key signature error の no-report/no-manifest を固定した。
- MCP: `mcp_server::tests::test_validate_tool_projects_explicit_attestation_state_to_report_and_manifest`
  を含む validate focused suite（16 passed）で inline manifest と report の同一 state を確認した。
- Regression: `cargo test -p lsharp-types`（221 unit と全 integration）、
  `cargo clippy -p lsharp-types --all-targets -- -D warnings`、変更 Rust files の rustfmt check を通過した。
  driver 全182 tests は 174 passed、8件は既存の temp/git/stdlib fixture 環境依存で失敗し、変更箇所の
  focused suite は通過した。

## Boundary

これは EC-M3-03 の explicit attestation/state wiring verified partial slice である。current
subject/source/provenance digest binding と expiry の明示 clock は
[`decisions-v0.3-review-explicit-context.md`](decisions-v0.3-review-explicit-context.md) で CLI の
verified partial slice として追加した。MCP の valid signature end-to-end fixture、source/selfhost/
native parity、Mac Apple Silicon/Linux x86_64 artifact/runtime evidence は未完了であり、EC-M3-03〜05
に残す。

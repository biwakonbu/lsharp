# ADR: v0.3 selfhost source attestation report projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: selfhost `EmbeddedCli` source validation report/manifest projection
- Related: [`decisions-v0.3-source-attestation-report-projection.md`](decisions-v0.3-source-attestation-report-projection.md)、
  [`decisions-v0.3-selfhost-source-attestation-graph.md`](decisions-v0.3-selfhost-source-attestation-graph.md)、
  `EC-M3-04`

## Context

`Tools.Validation.IntentSource` は named-field `:review-attestation` を `unverified` の
source-owned record として保持し、`Tools.Validation.Evidence` は graph と manifest の
`reviews[].verification_state` へ接続していた。一方、`App.EmbeddedCli` の source validation
report は attestation record を読まず、Rust CLI と同じ source fixture でも
`review_verifications` を欠落させていた。

selfhost source は trust store、lifecycle snapshot、current subject/source identity、clock を
持たない。そのため source record を `verified` と解釈せず、明示的な外部検証入力が未接続の
selfhost では常に `unverified` を report と manifest へ投影する必要がある。

## Decision

- `run-validate-source` は graph が保持する attestation を review ID の bytewise deterministic
  order へ正規化し、source-owned `review_verifications` fact として report に追加する。
- JSON report は attestation が存在する場合だけ、Rust wire と同じ
  `[{"review_id":"...","state":"unverified"}]` の `review_verifications` field を出力する。
  attestation がない既存 source の JSON shape は変更しない。
- text report は Rust projection と同じ
  `review-verification: <review-id>=<state>` の行を固定順で追加する。
- manifest は既存の graph serializer を使い、report と同じ source record の state
  (`unverified`) を `reviews[].verification_state` に投影する。trust/lifecycle/network から値を
  補完しない。
- review ID の比較は selfhost の byte accessor で行い、source 宣言順に依存した nondeterminism を
  report へ持ち込まない。duplicate/invalid attestation の fail-closed validation は既存
  `IntentSource` boundary に委ねる。

## Evidence

- RED: `test_e2e_selfhost_embedded_cli_validate_source_projects_review_attestation` で、二つの
  source attestation を含む `validate --source --format json` を実行すると、実装前の report は
  `review_verifications == null` だった。
- GREEN: 同テストで review 宣言を `reviewer-002` → `reviewer-001` の順に置き、JSON/text の
  verification facts が ID 昇順で二件返ること、manifest の二 review がともに `unverified` で
  あること、exit code `2` であることを actual EmbeddedCli Wasm で確認した。
- Regression: `test_e2e_selfhost_embedded_cli_main_with_args_validate_source_text_pass` は
  attestation のない既存 text report shape を維持することを確認した。

## Boundary

これは macOS 上の Rust host が生成した selfhost `EmbeddedCli` Wasm を実行する verified slice
である。native stage0 の current-source/package provenance、Linux x86_64 / Mac Apple Silicon
native runtime parity、selfhost MCP、trust store/lifecycle による `verified`/`stale`/`revoked`
の外部 verification projection、artifact/release gate は未完了であり、`TODO.md` の
`EC-M3-04` / `EC-M3-05` は `[~]` のまま維持する。

# ADR: v0.3 review trust/lifecycle の explicit input boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp validate` と `lsharp_validate` の trust store/lifecycle input path と wire parse
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-trust-store.md`](decisions-v0.3-review-trust-store.md)、
  [`review-provenance-v1.schema.json`](../schemas/review-provenance-v1.schema.json)

## Context

canonical model と Ed25519 verifier があっても、CLI/MCP が環境変数、current manifest、network
fetch から trust root を暗黙に補うと、同じ変更が自分自身を承認できる。入力 file の path と
JSON wire を先に fail-closed に固定し、後続の verification/report projection が同じ境界を
再利用できるようにする。

## Decision

- `lsharp validate` は `--trust-store FILE` と `--review-lifecycle FILE` を受け付ける。
- `lsharp_validate` MCP input は同じ `trust_store` / `review_lifecycle` field 名を使う。
- 両方の file は version 1 review provenance wire を読み、trust store input は `trust_store`
  field の存在を要求し、lifecycle input は `lifecycle` snapshot を読み取る。
- path は project-relative の通常 file に限定する。絶対 path、`..`、project root 外へ出る symlink、
  directory、missing file は拒否する。
- JSON の unknown field と duplicate field は `parse_review_wire` の schema boundary で拒否する。
- 明示 input がない場合、trust store/lifecycle を環境変数、current manifest、network、implicit
  default から補わない。
- path または wire の error は report/manifest を生成せず non-zero とする。読み込んだ snapshot は
  この slice では preflight として検証するだけで、attestation の graph projection・expiry state・
  lifecycle state と report の `verified/stale/revoked` 集計は後続 task の責務とする。

## Evidence

- RED: `crates/lsharp-driver/tests/review_input_cli.rs` で未接続の `--trust-store`/
  `--review-lifecycle` が拒否されることを確認した。
- GREEN: 同テスト 5件で project root 外 path、unknown/duplicate field/key の no-report boundary、明示した
  project-relative trust/lifecycle wire の受理と既存 `unknown` report を固定した。
- MCP schema/behavior: `lsharp_validate` の input schema に両 field を追加し、project root 外 path
  の拒否を `mcp_server::tests` で固定した（focused suite 30 passed）。
- Formatting/contract: changed/new Rust files の `rustfmt --check` と `git diff --check` を通過した。

## Boundary

これは EC-M3-03 の explicit input/preflight verified partial slice である。attestation と
manifest review record の subject/source digest binding、署名検証結果の JSON/text/MCP 投影、
expiry clock の report 接続、lifecycle/revocation report、selfhost/native producer parity、Mac
Apple Silicon/Linux x86_64 artifact/runtime evidence は未完了であり、後続 EC-M3-03〜05 に残す。

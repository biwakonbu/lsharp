# ADR: v0.2 review provenance registry and redaction boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `lsharp-types` version 1 manifest model/input/output と selfhost source producer
- Related: `EC-M2-02` / `EC-M2-03`、`docs/adr/decisions-v0.2-source-review-invalidation-edges.md`

## Context

`evaluates` / `invalidates` の review endpoint は、source から opaque な `ReviewId` として
保持する必要がある。一方、reviewer の author、本文、連絡先を graph manifest に混ぜると、
validation artifact の共有範囲を越えて個人情報や外部サービスの内部情報を漏らし得る。既存の
version 1 manifest には review endpoint の provenance registry がなく、edge の参照先が外部
identity なのか、認証済み record なのかを区別できなかった。

## Decision

- version 1 manifest に optional な `reviews` registry を追加する。record は stable
  `namespace` / `key`、opaque `provenance_digest`、`visibility` (`public` / `redacted`) だけを持つ。
- `IntentGraph` は `ReviewRecord` を重複なく登録し、provenance digest の空値を fail-closed に拒否する。
- manifest に review registry が明示された場合、`evaluates.review` と
  `invalidates.subject(kind=review)` は registry に存在する ID だけを参照できる。
  registry がない source graph では、既存の external `ReviewId` boundary を維持する。
- author、email、review 本文、URL、token、署名 material は schema の record に追加しない。
  `redacted` は個人情報を公開 artifact へ投影しない policy marker であり、digest の暗号学的検証や
  provider API の認証をこの slice で宣言しない。
- registry は optional field として出力し、空 registry の既存 manifest bytes は変更しない。

## Evidence

- RED: `crates/lsharp-types/tests/review_provenance.rs` は `ReviewRecord`、registry closure、
  redaction schema が未実装のため compile error になった。
- GREEN: `cargo test -p lsharp-types --test review_provenance -- --nocapture`（4 passed）。
- Regression: `cargo test -p lsharp-types --test validation_input -- --nocapture`（16 passed）、
  `validation_output`（5 passed）、`validation_schema`（2 passed）、`intent_validation`（6 passed）。
- CLI boundary: `cargo test -p lsharp-driver --test validate_review_registry -- --nocapture`
  （2 passed）で、`lsharp validate --format json --emit-manifest` の redacted registry
  roundtrip（private field を投影しない）と、未登録 review edge の non-zero/no-output を確認した。
- MCP boundary: `cargo test -p lsharp-driver mcp_server::tests -- --nocapture`（40 passed）で、
  `lsharp_validate` の manifest input/output schema が `reviews` の必須4 fieldと
  `public` / `redacted` enumを宣言し、`include_manifest` の inline artifact が同じ registry を
  private field なしで roundtrip することを確認した。
- Source producer boundary: `cargo test -p lsharp-types --test validation_source -- --nocapture`
  （31 passed）で、source `:review` の stable ID、opaque provenance digest、visibility を
  `ReviewRecord` へ投影し、未知 visibility・空 digest・不正 ID・duplicate ID を directive span
  付きで fail-closed に拒否することを確認した。`cargo test -p lsharp-driver --test
  validate_source_review_edges -- --nocapture`（2 passed）は `validate --source --emit-manifest`
  の review registry output と `evaluates` edge の同一 manifest projection を確認する。
- Selfhost producer boundary: `cargo test -p lsharp-wasm --test e2e selfhost_source_adapter_ -- --nocapture`
  （17 passed）で、selfhost parser の `:review` triple payload（kind 16）を node/edge と分離した
  opaque registry へ投影し、nested declaration の走査、duplicate ID、空白だけの provenance digest、
  未知 visibility の fail-closed と directive span を確認した。既存の evidence registry（17 passed）と
  parser metadata forms（28 passed）も回帰していない。これは Rust-host actual Wasm の verified slice
  であり、native stage0 の producer/runtime parity はまだ実行していない。

## Boundary

これは Rust canonical manifest、公開 CLI、Rust MCP の schema／inline artifact 入出力と、selfhost
source `:review` producer の Rust-host actual Wasm における opaque review registry／privacy field
boundary の verified slice である。native stage0 parity、provider/署名による provenance authentication、
暗号学的 digest format、review lifecycle/stale propagation、selfhost/native MCP、Mac Apple Silicon / Linux x86_64
artifact/runtime parity、EC-M2-02/03 aggregate completion は未完了である。
未接続境界は TODO の `[~]` として維持する。

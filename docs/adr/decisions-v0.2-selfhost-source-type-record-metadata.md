# ADR: v0.2 selfhost TypeDef / RecordDef source metadata projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `selfhost/src/Syntax/Parser.ls`, `selfhost/src/Tools/Validation/IntentSource.ls`,
  `crates/lsharp-wasm/tests/e2e/selfhost_intent_type_source_adapter.rs`
- Related: `EC-M2-01`,
  `docs/adr/decisions-v0.2-source-type-definition-metadata.md`,
  `docs/adr/decisions-v0.2-source-record-definition-metadata.md`,
  `docs/adr/decisions-v0.2-selfhost-source-adapter.md`

## Context

Rust の source adapter は ADT と record 定義の後ろにある `:intent` / `:claim` などを
typed graph へ投影できる一方、selfhost `Parser.ls` は type/record の variant または field
を読み終えた直後に metadata を保持せず、`IntentSource` も `ast-typedef` /
`ast-recorddef` を巡回していなかった。そのため同じ source を selfhost の source graph
へ渡すと、宣言が成功しても node/edge が空になる。

## Decision

- ADT variant の終端を外側の `)` に加えて metadata の `:` でも止め、既存の ordered
  metadata parser を再利用する。
- record field の終端後にも同じ metadata parser を呼び、既存 AST の payload 形状を保った
  まま metadata vector を末尾へ追加する。
- `IntentSource` は type/record payload の末尾 metadata を読み、defn と同じ source order、
  stable ID、span、typed node/edge validation を通す。
- `ast-typedef` / `ast-recorddef` の node collection と edge collection を同じ declaration
  traversal に追加する。ID を型名、field、span、宣言順から推測せず、source wire ID を正本とする。

## Evidence

- RED: `cargo test -p lsharp-wasm --test e2e selfhost_intent_type_source_adapter -- --nocapture`
  は実装前に両 fixture が `status=1`, `nodes=0` となり失敗した。
- GREEN: 同コマンドは type/record の node と typed edge を含む 2 tests passed。
- 回帰: `cargo test -p lsharp-wasm --test e2e selfhost_intent_source_adapter -- --nocapture`
  は既存 12 tests passed。
- parser 回帰: `cargo test -p lsharp-wasm --test e2e selfhost_parser_record -- --nocapture`
  は既存 record/trait/record-update/field parity の 4 tests passed。
- Rust oracle の TypeDef/RecordDef source adapter testsは既存の
  `crates/lsharp-types/tests/validation_source/nodes.rs` にあり、同じ stable ID と本文の
  contract を参照している。

## Boundary

これは Rust-host actual Wasm の selfhost parser → IntentSource graph projection を閉じた
verified slice である。native stage0、Linux x86_64、release artifact/runtime、公開
`validate`/MCP/EmbeddedCli、evidence registry の selfhost parity、EC-M2-01 aggregate の完了は
意味しない。未検証の境界は TODO の `[~]` として維持する。

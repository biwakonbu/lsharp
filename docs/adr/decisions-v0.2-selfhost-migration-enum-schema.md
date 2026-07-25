# ADR: selfhost migration row の enum/string schema を fail-closed にする

- Status: Accepted (verified partial)
- Date: 2026-07-25
- Scope: EC-M1-03 selfhost legacy migration row projection
- Related: `docs/development/operations/rust-boundary-reduction.md`

## Context

Selfhost の legacy migration row は、Rust 側の typed `LegacyDiagnosticCode`、
`LegacyMigrationDisposition`、`LegacySelectedSemantics` を Int wire value として保持している。
従来の文字列 projection は未知の code を `LS<number>`、未知の disposition を `manual-review`、
未知の selected semantics を `legacy-example-truthiness` として出力していた。この暗黙の丸めは、
破損した row や将来追加された enum を有効な migration 診断として扱うため、JSON/text consumer が
schema drift を検出できない。

## Decision

`Types.MetadataMigration` に次の fail-closed boundary を持たせる。

- `legacy-migration-row-schema-valid?` は row の最小 7 fields と wire enum を検査し、valid を `1`、
  invalid を `0` で返す。
- diagnostic code は `2001`（example）、`2002`（invariant）、`2003`（ambiguous）だけを受理する。
- disposition は `1`（docs-only example）、`2`（assertion）、`3`（property/postcondition）、
  `4`（manual review）だけを受理する。
- selected semantics は `1`（legacy example truthiness）または `2`（legacy invariant deterministic
  smoke）だけを受理し、`LS2002` には `2`、その他の code には `1` を要求する。
- `legacy-code-text`、`legacy-disposition-text`、`legacy-selected-semantics-text` は unknown value
  に対して空文字を返す。row detail text/JSON/summary text は validator が invalid を返した row を
  projection しない。

既存の valid row の index と出力文字列は変更しない。Rust の typed enum と migration classifier は
oracle/bootstrap として保持し、selfhost 側の schema consumer を独立した verified slice とする。

## Evidence

- `selfhost/src/Types/MetadataMigration.ls`
- `crates/lsharp-wasm/tests/e2e/selfhost_migration_schema.rs`
- `CARGO_PROFILE_DEV_DEBUG=0 cargo test -p lsharp-wasm --test e2e selfhost_migration_schema -- --nocapture`
  （1 passed）

RED は schema validator 未定義で unknown enum fixture が失敗し、GREEN は valid row `1`、unknown
code/disposition/selected semantics 各 `0` と invalid projection の空文字を確認した。

## Consequences and residual work

- selfhost の row consumer は未知 enum を成功扱いせず、将来の enum 追加や破損した wire row を
  fail-closed に扱える。
- この slice は parser が生成する全 metadata、CLI `check --json` の structured diagnostic/exit code、
  full legacy evaluator、module/private owner parity、native stage0 の Mac/Linux artifact/runtime を
  含まない。
- それらの evidence が揃うまで EC-M1-03 は TODO の `[~]` を維持し、この ADR を完了項目の移動先とは
  扱わない。

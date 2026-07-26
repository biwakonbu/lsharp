# ADR: validation input manifest wire schema の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_input.rs`, `crates/lsharp-types/src/validation_input/manifest.rs`, `crates/lsharp-types/tests/validation_input.rs`

## Context

M2-03 の JSON manifest 入力境界は、serde の wire schema、kebab-case enum、typed graph
への変換、referential closure を一つの `validation_input.rs` に保持していた。入力 shape
だけを変更なく独立させることで、今後の manifest versioning と source adapter の変更を
分離し、親の parse/closure 実装を小さく保つ必要がある。

## Decision

`Manifest`、各 `*Input` wire type、`SUPPORTED_SCHEMA_VERSION`、`SubjectKindInput` の
文字列表現、evidence method/outcome/independence の canonical conversion を
`validation_input/manifest.rs` に移動する。親 module は `mod manifest` と `pub(super)` の
内部 import のみを持ち、公開 API は `parse_intent_graph_json` と
`ValidationInputError` のまま維持する。serde の `deny_unknown_fields`、`kebab-case`、
`#[serde(default)]`、tagged edge relation の shape は変更しない。

## Evidence

- RED: `mod manifest` と内部 import を先に追加し、child file がない状態で focused test を
  実行して `E0583`（`validation_input/manifest.rs` 不在）を確認した。
- GREEN: wire schema を移動後、`parse_manifest_wire_schema_uses_kebab_case_node_kind` は
  `open-question` を受理し、`open_question` を JSON error として拒否した。
- `cargo test -p lsharp-types --tests`: 221 unit + 119 integration = 340 tests pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、workspace
  `cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check`、
  docs audit は pass。

## Consequences

manifest wire schema の ownership が `validation_input/manifest.rs` に集約され、親は 535
行から 321 行へ縮小した。既存の JSON shape、stable ID、edge/evidence conversion、
referential closure semantics と公開 parse API は維持される。selfhost/native parity、
manifest/runtime target gate、EC-M2 aggregate、I-01 / I-08 aggregate は未完了であり、
`LEGACY-MAINT-01` は `[~]` のまま継続する。

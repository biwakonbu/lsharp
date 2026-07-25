# ADR: v0.2 intent graph manifest の version 1 input parser

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/validation_input.rs`
- Related: `EC-M2-03`, `v0.2-validation-model.md`, `intent-graph.schema.json`

## Context

M2 の graph model と report projection は固定済みだが、Rust/selfhost producer が同じ
typed graph を受け取る versioned input boundary がなかった。CLI や source syntax adapter
まで同時に接続すると、manifest wire の診断と command exit code の責務が混ざる。

## Decision

- `parse_intent_graph_json` は `schema_version: 1` の JSON manifest だけを受け付け、
  `serde(deny_unknown_fields)` で未知 field を拒否する。
- node は typed `IntentNode` として構築し、stable ID、text、span、duplicate ID を検証する。
- evidence は method/outcome/execution/sampling/provenance/independence を typed value に
  変換し、既存の required-field validator を通してから graph に登録する。
- typed edge は relation ごとの ID kind を検証し、node/evidence の referential closure を
  fail-closed に検査する。graph-only の review/change/contract identity は暗黙に AST node へ
 変換しない。
- missing span / optional sampling arrays は deterministic default (`0..0`, 空 vector/map) を
  許し、canonical emitter は常に明示 shape を出力する。source syntax adapter、CLI/exit code、
  selfhost/native parity は後続契約として残す。

## Evidence

- RED: `validation_input` の 8 契約テストを parser 未実装状態で unresolved import として確認。
- GREEN: complete/unknown/empty graph、round-trip、unknown field、unsupported version、
  duplicate node、reversed span、empty required field、invalid subject の 10 tests が pass。
- `cargo test -p lsharp-types`、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、
  targeted Rustfmt、`git diff --check`、`bash scripts/audit_docs.sh` を gate とする。

## Consequences

version 1 manifest の input/output を Rust model で lossless に往復でき、後続 CLI と
selfhost producer は同じ診断境界を再利用できる。source syntax から manifest を作る
adapter、public `validate` command、exit code、両対応 target の native evidence は未完了で、
M2 完了や Rust-free 完了の証拠には拡大解釈しない。

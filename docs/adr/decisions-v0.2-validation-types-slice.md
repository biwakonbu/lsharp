# ADR: v0.2 intent validation の types-only 契約

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/{intent,evidence,validation,validation_output}.rs`
- Related: `EC-M2-01`, `EC-M2-02`, `EC-M2-03`, `v0.2-milestone-02.md`,
  `v0.2-validation-model.md`

## Context

M2 の intent/evidence graph は、source parser、CLI、selfhost/native parity を一度に
導入すると、wire identity、graph fact、assurance policy の failure boundary が混ざる。
一方で、Rust と後続 producer が共有する stable ID、typed node、evidence graph、report
projection の契約は先に固定できる。

## Decision

- `StableId::parse` と typed ID parser は `kind:namespace/key` を fail-closed に検証し、
  kind mismatch と graph-only kind を暗黙変換しない。
- `IntentGraph` は node/evidence の登録順を保持し、duplicate node、graph-owned node endpoint、
  required evidence field、missing evidence reference を検査する。direct `add_edge` でも
  source/manifest adapter と同じ fail-closed closure を適用する詳細は
  `decisions-v0.2-intent-edge-closure.md` に固定する。
- `IntentGraph::validate()` は `pass` / `fail` / `unknown` と trace gap、open question、
  independent review、contradiction の fact を返す。欠落を pass や `verified` shortcut に
  変換しない。
- validation report は strict JSON と deterministic text、graph manifest は
  `intent-graph.schema.json` に従う deterministic JSON として別々に投影する。
- この slice は `lsharp-types` の pure model/output に限定し、source syntax/manifest
  parser、CLI/config/main、exit code、selfhost/native parity、production policy は後続
  RED とする。

## Evidence

- Focused tests: intent AST 4、stable ID parse 3、typed node wire 3、evidence graph 5、
  required fields 4、validation model 4、validation JSON 1、validation text 1、
  manifest output 2 がすべて pass。
- `cargo test -p lsharp-types`、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、
  targeted Rustfmt、`git diff --check`、`bash scripts/audit_docs.sh` を gate とする。

## Consequences

Rust/selfhost producer は同じ stable ID、node kind、manifest/report wire shape を参照できる。
ただし source から graph を作る入力 adapter、`validate` の公開 command/exit code、native
artifact/runtime、両対応 target の parity は未完了であり、M2 完了や Rust-free 完了の証拠には
拡大解釈しない。

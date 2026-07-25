# ADR: v0.2 intent validation CLI の Rust 接続

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/main.rs`, `crates/lsharp-driver/tests/validate_cli.rs`
- Related: `EC-M2-03`, `v0.2-milestone-02.md`, `v0.2-validation-model.md`,
  `intent-validation.md`

## Context

M2 の validation model と version 1 JSON manifest parser は library 境界まで固定済み
だったが、公開 command が未接続だった。implementation conformance の `lsharp test` と
intent/evidence graph の `validate` を同じ終了値や `verified` shortcut で扱うと、実装が
通ったことと intent の追跡可能性を混同する。

## Decision

- `lsharp validate <manifest>` を Rust driver の公開 command として追加する。
- `--format text|json` は同じ `ValidationReport` facts を投影し、既定値は `text` とする。
- `pass=0`、`fail=1`、`unknown=2` を返し、`lsharp test` の conformance 結果とは分離する。
- manifest の読込・JSON parse・schema validation の失敗は report status に変換せず、miette
  診断を stderr へ出す非ゼロ入力エラーとする。
- report に top-level `verified` を追加せず、欠落を pass と解釈しない。source syntax adapter、
  selfhost/native parity、EmbeddedCli/MCP wiring、supported 2 targets の artifact/runtime
  evidence は後続契約として残す。

## Evidence

- RED: `validate` command 未実装時に `validate_cli` の 5 tests が clap の unknown command で
  失敗することを確認。
- GREEN: unknown (`2`)、invalid manifest (diagnostic)、complete pass (`0`)、contradiction
  fail (`1`)、`--help` の command listing を `crates/lsharp-driver/tests/validate_cli.rs`
  で固定。
- `cargo test -p lsharp-driver --test validate_cli`、driver 全体 test、clippy、workspace
  check、targeted Rustfmt、`git diff --check`、`bash scripts/audit_docs.sh` をこの commit の
  gate とする。

## Consequences

Rust の manifest → graph → validation → report 経路を利用者と tooling が再現でき、status
ごとの CI/review 分岐も安定する。一方、この CLI は JSON manifest input の verified partial
slice であり、L# source syntax や selfhost/native command、EmbeddedCli/MCP、Mac Apple
Silicon / Linux x86_64 の artifact/runtime parity を完了扱いにはしない。

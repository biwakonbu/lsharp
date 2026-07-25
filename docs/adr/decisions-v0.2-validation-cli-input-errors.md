# ADR: v0.2 `validate` CLI の manifest 入力エラー境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/tests/validate_cli.rs` の JSON manifest input mode
- Related: `EC-M2-03`, `docs/adr/decisions-v0.2-validation-input-parser.md`,
  `docs/adr/decisions-v0.2-validation-input-required-fields.md`

## Context

`lsharp validate <manifest> --format json` は、入力を parse できたときだけ validation
report を stdout へ出す必要がある。manifest の top-level required field が欠落した場合に
unknown report などへ変換すると、入力破損を検証結果と取り違え、exit code と stderr の
責務も曖昧になる。

## Decision

- version 1 manifest の required field 欠落は入力エラーとして扱い、validation report を
  stdout へ出力しない。
- CLI は非ゼロ exit code と入力エラーの stderr を返し、`--format json` でも入力エラーを
  report JSON に包まない。
- parser の required-field decode と CLI の report suppression を別々の failure boundary
  として回帰テストで固定する。

## Evidence

- `validate_rejects_manifest_missing_required_field_without_report_stdout` は `edges` を欠落
  させた manifest を `validate --format json` へ渡し、非ゼロ exit、空 stdout、`missing field`
  を含む stderr を確認する。
- 実行: `cargo test -p lsharp-driver --test validate_cli validate_rejects_manifest_missing_required_field_without_report_stdout -- --nocapture`

## Boundary

これは Rust CLI の manifest decode/error-output boundary に限定した verified slice である。
source input、selfhost/native stage0、EmbeddedCli/MCP、Mac Apple Silicon / Linux x86_64
artifact/runtime parity、EC-M2-03 aggregate の完了は意味しない。

# ADR: v0.2 selfhost stale validation の直接 parser import

- Status: Accepted (verified slice)
- Date: 2026-07-29
- Scope: `selfhost/src/Tools/Validation/Stale.ls`
- Related: EC-M2-03, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`

## Context

`Stale.ls` は rooted vector helper `vector-push-single-rooted-v3` を呼び出していたが、helper の定義元で
ある `Syntax.Parser` を直接 import していなかった。`IntentSource` 経由の間接 import では L# の module
可視性を満たさず、default EmbeddedCli の build script が `undefined: vector-push-single-rooted-v3` で停止し、
Rust driver の validate CLI parity testまで到達できなかった。

## Decision

`Stale.ls` に `(import Syntax.Parser)` を明示する。helper の実装や stale metrics の意味論は変更せず、
owner module を直接参照する import boundary だけを閉じる。

## Evidence

- RED: `test_e2e_selfhost_stale_validation_module_compiles` に直接 import assertion を追加し、修正前は
  `Stale.ls は rooted vector helper の owner である Syntax.Parser を直接 import するべき` で失敗した。
- GREEN: 同テストは `1 passed`。空 graph の stale review/evidence metrics `0/0` も actual selfhost Wasm
  runtime で確認した。
- `cargo test -q -p lsharp-driver --test validate_cli -- --nocapture` は 32 tests、
  `validate_review_registry` は 2 tests、`manifest_input_cli` は 8 tests が全て passした。これにより
  default EmbeddedCli build が blocker を越え、manifest/validate CLI oracleへ到達できることを確認した。
- 変更した2 Rust test file の `rustfmt --edition 2021 --check` と `git diff --check` を通過した。
  `cargo fmt --all -- --check` は origin/main に既存の未整形ファイルが多数あるため pass 扱いにしていない。

## Boundary / follow-up

これは module import visibility の verified sliceであり、native stage0 の current-source/runtime、MCP、
Mac Apple Silicon と Linux x86_64 の実行 parity、EC-M2-03 aggregate の完了を意味しない。Rust は引き続き
bootstrap/oracle 境界として保持する。

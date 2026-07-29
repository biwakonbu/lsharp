# ADR: v0.2 native validation の manifest roundtrip

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: EC-M2-03, `crates/lsharp-driver/tests/validate_cli.rs`

## Context

native source-file smoke は `validate --source --emit-manifest` の canonical bytes と source report を
検証していたが、生成した version 1 manifest をもう一度 `validate <manifest> --format json` へ渡した
report/exit parity を固定していなかった。Rust CLI には source と emitted manifest の同一 report/exit を
比較するテストがあるため、native 側にも同じ observable contract が必要である。

## Decision

canonical source fixtureから emit した `VALIDATION_MANIFEST` を positional manifest input として再検証する。
source input の JSON report と manifest input の JSON report は byte-for-byte 一致し、両方とも判定
`unknown` の exit `2`、stderr 空でなければならない。parse/graph/write の diagnostic-only failure は
引き続き report を返す判定 failure と分離する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は roundtrip marker が
  absent のため失敗した。
- GREEN: 同じ static/provenance harness は roundtrip invocation と byte comparison の追加後に通過した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  と `git diff --check` を通過した。
- Rust oracle の `validate_source_and_emitted_manifest_have_same_report_and_exit_code` は実行を試みたが、
  default EmbeddedCli build が `selfhost/src/Tools/Validation/Stale.ls` の未定義
  `vector-push-single-rooted-v3` で停止したため、今回の GREEN evidence には数えていない。

この evidence は fake Lima/provenance harness の contract 検証であり、current source-commit に一致する
実 stage0 artifact/runtime、selfhost/native/MCP parity、または両 supported target の実行証跡ではない。

## Boundary / follow-up

EC-M2-03 の native current-source stage0 producer/runtime、manifest input の selfhost/native parity、MCP、
Mac Apple Silicon と Linux x86_64 の実 runtime evidence は未完了であり、TODO の `[~]` を維持する。

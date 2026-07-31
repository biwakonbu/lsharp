# ADR: v0.2 validation CLI の入力 I/O 診断境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp validate <manifest>` / `lsharp validate --source <source>` の入力読み込み
- Related: `EC-M2-03`、`decisions-v0.2-validation-config.md`

## Context

公開 `validate` CLI は parser、source adapter、report/manifest の入力エラーを report と分離して
返していた。一方、positional manifest と `--source` のファイル読み込みだけは generic miette error
となり、他の driver filesystem boundary と同じ stable I/O code を持っていなかった。入力を読めない
場合に report を生成しない契約は維持しつつ、呼び出し側が parser error と I/O error を機械的に区別
できる必要がある。

## Decision

- positional manifest と `--source` の `read_to_string` failure は `driver_io_error` を通し、`[LS5001]`
  を付与する。
- 読み込み failure は `--format json` でも stdout を空にし、validation report と `--emit-manifest`
  の出力を生成しない。
- parser/source-adapter の stable code と report status、`pass=0` / `fail=1` / `unknown=2` の判定
  semantics は変更しない。

## Evidence

- RED: `validate_manifest_read_failure_preserves_driver_io_error_boundary` と
  `validate_source_read_failure_preserves_driver_io_error_boundary` は、実 binary が空 stdout と
  generic miette error を返し `[LS5001]` を欠いていたため失敗した。
- GREEN: 2 fixture がともに exit `1`、空 stdout、`[LS5001]` を含む stderr になった。
- 回帰: `cargo test -p lsharp-driver --test validate_cli -- --nocapture`（34 passed）。既存の
  manifest/source report、parser diagnostic、emit-manifest、config path、exit parity は維持した。

## Boundary and follow-up

これは Rust-host 公開 CLI の入力 I/O code を統一した verified partial sliceである。selfhost/native
stage0 の同一 code/span、MCP input、current-source artifact/runtime、Mac Apple Silicon / Linux x86_64
matrix、EC-M2-03 aggregate は未完了であり、TODO の `[~]` を維持する。

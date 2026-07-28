# ADR: v0.2 validation CLI の subject kind 診断境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp validate` の version 1 JSON manifest input error 出力
- Related: `EC-M2-02`、`EC-M2-03`、`EC-M3-01`

## Context

Rust canonical manifest parser は relation と subject kind の不一致を
`ValidationInputError::InvalidSubjectKind` として分類するようになった。しかし公開 CLI の
`validate <manifest> --format json` でこの入力エラーが report JSON と混ざらず、診断専用の
stderr と non-zero exit になることは回帰テストで固定されていなかった。入力を検証できない
場合に report を生成すると、`pass` / `fail` / `unknown` の判定結果として誤って消費される
可能性がある。

## Decision

- manifest input の typed subject kind error は exit code `1` で終了する。
- 入力エラー時は `--format json` であっても stdout を空にし、report JSON と manifest file を生成しない。
- stderr には relation path (`evaluates.subject`)、不正 kind (`contract`)、stable wire ID
  (`contract:checkout/cancel-case`) を含む miette 診断を出力する。
- report を返す validation failure (`status: fail`) と、report を生成できない input error を
  別の observable boundary として扱う。

## Evidence

- RED: `validate_rejects_invalid_subject_kind_without_report_or_manifest_output` と
  `validate_rejects_invalid_invalidates_subject_kind_without_report_or_manifest_output` は CLI の
  入力エラー出力契約が未固定の状態で追加した。
- GREEN: `evaluates` / `invalidates` の両 fixture が exit `1`、空 stdout、manifest file なし、
  relation/kind/stable ID を含む stderr で拒否される。
- `LSHARP_EMBED_COMPONENT_PATH=<既存の Rust-host component>` を指定した focused driver test
  は pass した。今回の fixture は manifest input 経路のため、EmbeddedCli の実行結果には依存しない。
- default EmbeddedCli build は `origin/main` の既存 selfhost source にある未定義名
  `vector-push-single-rooted-v3` で停止した。この別セッション所有の差分には触れていない。

## Boundary and follow-up

これは Rust canonical manifest parser を公開 `validate` CLI の diagnostic-only boundary へ接続した
verified partial slice である。source/native stage0 の同一診断、`--emit-manifest` の native writer、
MCP、current-source artifact/runtime、Mac Apple Silicon / Linux x86_64 matrix、EC-M2-02 /
EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。

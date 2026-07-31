# ADR: lsharp review YAML scalar の安全なエスケープ

- Status: Accepted
- Date: 2026-07-31
- Scope: crates/lsharp-docs/src/review.rs の format_yaml

## Context

lsharp review の YAML 出力は、ソース由来のファイル名、宣言名、reviewer、
metadata diagnostic を double-quoted scalar へ埋め込む。従来は値をそのまま補間していたため、
引用符で scalar の境界が崩れ、改行で別の YAML 行へ分裂し、バックスラッシュも別の escape として
解釈され得た。

## Decision

- YAML の double-quoted scalar を生成する共通 helper を lsharp-docs 内に置く。
- バックスラッシュ、引用符、改行、復帰、タブ、NUL、ASCII 制御文字を YAML escape へ変換する。
- file、entry name、freshness、reviewer、review timestamp、metadata issue の全てに同じ helper を使う。
- 数値と boolean は既存の unquoted 表現を維持する。

## Evidence

- RED: test_format_yaml_escapes_double_quoted_scalars が、引用符・改行・バックスラッシュを含む
  fixture で未エスケープ出力を検出した。
- GREEN: 同テストが pass。
- Regression: cargo test -p lsharp-docs --lib は 24 tests pass。

## Boundary

これは YAML text serialization の入力安全性を閉じる slice であり、review の freshness 判定、
CLI の他フォーマット、selfhost/native parity、署名・lifecycle 検証は変更しない。

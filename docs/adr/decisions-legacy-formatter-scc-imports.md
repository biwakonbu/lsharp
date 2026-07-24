# ADR: Formatter 3 module の explicit import と batch 特例除去

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `selfhost/src/Tools/Text/Formatter{Expr,Decl,}.ls`, `lsharp-ir` compile pipelines
- Related: `decisions-legacy-module-incremental-scc.md`

## Context

FormatterExpr は `format-expr`、FormatterDecl は式 formatter 群を参照するが、従来は
`Formatter.ls` の bundle 順に依存し、module source には dispatch provider への import がなかった。
Rust compile pipeline はこの暗黙依存を `try_infer_formatter_trio_batch` で merged 推論する特例として
補っていたため、通常の SCC contract と Formatter だけの contract が分岐していた。

## Decision

- FormatterExpr と FormatterDecl は `Tools.Text.Formatter` を明示 import する。
- `compile_multi_file_with_mode` と `compile_multi_file_incremental` は Formatter 固有の batch inference を
  呼ばず、通常の `infer_scc_type_surfaces` を使う。
- source override 入口の SCC-aware inference と canonical selfhost runtime parity は別の未完要件として残す。

## Evidence

- RED: `test_formatter_modules_declare_cross_module_dispatch_imports` は両 source に provider import がない
  ことを確認して失敗した。
- GREEN: 同テストは両 source の explicit import を確認する。
- batch 特例除去後も、相互再帰 SCC の compile、`:only` import visibility、lsharp-ir regression 243 tests
  （canonical Formatter probe 1 件を skip）、clippy、rustfmt、docs audit が通過した。
- canonical `test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` は 90 秒超で
  完了せず、プロセスを停止した。これは成功 evidence ではなく、残る長時間 compile blocker として扱う。

## Residual risk

明示 import と generic SCC 経路は固定したが、canonical Formatter 3 module の全 compile/runtime parity、
source override の循環推論、SCC segment reuse、Mac/Linux native stage0 evidence は未完了である。
`LEGACY-MODULE-01` aggregate 完了や Formatter 完全対応は宣言しない。

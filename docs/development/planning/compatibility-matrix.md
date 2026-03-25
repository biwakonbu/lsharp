# L# 互換マトリクス

Rust 実装と L# セルフホスト実装の機能パリティを追跡する。

## PR 更新ルール

Phase 11 完了まで、selfhost パイプラインに影響する PR では本マトリクスの更新を必須とする。

**対象 PR の判定基準**:
- `selfhost/*.ls` の変更を含む PR
- `crates/lsharp-wasm/tests/e2e.rs` の selfhost 関連テストに変更がある PR
- `crates/lsharp-ir/` または `crates/lsharp-wasm/` のコード変更を含む PR
- `docs/development/planning/compatibility-matrix.md` の更新が明示的にスコープ外と宣言されていない PR

**更新手順**:
1. 変更に関連する行の `L# status`, `Parity test` 列を現状に合わせて更新する
2. 変更によって parity が後退する場合は `Deletion gate` の条件と照合し、影響を PR 本文に記載する
3. `scripts/audit_docs.sh` を実行し、監査エラーが増加していないことを確認する

**レビューチェック**: PR レビュー時に互換マトリクスの更新漏れがないことを確認する。

## CLI サブコマンド

| Feature | Rust source | L# source | Parity test | Default path | Deletion gate | Evidence | Notes |
|---------|-------------|-----------|-------------|--------------|---------------|----------|-------|
| parse | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_parse_check_compile` (構造のみ) | Rust | `run-parse` が AST/diagnostics parity を返し CLI golden が揃うこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-parse` は `exit-success` を返すだけ |
| check | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_parse_check_compile` (構造のみ) | Rust | `run-check` が型診断 parity を返すこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-check` は `exit-success` を返すだけ |
| compile | 完成 | 暫定統合 pipeline + stub dispatch (`selfhost/Cli.ls`, `selfhost/Main.ls`) | `test_e2e_selfhost_cli_parse_check_compile`, `test_e2e_selfhost_pipeline_complete_stages` | Rust | true bootstrap + CLI parity + native default path | `selfhost/Cli.ls`, `selfhost/Main.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-compile` は stub、`Main.ls` は import-only 未達 |
| build | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_parse_check_compile` (構造のみ) | Rust | プロジェクト build contract と artifact parity を満たすこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-build` は `exit-success` を返すだけ |
| test | 完成 | 骨格のみ (`selfhost/Cli.ls`, `selfhost/TestRunner.ls`) | `test_e2e_selfhost_cli_parse_check_compile`, `test_e2e_selfhost_test_runner` | Rust | metadata semantics と CLI exit code parity を満たすこと | `selfhost/Cli.ls`, `selfhost/TestRunner.ls`, `crates/lsharp-wasm/tests/e2e.rs` | CLI は stub、TestRunner は `actual <- expected` / invariant 常時 pass |
| review | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_review_doc` (構造のみ) | Rust | review output schema と diagnostics parity を満たすこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-review` は `exit-success` を返すだけ |
| doc-ack | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_review_doc` (構造のみ) | Rust | doc ack contract / exit code parity を満たすこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-doc-ack` は `exit-success` を返すだけ |
| doc-check | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_review_doc` (構造のみ) | Rust | doc check schema / exit code parity を満たすこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-doc-check` は `exit-success` を返すだけ |
| install | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_review_doc` (構造のみ) | Rust | package install / checksum / archive parity を満たすこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-install` は `exit-success` を返すだけ |
| repl | 完成 | 骨格のみ (`selfhost/Cli.ls`) | `test_e2e_selfhost_cli_repl_lsp_fmt` (構造のみ) | Rust | REPL eval loop と runtime stability gate を満たすこと | `selfhost/Cli.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `run-repl` は `exit-success` を返すだけ |
| lsp | 完成 | 骨格のみ (`selfhost/Cli.ls`, `selfhost/LspServer.ls`) | `test_e2e_selfhost_cli_repl_lsp_fmt`, `test_e2e_selfhost_lsp_skeleton_v2`, `test_e2e_selfhost_lsp_10_methods` | Rust | 10 method parity + JSON snapshot + soak gate を満たすこと | `selfhost/Cli.ls`, `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | 多くのハンドラは `0` / 空 vector |
| fmt | 完成 | 骨格のみ (`selfhost/Cli.ls`, `selfhost/Formatter.ls`) | `test_e2e_selfhost_cli_repl_lsp_fmt`, `test_e2e_selfhost_formatter`, `test_e2e_selfhost_formatter_roundtrip_v2` | Rust | roundtrip/idempotency と CLI/LSP parity を満たすこと | `selfhost/Cli.ls`, `selfhost/Formatter.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `format-program` は placeholder |
| doc | 完成 | 部分実装 (`selfhost/Cli.ls`, `selfhost/DocTools.ls`, `selfhost/HtmlDoc.ls`) | `test_e2e_selfhost_cli_repl_lsp_fmt`, `test_e2e_selfhost_doc_schemas`, `test_e2e_selfhost_doc_deterministic_html` | Rust | HTML/doc schema 出力と distribution flow parity を満たすこと | `selfhost/Cli.ls`, `selfhost/DocTools.ls`, `selfhost/HtmlDoc.ls`, `crates/lsharp-wasm/tests/e2e.rs` | CLI dispatch は stub、doc generator は未 default path |

## LSP メソッド

| Feature | Rust source | L# source | Parity test | Default path | Deletion gate | Evidence | Notes |
|---------|-------------|-----------|-------------|--------------|---------------|----------|-------|
| initialize | 完成 | 骨格のみ (`handle-initialize`) | `test_e2e_selfhost_lsp_skeleton_v2`, `test_e2e_selfhost_lsp_10_methods` | Rust | capabilities JSON parity と initialize/shutdown lifecycle が揃うこと | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | capabilities vector を返すのみ |
| shutdown | 完成 | 骨格のみ (`handle-shutdown`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | shutdown 応答と server-loop が JSON-RPC で閉じること | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `0` を返すだけ |
| didOpen | 完成 | 骨格のみ (`handle-didOpen`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | diagnostics 生成と state 更新が観測可能であること | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `0` を返すだけ |
| didChange | 完成 | 骨格のみ (`handle-didChange`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | full-sync 再解析と diagnostics 更新が観測可能であること | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `0` を返すだけ |
| hover | 未実装 | 骨格のみ (`handle-hover`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | hover markdown / range parity を満たすこと | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `[range, contents]` の空 placeholder |
| goto_definition | 完成 | 骨格のみ (`handle-goto-definition`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | location parity を満たすこと | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | URI/range placeholder |
| references | 完成 | 骨格のみ (`handle-references`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | reference list parity を満たすこと | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | 空 vector |
| rename | 完成 | 骨格のみ (`handle-rename`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | WorkspaceEdit parity を満たすこと | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | changes placeholder |
| formatting | 完成 | 骨格のみ (`handle-formatting`) | `test_e2e_selfhost_lsp_10_methods`, `test_e2e_selfhost_formatter_lsp_integration` | Rust | Formatter 連携で TextEdit parity を満たすこと | `selfhost/LspServer.ls`, `selfhost/Formatter.ls`, `crates/lsharp-wasm/tests/e2e.rs` | 空 vector |
| completion | なし | 骨格のみ (`handle-completion`) | `test_e2e_selfhost_lsp_10_methods` (構造のみ) | Rust | completion item parity を満たすこと | `selfhost/LspServer.ls`, `crates/lsharp-wasm/tests/e2e.rs` | 空 vector |

## Selfhost パイプライン

| Feature | Rust source | L# source | Parity test | Default path | Deletion gate | Evidence | Notes |
|---------|-------------|-----------|-------------|--------------|---------------|----------|-------|
| Lexer | 完成 | 部分実装 (compile gate 通過) | `test_e2e_bootstrap_ci_all_modules_compile`, `test_e2e_selfhost_pipeline_complete_stages` | Rust | selfhost default path で token parity を満たすこと | `selfhost/Lexer.ls`, `crates/lsharp-wasm/tests/e2e.rs`, `scripts/ci/compile-phase11-inputs.sh` | fixed input set compile は通る |
| Parser | 完成 | 部分実装 (compile gate 通過) | `test_e2e_bootstrap_ci_all_modules_compile`, `test_e2e_selfhost_pipeline_complete_stages` | Rust | diagnostics / recovery parity を満たすこと | `selfhost/Parser.ls`, `crates/lsharp-wasm/tests/e2e.rs`, `scripts/ci/compile-phase11-inputs.sh` | fixed input set compile は通る |
| MacroExpand | 完成 | 部分実装 (compile gate 通過) | `test_e2e_bootstrap_ci_all_modules_compile`, `test_e2e_selfhost_pipeline_complete_stages` | Rust | macro parity + golden が揃うこと | `selfhost/MacroExpand.ls`, `crates/lsharp-wasm/tests/e2e.rs`, `scripts/ci/compile-phase11-inputs.sh` | stage0 compile は成功 |
| TypeInfer | 完成 | 部分実装 (compile gate 通過) | `test_e2e_bootstrap_ci_all_modules_compile`, `test_e2e_selfhost_pipeline_complete_stages` | Rust | type error parity / deterministic ordering を満たすこと | `selfhost/TypeInfer.ls`, `crates/lsharp-wasm/tests/e2e.rs`, `scripts/ci/compile-phase11-inputs.sh` | stage0 compile は成功 |
| Lower/Compiler | 完成 | 部分実装 (Lower split 未完) | `test_e2e_bootstrap_stage1_compile_selfhost_sources`, `test_e2e_selfhost_compiler` | Rust | `Lower.ls` / `LowerPattern.ls` の blocker 解消 + IR parity | `selfhost/Compiler.ls`, `selfhost/Lower.ls`, `selfhost/LowerDecl.ls`, `selfhost/LowerExpr.ls`, `selfhost/LowerPattern.ls`, `crates/lsharp-wasm/tests/e2e.rs` | `Lower.ls`, `LowerPattern.ls` は stage0 compile で stack overflow |
| WasmEmit | 完成 | 部分実装 (compile gate 通過) | `test_e2e_bootstrap_stage1_compile_selfhost_sources`, `test_e2e_bootstrap_stage1_symbol_stability` | Rust | section/symbol fixed point が true bootstrap で通ること | `selfhost/WasmEmit.ls`, `crates/lsharp-wasm/tests/e2e.rs`, `scripts/ci/compile-phase11-inputs.sh` | true stage1->stage2->stage3 は未接続 |
| NativeEmit | N/A | 骨格実装のみ (`NativeTarget`, `NativeCodegen`, `NativeEmit`, `Linker`) | `test_e2e_selfhost_native_self_regeneration`, `test_e2e_selfhost_wasm_native_differential` (構造のみ) | Rust | self-regeneration + Wasm/native diff zero を実行ベースで満たすこと | `selfhost/NativeTarget.ls`, `selfhost/NativeCodegen.ls`, `selfhost/NativeEmit.ls`, `selfhost/Linker.ls`, `crates/lsharp-wasm/tests/e2e.rs` | 実行ベース differential / native fixed point は未接続 |

## 凡例

- **Feature**: 機能名
- **Rust source**: Rust 実装の現在の状態
- **L# source**: L# セルフホスト実装の現在の状態
- **Parity test**: Rust と L# の出力が一致することを検証するテストの有無
- **Default path**: 現在デフォルトで使用される実装 (Rust or L#)
- **Deletion gate**: Rust 実装を削除可能になる条件
- **Evidence**: テスト名、ファイル名等のエビデンス
- **Notes**: 補足事項

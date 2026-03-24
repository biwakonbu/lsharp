# L# 互換マトリクス

Rust 実装と L# セルフホスト実装の機能パリティを追跡する。

## CLI サブコマンド

| コマンド | Rust status | L# status | Parity test | Default path | Deletion gate |
|----------|-------------|-----------|-------------|--------------|---------------|
| parse | 完成 | なし | - | Rust | L# parity test 全 PASS |
| check | 完成 | なし | - | Rust | L# parity test 全 PASS |
| compile | 完成 | PoC (Main.ls) | - | Rust | L# parity test 全 PASS |
| build | 完成 | なし | - | Rust | L# parity test 全 PASS |
| test | 完成 | なし | - | Rust | L# parity test 全 PASS |
| review | 完成 | なし | - | Rust | L# parity test 全 PASS |
| doc-ack | 完成 | なし | - | Rust | L# parity test 全 PASS |
| doc-check | 完成 | なし | - | Rust | L# parity test 全 PASS |
| install | 完成 | なし | - | Rust | L# parity test 全 PASS |
| repl | 完成 | なし | - | Rust | L# parity test 全 PASS |
| lsp | 完成 | PoC (JsonRpc.ls) | - | Rust | L# parity test 全 PASS |
| fmt | 完成 | PoC (Formatter.ls) | - | Rust | L# parity test 全 PASS |
| doc | 完成 | なし | - | Rust | L# parity test 全 PASS |

## LSP メソッド

| メソッド | Rust status | L# status | Parity test | Default path | Deletion gate |
|----------|-------------|-----------|-------------|--------------|---------------|
| initialize | 完成 | 設計のみ | - | Rust | L# parity test 全 PASS |
| shutdown | 完成 | 設計のみ | - | Rust | L# parity test 全 PASS |
| didOpen | 完成 | 設計のみ | - | Rust | L# parity test 全 PASS |
| didChange | 完成 | 設計のみ | - | Rust | L# parity test 全 PASS |
| hover | 未実装 | なし | - | Rust | L# parity test 全 PASS |
| goto_definition | 完成 | なし | - | Rust | L# parity test 全 PASS |
| references | 完成 | なし | - | Rust | L# parity test 全 PASS |
| rename | 完成 | なし | - | Rust | L# parity test 全 PASS |
| formatting | 完成 | PoC | - | Rust | L# parity test 全 PASS |
| completion | なし | なし | - | - | - |

## Selfhost パイプライン

| コンポーネント | Rust status | L# status | Parity test | Default path | Deletion gate |
|----------------|-------------|-----------|-------------|--------------|---------------|
| Lexer | 完成 | 部分実装 (75%) | E2E 6件 | Rust | L# parity test 全 PASS |
| Parser | 完成 | 部分実装 (65%) | E2E 4件 | Rust | L# parity test 全 PASS |
| MacroExpand | 完成 | なし | - | Rust | L# parity test 全 PASS |
| TypeInfer | 完成 | なし | - | Rust | L# parity test 全 PASS |
| Lower/Compiler | 完成 | 部分実装 (70%) | E2E 2件 | Rust | L# parity test 全 PASS |
| WasmEmit | 完成 | 部分実装 (50%) | - | Rust | L# parity test 全 PASS |
| NativeEmit | N/A | なし | - | - | - |

## 凡例

- **Rust status**: Rust 実装の現在の状態
- **L# status**: L# セルフホスト実装の現在の状態
- **Parity test**: Rust と L# の出力が一致することを検証するテストの有無
- **Default path**: 現在デフォルトで使用される実装 (Rust or L#)
- **Deletion gate**: Rust 実装を削除可能になる条件

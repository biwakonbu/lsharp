# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 言語規則

- **自然言語**: 日本語を使用
- **コメント**: 日本語で記述
- **変数・関数名**: 英語（国際標準）
- **コード**: 英語（国際標準）

## プロジェクト概要

L# (lsharp) は S 式構文 + Hindley-Milner 型推論の言語。WebAssembly (WASI) をターゲットに、wasmtime で直接実行可能。

## ビルド・テスト・リント

```bash
cargo build                        # ビルド
cargo test                         # 全テスト実行
cargo test test_e2e_fibonacci      # 個別テスト実行
cargo test -p lsharp-wasm          # クレート単位でテスト
cargo clippy                       # リント
```

## CLI コマンド

```bash
cargo run -- compile examples/fib.ls -o fib.wasm  # 公開 CLI の基本動線
cargo run -- test examples/fib.ls                 # メタデータテスト (:example, :invariant)
cargo run -- lsp                                  # IDE 向けバックエンド
cargo run -- mcp-server                           # AI 向けバックエンド
```

公開 CLI は `compile` 中心で案内する。`parse` / `check` / `fmt` は LSP / MCP が利用する内部 API として扱い、
ユーザー向けの手順や smoke test には載せない。

## ワークスペース構成

7 クレートの Cargo ワークスペース。コンパイラパイプライン順:

| クレート | 役割 |
|---------|------|
| `lsharp-syntax` | Lexer + Parser → AST 生成 |
| `lsharp-types` | Hindley-Milner 型推論・制約解決・メタデータ検証 |
| `lsharp-ir` | AST → IR への変換 (lowering)、モジュールリンク |
| `lsharp-wasm` | IR → WebAssembly バイナリ生成 (WASI) |
| `lsharp-driver` | CLI エントリポイント、プロジェクト管理 |
| `lsharp-lsp` | LSP サーバー (tower-lsp 統合) |
| `lsharp-docs` | ドキュメント追跡・レビュー管理 |

## コンパイラパイプライン

```
Source (.ls)
  → Lexer (lsharp-syntax/lexer.rs) → Token列
  → Parser (lsharp-syntax/parser.rs) → AST (Program)
  → Type Inference (lsharp-types/infer.rs) → 型チェック済み AST
  → Lowering (lsharp-ir/lower.rs) → IR (Module)
  → Codegen (lsharp-wasm/wasi.rs) → .wasm バイナリ
```

## 主要な型

- **AST**: `Program`, `Expr`, `Decl`, `Pattern`, `Literal`, `Metadata` (lsharp-syntax/ast.rs)
- **型システム**: `Type` (Con/Var/Fun/App/Record), `TypeScheme`, `Substitution`, `TypeEnv` (lsharp-types/types.rs)
- **IR**: `Module`, `Function`, `Instruction`, `IrType` (lsharp-ir/lib.rs)
- **制約**: `TraitConstraint`, `ConstrainedTypeInfo`, `ConstraintDef` (lsharp-types/constraints.rs)

## テスト構成

- **E2E テスト**: `crates/lsharp-wasm/tests/e2e.rs` — フルパイプライン (parse → infer → lower → codegen → WASI 実行)
- **スナップショットテスト**: `insta` クレートによる IR/型出力の回帰テスト
- **メタデータテスト**: `:example` / `:invariant` アノテーションからの自動テスト生成

## TDD ワークフロー (必須)

実装タスクは必ず TDD (テスト駆動開発) で進める。テストなしの実装は完了と見なさない。

### フロー

1. **RED**: テストを先に書く → `cargo test` で **失敗を確認**
2. **GREEN**: 実装を書く → `cargo test` で **成功を確認**
3. **REFACTOR**: リファクタリング → テスト成功を維持
4. **UPDATE**: TODO.md を更新 (完了項目は ADR / 運用記録へ移して削除。テスト数を注記)

### ルール

- 実装ファイルを編集する前に、必ず対応するテストを書く
- **`TODO.md` に `[x]` は使わない。** `[ ]` (未着手) / `[~]` (verified slice はあるが completion
  boundary 未達) / `[BLOCKED: 理由]` の 3 つだけを使う。完了した項目は ADR / 運用記録へ移して
  `TODO.md` から削除する (正本は `TODO.md` 冒頭の凡例と `AGENTS.md`)
- テストが 0 個の項目は完了扱いにしない (`[~]` で留める)
- テストが失敗したら実装を修正する (テストの期待値を変更しない)
- `/tdd <タスク>` コマンドで TDD ワークフローを起動できる (例: `/tdd P6-3 Computation Expression の脱糖実装`)

## ドキュメント同期ワークフロー (必須)

TDD が「テストを先に書く」規律であるのと同じ意味で、**「決定を先に書く」** 規律を持つ。
ドキュメント更新は後片付けではなく作業の一部であり、依頼されて初めて更新するものではない。

1. **doc-RED**: 実装前に、何が問題か / 何を決めたかを正本へ書く
2. **実装**: TDD で進める
3. **doc-GREEN**: 実測値・受入判定を正本へ戻す。**満たせなかった受入条件は必ず明示する**

正本の分担 (混ぜると二重管理になる):

| 正本 | 持つもの |
|---|---|
| `ISSUES.md` | 何が問題か・根拠・状態。チェックボックスは置かない |
| `TODO.md` | 未完了タスクだけ |
| `docs/adr/decisions-*.md` | 判断と却下理由 |
| `docs/development/operations/` | 実測値と運用手順 |
| `AGENTS.md` | 日常の作業手順 |

- 規約の正本: `.claude/rules/doc-sync.md` (いつ書くか) と `.claude/rules/docs-organization.md` (どこに置くか)
- 手順は `doc-sync` スキルを **Skill ツールで呼び出して** 使う (内容を読んで手動実行しない)
- `.claude/hooks/doc-guard.sh` が実装ファイル編集時に未更新を警告する。**ブロックはしない** ので、
  判断を含まない機械的な変更 (typo / rustfmt / test split / 挙動不変のリファクタ) では無視してよい
- 作業中に繰り返し必要になる手順が現れたら `.claude/skills/<name>/SKILL.md` として切り出す

## hooks/スキルのトラブルシューティング

hooks やスキルに問題が発生した場合は `.claude/rules/hook-troubleshooting.md` を参照。
注意: hook の stderr 出力 ([TDD Guard], [TDD Tracker]) は正常な情報メッセージであり、エラーとして対処する必要はない。

## ファイルサイズ制限

- 1 ファイルあたり **500〜800 行**に収める
- これを超えるとエージェントの解析精度が落ちるため、早めにモジュール分割・リファクタリングを行う
- 新規実装時も既存ファイルが肥大化しないよう注意する

## 主要依存関係

- `miette`: ソーススパン付きリッチエラーレポート
- `wasm-encoder`: WebAssembly バイナリ生成
- `wasmtime` + `wasmtime-wasi`: Wasm 実行ランタイム
- `insta`: スナップショットテスト
- `clap`: CLI 引数パース
- `tower-lsp`: LSP サーバーフレームワーク

## 言語機能

- S 式構文 (Clojure 風)
- ADT + パターンマッチ → リニアメモリ上の struct (タグによる判別)
- レコード型 → リニアメモリ上の struct
- モジュールシステム: `(module Name)`, `(import Module)`, `(open Module)`
- トレイト: 辞書引数による静的ディスパッチ
- 計算式: `let!` によるモナディックバインド
- メタデータ: `:doc`, `:example`, `:invariant`, `:transitions`

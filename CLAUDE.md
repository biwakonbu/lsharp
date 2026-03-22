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
cargo run -- parse examples/fib.ls --ast    # AST 表示
cargo run -- check examples/fib.ls          # 型チェック
cargo run -- compile examples/fib.ls -o fib.wasm  # Wasm コンパイル
cargo run -- test examples/fib.ls           # メタデータテスト (:example, :invariant)
```

## ワークスペース構成

6 クレートの Cargo ワークスペース。コンパイラパイプライン順:

| クレート | 役割 |
|---------|------|
| `lsharp-syntax` | Lexer + Parser → AST 生成 |
| `lsharp-types` | Hindley-Milner 型推論・制約解決・メタデータ検証 |
| `lsharp-ir` | AST → IR への変換 (lowering)、モジュールリンク |
| `lsharp-wasm` | IR → WebAssembly バイナリ生成 (WASI) |
| `lsharp-driver` | CLI エントリポイント、プロジェクト管理 |
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
4. **UPDATE**: TODO.md の項目を `[x]` に更新 (テスト数を注記)

### ルール

- 実装ファイルを編集する前に、必ず対応するテストを書く
- テストが 0 個の項目は `[x]` にしない (`[~]` で留める)
- テストが失敗したら実装を修正する (テストの期待値を変更しない)
- `/tdd <タスク>` コマンドで TDD ワークフローを起動できる (例: `/tdd P6-3 Computation Expression の脱糖実装`)

## hooks/スキルのトラブルシューティング

hooks やスキルに問題が発生した場合 (hook がエラーを出す、期待通りに動作しない、設定が壊れた等)、**サブエージェントを起動して調査・修正する**。

### 検知トリガー

- hook の stderr に `[tdd-guard]` や `[test-tracker]` 以外のエラーメッセージが出た場合
- hook がタイムアウトした場合
- `/tdd` コマンドの手順が途中で失敗した場合
- `cargo test` の結果が hook で検知されない場合
- ユーザーから hook/スキルの不具合報告があった場合

### 対応フロー

1. **Explore サブエージェント**を起動してエラーログ (`/tmp/lsharp-hook-errors.log`) と hook スクリプト (`.claude/hooks/`) を調査
2. 問題の根本原因を特定
3. hook スクリプト、settings.json、rules、コマンド定義を修正
4. 修正後、テスト用 JSON をパイプして動作検証 (例: `echo '{"tool_name":"Edit",...}' | .claude/hooks/tdd-guard.sh`)
5. 検証が通ったらユーザーに報告

### 対象ファイル

- `.claude/hooks/tdd-guard.sh` — PreToolUse (Edit|Write) ガード
- `.claude/hooks/test-result-tracker.sh` — PostToolUse (Bash) トラッカー
- `.claude/settings.json` — hooks 設定
- `.claude/commands/tdd.md` — /tdd コマンド定義
- `.claude/rules/tdd-workflow.md` — TDD ルール

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

## 言語機能

- S 式構文 (Clojure 風)
- ADT + パターンマッチ → WasmGC struct ($tag による判別)
- レコード型 → WasmGC struct
- モジュールシステム: `(module Name)`, `(import Module)`, `(open Module)`
- トレイト: 辞書引数による静的ディスパッチ
- 計算式: `let!` によるモナディックバインド
- メタデータ: `:doc`, `:example`, `:invariant`, `:transitions`

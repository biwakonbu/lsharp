# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

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
4. **UPDATE**: TODO.md の項目を `[x]` に更新 (テスト数を注記)

### ルール

- 実装ファイルを編集する前に、必ず対応するテストを書く
- テストが 0 個の項目は `[x]` にしない (`[~]` で留める)
- テストが失敗したら実装を修正する (テストの期待値を変更しない)
- `/tdd <タスク>` コマンドで TDD ワークフローを起動できる (例: `/tdd P6-3 Computation Expression の脱糖実装`)

## Rust-free selfhost の進め方

L# の最終目標は、Rust 実装を正本として残したまま一部のコマンドだけを動かすことではなく、L# の全言語機能と公開コマンドを自己ホスト実装へ段階的に移し、通常の開発・テスト・Wasm 出力を Rust なしで完走できる状態にすることである。作業中は Rust を bootstrap、oracle/differential 検証、障害時の rollback、未移行 host integration のために保持するが、それを理由に未対応機能を完了扱いにしない。

- 対応 target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) に限定する。日常の core CLI は provenance が検証された native stage0 と `scripts/native-selfhost-dev.sh` を入口にし、成功経路で `cargo`、`rustc`、host `lsharp`、Rust fallback を呼ばない。
- 言語機能を Rust-free 完了とするには、parser、型推論、lowering、codegen、runtime、source/ftable/import の必要経路を同じ仕様で閉じ、対応 target の native program から実際に実行する E2E テストを追加する。単一レイヤーの unit test、Rust driver 経由の成功、summary/header の生成だけでは完了としない。
- Rust oracle は parity を確認するために使う。新しい L# 実装は RED テスト、Rust との診断/出力差分確認、native stage0 の実行確認、regression test の順で進め、未対応機能は誤った Wasm を出さず明示的な診断または明示的な外部境界を返す。
- `compile` / `build` の全 target、EmbeddedCli/Component の実成果物、LSP/MCP/REPL/install/doc などの公開 surface を個別に検証する。Rust host fallback が欠落を隠さないよう、guest-success、artifact bytes、standalone runtime、外部 helper の境界を分けてテストする。
- 長時間の stage regeneration や Linux VM gate の実行中は、対象を共有しない parser/type/runtime の focused test、docs、診断、fixture、契約テストを並行して進める。VM の待ち時間を理由に実装を止めず、完了後に native gate と fixed-point evidence を統合する。
- `TODO.md` と `docs/development/operations/rust-boundary-reduction.md` は current truth として更新する。`[x]` はこの完了基準を満たした項目だけにし、partial parity、既知の Rust-only surface、未検証の ABI は `[~]` と残リスクに記録する。
- stage0 の生成・配布・source commit provenance・rollback は運用上の bootstrap boundary であり、通常開発から Rust を外せても、公開 release の再現性と緊急復旧を検証するまで削除しない。

## hooks/スキルのトラブルシューティング

hooks やスキルに問題が発生した場合は `.Codex/rules/hook-troubleshooting.md` を参照。
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

# TODO 全残タスク完了 - 設計書

> 最終更新: 2026-03-24

## 概要

既存の L# コンパイラパイプライン (Syntax -> Types -> IR -> Wasm) を基盤に、構造的バグ修正、品質改善、エコシステム拡充を行った。新規クレートの追加は不要で、全変更は既存クレート内のモジュール修正・追加で完結している。4 フェーズ構成で実施し、Phase A-D の順序で依存関係を考慮しながら進行した。

## アーキテクチャ

### 実行フェーズ構成

```
Phase A (検証)     : IMP-1/2/3/4, QA-3  -- 既存実装の確認とテスト追加
Phase B (バグ修正)  : BUG-1/2, BUG-3     -- lower/decl.rs, lower/pattern.rs
Phase C (品質)     : QA-1/2/4/5, CI/CD   -- テスト基盤・ベンチマーク・CI
Phase D (大規模)   : P9-2, P1-2, P8-5   -- LSP・文字列ヒープ化・セルフホスト
```

### 並列実行可能なタスクグループ

- **グループ 1**: IMP-1, IMP-2, IMP-3, IMP-4, QA-3 (全て独立した検証タスク)
- **グループ 2**: BUG-1+BUG-2 (同一関数修正) と BUG-3 (別ファイル) は並列可能
- **グループ 3**: QA-2, QA-4, QA-5, CI/CD (全て独立)
- **グループ 4**: P9-2 と QA-1 は並列可能
- **直列必須**: P1-2 -> P8-5 (文字列ヒープ化がセルフホストの前提条件)

## コンポーネント

### BUG-1/2: infer_expr_type_name 拡張

- **責務**: 式から型名を静的に推定する関数の対応範囲を拡張
- **変更箇所**: `crates/lsharp-ir/src/lower/decl.rs`, `crates/lsharp-ir/src/lower/expr.rs`
- **対応済み**: Lit, Var, Ann, RecordLit, App
- **追加対応**: FieldAccess, Let, Match, If, Lambda, RecordUpdate

```rust
fn infer_expr_type_name(&self, expr: &Expr) -> Option<String> {
    match expr {
        Expr::FieldAccess { expr: base, field, .. } => {
            // base の型名を取得 -> record_fields から field の型を解決
            let base_type = self.infer_expr_type_name(base)?;
            // record_fields[base_type] のフィールド定義から field の型を返す
        }
        Expr::Let { body, .. } => {
            self.infer_expr_type_name(body)
        }
        Expr::If { then_branch, .. } => {
            self.infer_expr_type_name(then_branch)
        }
        _ => None,
    }
}
```

### BUG-3: lower_match_arms タグ比較修正

- **責務**: コンストラクタパターンの最後の腕でもタグ比較を実施
- **変更箇所**: `crates/lsharp-ir/src/lower/pattern.rs`
- **方針**: 全ての腕でタグ比較を行い、最後の腕のタグ不一致時は unreachable を出力

```rust
// 変更前: 最後の腕はタグ比較をスキップ
let need_branch = idx < arms.len() - 1;

// 変更後: 常にタグ比較を行う
let need_branch = true;
```

### QA-1: 正規表現エンジン NFA->DFA + モジュール分割

- **責務**: constraints.rs から正規表現エンジンを分離し、NFA->DFA 変換を実装
- **分割結果**:
  - `crates/lsharp-types/src/regex/` -- 正規表現エンジンモジュール (NFA/DFA)
  - `crates/lsharp-types/src/constraints.rs` -- 制約ロジックのみ (正規表現は regex モジュールを使用)
- **DFA 変換方式**: Thompson NFA -> epsilon 除去 -> サブセット構成法 -> DFA
- **ハイブリッド方式**: 後方参照使用時はフォールバックで NFA バックトラッキングを使用 (DFA では表現不可のため)
- **Unicode サポート**: `\p{L}` (Letter), `\p{N}` (Number), `\p{Z}` (Separator) の基本カテゴリ

### QA-2: generate_sample_args 最適化

- **変更箇所**: `crates/lsharp-types/src/test_runner.rs`
- **方針**: `generate_sample_args` の結果を事前計算し、`parse_test_output` の引数として渡すことで重複呼び出しを解消

### QA-3: 型推論結果 HashMap 化

- **変更箇所**: `crates/lsharp-types/src/infer.rs` の `infer_program` 戻り値型
- **方針**: `infer_program` の戻り値を `HashMap<String, TypeScheme>` に変更し、呼び出し側の Vec -> HashMap 変換コードを削除

### QA-4: スナップショットテスト拡大

- **変更箇所**: `crates/lsharp-wasm/tests/e2e.rs`
- **方針**: 代表的な L# プログラム (fibonacci, factorial, ADT, record, pattern match) の Wasm 出力を wat テキストに変換し、`insta::assert_snapshot!` で回帰テスト

### QA-5: criterion ベンチマーク

- **新規ファイル**: `benches/pipeline.rs`, `Cargo.toml` に criterion dev-dependency 追加
- **ベンチマーク対象**: parse, infer, lower, codegen の各ステージ + フルパイプライン

### P9-2: LSP 全機能

- **変更箇所**: `crates/lsharp-lsp/src/` (lib.rs, util.rs, references.rs, rename.rs, format.rs)
- **ドキュメントキャッシュ**: `Arc<RwLock<HashMap<Url, String>>>` による管理
- **実装機能**:
  - **hover**: documents -> parse -> infer -> Position からシンボル解決 -> 型情報返却
  - **goto_definition**: documents -> parse -> find_definition -> Location 返却
  - **completion**: トークン位置からキーワード + スコープ内変数名を候補として返す
  - **references**: AST 上のシンボル走査で参照箇所を収集
  - **rename**: references と同じシンボル走査 + 一括置換
  - **formatting**: parse -> AST -> pretty print

### P1-2: 文字列ヒープ化

- **変更箇所**: `crates/lsharp-ir/src/lower/expr.rs`, `crates/lsharp-wasm/src/wasi.rs`
- **メモリレイアウト**: `[tag=1: i32][len: i32][bytes: u8...]`
- **コンパイル変更**: 文字列リテラル -> `__alloc` 呼び出し -> ヒープ書き込み -> ポインタ返却
- **ビルトイン更新**: string-char-at, substring, string-concat, string-eq, string-length, print-string

### P8-5: セルフホスティング統合

- **新規ファイル**: `selfhost/Main.ls`
- **統合パイプライン**: Source -> Lexer -> Parser -> TypeInfer -> Compiler -> WasmEmit -> .wasm
- **検証**: stage1.wasm で最小 L# プログラムをコンパイルし、正しい Wasm が生成されることを確認
- **未完了**: stage2.wasm 生成・固定点検証 (L# 言語機能のカバレッジ拡張が前提)

### パターンマッチ改善

- **変更箇所**: `crates/lsharp-syntax/src/ast.rs`, `crates/lsharp-syntax/src/parser.rs`, `crates/lsharp-ir/src/lower/pattern.rs`, `crates/lsharp-ir/src/lower/closure.rs`
- **ネストパターン**: コンストラクタパターン内に更にコンストラクタパターンを記述可能
- **ガード条件**: `(when condition)` 構文によるパターンマッチのガード

### CI/CD: GitHub Actions

- **新規ファイル**: `.github/workflows/ci.yml`
- **トリガー**: push (main), pull_request
- **ジョブ**: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt -- --check`
- **マトリクス**: stable Rust + latest Ubuntu
- **PR マージブロック**: CI パスを必須条件に設定

## データ設計

### BUG-1/2 修正後のデータフロー

```
FieldAccess(expr, field)
  -> infer_expr_type_name(expr)
  -> Some(type_name): record_fields[type_name] から正確な型情報取得
  -> None: エラー (フォールバック走査を廃止または警告付き)
```

### P1-2 文字列メモリレイアウト

```
ヒープ上の String オブジェクト:
  offset+0: tag (i32) = 1    -- 文字列型識別子
  offset+4: len (i32)        -- バイト長
  offset+8: bytes (u8[len])  -- UTF-8 バイト列
```

## エラーハンドリング

- **BUG-1/2**: `infer_expr_type_name` が None を返す場合、フォールバック走査を行うが警告を出力。将来的にはエラーにする
- **BUG-3**: タグ不一致の最後の腕で `unreachable` Wasm 命令を出力
- **P1-2**: ヒープ確保失敗時は `__alloc` のトラップでプロセス終了
- **LSP**: parse/infer エラーはクライアントに Diagnostic として通知

## テスト戦略

### ユニットテスト

- BUG-1/2: `infer_expr_type_name` の各式パターンに対するテスト (7件)
- BUG-3: パターンマッチのタグ比較テスト (3件)
- IMP-1/2/3/4: 既存実装の動作検証テスト (12件)
- QA-1: 正規表現エンジンの NFA->DFA 変換テスト、Unicode 文字クラステスト (34件)
- QA-2/3: 最適化・HashMap 化のテスト (5件)
- P9-2: LSP 各機能のテスト (27件)

### E2E テスト

- BUG-1/2: 同名フィールドの別レコード型を使った FieldAccess/RecordUpdate テスト
- BUG-3: 非網羅パターンマッチのテスト
- P1-2: 文字列操作の全テスト更新 (8件)
- P8-5: セルフホストコンパイラでのコンパイルテスト (2件)

### スナップショットテスト

- QA-4: 代表プログラムの Wasm 出力スナップショット (22件)

### ベンチマーク

- QA-5: criterion による各ステージのベンチマーク (10件)

### テスト実績

- テスト総数: 701件 (既存約 570件 + 追加約 130件)
- 全テストパス、失敗 0件

## 関連ドキュメント

- [要件定義書](./requirements.md)
- [TODO 未完了タスク並列実装 設計書](../todo-parallel-implementation/design.md)

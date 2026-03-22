# L# 型システム実装 TODO

> `docs/type-system-roadmap.md` から抽出。並列作業用に依存関係を明示。
> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-5 は `docs/todo/completed.md` を参照
> **コードレビューログ**: `docs/todo/review-log.md` を参照

---

## Phase 6: 高度な型機能

### P6-1: 高カインド型 (HKT)
- [x] `Type` に kind (種) の概念追加 -- Kind enum (Star, Arrow) 定義
- [x] kind 推論の実装
- [x] 型コンストラクタに対するトレイト (`Functor`, `Monad` 等)
- [x] Kind 整合性チェック (NC-12) -- テスト 3 個追加

### P6-2: GADT
- [x] Algorithm W → バイディレクショナル型チェックへの移行
- [x] `:gadt` キーワードとバリアント別の戻り型指定構文
- [x] パターンマッチでの型の絞り込み -- テスト 7 個

### P6-3: Computation Expressions
- [x] `computation-builder` 宣言のパース
- [x] `let!` 構文糖衣の脱糖パス
- [x] `return` キーワードの文脈依存解決 -- テスト 6 個

### P6-4: ネストモジュール
- [x] モジュール内モジュール宣言
- [x] ネストされた名前空間の解決

---

## 未完了タスク

### 高優先度

| ID | 内容 | 状態 |
|----|------|------|
| P3-3 | `:invariant` の実行評価 (構造チェックのみ実装済み) | `[~]` Wasm 実行パイプライン必要 |
| P3-3 | `:example` の実行評価 (構造チェックのみ実装済み) | `[~]` Wasm 実行パイプライン必要 |

### 完了済み (今回対応)

| ID | 内容 | 対応内容 |
|----|------|---------|
| R-M5 | FieldAccess の型解決 | `infer_expr_type_name()` で型指向解決、ユニットテスト 3 個 |
| NC-8 | パーサーのエラーリカバリ | `parse_program_recovering()` + `ParseError::Multiple`、テスト 4 個 |
| NC-9 | 制約階層の互換性チェック | 実装済み確認 (`check_constraint_compatibility` + テスト 6 個) |
| NC-10 | config.rs のエラーハンドリング | `ConfigError` 型 + `validate_config()`、テスト 10 個 |
| NC-11 | 正規表現エンジンのパフォーマンス | thread_local ステップ制限 (100K回)、テスト 2 個 |
| R-m2 | `run_wasm_wasi` ヘルパー統合 | `wasi_runner.rs` に抽出、3 箇所統合 |
| R-m3 | RecordUpdate の型推定改善 | `infer_expr_type_name(base)` で型名取得、テスト 1 個 |
| R-m8 | `parse_test_output` 重複解消 | ループ前キャッシュ化 |
| R-m9 | コンストラクタパターン比較条件 | `I64Const(tag) + I64Eq` を If 前に発行、テスト 1 個 |
| R-S2 | 型推論結果の HashMap 化 | 既に `HashMap<String, Type>` で実装済み確認 |
| R-S7 | TODO.md 制限事項の明記 | 既知の制限事項セクション追加 |

### 任意の改善提案 (Suggestion)

| ID | 内容 | 対象 | 備考 |
|----|------|------|------|
| R-S1 | エラー型の統一 (`thiserror`) | 全クレート | 大規模リファクタリング |
| R-S3 | WasmGC feature flag 導入 | wasm | アーキテクチャ設計変更 |
| R-S4 | snapshot テストの活用拡大 | wasm | 中規模 |
| R-S5 | ベンチマーク追加 (`criterion`) | 全体 | 新規ツール導入 |
| R-S6 | `string_data` の RefCell 見直し | `lower.rs` | 構造変更 |
| R-S8 | ドキュメントと実装の整合性検証 | 全体 | 新規機能 |

---

## テスト検証債務

### 高優先度 (実行未検証)

| 機能 | 現状 | 必要なテスト |
|------|------|-------------|
| `:invariant` 自動実行 | 構造チェックのみ | 式の実行・論理的正当性評価 |
| `:example` 自動実行 | 構造チェックのみ | 式の実際の実行・結果検証 |

### 中優先度 (wasmtime GC 有効化待ち)

| 機能 | 現状 | 必要なテスト |
|------|------|-------------|
| レコード型リテラル + アクセス | コンパイルのみ | GC 実行テスト |
| レコード更新 | コンパイルのみ | GC 実行テスト |
| ADT コンストラクタ (GC) | コンパイルのみ | GC 実行テスト |
| モジュール import 実行 | テストなし | multi-file E2E テスト |

---

## 既知の制限事項

### MVP i64 フォールバック
- レコード型・ADT は WasmGC struct ではなく i64 フォールバックで実装
- wasmtime が WasmGC を安定サポートするまでの暫定措置
- `StructNew`, `StructGet`, `StructSet` 命令は IR に発行されるが、実行時は i64 で代替

### Lambda (クロージャ)
- 自由変数キャプチャは未実装 (ローカル関数として lowering)
- クロージャ変換 (lambda lifting) は将来課題

### パターンマッチ
- ネストしたコンストラクタパターンは未対応
- ワイルドカードパターン `_` のみ対応、ガード条件は未実装

### 正規表現エンジン
- NFA → DFA 変換による最適化は未実装 (ステップ制限で病的入力を防止)
- Unicode 文字クラス (`\p{L}` 等) は未対応 (char ベースの基本 Unicode は動作)

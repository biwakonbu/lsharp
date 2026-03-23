# TODO 未完了タスク並列実装 - 設計書

> 最終更新: 2026-03-23

## 概要

既存の L# コンパイラパイプライン (Lexer -> Parser -> TypeInfer -> IR -> Wasm) を基盤として、12 タスクを 3 フェーズに分けて実装する。各タスクは既存クレートへの機能追加またはリファクタリングであり、新規クレートの追加は不要。

## アーキテクチャ

```
フェーズ 1 (並列 8 タスク)
  +-- グループ A: LSP / stdlib / エラー統一 / RefCell (4 並列)
  +-- グループ B: P8-1~P8-4 比較テスト (4 並列)
  |
フェーズ 2 (2 タスク)
  +-- P2-3 HashMap 文字列キー
  +-- P3-3 invariant/example 実行
  |
フェーズ 3 (1 タスク)
  +-- P8-5 stage1 統合
```

フェーズ 1 内のタスクは全て独立しており、相互依存なく並列実行可能。フェーズ 2 のタスクも互いに独立だが、フェーズ 1 完了後に着手する。フェーズ 3 はコンパイラ制限解消を含む最大工数タスクで、フェーズ 2 完了後に着手する。

## コンポーネント

### LSP 定義ジャンプ (P9-2)

- **対象ファイル**: `crates/lsharp-lsp/src/lib.rs`
- **責務**: `textDocument/definition` リクエストを処理し、シンボルの定義位置を返す
- **設計**:
  1. `ServerCapabilities` に `definition_provider: Some(OneOf::Left(true))` を追加
  2. `GotoDefinition` リクエストハンドラを実装
  3. シンボルテーブル (型推論結果の TypeEnv) からシンボル定義位置を検索
  4. URI -> ソーステキストの管理には `HashMap<Url, String>` をサーバ状態に追加
- **インターフェース**: LSP プロトコル `textDocument/definition`
- **依存**: `lsharp-syntax` (パーサー), `lsharp-types` (TypeEnv)

### stdlib テスト自動化 (P7)

- **対象ファイル**: `crates/lsharp-wasm/tests/e2e.rs`
- **責務**: stdlib/ 全 9 モジュールの E2E テスト追加
- **設計**:
  1. 既存の Char/Debug/Set テスト (3 件) をベースにパターン化
  2. 残り 6 モジュール (Core, IO, List, Map, String, Vector) のテスト追加
  3. 各モジュールの主要関数を compile -> run -> assert するテストケース
- **テスト命名**: `test_e2e_stdlib_{module_name}_{function}`

### エラー型統一 (R-S1)

- **対象**: `crates/lsharp-driver/src/error.rs`
- **責務**: 全クレートのエラー型を統一 enum でラッピング
- **設計**:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum LsharpError {
      #[error(transparent)]
      Lex(#[from] lsharp_syntax::LexError),
      #[error(transparent)]
      Parse(#[from] lsharp_syntax::ParseError),
      #[error(transparent)]
      Infer(#[from] lsharp_types::InferError),
      #[error(transparent)]
      Constraint(#[from] lsharp_types::ConstraintError),
      #[error(transparent)]
      Lower(#[from] lsharp_ir::LowerError),
      #[error(transparent)]
      Codegen(#[from] lsharp_wasm::CodegenError),
      #[error(transparent)]
      ModuleGraph(#[from] lsharp_ir::ModuleGraphError),
  }
  ```
- **移行方針**: ドライバ層で `Result<T, LsharpError>` を使用、各クレート内部は既存エラー型を維持

### RefCell 見直し (R-S6)

- **対象ファイル**: `crates/lsharp-ir/src/lower/mod.rs`
- **責務**: `string_data`, `lifted_functions`, `lifted_func_indices` の RefCell を除去
- **設計**:
  1. Lower 構造体のメソッドシグネチャを `&self` -> `&mut self` に変更
  2. RefCell を通常の `Vec` / `HashMap` に置き換え
  3. 呼び出し側を `&mut self` に合わせて修正

### セルフホスト比較テスト (P8-1 ~ P8-4)

- **対象ファイル**: `crates/lsharp-wasm/tests/e2e.rs`
- **責務**: L# セルフホスト版と Rust 版の出力一致を検証
- **設計**:
  1. テスト入力データを定義 (L# ソースコード片)
  2. Rust 版パイプラインで処理 -> 期待出力を取得
  3. L# セルフホスト版 (Wasm 実行) で同じ入力を処理 -> 実際出力を取得
  4. 両者の出力を比較
- **テスト命名**: `test_e2e_selfhost_{module}_comparison_{case}`
- **補足**: 出力フォーマットの正規化が必要な場合がある

### HashMap 文字列キー (P2-3)

- **対象ファイル**: `crates/lsharp-ir/src/lower/expr.rs`, `crates/lsharp-wasm/src/wasi.rs`
- **責務**: FNV-1a ハッシュによる文字列キーのハッシュマップ
- **設計**:
  1. FNV-1a ハッシュ関数を Wasm ヘルパー関数として実装
     - `$fnv1a_hash(offset: i32, len: i32) -> i32`
     - data section 上の文字列バイト列をハッシュ化
  2. `map-insert` / `map-get` / `map-contains?` / `map-remove` を文字列キーに対応
  3. 文字列比較ヘルパー `$str_eq(off1: i32, len1: i32, off2: i32, len2: i32) -> i32`
  4. 衝突解決: オープンアドレス法 (線形探索、既存実装を拡張)

### invariant/example 実行評価 (P3-3)

- **対象ファイル**: `crates/lsharp-types/src/metadata_check.rs`, `crates/lsharp-driver/src/lib.rs`
- **責務**: テスト生成結果を実際に Wasm 実行して検証
- **設計**:
  1. `generate_tests()` の出力 (テスト式) をフルパイプライン (parse -> infer -> lower -> codegen) で処理
  2. wasmtime で実行し、:example の期待値との一致を検証
  3. :invariant の述語が true を返すことを検証
  4. `cargo run -- test <file>` コマンドで結果をレポート

### stage1 統合 (P8-5) - 次イテレーション

- **対象**: `crates/lsharp-ir/src/linker.rs`, コンパイラ本体
- **責務**: 9 モジュールを結合して stage1 コンパイラを構築
- **設計**:
  1. 前提: 相互再帰関数の前方参照対応 (Parser.ls, TypeScheme.ls 向け)
  2. 前提: 深いネスト if 式のパーサー修正 (Lexer.ls 向け)
  3. モジュールリンカーで 9 モジュールを 1 つの Wasm バイナリに結合
  4. stage1 コンパイラが簡単な L# プログラムをコンパイルできることを検証
- **リスク**: コンパイラ制限の解消が最も困難。工数が想定を超える可能性がある

## データ設計

### LSP シンボルテーブル

```rust
struct SymbolInfo {
    name: String,
    location: Location,  // ファイル URI + Range
    kind: SymbolKind,     // Function, Variable, Type, Module
}

struct ServerState {
    documents: HashMap<Url, String>,
    symbols: HashMap<String, Vec<SymbolInfo>>,
}
```

### 統一エラー型

各クレートの既存エラー型はそのまま維持。ドライバ層で From トレイトによる自動変換を利用。

### FNV-1a ハッシュ (Wasm 内部)

```
FNV_OFFSET_BASIS = 2166136261 (0x811c9dc5)
FNV_PRIME = 16777619 (0x01000193)

hash = FNV_OFFSET_BASIS
for each byte in string:
    hash = hash XOR byte
    hash = hash * FNV_PRIME
return hash
```

## テスト戦略

### ユニットテスト

| タスク | テスト配置 | テスト数 |
|--------|-----------|---------|
| P9-2 LSP | `crates/lsharp-lsp/src/lib.rs` | 4 |
| R-S1 エラー統一 | `crates/lsharp-driver/src/error.rs` | 7 |
| R-S6 RefCell | `crates/lsharp-ir/src/lower/mod.rs` 既存テスト維持 | 0 (回帰確認) |
| P2-3 HashMap | `crates/lsharp-ir/src/lower/expr.rs` | 4 |

### E2E テスト

| タスク | テスト配置 | テスト数 |
|--------|-----------|---------|
| P7 stdlib | `crates/lsharp-wasm/tests/e2e.rs` | 7 |
| P8-1~P8-4 比較 | `crates/lsharp-wasm/tests/e2e.rs` | 6 |
| P3-3 invariant | `crates/lsharp-wasm/tests/e2e.rs` | 4 |

### 回帰テスト

- 全タスク実装前後で `cargo test` がフルパス (650 件)
- `cargo clippy` で新規警告なし

## エラーハンドリング

- 統一エラー型 `LsharpError` でパイプライン全体のエラーをキャッチ
- LSP はエラー時に diagnostics としてクライアントに通知 (既存パターン踏襲)
- invariant/example 実行失敗時は、テスト結果レポートに失敗理由を表示

## コミット戦略

- フェーズ 1 のグループ A 各タスク完了時に個別 commit (4 commits)
- フェーズ 1 のグループ B をまとめて 1 commit (比較テスト 4 件)
- フェーズ 2 の各タスク完了時に個別 commit (2 commits)
- フェーズ 3 は段階的に commit (制限解消 + 統合で 2-3 commits)
- 合計: 9-10 commits 想定

## 未決定事項

- P8-5 のコンパイラ制限解消の具体的アプローチ (前方参照: 2 パス方式 vs 遅延解決)
- stage2 コンパイラの設計 (本スコープ外)

## 関連ドキュメント

- [要件定義書](./requirements.md)

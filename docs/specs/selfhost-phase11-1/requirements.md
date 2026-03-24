# selfhost-phase11-1 - 要件定義書

> 最終更新: 2026-03-25

## 概要

L# セルフホスティング・コンパイラの完成と段階的な Rust 撤去に向けて、Phase 11 の第 1 段階として以下を実施した。

1. P11-1 系ドキュメント監査タスクの完了（差分判定規則・受け入れ基準の明文化）
2. `selfhost/MacroExpand.ls` の新規実装（マクロ展開エンジン）
3. `selfhost/TypeInfer.ls` の新規実装（Hindley-Milner 型推論コア）
4. `selfhost/Main.ls` の再構成（ファイルサイズ制限内への削減）
5. `stage1.wasm -> stage2.wasm` の固定点検証 E2E テスト追加

## 機能要件

### P11-1 系ドキュメント監査

- **FR-001**: 「Rust 完全撤去の完了条件」を `bootstrap oracle` / `legacy reference` / `native release` の 3 条件で明確に定義する
- **FR-002**: `docs/compatibility-matrix.md` に、Phase 11 完了まで PR ごとの更新が必須である旨を明記する
- **FR-003**: 差分を `仕様差分 / 実装欠落 / 出力差分 / 性能差分 / 運用差分` の 5 種に分類する規則を明文化する
- **FR-004**: 完了表示の各項目に一次エビデンス（テスト名 / ADR / ファイルパス）を紐付ける規則を明文化する

### MacroExpand.ls 実装

- **FR-010**: `selfhost/MacroExpand.ls` を新規作成し、マクロ展開エンジンを L# で実装する
- **FR-011**: `defmacro` によるマクロ登録機能を実装する
- **FR-012**: `~param` 置換（単一値スプライス）を実装する
- **FR-013**: `~@param` splice（リスト展開）を実装する
- **FR-014**: 再帰展開の制限（最大 128 回）を実装する
- **FR-015**: ファイルサイズ制限 500〜800 行を遵守する
- **FR-016**: gensym / 衛生マクロは v1 では ASCII カウンタ方式（`__gen0`, `__gen1`, ...）で実装する（完全衛生は後回し）
- **FR-017**: 対応する E2E テストを 1 件以上追加する

### TypeInfer.ls 実装

- **FR-020**: `selfhost/TypeInfer.ls` を新規作成し、Hindley-Milner 型推論コアを L# で実装する
- **FR-021**: 型変数生成（fresh type variable）機能を実装する
- **FR-022**: 単一化（unification, `unify`）アルゴリズムを実装する
- **FR-023**: 型環境管理（`TypeEnv`）を実装し、変数→型スキームの写像を管理する
- **FR-024**: `TypeScheme.ls` の `instantiate` / `generalize` を再利用する（新規実装しない）
- **FR-025**: v1 では HM コアのみ実装する。トレイト制約・レコード型の型推論は後回しとする
- **FR-026**: ファイルサイズ制限 500〜800 行を遵守する
- **FR-027**: 対応する E2E テストを 1 件以上追加する

### Main.ls 再構成

- **FR-030**: `Main.ls` を再構成し、800 行以内に収める
- **FR-031**: Lexer / Parser / MacroExpand / TypeInfer / Compiler / WasmEmit の各モジュールを接続する
- **FR-032**: `compile-selfhost-wasm` エントリポイントを整備し、Wasm 出力経路を明確にする
- **FR-033**: 変更後も既存 E2E テストが全通過すること

### 固定点検証 E2E テスト

- **FR-040**: `stage1.wasm` が L# ソースをコンパイルして `stage2.wasm` を生成する E2E テストを追加する
- **FR-041**: テスト名を `test_e2e_bootstrap_stage1_to_stage2` として `crates/lsharp-wasm/tests/e2e.rs` に追加する
- **FR-042**: 生成された `stage2.wasm` のバイト列が決定的であること（2 回実行で一致）を検証する

## 非機能要件

### ファイルサイズ制限

- **NFR-SIZE-001**: `selfhost/` 配下の全 `.ls` ファイルは 500〜800 行以内とする
- **NFR-SIZE-002**: `MacroExpand.ls` と `TypeInfer.ls` は実装前に分割方針を決定し、上限を超えない構成で設計する

### TDD 遵守

- **NFR-TDD-001**: 実装ファイルを編集する前にテストを先に書き、RED -> GREEN -> REFACTOR の順で進める
- **NFR-TDD-002**: 新規実装ファイルには E2E テストを 1 件以上追加する
- **NFR-TDD-003**: テスト 0 件の TODO 項目は完了扱いにしない

### コード規約

- **NFR-CONV-001**: selfhost コードの関数名はケバブケース（例: `macro-expand`, `type-infer`）とする
- **NFR-CONV-002**: 変数・関数名は英語（国際標準）、コメントは日本語とする
- **NFR-CONV-003**: 既存の `vector-new / vector-push / ref-new / ref-get / ref-set` パターンを継承する

### 後方互換性

- **NFR-COMPAT-001**: 既存の E2E テストが全通過することを維持する
- **NFR-COMPAT-002**: `cargo test` 全通過を各実装ステップで確認してから次のステップに進む

### 保守性

- **NFR-MAINT-001**: 各完了タスクに `TODO.md` の更新とエビデンス（テスト名・ファイルパス）を紐付ける
- **NFR-MAINT-002**: 完了できないタスクは進捗を記録し、次フェーズへ引き継ぐ

## 受入条件

- **AC-001**: P11-1 系タスク 10 件が完了扱いに更新されていること
- **AC-002**: `docs/compatibility-matrix.md` に PR 更新ルールが追記されていること
- **AC-003**: 差分判定 5 種の定義が記述されていること
- **AC-010**: `(defmacro my-when [c body] (if c body nil))` 相当のマクロが展開できること
- **AC-011**: 再帰制限を超えた場合にエラーを返すこと
- **AC-012**: `test_e2e_selfhost_macro_expand_basic` が通過すること
- **AC-013**: `test_e2e_selfhost_macro_expand_recursion_limit` が通過すること
- **AC-014**: `MacroExpand.ls` が 800 行以内（実績: 313 行）であること
- **AC-020**: `(let x 42) x` 相当の let 多相が推論できること
- **AC-021**: 型が合わない場合にエラーを返すこと
- **AC-022**: `test_e2e_selfhost_type_infer_basic` が通過すること
- **AC-023**: `test_e2e_selfhost_type_infer_let_poly` が通過すること
- **AC-024**: `test_e2e_selfhost_type_infer_error` が通過すること
- **AC-025**: `TypeInfer.ls` が 800 行以内（実績: 526 行）であること
- **AC-030**: `Main.ls` が 800 行以内（実績: 795 行）であること
- **AC-031**: selfhost 系 E2E テストが全通過すること
- **AC-032**: `compile-selfhost-wasm` エントリポイントが定義されていること
- **AC-040**: `test_e2e_bootstrap_stage1_to_stage2` が通過すること
- **AC-041**: `stage2.wasm` の決定性（2 回実行で同一バイト列）が確認されること

## 制約条件

- **CON-001**: `MacroExpand.ls` の移植元（`macro_expand.rs`）は 1614 行。L# への移植は 500〜800 行に圧縮する必要がある
- **CON-002**: `TypeInfer.ls` の移植元（`infer.rs`）は 3376 行。複数ファイルへの分割が必要な場合がある
- **CON-003**: selfhost 言語はループを末尾再帰でしか表現できない。複雑な展開ロジックは再帰で実装する
- **CON-004**: `WasmEmit.ls` の Code セクションは最大 8 命令のネスト if で実装されており、ループ制限がある

## 除外事項（v1 スコープ外）

以下は本フェーズのスコープ外とする。次フェーズ（P11-2b 以降）で対応する。

- Native backend（P11-2b）のコード実装
- P11-2c ランタイム接続のコード実装
- P11-3〜P11-6 のコード実装
- 衛生マクロの完全実装（gensym のスコープ追跡）
- トレイト制約・レコード型の型推論（`TypeInfer.ls` v1 は HM コアのみ）
- Windows 向け Native backend

## 既知の制限

| 制限 | 内容 | 優先度 |
|------|------|--------|
| ISSUE-001 | `TypeInfer.ls` の多相型は selfhost 環境内での完全動作が未検証 | 低（v1 許容） |
| ISSUE-002 | `MacroExpand.ls` の `macro-substitute` は 4 引数以上で動作未確認 | 低（v1 許容） |

## 達成結果

本フェーズの完了時点での実績:

| 指標 | 値 |
|------|-----|
| テスト総件数 | 280 件（新規 6 件追加） |
| テスト失敗 | 0 件 |
| 静的解析エラー | 0 件 |
| 達成度 | 87%（P11-1 スコープは 100%） |
| コミット | a6920bc |

## 関連ドキュメント

- [設計書](./design.md)
- [互換性マトリクス](../../compatibility-matrix.md)

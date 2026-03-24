# selfhost-phase11-1 - 設計書

> 最終更新: 2026-03-25

## 概要

L# セルフホスティング・コンパイラ Phase 11-1 の設計方針を記述する。
`selfhost/` ディレクトリに `MacroExpand.ls` と `TypeInfer.ls` を追加し、
コンパイラパイプラインをマクロ展開・型推論まで接続する。

## アーキテクチャ

### selfhost/ ディレクトリ構成

```
selfhost/
  Token.ls        -- トークン定数 (72 行)
  AST.ls          -- AST 定義 (213 行)
  IR.ls           -- IR 定義 (63 行)
  Lexer.ls        -- 字句解析 (286 行)
  Parser.ls       -- 構文解析 (477 行)
  MacroExpand.ls  -- マクロ展開 (313 行) [Phase 11-1 新規]
  TypeScheme.ls   -- 型スキーム (192 行)
  TypeInfer.ls    -- HM 型推論 (526 行) [Phase 11-1 新規]
  Compiler.ls     -- AST -> IR 変換 (550 行)
  WasmEmit.ls     -- Wasm バイナリ生成 (698 行)
  Main.ls         -- パイプライン統合 (795 行)
  Linter.ls       -- リンター (310 行, PoC)
  Formatter.ls    -- フォーマッタ (170 行, PoC)
  JsonRpc.ls      -- JSON-RPC/LSP (171 行, PoC)
```

### コンパイラパイプライン

```
Source (.ls)
  -> Lexer          -- 字句解析
  -> Parser         -- 構文解析
  -> MacroExpand    -- マクロ展開 [Phase 11-1 新規]
  -> TypeInfer      -- 型推論 [Phase 11-1 新規]
  -> Compiler       -- AST -> IR 変換
  -> WasmEmit       -- IR -> .wasm
```

## コンポーネント

### P11-1 系ドキュメント更新

#### 差分判定規則（5 種）

Phase 11 完了判定に使用する差分分類:

| 分類 | 定義 | 確認方法 |
|------|------|---------|
| 仕様差分 | TODO/README/book の記述と実装が不一致 | 手動レビュー |
| 実装欠落 | コードが存在しない、または PoC のみ | ファイル確認 |
| 出力差分 | テスト期待値と実際の出力が不一致 | `cargo test` |
| 性能差分 | ベンチマーク回帰（Phase 11 の blocking 条件には含めない） | ベンチマーク |
| 運用差分 | CI, 配布, 署名, VSCode 連携, インストール手順の不一致 | CI ログ確認 |

#### 受け入れ基準（P11-1d）

完了表示に必要な 4 項目:

1. 完了表示の各項目に一次エビデンスを紐付ける（テスト名 / ADR / ファイルパス のいずれか必須）
2. README と book の導入手順が現行 mainline で再現できることを smoke test で確認する
3. `TODO.md` の Phase 11 各完了条件が具体的なテスト / ドキュメント / CI gate に接続されている
4. 監査完了後は「Rust 完全撤去」の曖昧な用語を禁止し、定義済み語彙（`bootstrap oracle`, `legacy reference`, `native release`）へ統一する

### MacroExpand.ls

#### 責務

- `defmacro` 宣言を処理し、マクロ登録テーブルに格納する
- S 式 AST を走査し、登録済みマクロを再帰的に展開する
- 展開回数を追跡し、再帰制限（128 回）を超えたらエラーを返す
- gensym を ASCII カウンタ方式（`__gen0`, `__gen1`, ...）で実装する

#### データ構造

```lisp
;; マクロテーブル: name -> (params . body)
;; (vector-new 16) で 16 エントリのテーブルを確保
(defn macro-table-new [] (vector-new 16))

;; エントリ: (name-hash, params-count, params-vec, body-ast)
```

#### 主要関数

| 関数名 | 責務 |
|--------|------|
| `macro-table-new` | マクロ登録テーブルを初期化 |
| `macro-table-register` | `defmacro` をテーブルに登録 |
| `macro-table-lookup` | 名前ハッシュで検索、見つからなければ -1 |
| `macro-expand-once` | 1 ステップのマクロ展開 |
| `macro-expand` | 再帰制限付きの完全展開 |
| `macro-substitute` | `~param` / `~@param` のテンプレート置換 |
| `gensym-next` | ASCII カウンタ方式で新しいシンボル名を生成 |

#### 公開 API

```lisp
;; マクロテーブル作成
(defn macro-table-new [] ...)

;; defmacro 登録: table name params body -> table (更新後)
(defn macro-register [table name-str params body] ...)

;; 完全展開: table ast limit -> ast (展開済み) または エラー値
(defn macro-expand [table ast limit] ...)
```

#### エラー処理

| エラー種別 | 発生条件 | 対応 |
|-----------|---------|------|
| recursion-limit-exceeded | 展開回数 > 128 | エラー値を返して展開を中止 |
| unknown-macro | 未登録マクロの使用 | そのまま通過させる（エラーにしない） |
| arity-mismatch | パラメータ数不一致 | エラー値を返す |

### TypeInfer.ls

#### 責務

- 型変数の生成（`fresh-type-var`）と管理
- Hindley-Milner 単一化（`unify`）による型制約の解決
- 型環境（`TypeEnv`）による変数名 -> 型スキームの写像
- `TypeScheme.ls` の `instantiate` / `generalize` を呼び出して let 多相を実現する

#### 型表現

| タグ | 型 | 表現 |
|------|----|----|
| 0 | Int | `(type-int)` |
| 1 | Bool | `(type-bool)` |
| 2 | String | `(type-string)` |
| 3 | Fun | `(type-fun arg-type ret-type)` |
| 4 | Var | `(type-var id)` |

#### 主要関数

| 関数名 | 責務 |
|--------|------|
| `type-env-new` | 型環境を初期化 |
| `type-env-extend` | 変数->型スキームのバインディングを追加 |
| `type-env-lookup` | 名前ハッシュで型スキームを検索 |
| `subst-new` | 代入環境を初期化 |
| `subst-apply` | 代入を型に適用 |
| `unify` | 2 つの型を単一化し、代入を返す |
| `infer-expr` | 式の型推論メインルーティン |
| `infer-lit` | リテラルの型推論 |
| `infer-var` | 変数参照の型推論（環境から lookup, instantiate） |
| `infer-app` | 関数適用の型推論 |
| `infer-lam` | ラムダ式の型推論 |
| `infer-let` | let 式の型推論（generalize して多相化） |

#### 公開 API

```lisp
;; 型環境作成
(defn type-env-new [] ...)

;; 型推論: env expr counter -> (type, subst, counter)
(defn infer-expr [env expr counter] ...)

;; 単一化: ty1 ty2 subst -> subst (更新後) または エラー値
(defn unify [ty1 ty2 subst] ...)
```

#### TypeScheme.ls との連携

```lisp
;; 再利用する TypeScheme.ls の関数
;; (instantiate scheme counter) -> 型変数を新鮮なものに置換
;; (generalize env ty) -> 自由変数を全称量化
```

#### エラー処理

| エラー種別 | 発生条件 | 対応 |
|-----------|---------|------|
| unification-failure | 型が一致しない | エラー値を返す |
| unbound-variable | 型環境に変数が存在しない | エラー値を返す |
| infinite-type | occurs check 失敗 | エラー値を返す |

### Main.ls 再構成

#### 変更内容

- 812 行から 795 行に削減（重複関数整理）
- `compile-selfhost-wasm` エントリポイントを 711 行目で明確に定義
- MacroExpand.ls / TypeInfer.ls をパイプラインに接続

#### エントリポイント

```lisp
;; Wasm コンパイル経路
(defn compile-selfhost-wasm [source-str] ...)
;; -> wasm-bytes vector または エラー値
;; pipeline: lex -> parse -> macro-expand -> type-infer -> compile -> emit
```

## データ設計

### MacroExpand.ls のデータフロー

```
defmacro 宣言
  -> macro-table-register
  -> (name-hash, params, body) をテーブルに格納

S 式 AST
  -> macro-expand
    -> macro-table-lookup (名前ハッシュで検索)
    -> macro-substitute (テンプレート置換)
    -> macro-expand (再帰展開, 最大 128 回)
  -> 展開済み S 式 AST
```

### TypeInfer.ls のデータフロー

```
式 AST + 型環境
  -> infer-expr
    -> infer-lit / infer-var / infer-app / infer-lam / infer-let
    -> unify (型制約の解決, 代入環境を更新)
    -> subst-apply (代入を型に適用)
  -> 型 + 更新済み代入環境

let 式:
  -> infer-expr (body)
  -> generalize (型環境で自由変数を全称量化)
  -> type-env-extend (多相型スキームを環境に追加)
```

### エラー値の共通表現

```lisp
;; エラー値: (vector-new 2) で [tag, message-hash] を保持
;; tag 0 = 成功, tag 1 = エラー
(defn make-error [message-hash] ...)
(defn is-error [val] ...)
```

## テスト戦略

### TDD フロー

1. RED: テストを `crates/lsharp-wasm/tests/e2e.rs` に追加 -> `cargo test` で失敗を確認
2. GREEN: `selfhost/MacroExpand.ls` または `selfhost/TypeInfer.ls` を実装 -> `cargo test` で成功を確認
3. REFACTOR: リファクタリング -> テスト全通過を維持
4. UPDATE: `TODO.md` の該当タスクを完了に更新し、テスト名をエビデンスとして注記

### 追加した E2E テスト

| テスト名 | 対象 | 結果 |
|---------|------|------|
| `test_e2e_selfhost_macro_expand_basic` | MacroExpand.ls 基本展開 | Pass |
| `test_e2e_selfhost_macro_expand_recursion_limit` | 再帰制限 | Pass |
| `test_e2e_selfhost_type_infer_basic` | TypeInfer.ls 基本型推論 | Pass |
| `test_e2e_selfhost_type_infer_let_poly` | let 多相 | Pass |
| `test_e2e_selfhost_type_infer_error` | 型エラー検出 | Pass |
| `test_e2e_bootstrap_stage1_to_stage2` | stage1->stage2 生成 | Pass |

### 既存テストの保護

- 全 280 件の E2E テストが各ステップ後も通過すること
- selfhost 系テストが特に重要

## 次フェーズへの引き継ぎ事項

### P11-2b: Native backend 設計方針

- Native IR を新設せず、既存 Lowered IR から `NativeInstr` へ 1 段で落とす
- calling convention: 「L# 関数内部 ABI」と「外部ランタイム ABI」を分離
- レジスタ割付: v1 は linear-scan に固定、spill は stack slot へ
- 対象ターゲット: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu` の 3 ターゲット
- 出力形式: object file + 最小ランタイム + system linker 呼び出し（Mach-O/ELF 直書きは後回し）
- 実装ファイル: `selfhost/NativeEmit.ls`（P11-2b 着手時に新規作成）

### P11-2c: ランタイム接続 API

対象 API:

```
alloc / print / read_file / write_file / file_exists / clock_now_millis
```

- 値表現: `LsharpWord`（immediate と heap pointer のタグ付き word 表現）
- Wasm/native で共通化
- GC 導入前は bump allocator で実装し、GC 導入後に差し替える

### P11-2d: 固定点検証の発展

- `stage1.wasm -> stage2.wasm -> stage3.wasm` の 3 段比較を正本とする（P11-2b 完了後）
- 比較点: raw wasm bytes, exported symbol list, data section bytes, compiler diagnostics

### P11-3〜P11-6: 実施順序

| フェーズ | 内容 |
|---------|------|
| P11-3a | syntax parity（span/token/lexer/parser/macro_expand の L# 移植） |
| P11-3b | types parity（HM 推論, 制約, metadata check の L# 移植） |
| P11-3c | IR parity（module graph, closure conversion の L# 移植） |
| P11-3d | backend parity（Wasm backend 完全化） |
| P11-4 | ツールチェイン parity（CLI 13 コマンド, LSP 10 メソッド） |
| P11-5 | ランタイム安定化（GC 導入） |
| P11-6 | CI 切替・Rust 撤去 |

### 未決定事項

次フェーズで検討が必要な事項:

- Native backend の命令選択（x86_64/aarch64 の ISA 差分吸収方針の詳細）
- GC safe point の具体的な実装（`root_push` / `root_pop` の最適化）
- VSCode 拡張のネイティブ LSP バイナリ統合のタイムライン
- Windows x86_64 サポートの優先度（P11-4 以降に判断）

## 関連ドキュメント

- [要件定義書](./requirements.md)
- [互換性マトリクス](../../compatibility-matrix.md)
- 実装ファイル: `selfhost/MacroExpand.ls`
- 実装ファイル: `selfhost/TypeInfer.ls`
- 実装ファイル: `selfhost/Main.ls`
- テストファイル: `crates/lsharp-wasm/tests/e2e.rs`

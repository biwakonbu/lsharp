# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P9-1/3/4 は完了。詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-122) を参照

---

## Phase 1: 文字列操作

### P1-2: 文字列リテラルのヒープ化
- [x] data section offset → ヒープ上 String オブジェクト (tag=1, len, bytes) への変換 -- E2E 8件追加、既存 string 15件 + stdlib/selfhost/map 20件パス
- [x] 既存の文字列関連テストが引き続きパスすることを確認 -- 全 199 E2E テストパス

### P1-3: WASI ファイル I/O & 標準入出力 (P9-6 前提)
- [ ] fd_read / fd_write の WASI syscall ラッパー (stdin/stdout/stderr)
- [ ] fd_open / fd_close / fd_seek のファイル操作
- [ ] パス操作ユーティリティ (L# stdlib)
- [ ] JSON パーサー (L# stdlib) -- LSP プロトコルに必要

---

## Phase 2: 動的コレクション

### P2-3: ハッシュマップ
- [x] FNV-1a + Open Addressing HashMap -- 整数キー・文字列キー両対応、insert/get/contains/remove/overwrite、E2E テスト 15件 + stdlib E2E 3件

---

## Phase 7: 標準ライブラリ (L# で記述)

- [x] stdlib のコンパイル・テスト自動化 -- E2E 10件追加 (Char/Debug/Set + IO 2件 + Map 3件 + Vector 3件)

---

## Phase 8: セルフホスティング

### ブートストラップ戦略
> 最小サブセットで開始: `let` / 再帰 / `if` / `match` / ADT / Record / モジュール
> HKT/GADT/トレイト制約等の高度機能はセルフホスト後に段階追加

### P8-1: L# で Lexer を実装
- [x] Token ADT 定義 -- selfhost/Token.ls (整数タグ方式)
- [x] 文字列走査による字句解析 -- selfhost/Lexer.ls (tokenize/lex-one/classify-symbol)
- [x] Rust 版 lexer との出力比較テスト -- E2E テスト 3件パス (基本トークナイズ + 比較テスト 2件)

### P8-2: L# で Parser を実装
- [x] AST の ADT 定義 -- selfhost/AST.ls (整数タグ + Vector 方式)
- [x] 再帰降下パーサー -- selfhost/Parser.ls (parse-expr/parse-sexp)
- [x] Rust 版 parser との出力比較テスト -- E2E テスト 3件パス (基本 S 式パース + 比較テスト 2件)

### P8-3: L# で型推論を実装
- [x] 型 ADT (Con, Var, Fun) 定義 -- selfhost/Type.ls (整数タグ + Vector)
- [x] Substitution (HashMap ベース) -- subst-new/bind/lookup 実装
- [x] Unification アルゴリズム -- unify-simple (Con/Var), occurs-check, E2E テスト 1件
- [x] let 多相 + 型注釈 -- selfhost/TypeScheme.ls (instantiate/generalize/free-vars), E2E 1件
- [x] Rust 版型推論との出力比較テスト -- E2E テスト 3件パス (型構築 + Substitution + Unification + 比較テスト 1件)

### P8-4: L# で IR Lowering + Codegen を実装
- [x] IR ADT 定義 -- selfhost/IR.ls (命令タグ + Vector)
- [x] AST → IR 変換 -- selfhost/Compiler.ls (compile-expr: lit/var/bool), E2E 1件
- [x] LEB128 エンコーディング -- selfhost/Compiler.ls (leb128-unsigned), E2E 1件
- [x] Wasm バイナリ生成 -- selfhost/WasmEmit.ls (ヘッダー/Type セクション/LEB128), E2E 1件
- [x] Rust 版 codegen との出力比較テスト -- E2E テスト 3件パス (IR 命令構築 + Compiler + 比較テスト 1件)

### P8-5: ブートストラップ検証
- [x] Rust 版 → stage1.wasm (L# コンパイラ) -- 個別モジュール E2E 9件 + Main.ls 統合パイプライン E2E 2件 (AST→IR→Wasm 統合検証)
- [ ] stage1.wasm → stage2.wasm (セルフコンパイル) -- 完全なセルフホストコンパイラ統合が前提
- [ ] stage1.wasm == stage2.wasm (固定点検証) -- stage2 生成が前提
- [ ] CI でのブートストラップ自動検証 -- 固定点検証が前提

---

## Phase 9: エコシステム

### P9-2: LSP
- [x] `crates/lsharp-lsp` クレート作成 -- tower-lsp 0.20, LsharpBackend 構造体
- [x] tower-lsp 統合 -- initialize/shutdown/did_open/did_change ハンドラ
- [x] エラー診断発行 -- parse_and_check で Diagnostic 化
- [x] 定義ジャンプ -- find_definition + ドキュメントキャッシュ + goto_definition ハンドラ接続、テスト 5件
- [x] 型ホバー -- find_type_at_position + hover ハンドラ接続、テスト 3件
- [x] completion 基本実装 -- キーワード 17種 + 関数名/変数名収集、テスト 5件
- [x] references / rename / formatting -- モジュール分割 (util/references/rename/format.rs) + ソースキャッシュ追加、ユニットテスト 23件追加 (計 27件)

### P9-6: VSCode 拡張 (L# ネイティブ)
> 全コアロジックを L# → Wasm で実装。VSCode 拡張シェルのみ TypeScript (最小限)
> 前提: P1-3 (WASI ファイル I/O) の完了

#### P9-6a: シンタックスハイライト
- [ ] L# トークナイザーベースのセマンティックハイライトエンジン (selfhost/Lexer.ls 拡張)
- [ ] TextMate grammar 生成 (L# から .tmLanguage.json を出力)
- [ ] VSCode 拡張シェル (TypeScript 最小限) + Wasm バインディング

#### P9-6b: LSP サーバー (L# 実装)
- [ ] JSON-RPC パーサー/シリアライザー (L# stdlib)
- [ ] LSP プロトコルハンドラ: initialize / textDocument/didOpen / didChange
- [ ] 診断発行 (parse エラー + 型エラー → LSP Diagnostic)
- [ ] 定義ジャンプ (selfhost/AST.ls + シンボルテーブル)
- [ ] 型ホバー (selfhost/Type.ls + TypeScheme.ls 活用)
- [ ] 補完 (シンボル補完 + キーワード補完)

#### P9-6c: リンター (L# 実装)
- [ ] AST ベースのリントルール基盤 (selfhost/AST.ls 拡張)
- [ ] 組み込みルール: 未使用変数、未使用 import、型注釈推奨
- [ ] カスタムルール定義 API
- [ ] LSP 統合 (diagnostics として報告)

#### P9-6d: フォーマッタ (L# 実装)
- [ ] AST プリティプリンタ (S 式の整形出力)
- [ ] インデント・改行ルール設定
- [ ] LSP textDocument/formatting ハンドラ統合
- [ ] CLI フォーマッタコマンド (`lsharp fmt`)

---

## Phase 10: マクロシステム (型付き衛生マクロ)

> Template Haskell + Typed Racket のハイブリッド。S式構文との親和性を活かし、
> Computation Expression の脱糖パターンを拡張する形で段階的に実装。
> パイプライン: Source → Lexer → Parser → AST → **MacroExpand** → Type Inference → Lowering → Wasm

### P10-1: Quote/Unquote 基盤
- [ ] Lexer: `'` (quote) `~` (unquote) `~@` (splice-unquote) トークン追加 (token.rs, lexer.rs)
- [ ] AST: `Expr::Quote`, `Expr::Unquote`, `Expr::UnquoteSplice` 追加 (ast.rs)
- [ ] Parser: quote/unquote 式のパース (parser.rs)

### P10-2: defmacro 定義と展開
- [ ] AST: `Decl::DefMacro { name, params, macro_type, body }` 追加 (ast.rs)
- [ ] Parser: `parse_defmacro()` 追加 (parser.rs)
- [ ] マクロ展開エンジン新規作成 (lsharp-syntax/src/macro_expand.rs)
- [ ] パイプライン統合: parse 後にマクロ展開パスを挿入
- [ ] 簡易 gensym による衛生性

### P10-3: 型付きマクロ
- [ ] マクロの `:type` シグネチャのパースと検証
- [ ] マクロ展開トレースバック (型エラー時にマクロ展開元を表示、miette 活用)
- [ ] 再帰マクロ (深度制限 128)
- [ ] `~@` (unquote-splicing) の可変長引数展開

### P10-4: 衛生マクロの完全化
- [ ] Scope ID システム (`HygienicIdent` 導入)
- [ ] Sets of Scopes による名前解決 (Typed Racket 方式)
- [ ] `(unhygienic name)` escape hatch (anaphoric macro 用)

### P10-5: 組み込みマクロ & Computation 統合 (将来)
- [ ] 組み込みマクロ: `when`, `unless`, `cond`, `|>`, `assert`
- [ ] `derive-show`, `derive-eq` 等の型レベルマクロ (`reify-type`)
- [ ] Computation Expression のマクロ化 (既存テスト互換維持)

---

## 構造的バグ (ADR proposed 高優先度)

- [x] BUG-1 (ADR-064): FieldAccess の型解決がフィールド名のみに依存 -- infer_expr_type_name に FieldAccess/Let/RecordUpdate/Match/If 対応追加、テスト 7件
- [x] BUG-2 (ADR-067): RecordUpdate の型推定がフィールド名のみに依存 -- BUG-1 と同時修正 (infer_expr_type_name 拡張)
- [x] BUG-3 (ADR-073): lower_match_arms のコンストラクタパターンでタグ比較命令がスタックに積まれず If 発行 -- 引数なし/付き両方で最後の腕でもタグ比較を発行、テスト 3件

---

## リファクタリング・改善 (ADR proposed 中優先度)

- [x] IMP-1 (ADR-054): パーサーのエラーリカバリ -- parse_program_recovering + ParseError::Multiple で実装済み、検証テスト 2件追加
- [x] IMP-2 (ADR-055): 制約階層の互換性チェック -- check_constraint_compatibility + is_subtype_constraints で実装済み、検証テスト 3件追加
- [x] IMP-3 (ADR-056): config.rs のエラーハンドリング改善 -- load_config_result + ConfigError で実装済み、検証テスト 3件追加
- [x] IMP-4 (ADR-066): run_wasm_wasi ヘルパー 3 箇所の重複解消 -- wasi_runner.rs に統合済み、検証テスト 4件追加

---

## テスト・品質基盤 (低優先度)

- [x] QA-1 (ADR-057): 正規表現エンジン NFA→DFA 変換 + Unicode 文字クラス -- regex/ モジュール分割 + 部分集合構成法 DFA + \p{L}/\p{N} 対応、テスト 34件
- [x] QA-2 (ADR-072): parse_test_output の generate_sample_args 重複呼び出し最適化 -- HashMap キャッシュ導入、テスト 3件追加
- [x] QA-3 (ADR-075): 型推論結果の HashMap 化 (線形探索→O(1)) -- Lower.type_results が HashMap であることを検証、テスト 2件追加
- [x] QA-4 (ADR-077): snapshot テスト拡大 (codegen/wasi の Wasm バイナリ) -- Wasm 14件 + IR 8件 = 22 スナップショットテスト追加
- [x] QA-5 (ADR-078): criterion ベンチマーク追加 -- parse/infer/lower/codegen/full_pipeline x simple/fibonacci = 10 ベンチマーク

---

## CI/CD

- [x] GitHub Actions ワークフロー作成 (`cargo test` + `cargo clippy` + `cargo fmt --check`) -- .github/workflows/ci.yml
- [ ] ブートストラップ CI (P8-5 完了後: stage1 生成 → 比較)
- [x] PR 自動テスト + マージブロック設定 -- ci-gate 集約ジョブ追加、docs/CI.md に設定手順記載

---

## 既存の未完了タスク (Phase に統合済み)

| 旧 ID | 内容 | 統合先 |
|--------|------|--------|
| P3-3 | `:invariant` の実行評価 | **完了** -- E2E テスト 4件追加 (実行評価パイプライン検証) |
| P3-3 | `:example` の実行評価 | **完了** -- 同上 |
| R-S1 | エラー型の統一 (`thiserror`) | **完了** -- `LsharpError` 統一エラー型 + テスト 7件 |
| R-S3 | WasmGC feature flag 導入 | アーキテクチャ方針: リニアメモリ正式基盤化で不要に (ADR-076 も同様) |
| R-S6 | `string_data` の RefCell 見直し | **完了** -- RefCell/Cell を直接フィールドに置換、&mut self 統一 |

---

## 既知の制限事項

### リニアメモリランタイム
- [ ] WasmGC 最適化バックエンド -- 現在はオプショナル、リニアメモリ上の Bump Allocator で全ヒープデータを管理
- [ ] Region GC 導入 -- REPL 等の長寿命プロセス向け (Phase 9)

### パターンマッチ
- [x] 引数付きコンストラクタパターン (深さ 1) は対応済み
- [x] ネストしたコンストラクタパターン (深さ 2 以上) -- E2E テスト 2件追加
- [x] ガード条件 (when 節) -- E2E テスト 2件追加
- ワイルドカード `_` + リテラル + 変数 + Bool パターン対応

### 正規表現エンジン
- [x] NFA → DFA 変換による最適化 -- 部分集合構成法、状態上限 256、NFA フォールバック
- [x] Unicode 文字クラス (`\p{L}`, `\p{N}`) -- char::is_alphabetic/is_numeric による判定

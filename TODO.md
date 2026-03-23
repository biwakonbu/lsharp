# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P9-1/3/4 は完了。詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-122) を参照

---

## Phase 1: 文字列操作

### P1-2: 文字列リテラルのヒープ化 (DEFERRED: 破壊的変更のため後回し)
- [ ] data section offset → ヒープ上 String オブジェクト (tag=1, len, bytes) への変換
- [ ] 既存の文字列関連テストが引き続きパスすることを確認

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
- [~] Rust 版 → stage1.wasm (L# コンパイラ) -- 個別モジュール E2E 検証済み (Token/Lexer/AST/Parser/IR/Type/TypeScheme/Compiler/WasmEmit)、統合は未完
- [ ] stage1.wasm → stage2.wasm (セルフコンパイル) -- 完全なセルフホストコンパイラ統合が前提
- [ ] stage1.wasm == stage2.wasm (固定点検証) -- stage2 生成が前提
- [ ] CI でのブートストラップ自動検証 -- 固定点検証が前提

---

## Phase 9: エコシステム

### P9-2: LSP
- [x] `crates/lsharp-lsp` クレート作成 -- tower-lsp 0.20, LsharpBackend 構造体
- [x] tower-lsp 統合 -- initialize/shutdown/did_open/did_change ハンドラ
- [x] エラー診断発行 -- parse_and_check で Diagnostic 化
- [~] 定義ジャンプ -- find_definition 関数実装 (テスト 3件) だが goto_definition ハンドラは URI→ソース未接続
- [~] 型ホバー -- hover ハンドラは URI→ソース未接続 (プレースホルダー)
- [ ] completion / references / rename / formatting

---

## 構造的バグ (ADR proposed 高優先度)

- [ ] BUG-1 (ADR-064): FieldAccess の型解決がフィールド名のみに依存 -- 同名フィールドの別レコード型で誤選択 (lower/expr.rs)
- [ ] BUG-2 (ADR-067): RecordUpdate の型推定がフィールド名のみに依存 -- 同フィールドセットで誤選択 (lower/expr.rs)
- [ ] BUG-3 (ADR-073): lower_match_arms のコンストラクタパターンでタグ比較命令がスタックに積まれず If 発行 (lower/pattern.rs)

---

## リファクタリング・改善 (ADR proposed 中優先度)

- [ ] IMP-1 (ADR-054): パーサーのエラーリカバリ -- 複数エラー一括報告 (parser.rs)
- [ ] IMP-2 (ADR-055): 制約階層の互換性チェック -- 親子制約の包含判定 (constraints.rs)
- [ ] IMP-3 (ADR-056): config.rs のエラーハンドリング改善 -- load_config エラー swallow (config.rs)
- [ ] IMP-4 (ADR-066): run_wasm_wasi ヘルパー 3 箇所の重複解消 (main.rs, e2e.rs, test_runner.rs)

---

## テスト・品質基盤 (低優先度)

- [ ] QA-1 (ADR-057): 正規表現エンジン NFA→DFA 変換 + Unicode 文字クラス
- [ ] QA-2 (ADR-072): parse_test_output の generate_sample_args 重複呼び出し最適化
- [ ] QA-3 (ADR-075): 型推論結果の HashMap 化 (線形探索→O(1))
- [ ] QA-4 (ADR-077): snapshot テスト拡大 (codegen/wasi の Wasm バイナリ)
- [ ] QA-5 (ADR-078): criterion ベンチマーク追加

---

## CI/CD

- [ ] GitHub Actions ワークフロー作成 (`cargo test` + `cargo clippy` + `cargo fmt --check`)
- [ ] ブートストラップ CI (P8-5 完了後: stage1 生成 → 比較)
- [ ] PR 自動テスト + マージブロック設定

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

### リニアメモリランタイム (Phase 0 で正式基盤化)
- WasmGC はオプショナルな最適化バックエンドとして位置づけ
- リニアメモリ上の Bump Allocator で全ヒープデータを管理
- GC は Phase 9 (REPL 等の長寿命プロセス) で Region GC として導入予定

### パターンマッチ
- 引数付きコンストラクタパターン (深さ 1) は対応済み
- ネストしたコンストラクタパターン (深さ 2 以上) は未対応
- ワイルドカード `_` + リテラル + 変数 + Bool パターン対応
- ガード条件は未実装

### 正規表現エンジン
- NFA → DFA 変換による最適化は未実装 (ステップ制限で病的入力を防止)
- Unicode 文字クラス (`\p{L}` 等) は未対応

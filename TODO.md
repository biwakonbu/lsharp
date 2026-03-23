# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P9-1/3/4 は完了。詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-122) を参照
> **ロードマップ**: `.claude/plans/logical-riding-russell.md` を参照

---

## Phase 1: 文字列操作

### P1-2: 文字列リテラルのヒープ化 (DEFERRED: 破壊的変更のため後回し)
- [ ] data section offset → ヒープ上 String オブジェクト (tag=1, len, bytes) への変換
- [ ] 既存の文字列関連テストが引き続きパスすることを確認

---

## Phase 2: 動的コレクション

### P2-3: ハッシュマップ
- [x] FNV-1a ハッシュ関数 (文字列キー用) -- Wasm ヘルパー実装、E2E テスト 4件追加 (insert/get/contains/remove/overwrite)

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
- [x] tower-lsp 統合 -- initialize/shutdown/did_open/did_change/hover ハンドラ
- [x] 型ホバー、エラー診断、定義ジャンプ -- ホバー・診断 + find_definition 実装、テスト 4件

---

## 既存の未完了タスク (Phase に統合済み)

| 旧 ID | 内容 | 統合先 |
|--------|------|--------|
| P3-3 | `:invariant` の実行評価 | **完了** -- E2E テスト 4件追加 (実行評価パイプライン検証) |
| P3-3 | `:example` の実行評価 | **完了** -- 同上 |
| R-S1 | エラー型の統一 (`thiserror`) | **完了** -- `LsharpError` 統一エラー型 + テスト 7件 |
| R-S3 | WasmGC feature flag 導入 | アーキテクチャ方針: リニアメモリ正式基盤化で不要に |
| R-S6 | `string_data` の RefCell 見直し | **完了** -- RefCell/Cell を直接フィールドに置換、&mut self 統一 |

---

## 既知の制限事項

### リニアメモリランタイム (Phase 0 で正式基盤化)
- WasmGC はオプショナルな最適化バックエンドとして位置づけ
- リニアメモリ上の Bump Allocator で全ヒープデータを管理
- GC は Phase 9 (REPL 等の長寿命プロセス) で Region GC として導入予定

### Lambda (クロージャ) → Phase 3 で対応
- 自由変数キャプチャは未実装 (ローカル関数として lowering)
- Phase 3 でクロージャ変換 (lambda lifting) を実装

### パターンマッチ
- ネストしたコンストラクタパターンは未対応
- ワイルドカードパターン `_` のみ対応、ガード条件は未実装

### 正規表現エンジン
- NFA → DFA 変換による最適化は未実装 (ステップ制限で病的入力を防止)
- Unicode 文字クラス (`\p{L}` 等) は未対応

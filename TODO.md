# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-6 (型システム) は完了。詳細は `docs/todo/archive.jsonl` を参照
> **ロードマップ**: `.claude/plans/logical-riding-russell.md` を参照

---

## Phase 0: 基盤整備

### P0-0: lower.rs リファクタリング (前提作業)
- [x] `lower.rs` (~2000行) を `lower/mod.rs`, `lower/expr.rs`, `lower/pattern.rs`, `lower/decl.rs` に分割 -- 5ファイルに分割済み
- [x] 既存テスト 422 個が全パスすることを確認 -- 469テスト全パス

### P0-1: リニアメモリ Bump Allocator
- [x] グローバル `$heap_ptr` の追加 (文字列定数データ末尾から開始) -- wasi.rs GlobalSection 追加
- [x] `__alloc(size: i64) -> i64` ビルトイン関数を `wasi.rs` に生成 -- emit_alloc_func 実装、型推論に builtin 登録、IR lowering 対応
- [x] ページ不足時 `memory.grow` で自動拡張 -- emit_alloc_func 内で memory.grow 実装
- [x] E2E テスト: メモリ確保・アライメント・大規模確保 -- E2E テスト 3 個追加 (test_e2e_alloc_basic, test_e2e_alloc_alignment, test_e2e_alloc_memory_grow)

### P0-2: メモリ操作 IR 命令
- [x] `I32Load`, `I32Store`, `I32Load8U`, `I32Store8` を `Instruction` enum に追加 -- 16種のメモリ操作命令
- [x] `emit.rs` で Wasm 命令への変換を実装 -- emit.rs に全命令マッピング
- [x] ユニットテスト: 各メモリ命令の正常動作 -- ユニットテスト追加

### P0-3: タグ付きワードとヒープオブジェクト基盤
- [x] i64 の上位ビットでタグ判定 (integer vs pointer) の規約を設計・実装 -- MSB=1 でポインタ判定
- [x] ヒープオブジェクト共通ヘッダ `[tag: i32, size: i32, ...]` の生成 -- emit_tag_pointer/emit_untag_pointer/emit_write_heap_header
- [x] E2E テスト: ヒープオブジェクトの確保・タグ判定・フィールドアクセス -- Ref Cell E2E テスト 4件で検証

---

## Phase 1: 文字列操作

### P1-1: 文字列ランタイム関数
- [x] `string-length [s] -> Int` ビルトイン -- パック文字列から下位32bit取得、E2E 3件
- [x] `string-concat [a b] -> String` ビルトイン -- __alloc + memory.copy、E2E 2件
- [ ] `string-char-at [s i] -> Int` ビルトイン (バイト単位)
- [ ] `substring [s start end] -> String` ビルトイン
- [x] `string-eq [a b] -> Bool` ビルトイン -- 長さ比較 + バイト比較ループ、E2E 4件
- [ ] `int-to-string [n] -> String` ビルトイン
- [x] `print-string [s] -> Unit` ビルトイン -- fd_write でパック文字列出力、E2E 3件
- [x] E2E テスト: 各関数の正常動作 + 境界条件 -- 12件パス

### P1-2: 文字列リテラルのヒープ化
- [ ] data section offset → ヒープ上 String オブジェクト (tag=1, len, bytes) への変換
- [ ] 既存の文字列関連テストが引き続きパスすることを確認

### P1-3: print の多相化
- [ ] `print-int` / `print-string` の分離
- [ ] 既存 `print` の後方互換性維持 (整数引数時は `print-int` にフォールバック)

---

## Phase 2: 動的コレクション (同時並行で実装可能)

### P2-1: ADT リニアメモリ版 lowering
- [x] WasmGC struct → リニアメモリ上のヒープオブジェクト (tag=3) への変換 -- decl.rs generate_adt_constructor
- [x] コンストラクタ呼出: ヒープにフィールド確保 → ポインタを返す -- emit_tag_pointer でタグ付きポインタ返却
- [x] パターンマッチ: タグ比較 + フィールド読み出し -- emit_untag_pointer + I32Load/I64Load
- [x] Cons リスト `(type (List a) (Cons a (List a)) Nil)` が実行可能に -- E2E テスト sum/length パス
- [x] E2E テスト: ADT 構築・分解・パターンマッチ -- E2E テスト 12件パス

### P2-2: 可変長配列 (Vector)
- [ ] `vector-new [capacity] -> Vector` ビルトイン
- [ ] `vector-push [v x] -> Vector` ビルトイン
- [ ] `vector-get [v i] -> a` ビルトイン
- [ ] `vector-set [v i x] -> Vector` ビルトイン
- [ ] `vector-length [v] -> Int` ビルトイン
- [ ] capacity 超過時のリアロケーション
- [ ] E2E テスト: 基本操作 + リアロケーション

### P2-3: ハッシュマップ
- [ ] `map-new [] -> Map` ビルトイン
- [ ] `map-insert [m key value] -> Map` ビルトイン
- [ ] `map-get [m key] -> Option` ビルトイン
- [ ] `map-contains? [m key] -> Bool` ビルトイン
- [ ] `map-remove [m key] -> Map` ビルトイン
- [ ] `map-size [m] -> Int` ビルトイン
- [ ] FNV-1a ハッシュ関数 (文字列キー用)
- [ ] E2E テスト: 挿入・取得・削除・衝突処理

---

## Phase 3: クロージャ

### P3-1: 自由変数解析
- [x] Lambda body の走査による自由変数収集 -- closure.rs に free_variables() 実装
- [x] `crates/lsharp-ir/src/closure.rs` モジュール作成 -- 295行、全Exprバリアント対応
- [x] ユニットテスト: 各種 Lambda パターンの自由変数抽出 -- ユニットテスト 10件

### P3-2: クロージャ変換 (Lambda Lifting)
- [x] Lambda → 通常関数 (環境パラメータ追加) へのリフト -- 統一呼び出し規約 (params + closure_ptr)
- [x] クロージャオブジェクト (tag=4, func_idx, captured values) のヒープ確保 -- FuncIdx IR 命令で codegen リマップ
- [x] `call_indirect` によるクロージャ呼び出し -- table/element section + CallIndirect 型マッピング
- [x] E2E テスト: 自由変数キャプチャ + クロージャ呼出 -- E2E 5件パス

### P3-3: 高階関数の有効化
- [ ] `list-map [f xs] -> List` (クロージャ対応)
- [ ] `list-filter [f xs] -> List` (クロージャ対応)
- [ ] `list-fold [f init xs] -> a` (クロージャ対応)
- [ ] `vector-map`, `vector-filter` の追加
- [ ] E2E テスト: 高階関数の組み合わせ

---

## Phase 4: エラー処理 & ミュータビリティ

### P4-1: Result/Option ランタイム
- [x] `(type (Option a) (Some a) None)` が実行時に動作 -- ADT リニアメモリで動作確認
- [x] `(type (Result a e) (Ok a) (Err e))` が実行時に動作 -- Ok/Err パターンマッチ E2E パス
- [x] `unwrap`, `map`, `and-then` ユーティリティ -- safe-div/unwrap/unwrap-or 実装確認
- [x] E2E テスト: Option/Result のパターンマッチ -- E2E 5件パス

### P4-2: 可変参照 (Ref Cell)
- [x] `ref-new [v] -> Ref` ビルトイン -- ヒープ確保 + タグ付きポインタ返却
- [x] `ref-get [r] -> a` ビルトイン -- ポインタ解除 + I64Load
- [x] `ref-set [r v] -> Unit` ビルトイン -- ポインタ解除 + I64Store
- [x] E2E テスト: 可変状態の読み書き -- E2E テスト 4件 (new/get, set/get, multiple_updates, in_loop)

---

## Phase 5: File I/O & WASI 拡張

### P5-1: WASI import 追加
- [ ] `path_open` import
- [ ] `fd_read` import
- [ ] `fd_close` import
- [ ] `fd_seek` import
- [ ] `fd_filestat_get` import
- [ ] `args_get`, `args_sizes_get` import
- [ ] `proc_exit` import

### P5-2: ファイル操作ビルトイン
- [ ] `read-file [path] -> String` ビルトイン
- [ ] `write-file [path content] -> Unit` ビルトイン
- [ ] `file-exists? [path] -> Bool` ビルトイン
- [ ] コマンドライン引数取得
- [ ] E2E テスト: ファイル読み書き + 引数取得

---

## Phase 6: マルチファイルコンパイル

### P6-1: モジュール探索
- [ ] `(import ModuleName)` → ファイル探索規約の実装
- [ ] 既存 `module_graph.rs` の活用
- [ ] ユニットテスト: モジュール名 → ファイルパス解決

### P6-2: クロスモジュール型環境
- [ ] トポロジカルソート順コンパイル
- [ ] export シンボルの型環境注入
- [ ] ユニットテスト: クロスモジュール型解決

### P6-3: IR リンク
- [ ] 全モジュール IR の結合
- [ ] 関数インデックス再割当て
- [ ] E2E テスト: マルチファイルプロジェクトのコンパイル・実行

---

## Phase 7: 標準ライブラリ (L# で記述)

- [ ] `stdlib/Core.ls` -- Bool, Option, Result, 基本関数
- [ ] `stdlib/String.ls` -- concat, split, trim, contains, starts-with
- [ ] `stdlib/List.ls` -- map, filter, fold, append, reverse, zip
- [ ] `stdlib/Vector.ls` -- 可変長配列ラッパー
- [ ] `stdlib/Map.ls` -- HashMap (Vector + ハッシュ関数)
- [ ] `stdlib/Set.ls` -- HashSet
- [ ] `stdlib/IO.ls` -- read-file, write-file, read-line
- [ ] `stdlib/Debug.ls` -- debug-print, assert
- [ ] `stdlib/Char.ls` -- is-digit, is-alpha, is-whitespace
- [ ] stdlib のコンパイル・テスト自動化

---

## Phase 8: セルフホスティング

### ブートストラップ戦略
> 最小サブセットで開始: `let` / 再帰 / `if` / `match` / ADT / Record / モジュール
> HKT/GADT/トレイト制約等の高度機能はセルフホスト後に段階追加

### P8-1: L# で Lexer を実装
- [ ] Token ADT 定義
- [ ] 文字列走査による字句解析
- [ ] Rust 版 lexer との出力比較テスト

### P8-2: L# で Parser を実装
- [ ] AST の ADT 定義
- [ ] 再帰降下パーサー
- [ ] Rust 版 parser との出力比較テスト

### P8-3: L# で型推論を実装
- [ ] 型 ADT (Con, Var, Fun) 定義
- [ ] Substitution (HashMap ベース)
- [ ] Unification アルゴリズム
- [ ] let 多相 + 型注釈
- [ ] Rust 版型推論との出力比較テスト

### P8-4: L# で IR Lowering + Codegen を実装
- [ ] IR ADT 定義
- [ ] AST → IR 変換
- [ ] LEB128 エンコーディング
- [ ] Wasm バイナリ生成
- [ ] Rust 版 codegen との出力比較テスト

### P8-5: ブートストラップ検証
- [ ] Rust 版 → stage1.wasm (L# コンパイラ)
- [ ] stage1.wasm → stage2.wasm (セルフコンパイル)
- [ ] stage1.wasm == stage2.wasm (固定点検証)
- [ ] CI でのブートストラップ自動検証

---

## Phase 9: エコシステム (セルフホスト完了後)

### P9-1: REPL
- [ ] `lsharp repl` サブコマンド
- [ ] readline ライブラリ統合
- [ ] 式入力 → パイプライン実行 → 結果表示

### P9-2: LSP
- [ ] `crates/lsharp-lsp` クレート作成
- [ ] tower-lsp 統合
- [ ] 型ホバー、エラー診断、定義ジャンプ

### P9-3: パッケージマネージャ
- [ ] `lsharp.toml` の `[dependencies]` セクション
- [ ] Git リポジトリベースの依存解決
- [ ] ロックファイル生成

### P9-4: ドキュメント生成
- [ ] `:doc` メタデータから HTML 生成
- [ ] 型シグネチャ・例の自動抽出

---

## 既存の未完了タスク (Phase に統合済み)

| 旧 ID | 内容 | 統合先 |
|--------|------|--------|
| P3-3 | `:invariant` の実行評価 | Phase 2-1 (ADT リニアメモリ化) + Phase 7 (stdlib) 完了後に対応 |
| P3-3 | `:example` の実行評価 | 同上 |
| R-S1 | エラー型の統一 (`thiserror`) | P0-0 リファクタリング時に検討 |
| R-S3 | WasmGC feature flag 導入 | アーキテクチャ方針: リニアメモリ正式基盤化で不要に |
| R-S6 | `string_data` の RefCell 見直し | P0-0 リファクタリング時に対応 |

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

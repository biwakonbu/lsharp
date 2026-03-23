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
- [x] `string-char-at [s i] -> Int` ビルトイン (バイト単位) -- パック文字列 offset + index、E2E 3件
- [x] `substring [s start end] -> String` ビルトイン -- __alloc + メモリコピー、E2E 3件
- [x] `string-eq [a b] -> Bool` ビルトイン -- 長さ比較 + バイト比較ループ、E2E 4件
- [x] `int-to-string [n] -> String` ビルトイン -- emit_int_to_string_func ヘルパー、E2E 5件
- [x] `print-string [s] -> Unit` ビルトイン -- fd_write でパック文字列出力、E2E 3件
- [x] E2E テスト: 各関数の正常動作 + 境界条件 -- 12件パス

### P1-2: 文字列リテラルのヒープ化
- [ ] data section offset → ヒープ上 String オブジェクト (tag=1, len, bytes) への変換
- [ ] 既存の文字列関連テストが引き続きパスすることを確認

### P1-3: print の多相化
- [x] `print-int` / `print-string` の分離 -- infer_expr_type_name で型判定、E2E 2件
- [x] 既存 `print` の後方互換性維持 (整数引数時は `print-int` にフォールバック)

---

## Phase 2: 動的コレクション (同時並行で実装可能)

### P2-1: ADT リニアメモリ版 lowering
- [x] WasmGC struct → リニアメモリ上のヒープオブジェクト (tag=3) への変換 -- decl.rs generate_adt_constructor
- [x] コンストラクタ呼出: ヒープにフィールド確保 → ポインタを返す -- emit_tag_pointer でタグ付きポインタ返却
- [x] パターンマッチ: タグ比較 + フィールド読み出し -- emit_untag_pointer + I32Load/I64Load
- [x] Cons リスト `(type (List a) (Cons a (List a)) Nil)` が実行可能に -- E2E テスト sum/length パス
- [x] E2E テスト: ADT 構築・分解・パターンマッチ -- E2E テスト 12件パス

### P2-2: 可変長配列 (Vector)
- [x] `vector-new [capacity] -> Vector` ビルトイン -- ヒープ確保 + タグ付きポインタ、E2E 2件
- [x] `vector-push [v x] -> Vector` ビルトイン -- capacity超過時リアロケーション対応、E2E 2件
- [x] `vector-get [v i] -> a` ビルトイン -- インデックス指定要素取得、E2E 1件
- [x] `vector-set [v i x] -> Vector` ビルトイン -- ミューテーション、E2E 1件
- [x] `vector-length [v] -> Int` ビルトイン -- mem[addr+8] 読み出し
- [x] capacity 超過時のリアロケーション -- vector-push で自動拡張 (cap*2, 最低4)
- [x] E2E テスト: 基本操作 + リアロケーション -- E2E 6件パス

### P2-3: ハッシュマップ
- [x] `map-new [] -> Map` ビルトイン -- 16エントリ容量、MemoryFill で初期化、E2E 1件
- [x] `map-insert [m key value] -> Map` ビルトイン -- 線形探索、key=0 空判定、上書き対応、E2E 3件
- [x] `map-get [m key] -> Option` ビルトイン -- 全スロット走査、未存在時 0 返却、E2E 2件
- [x] `map-contains? [m key] -> Bool` ビルトイン -- 全スロット走査、E2E 2件
- [x] `map-remove [m key] -> Map` ビルトイン -- tombstone (key=-1) 方式、E2E 1件
- [x] `map-size [m] -> Int` ビルトイン -- mem[addr+8] 読み出し
- [~] FNV-1a ハッシュ関数 (文字列キー用) -- 現在は整数キーのみ対応
- [x] E2E テスト: 挿入・取得・削除・衝突処理 -- E2E 9件パス

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
- [x] `list-map [f xs] -> List` (クロージャ対応) -- ユーザー定義関数 + Lambda、E2E 2件
- [x] `list-filter [f xs] -> List` (クロージャ対応) -- E2E 2件
- [x] `list-fold [f init xs] -> a` (クロージャ対応) -- E2E テストで動作確認
- [x] `vector-map`, `vector-filter` の追加 -- E2E テスト 2件パス
- [x] E2E テスト: 高階関数の組み合わせ -- list-map/filter 組み合わせ E2E パス

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
- [x] `path_open` import -- wasi.rs Import Section に追加
- [x] `fd_read` import -- wasi.rs Import Section に追加
- [x] `fd_close` import -- wasi.rs Import Section に追加
- [x] `fd_seek` import -- wasi.rs Import Section に追加
- [x] `fd_filestat_get` import -- wasi.rs Import Section に追加
- [x] `args_get`, `args_sizes_get` import -- wasi.rs Import Section に追加
- [x] `proc_exit` import -- wasi.rs Import Section + E2E テスト 3件

### P5-2: ファイル操作ビルトイン
- [x] `read-file [path] -> String` ビルトイン -- WASI path_open+fd_read+fd_filestat_get, E2E 1件
- [x] `write-file [path content] -> Int` ビルトイン -- WASI path_open+fd_write+fd_close, E2E 1件
- [x] `file-exists? [path] -> Bool` ビルトイン -- WASI path_open エラー判定, E2E 1件
- [x] コマンドライン引数取得 -- command-line-args ビルトイン, E2E 1件
- [x] E2E テスト: ファイル読み書き + 引数取得 -- E2E 3件パス

---

## Phase 6: マルチファイルコンパイル

### P6-1: モジュール探索
- [x] `(import ModuleName)` → ファイル探索規約の実装 -- ユニットテスト 11 個追加
- [x] 既存 `module_graph.rs` の活用 -- module_name_to_paths, resolve_module_file, build_from_entry 追加
- [x] ユニットテスト: モジュール名 → ファイルパス解決 -- resolve_tests 11 個

### P6-2: クロスモジュール型環境
- [x] トポロジカルソート順コンパイル -- compile_multi_file でトポソ順にパース+型チェック
- [x] export シンボルの型環境注入 -- Infer.inject_external_types + external_types フィールド追加
- [x] ユニットテスト: クロスモジュール型解決 -- E2E テスト 4 個 (multi_file_compile, chain, single, missing_import)

### P6-3: IR リンク
- [x] 全モジュール IR の結合 -- AST マージ方式で単一 Lower に統合
- [x] 関数インデックス再割当て -- マージされた Program で一貫したインデックス空間を使用
- [x] E2E テスト: マルチファイルプロジェクトのコンパイル・実行 -- E2E 4 個追加

---

## Phase 7: 標準ライブラリ (L# で記述)

- [x] `stdlib/Core.ls` -- Bool, Option, Result, 基本関数
- [x] `stdlib/String.ls` -- concat, split, trim, contains, starts-with
- [x] `stdlib/List.ls` -- map, filter, fold, append, reverse, zip
- [x] `stdlib/Vector.ls` -- 可変長配列ラッパー (vector-map/filter/fold/sum)
- [x] `stdlib/Map.ls` -- HashMap ラッパー (map-empty?/map-get-or)
- [x] `stdlib/Set.ls` -- HashSet (HashMap ベース) -- E2E 1件パス
- [x] `stdlib/IO.ls` -- read-file, write-file ラッパー
- [x] `stdlib/Debug.ls` -- debug-print, assert -- E2E 1件パス
- [x] `stdlib/Char.ls` -- is-digit, is-alpha, is-whitespace -- E2E 1件パス
- [~] stdlib のコンパイル・テスト自動化 -- E2E 3件追加 (Char/Debug/Set)

---

## Phase 8: セルフホスティング

### ブートストラップ戦略
> 最小サブセットで開始: `let` / 再帰 / `if` / `match` / ADT / Record / モジュール
> HKT/GADT/トレイト制約等の高度機能はセルフホスト後に段階追加

### P8-1: L# で Lexer を実装
- [x] Token ADT 定義 -- selfhost/Token.ls (整数タグ方式)
- [x] 文字列走査による字句解析 -- selfhost/Lexer.ls (tokenize/lex-one/classify-symbol)
- [~] Rust 版 lexer との出力比較テスト -- E2E テスト 1件パス (基本トークナイズ)

### P8-2: L# で Parser を実装
- [x] AST の ADT 定義 -- selfhost/AST.ls (整数タグ + Vector 方式)
- [x] 再帰降下パーサー -- selfhost/Parser.ls (parse-expr/parse-sexp)
- [~] Rust 版 parser との出力比較テスト -- E2E テスト 1件パス (基本 S 式パース)

### P8-3: L# で型推論を実装
- [x] 型 ADT (Con, Var, Fun) 定義 -- selfhost/Type.ls (整数タグ + Vector)
- [x] Substitution (HashMap ベース) -- subst-new/bind/lookup 実装
- [x] Unification アルゴリズム -- unify-simple (Con/Var), occurs-check, E2E テスト 1件
- [x] let 多相 + 型注釈 -- selfhost/TypeScheme.ls (instantiate/generalize/free-vars), E2E 1件
- [~] Rust 版型推論との出力比較テスト -- E2E テスト 2件パス (型構築 + Substitution + Unification)

### P8-4: L# で IR Lowering + Codegen を実装
- [x] IR ADT 定義 -- selfhost/IR.ls (命令タグ + Vector)
- [x] AST → IR 変換 -- selfhost/Compiler.ls (compile-expr: lit/var/bool), E2E 1件
- [x] LEB128 エンコーディング -- selfhost/Compiler.ls (leb128-unsigned), E2E 1件
- [ ] Wasm バイナリ生成
- [~] Rust 版 codegen との出力比較テスト -- E2E テスト 2件パス (IR 命令構築 + Compiler)

### P8-5: ブートストラップ検証
- [ ] Rust 版 → stage1.wasm (L# コンパイラ)
- [ ] stage1.wasm → stage2.wasm (セルフコンパイル)
- [ ] stage1.wasm == stage2.wasm (固定点検証)
- [ ] CI でのブートストラップ自動検証

---

## Phase 9: エコシステム (セルフホスト完了後)

### P9-1: REPL
- [x] `lsharp repl` サブコマンド -- cmd_repl 実装、式入力→コンパイル→WASI実行
- [x] readline ライブラリ統合 -- rustyline 15, 履歴ファイル対応
- [x] 式入力 → パイプライン実行 → 結果表示 -- parse→infer→lower→codegen→wasmtime

### P9-2: LSP
- [x] `crates/lsharp-lsp` クレート作成 -- tower-lsp 0.20, LsharpBackend 構造体
- [x] tower-lsp 統合 -- initialize/shutdown/did_open/did_change/hover ハンドラ
- [~] 型ホバー、エラー診断、定義ジャンプ -- ホバー・診断の基本実装あり、定義ジャンプは未実装

### P9-3: パッケージマネージャ
- [x] `lsharp.toml` の `[dependencies]` セクション -- DependencySpec enum (Version/Git/Path)、テスト 5件
- [x] `lsharp install` コマンド -- Path 依存のシンボリックリンク解決、テスト 2件
- [ ] Git リポジトリベースの依存解決
- [x] ロックファイル生成 -- lockfile.rs (generate/write/read), TOML 形式, テスト 5件

### P9-4: ドキュメント生成
- [x] `:doc` メタデータから HTML 生成 -- `lsharp doc <file>` コマンド
- [x] 型シグネチャ・例の自動抽出 -- 型推論結果 + メタデータのパラメータ/戻り値

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

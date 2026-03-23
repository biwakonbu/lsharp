# 実装計画

## メタ情報
- 作成日時: 2026-03-23 14:30:00
- タスクID: 20260323-135606_c0c44d
- 総ステップ数: 24
- 計画策定AI: Claude Code (単独)

## 1. 概要

### 1.1 目標
L# コンパイラの全 Phase (0-9) を実装し、セルフホスティング可能な言語処理系とエコシステムを完成させる。60 タスク (TEST: 29, TASK: 31) を 24 グループに分割し、TDD で段階的に構築する。

### 1.2 スコープ
- 対象: Phase 0 (基盤整備) から Phase 9 (エコシステム) までの全機能
- 除外: WasmGC ネイティブバックエンド最適化、GC 本格実装、Unicode 文字クラス、infer.rs リファクタリング、ネストしたコンストラクタパターン

## 2. 変更対象ファイル

### 2.1 変更ファイル
| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `crates/lsharp-ir/src/lower.rs` | delete | `lower/` ディレクトリに分割して削除 |
| `crates/lsharp-ir/src/lower/mod.rs` | create | Lower struct、lower_program()、型登録、公開 API |
| `crates/lsharp-ir/src/lower/expr.rs` | create | lower_expr()、emit_binop()、式の IR 変換 |
| `crates/lsharp-ir/src/lower/pattern.rs` | create | lower_match_arms()、パターンマッチ IR 生成 |
| `crates/lsharp-ir/src/lower/decl.rs` | create | lower_function()、ジェネレータ群 |
| `crates/lsharp-ir/src/lower/tests.rs` | create | テストコード (insta スナップショット含む) |
| `crates/lsharp-ir/src/lib.rs` | modify | Instruction enum にメモリ操作命令、I32系命令、CallIndirect 追加。IrType に I32 追加 |
| `crates/lsharp-ir/src/closure.rs` | create | 自由変数解析モジュール |
| `crates/lsharp-ir/src/module_graph.rs` | modify | ファイル探索ロジック追加 |
| `crates/lsharp-wasm/src/wasi.rs` | modify | Bump Allocator、ビルトイン関数群、WASI import 追加、funcref テーブル |
| `crates/lsharp-wasm/src/emit.rs` | modify | メモリ操作命令の Wasm 変換追加 |
| `crates/lsharp-wasm/src/wasi_runner.rs` | modify | preopened_dir 追加、ファイルシステムアクセス有効化 |
| `crates/lsharp-wasm/tests/e2e.rs` | modify | 全 Phase の E2E テスト追加 |
| `crates/lsharp-types/src/infer.rs` | modify | ビルトイン関数の型シグネチャ追加 |
| `crates/lsharp-driver/src/main.rs` | modify | repl/lsp/pkg サブコマンド追加、マルチファイルコンパイル対応 |
| `crates/lsharp-lsp/src/lib.rs` | create | LSP サーバー実装 (tower-lsp) |
| `stdlib/Core.ls` | create | Bool, Option, Result, 基本関数 |
| `stdlib/String.ls` | create | 文字列操作ユーティリティ |
| `stdlib/List.ls` | create | リスト操作 (map, filter, fold) |
| `stdlib/Vector.ls` | create | 可変長配列ラッパー |
| `stdlib/Map.ls` | create | HashMap ラッパー |
| `stdlib/Set.ls` | create | HashSet |
| `stdlib/IO.ls` | create | ファイル I/O ユーティリティ |
| `stdlib/Debug.ls` | create | debug-print, assert |
| `stdlib/Char.ls` | create | 文字操作ユーティリティ |
| `self-hosted/Lexer.ls` | create | L# セルフホスティング Lexer |
| `self-hosted/Parser.ls` | create | L# セルフホスティング Parser |
| `self-hosted/TypeInfer.ls` | create | L# セルフホスティング型推論 |
| `self-hosted/Codegen.ls` | create | L# セルフホスティング Codegen |

### 2.2 影響を受けるファイル
- `crates/lsharp-syntax/src/ast.rs`: Phase 0-6 では変更不要 (既存 AST で対応)
- `crates/lsharp-syntax/src/parser.rs`: Phase 0-6 では変更不要
- `crates/lsharp-wasm/src/codegen.rs`: emit.rs の変更に伴う間接的影響
- `Cargo.toml` (workspace): lsharp-lsp クレート追加時に修正

## 3. 実装ステップ

### Step 1: Phase 0 テスト作成 (Group A)
**対象タスク**: TEST-001, TEST-002, TEST-003

**変更内容**:
1. P0-0 lower.rs リファクタリング用テストハーネスを作成 -- 分割後も 422 テストが通ることを検証する構造を準備
2. P0-1 Bump Allocator 用 E2E テスト -- `__alloc` 呼び出し、ページ拡張、8 バイトアラインメントの検証
3. P0-2 メモリ操作 IR 命令用ユニットテスト -- I32Load/I32Store/I64Load/I64Store の IR 構築と emit 検証

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: Bump Allocator E2E テスト追加
- `crates/lsharp-ir/src/lib.rs`: IR 命令構築テスト追加 (inline test)
- `crates/lsharp-wasm/src/emit.rs`: emit ユニットテスト追加 (inline test)

**検証方法**:
- `cargo test` で全テストが RED (新規テストのみ FAIL、既存 422 テストは PASS)

**依存**: なし

**並列実行**: TEST-001, TEST-002, TEST-003 は 3 Worker で同時実行可能

---

### Step 2: Phase 0 基盤前半実装 (Group B)
**対象タスク**: TASK-001, TASK-002, TASK-003

**変更内容**:
1. TASK-001: `lower.rs` を `lower/` ディレクトリに分割
   - `lower/mod.rs`: Lower struct (pub(crate) フィールド)、lower_program()、型登録 (~400行)
   - `lower/expr.rs`: lower_expr()、emit_binop() (~400行)
   - `lower/pattern.rs`: lower_match_arms() (~150行)
   - `lower/decl.rs`: lower_function()、ジェネレータ群 (~400行)
   - `lower/tests.rs`: テストコード (~600行)
   - insta スナップショットファイルの参照パス更新
2. TASK-002: Bump Allocator を wasi.rs に実装
   - グローバル `$heap_ptr` (i32) 追加、初期値 = 文字列定数データ末尾
   - `__alloc(size: i32) -> i32` をインライン Wasm 関数として生成
   - 8 バイトアラインメント: `(size + 7) & ~7`
   - `memory.grow` による自動ページ拡張
3. TASK-003: メモリ操作 IR 命令を lib.rs + emit.rs に追加
   - Instruction enum に I32Load/I32Store/I32Load8U/I32Store8/I64Load/I64Store 追加
   - I32WrapI64/I64ExtendI32U/I32Const/I32Add/I32Sub/I32Mul/I32GtU/I32GeU 追加
   - I32And/I32Or/I32Shl/I32ShrU/MemoryGrow/MemorySize 追加
   - emit.rs に対応する wasm_encoder::Instruction 変換を実装

**対象ファイル**:
- `crates/lsharp-ir/src/lower.rs` -> `crates/lsharp-ir/src/lower/` (5 ファイル)
- `crates/lsharp-ir/src/lib.rs`: Instruction enum 拡張
- `crates/lsharp-wasm/src/wasi.rs`: $heap_ptr、__alloc 関数生成
- `crates/lsharp-wasm/src/emit.rs`: メモリ操作命令変換

**検証方法**:
- `cargo test` -- 既存 422 テスト全パス + TEST-001/002/003 が GREEN

**依存**: Step 1 (TEST-001, TEST-002, TEST-003)

**並列実行**: TASK-001, TASK-002, TASK-003 は 3 Worker で同時実行可能

---

### Step 3: Phase 0-3 テスト作成 (Group C)
**対象タスク**: TEST-004

**変更内容**:
1. P0-3 タグ付きワード用 E2E テスト -- i64 上位ビットタグ判定、ヒープオブジェクトヘッダ生成・読み出し

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: タグ付きワード E2E テスト追加

**検証方法**:
- `cargo test` で TEST-004 が RED (FAIL)

**依存**: Step 2 (TASK-002, TASK-003 完了が前提)

---

### Step 4: Phase 0-3 タグ付きワード実装 (Group D)
**対象タスク**: TASK-004

**変更内容**:
1. i64 最上位ビット = 0 で即値整数 (63-bit signed integer)
2. i64 最上位ビット = 1 でヒープポインタ (下位 32 ビットがアドレス)
3. ヒープオブジェクト共通ヘッダ `[tag: i32, size: i32]` 生成ヘルパーを lower/expr.rs に実装
4. タグ付け/タグ解除のヘルパー IR 命令列を生成する関数を実装

**対象ファイル**:
- `crates/lsharp-ir/src/lower/expr.rs`: タグ付け/タグ解除ヘルパー
- `crates/lsharp-ir/src/lower/mod.rs`: ヘッダ生成ユーティリティ
- `crates/lsharp-wasm/src/wasi.rs`: ヘッダ書き込み用ビルトイン関数 (必要に応じて)

**検証方法**:
- `cargo test` -- TEST-004 が GREEN + 既存テスト全パス

**依存**: Step 3 (TEST-004)

---

### Step 5: Phase 1/2-1/3-1 テスト作成 (Group E)
**対象タスク**: TEST-005, TEST-006, TEST-007

**変更内容**:
1. TEST-005: P1-1 文字列ランタイム E2E テスト -- string-length/string-concat/string-char-at/substring/string-eq/int-to-string/print-string
2. TEST-006: P2-1 ADT リニアメモリ E2E テスト -- Cons リスト構築、パターンマッチ、フィールドアクセス
3. TEST-007: P3-1 自由変数解析ユニットテスト -- let 束縛、ネスト Lambda、再帰パターン

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: 文字列・ADT テスト追加
- `crates/lsharp-ir/src/closure.rs`: 自由変数解析テスト (ファイル新規作成、テストを先に記述)

**検証方法**:
- `cargo test` で TEST-005/006/007 が RED

**依存**: Step 4 (TASK-004)

**並列実行**: TEST-005, TEST-006, TEST-007 は 3 Worker で同時実行可能

---

### Step 6: Phase 1-1/2-1/3-1 実装 (Group F)
**対象タスク**: TASK-005, TASK-006, TASK-007

**変更内容**:
1. TASK-005 (P1-1 文字列ランタイム):
   - wasi.rs に string-length/string-concat/string-char-at/substring/string-eq/int-to-string/print-string をインライン Wasm 関数として生成
   - infer.rs にビルトイン型シグネチャ登録 (String -> Int, String/String -> String ...)
   - lower/expr.rs でビルトイン関数呼び出しを Call 命令に変換
2. TASK-006 (P2-1 ADT リニアメモリ):
   - lower/decl.rs の generate_adt_constructor を改修: __alloc でヒープ確保 -> tag=3 ヘッダ + variant_tag + field_count + フィールド書き込み -> タグ付きポインタ返却
   - lower/pattern.rs: ポインタからタグ読み出し -> variant_tag で分岐 -> フィールド読み出し
3. TASK-007 (P3-1 自由変数解析):
   - crates/lsharp-ir/src/closure.rs に free_variables(expr, bound) -> Vec<String> を実装
   - AST を再帰走査、let/fn パラメータを bound に追加、bound 外の変数参照を収集

**対象ファイル**:
- `crates/lsharp-wasm/src/wasi.rs`: 文字列ビルトイン関数 7 個
- `crates/lsharp-types/src/infer.rs`: ビルトイン型シグネチャ追加
- `crates/lsharp-ir/src/lower/expr.rs`: ビルトイン呼び出し変換
- `crates/lsharp-ir/src/lower/decl.rs`: ADT コンストラクタ改修
- `crates/lsharp-ir/src/lower/pattern.rs`: ADT パターンマッチ改修
- `crates/lsharp-ir/src/closure.rs`: 自由変数解析実装

**検証方法**:
- `cargo test` -- TEST-005/006/007 が GREEN + 既存テスト全パス

**依存**: Step 5 (TEST-005, TEST-006, TEST-007)

**並列実行**: TASK-005, TASK-006, TASK-007 は 3 Worker で同時実行可能

---

### Step 7: Phase 1 後半 + Phase 2 コレクション テスト作成 (Group G)
**対象タスク**: TEST-008, TEST-009, TEST-010

**変更内容**:
1. TEST-008: P1-2/P1-3 文字列ヒープ化 + print 多相化 E2E テスト
2. TEST-009: P2-2 Vector E2E テスト -- vector-new/push/get/set/length、capacity 超過リアロケーション
3. TEST-010: P2-3 HashMap E2E テスト -- map-new/insert/get/contains?/remove/size

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: 文字列ヒープ化・Vector・HashMap テスト追加

**検証方法**:
- `cargo test` で TEST-008/009/010 が RED

**依存**: Step 6 (TASK-005 for TEST-008, TASK-006 for TEST-009/010)

**並列実行**: TEST-008, TEST-009, TEST-010 は 3 Worker で同時実行可能

---

### Step 8: Phase 1 後半 + Phase 2 コレクション実装 (Group H)
**対象タスク**: TASK-008, TASK-009, TASK-010

**変更内容**:
1. TASK-008 (P1-2 + P1-3):
   - 文字列リテラルのヒープ化: data section offset を起動時に String オブジェクト (tag=1, len, bytes) に変換
   - print 多相化: print-int (既存 __print_i64 改名)、print-string (新規) に分離
   - 既存 print の後方互換性: タグ判定で自動ディスパッチ
2. TASK-009 (P2-2 Vector):
   - wasi.rs に vector-new/push/get/set/length をインライン Wasm 関数として生成
   - tag=5、ヘッダ = [tag, size, len, cap, data_ptr]
   - capacity 超過時: 新バッファ alloc (cap*2) -> データコピー -> data_ptr 更新
3. TASK-010 (P2-3 HashMap):
   - wasi.rs に map-new/insert/get/contains?/remove/size を生成
   - FNV-1a ハッシュ (offset_basis=2166136261, prime=16777619)
   - チェイン法衝突解決、初期容量 16、負荷率 0.75 で 2 倍拡張

**対象ファイル**:
- `crates/lsharp-wasm/src/wasi.rs`: 文字列ヒープ化、Vector、HashMap ビルトイン
- `crates/lsharp-ir/src/lower/expr.rs`: print 多相化、Vector/HashMap 呼び出し変換
- `crates/lsharp-types/src/infer.rs`: Vector/HashMap ビルトイン型シグネチャ

**検証方法**:
- `cargo test` -- TEST-008/009/010 が GREEN + 既存テスト全パス

**依存**: Step 7 (TEST-008, TEST-009, TEST-010)

**並列実行**: TASK-008, TASK-009, TASK-010 は 3 Worker で同時実行可能

---

### Step 9: Phase 3 クロージャ + Phase 4 + Phase 6 テスト作成 (Group I)
**対象タスク**: TEST-011, TEST-012, TEST-013

**変更内容**:
1. TEST-011: P3-2 クロージャ変換 E2E テスト -- Lambda Lifting、tag=4 ヒープ確保、call_indirect
2. TEST-012: P4-1 Result/Option ランタイム E2E テスト -- unwrap/map/and-then
3. TEST-013: P4-2 Ref Cell E2E テスト -- ref-new/ref-get/ref-set (tag=7)

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: クロージャ・Option/Result・Ref Cell テスト追加

**検証方法**:
- `cargo test` で TEST-011/012/013 が RED

**依存**: Step 6 (TASK-007 for TEST-011, TASK-006 for TEST-012), Step 4 (TASK-004 for TEST-013)

**並列実行**: TEST-011, TEST-012, TEST-013 は 3 Worker で同時実行可能

---

### Step 10: Phase 3 クロージャ + Phase 4 実装 (Group J)
**対象タスク**: TASK-011, TASK-012, TASK-013

**変更内容**:
1. TASK-011 (P3-2 クロージャ変換):
   - lower/expr.rs で Expr::Lambda 検出 -> closure::free_variables() で自由変数解析
   - 新しいトップレベル関数生成 (元パラメータ + 環境パラメータ)
   - クロージャオブジェクト tag=4 をヒープ確保 (func_idx, env_count, captured values)
   - wasi.rs に funcref テーブル追加、call_indirect 命令サポート
2. TASK-012 (P4-1 Result/Option):
   - P2-1 の ADT リニアメモリ化の上で Option (Some/None)、Result (Ok/Err) が動作することを検証
   - unwrap/map/and-then を L# ユーティリティ関数として記述
3. TASK-013 (P4-2 Ref Cell):
   - wasi.rs に ref-new/ref-get/ref-set をインライン Wasm 関数として生成
   - __alloc(16) で tag=7 オブジェクト確保
   - ref-get: I64Load(offset+8)、ref-set: I64Store(offset+8, value)

**対象ファイル**:
- `crates/lsharp-ir/src/lower/expr.rs`: Lambda Lifting 実装
- `crates/lsharp-ir/src/lower/mod.rs`: リフトされた関数の登録
- `crates/lsharp-wasm/src/wasi.rs`: funcref テーブル、call_indirect、Ref Cell ビルトイン
- `crates/lsharp-wasm/src/emit.rs`: CallIndirect 命令変換
- `crates/lsharp-types/src/infer.rs`: ref-new/ref-get/ref-set 型シグネチャ

**検証方法**:
- `cargo test` -- TEST-011/012/013 が GREEN + 既存テスト全パス

**依存**: Step 9 (TEST-011, TEST-012, TEST-013)

**並列実行**: TASK-011, TASK-012, TASK-013 は 3 Worker で同時実行可能

---

### Step 11: Phase 3 高階関数 + Phase 5 + Phase 6 テスト作成 (Group K)
**対象タスク**: TEST-014, TEST-015, TEST-016

**変更内容**:
1. TEST-014: P3-3 高階関数 E2E テスト -- list-map/list-filter/list-fold/vector-map/vector-filter
2. TEST-015: P5-1 WASI import テスト -- path_open/fd_read/fd_close/fd_seek/args_get/proc_exit
3. TEST-016: P6-1 モジュール探索ユニットテスト -- import -> ファイルパス解決

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: 高階関数・WASI テスト追加
- `crates/lsharp-ir/src/module_graph.rs`: モジュール探索テスト追加

**検証方法**:
- `cargo test` で TEST-014/015/016 が RED

**依存**: Step 10 (TASK-011 + TASK-009 for TEST-014), Step 4 (TASK-004 for TEST-015), Step 2 (TASK-001 for TEST-016)

**並列実行**: TEST-014, TEST-015, TEST-016 は 3 Worker で同時実行可能

---

### Step 12: Phase 3 高階関数 + Phase 5-1 + Phase 6-1 実装 (Group L)
**対象タスク**: TASK-014, TASK-015, TASK-016

**変更内容**:
1. TASK-014 (P3-3 高階関数):
   - list-map/list-filter/list-fold を L# ビルトインまたは stdlib で実装
   - vector-map/vector-filter を追加
   - クロージャ (call_indirect) を引数として受け取る関数の IR 生成
2. TASK-015 (P5-1 WASI import):
   - wasi.rs の emit_imports() に path_open/fd_read/fd_close/fd_seek/fd_filestat_get 追加
   - args_get/args_sizes_get/proc_exit 追加
3. TASK-016 (P6-1 モジュール探索):
   - `(import Foo)` -> `./Foo.ls`、`(import Foo.Bar)` -> `./Foo/Bar.ls` の規約実装
   - module_graph.rs にファイル探索ロジック追加

**対象ファイル**:
- `crates/lsharp-ir/src/lower/expr.rs`: 高階関数の IR 生成
- `crates/lsharp-wasm/src/wasi.rs`: WASI import 追加
- `crates/lsharp-ir/src/module_graph.rs`: ファイル探索ロジック

**検証方法**:
- `cargo test` -- TEST-014/015/016 が GREEN + 既存テスト全パス

**依存**: Step 11 (TEST-014, TEST-015, TEST-016)

**並列実行**: TASK-014, TASK-015, TASK-016 は 3 Worker で同時実行可能

---

### Step 13: Phase 5/6 後半テスト作成 (Group M)
**対象タスク**: TEST-017, TEST-018, TEST-019

**変更内容**:
1. TEST-017: P5-2 ファイル操作 E2E テスト -- read-file/write-file/file-exists?/引数取得
2. TEST-018: P6-2 クロスモジュール型ユニットテスト -- トポロジカルソート順コンパイル、export 型環境注入
3. TEST-019: P6-3 IR リンク E2E テスト -- マルチファイルプロジェクト実行

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: ファイル操作・マルチファイル E2E テスト
- `crates/lsharp-ir/src/module_graph.rs`: クロスモジュール型テスト
- `crates/lsharp-ir/src/lib.rs`: IR リンクテスト

**検証方法**:
- `cargo test` で TEST-017/018/019 が RED

**依存**: Step 12 (TASK-015 + TASK-005 for TEST-017, TASK-016 for TEST-018/019)

**並列実行**: TEST-017, TEST-018, TEST-019 は 3 Worker で同時実行可能

---

### Step 14: Phase 5-2/6-2/6-3 実装 (Group N)
**対象タスク**: TASK-017, TASK-018, TASK-019

**変更内容**:
1. TASK-017 (P5-2 ファイル操作):
   - wasi.rs に read-file/write-file/file-exists? をインライン Wasm 関数として生成
   - wasi_runner.rs に WasiCtxBuilder::preopened_dir() 追加
   - args_get/args_sizes_get でコマンドライン引数取得
2. TASK-018 (P6-2 クロスモジュール型環境):
   - エントリファイルから import 宣言を再帰的に収集
   - ModuleGraph でトポロジカルソート
   - 各モジュールをソート順にコンパイルし、export シンボルの TypeEnv を次モジュールに注入
3. TASK-019 (P6-3 IR リンク):
   - 既存 link_modules() を活用して全モジュール IR を結合
   - driver のコンパイルフローに統合

**対象ファイル**:
- `crates/lsharp-wasm/src/wasi.rs`: ファイル操作ビルトイン
- `crates/lsharp-wasm/src/wasi_runner.rs`: preopened_dir 追加
- `crates/lsharp-types/src/infer.rs`: クロスモジュール TypeEnv 注入
- `crates/lsharp-ir/src/module_graph.rs`: トポロジカルソート順コンパイル
- `crates/lsharp-driver/src/main.rs`: マルチファイルコンパイルフロー

**検証方法**:
- `cargo test` -- TEST-017/018/019 が GREEN + 既存テスト全パス

**依存**: Step 13 (TEST-017, TEST-018, TEST-019)

**並列実行**: TASK-017, TASK-018, TASK-019 は 3 Worker で同時実行可能

---

### Step 15: Phase 7 標準ライブラリテスト作成 (Group O)
**対象タスク**: TEST-020

**変更内容**:
1. stdlib の各モジュール (Core, String, List, Vector, Map, Set, IO, Debug, Char) のコンパイル・実行テスト

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: stdlib コンパイル・実行テスト追加

**検証方法**:
- `cargo test` で TEST-020 が RED

**依存**: Step 14 (TASK-017, TASK-018, TASK-019) + Step 10 (TASK-014)

---

### Step 16: Phase 7 標準ライブラリ実装 (Group P)
**対象タスク**: TASK-020, TASK-021, TASK-022

**変更内容**:
1. TASK-020 (P7-1): stdlib/Core.ls + stdlib/String.ls を L# で記述
2. TASK-021 (P7-2): stdlib/List.ls + stdlib/Vector.ls + stdlib/Set.ls を L# で記述
3. TASK-022 (P7-3): stdlib/Map.ls + stdlib/IO.ls + stdlib/Debug.ls + stdlib/Char.ls + stdlib 自動テスト

**対象ファイル**:
- `stdlib/Core.ls`, `stdlib/String.ls`, `stdlib/List.ls`, `stdlib/Vector.ls`, `stdlib/Map.ls`, `stdlib/Set.ls`, `stdlib/IO.ls`, `stdlib/Debug.ls`, `stdlib/Char.ls`: 全て新規作成

**検証方法**:
- `cargo test` -- TEST-020 が GREEN
- `cargo run -- test stdlib/` で stdlib テストパス

**依存**: Step 15 (TEST-020)

**並列実行**: TASK-020, TASK-021, TASK-022 は 3 Worker で同時実行可能

---

### Step 17: Phase 8 セルフホスティング前半テスト作成 (Group Q)
**対象タスク**: TEST-021, TEST-022

**変更内容**:
1. TEST-021: P8-1 L# Lexer テスト -- Token ADT、Rust 版との出力比較
2. TEST-022: P8-2 L# Parser テスト -- AST ADT、Rust 版との出力比較

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: セルフホスティング Lexer/Parser テスト

**検証方法**:
- `cargo test` で TEST-021/022 が RED

**依存**: Step 16 (TASK-020, TASK-021, TASK-022)

**並列実行**: TEST-021, TEST-022 は 2 Worker で同時実行可能

---

### Step 18: Phase 8 セルフホスティング前半実装 (Group R)
**対象タスク**: TASK-023, TASK-024

**変更内容**:
1. TASK-023 (P8-1 L# Lexer):
   - Token ADT 定義 (L# で記述)
   - 文字列走査による字句解析
   - Rust 版 lexer との出力比較テスト
2. TASK-024 (P8-2 L# Parser):
   - AST の ADT 定義 (L# で記述)
   - 再帰降下パーサー
   - Rust 版 parser との出力比較テスト

**対象ファイル**:
- `self-hosted/Lexer.ls`: L# Lexer 実装
- `self-hosted/Parser.ls`: L# Parser 実装

**検証方法**:
- `cargo test` -- TEST-021/022 が GREEN
- Rust 版との出力比較テストパス

**依存**: Step 17 (TEST-021, TEST-022)

**並列実行**: TASK-023, TASK-024 は 2 Worker で同時実行可能

---

### Step 19: Phase 8 後半テスト作成 (Group S)
**対象タスク**: TEST-023, TEST-024

**変更内容**:
1. TEST-023: P8-3 L# 型推論テスト -- 型 ADT、Substitution、Unification、Rust 版比較
2. TEST-024: P8-4 L# Codegen テスト -- IR ADT、LEB128、Wasm バイナリ生成、Rust 版比較

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: セルフホスティング型推論/Codegen テスト

**検証方法**:
- `cargo test` で TEST-023/024 が RED

**依存**: Step 18 (TASK-023, TASK-024)

**並列実行**: TEST-023, TEST-024 は 2 Worker で同時実行可能

---

### Step 20: Phase 8 後半実装 (Group T)
**対象タスク**: TASK-025, TASK-026

**変更内容**:
1. TASK-025 (P8-3 L# 型推論):
   - 型 ADT (Con, Var, Fun)
   - Substitution (HashMap ベース)
   - Unification アルゴリズム
   - let 多相 + 型注釈
2. TASK-026 (P8-4 L# Codegen):
   - IR ADT 定義
   - AST -> IR 変換
   - LEB128 エンコーディング
   - Wasm バイナリ生成

**対象ファイル**:
- `self-hosted/TypeInfer.ls`: L# 型推論実装
- `self-hosted/Codegen.ls`: L# Codegen 実装

**検証方法**:
- `cargo test` -- TEST-023/024 が GREEN

**依存**: Step 19 (TEST-023, TEST-024)

**並列実行**: TASK-025, TASK-026 は 2 Worker で同時実行可能

---

### Step 21: Phase 8 ブートストラップ検証 (Group U)
**対象タスク**: TEST-025, TASK-027

**変更内容**:
1. TEST-025: ブートストラップ検証テスト -- stage1.wasm -> stage2.wasm 固定点検証
2. TASK-027: Rust 版で stage1.wasm 生成 -> stage1 で stage2.wasm セルフコンパイル -> バイナリ一致確認

**対象ファイル**:
- `crates/lsharp-wasm/tests/e2e.rs`: ブートストラップ検証テスト
- CI 設定ファイル (自動化)

**検証方法**:
- stage1.wasm と stage2.wasm のバイナリ一致
- `cargo test` -- TEST-025 が GREEN

**依存**: Step 20 (TASK-025, TASK-026)

---

### Step 22: Phase 9 エコシステムテスト作成 (Group V)
**対象タスク**: TEST-026, TEST-027, TEST-028

**変更内容**:
1. TEST-026: P9-1 REPL インテグレーションテスト
2. TEST-027: P9-2 LSP インテグレーションテスト
3. TEST-028: P9-3 パッケージマネージャユニットテスト

**対象ファイル**:
- `crates/lsharp-driver/tests/`: REPL テスト
- `crates/lsharp-lsp/tests/`: LSP テスト
- `crates/lsharp-driver/tests/`: パッケージマネージャテスト

**検証方法**:
- `cargo test` で TEST-026/027/028 が RED

**依存**: Step 21 (TASK-027)

**並列実行**: TEST-026, TEST-027, TEST-028 は 3 Worker で同時実行可能

---

### Step 23: Phase 9 エコシステム実装 (Group W)
**対象タスク**: TASK-028, TASK-029, TASK-030

**変更内容**:
1. TASK-028 (P9-1 REPL):
   - lsharp repl サブコマンド追加
   - readline 統合 (rustyline)
   - wasmtime インプロセス実行
2. TASK-029 (P9-2 LSP):
   - crates/lsharp-lsp クレート新規作成
   - tower-lsp 統合
   - 型ホバー、エラー診断 (miette 連携)、定義ジャンプ
3. TASK-030 (P9-3 パッケージマネージャ):
   - lsharp.toml [dependencies] セクション解析
   - Git リポジトリベース依存解決
   - ロックファイル生成

**対象ファイル**:
- `crates/lsharp-driver/src/main.rs`: repl/lsp/pkg サブコマンド
- `crates/lsharp-lsp/`: 新規クレート
- `Cargo.toml` (workspace): lsharp-lsp 追加

**検証方法**:
- `cargo test` -- TEST-026/027/028 が GREEN

**依存**: Step 22 (TEST-026, TEST-027, TEST-028)

**並列実行**: TASK-028, TASK-029, TASK-030 は 3 Worker で同時実行可能

---

### Step 24: Phase 9 ドキュメント生成 (Group X)
**対象タスク**: TEST-029, TASK-031

**変更内容**:
1. TEST-029: ドキュメント生成テスト -- :doc メタデータから HTML 生成
2. TASK-031: lsharp-docs クレート活用、型シグネチャ・例の自動抽出

**対象ファイル**:
- `crates/lsharp-docs/src/`: ドキュメント生成ロジック追加

**検証方法**:
- `cargo test` -- TEST-029 が GREEN
- サンプルプロジェクトで HTML ドキュメント生成を確認

**依存**: Step 23 (TASK-028)

---

## 4. 依存関係図

```
Step 1 (Group A: P0 テスト) [3 Worker 並列]
    |
Step 2 (Group B: P0 基盤前半) [3 Worker 並列]
    |
Step 3 (Group C: P0-3 テスト)
    |
Step 4 (Group D: P0-3 タグ付きワード)
    |
Step 5 (Group E: P1/P2-1/P3-1 テスト) [3 Worker 並列]
    |
Step 6 (Group F: P1-1/P2-1/P3-1 実装) [3 Worker 並列]
    |
Step 7 (Group G: P1後半/P2コレクション テスト) [3 Worker 並列]
    |
Step 8 (Group H: P1後半/P2コレクション 実装) [3 Worker 並列]
    |
Step 9 (Group I: P3クロージャ/P4 テスト) [3 Worker 並列]
    |
Step 10 (Group J: P3クロージャ/P4 実装) [3 Worker 並列]
    |
Step 11 (Group K: P3高階関数/P5/P6 テスト) [3 Worker 並列]
    |
Step 12 (Group L: P3高階関数/P5-1/P6-1 実装) [3 Worker 並列]
    |
Step 13 (Group M: P5/P6 後半テスト) [3 Worker 並列]
    |
Step 14 (Group N: P5-2/P6-2/P6-3 実装) [3 Worker 並列]
    |
Step 15 (Group O: P7 テスト)
    |
Step 16 (Group P: P7 実装) [3 Worker 並列]
    |
Step 17 (Group Q: P8 前半テスト) [2 Worker 並列]
    |
Step 18 (Group R: P8 前半実装) [2 Worker 並列]
    |
Step 19 (Group S: P8 後半テスト) [2 Worker 並列]
    |
Step 20 (Group T: P8 後半実装) [2 Worker 並列]
    |
Step 21 (Group U: P8 ブートストラップ)
    |
Step 22 (Group V: P9 テスト) [3 Worker 並列]
    |
Step 23 (Group W: P9 実装) [3 Worker 並列]
    |
Step 24 (Group X: P9 ドキュメント生成)
```

## 5. リスクと対策

### 5.1 技術的リスク
| リスク | 影響度 | 対策 |
|--------|-------|------|
| lower.rs 分割時に insta スナップショットパスが壊れる | 高 | `cargo insta review` でスナップショットを再承認。分割前にスナップショットファイル一覧を記録 |
| Bump Allocator の heap_ptr 初期値が文字列定数データと衝突 | 高 | string_offset の最終値を __alloc の初期値に設定。E2E テストで文字列定数 + 動的確保の組合せを検証 |
| 既存 422 テストの回帰 | 高 | 各 Step 完了時に `cargo test` を必ず実行。CI パイプラインで自動検証 |
| wasi.rs のファイルサイズ肥大化 (ビルトイン関数追加で 500-800 行超過) | 中 | Phase 2 完了時点で wasi.rs をモジュール分割 (wasi/mod.rs, wasi/builtins.rs, wasi/allocator.rs) |
| タグ付きワードと既存 i64 演算の互換性 | 高 | 即値整数は上位ビット = 0 なので既存の i64 演算がそのまま動作することを検証。ただし 63-bit オーバーフローのエッジケースに注意 |
| セルフホスティング (P8) で L# の表現力不足が判明 | 高 | Phase 7 で stdlib を実装する過程で表現力を検証。不足機能は P7 完了前に追加 |
| call_indirect のテーブル管理が複雑 | 中 | funcref テーブルのインデックス管理を Lower struct のフィールドに集約。テスト駆動で段階的に実装 |
| HashMap の FNV-1a ハッシュ衝突率が高い | 低 | チェイン法で衝突を処理。初期容量 16 + 負荷率 0.75 で拡張。パフォーマンステストで検証 |

### 5.2 フォールバックプラン
- **lower.rs 分割が困難な場合**: 段階的に分割 (まずテストだけ分離 -> expr 分離 -> pattern 分離 -> decl 分離)
- **wasi.rs 肥大化**: ビルトイン関数を別ファイル (wasi/builtins/) に分割し、モジュール単位で管理
- **セルフホスティングが困難な場合**: 最小サブセット (算術式 + let + if のみ) でブートストラップを検証し、段階的に機能を追加
- **LSP 実装が複雑な場合**: 型ホバーのみを最小 MVP として実装し、エラー診断・定義ジャンプは後続タスクに分割

## 6. 検証計画

### 6.1 静的解析
- [ ] `cargo clippy` -- 全 Phase を通じて警告 0 件
- [ ] `cargo build` -- コンパイルエラー 0 件
- [ ] ファイルサイズ 500-800 行チェック (lower.rs 分割後の各ファイル、wasi.rs の監視)

### 6.2 テスト
- [ ] `cargo test` -- 各 Step 完了時に全テストパス
- [ ] 新規テストの追加数: 各 Phase で E2E 5-20 テスト + ユニットテスト
- [ ] insta スナップショットの整合性確認

### 6.3 手動確認
- [ ] Phase 0 完了時: サンプルプログラムで Bump Allocator の動的メモリ確保が動作
- [ ] Phase 1 完了時: 文字列操作を含むプログラムが正しく実行
- [ ] Phase 3 完了時: クロージャを使った高階関数プログラムが動作
- [ ] Phase 6 完了時: マルチファイルプロジェクトのコンパイル・実行
- [ ] Phase 8 完了時: stage1.wasm と stage2.wasm のバイナリ一致
- [ ] Phase 9 完了時: REPL で式を入力して結果が表示される

## 7. 次フェーズへの引き継ぎ

### 最初に実行すべきステップ
1. Step 1: Phase 0 テスト作成 (Group A) -- TEST-001, TEST-002, TEST-003 を 3 Worker で並列実行

### 注意点
- **TDD 必須**: テストを先に書いてから実装すること。テストなしの実装は完了と見なさない
- **既存テスト回帰**: 各ステップ完了後に必ず `cargo test` で 422+ テストが全パスすることを確認
- **ファイルサイズ監視**: wasi.rs はビルトイン関数追加で肥大化するリスクが高い。500 行を超えた時点でモジュール分割を検討
- **Phase 間の強い依存関係**: P0 -> P1 -> P2 -> P3 は厳密に順序依存。Step をスキップしない
- **infer.rs (3008行)**: 既にファイルサイズ制限を超過しているが、TODO には含まれていない。ビルトイン型シグネチャ追加時にさらに肥大化するリスクがある

## 8. 完了条件（v2.13.0+）

### 8.1 タスク全体の完了条件

以下を**全て**満たした場合にタスク完了:

- [ ] Phase 0: lower.rs が 5 ファイルに分割され、Bump Allocator で動的メモリ確保が動作し、タグ付きワードで型判別が可能
- [ ] Phase 1: 文字列ビルトイン 7 関数が E2E テストで動作し、print が多相化
- [ ] Phase 2: ADT リニアメモリ版、Vector、HashMap が E2E テストで動作し、Cons リストが実行可能
- [ ] Phase 3: クロージャが自由変数をキャプチャし、高階関数 (map/filter/fold) が E2E テストで動作
- [ ] Phase 4: Option/Result がランタイムで動作し、Ref Cell による可変状態が使用可能
- [ ] Phase 5: ファイル読み書き、コマンドライン引数取得が E2E テストで動作
- [ ] Phase 6: 複数ファイルの L# プロジェクトがコンパイル・実行可能
- [ ] Phase 7: 標準ライブラリ 9 モジュールが L# で記述され、自動テストがパス
- [ ] Phase 8: L# コンパイラが自身をコンパイルし、stage1.wasm と stage2.wasm が一致 (固定点)
- [ ] Phase 9: REPL、LSP、パッケージマネージャ、ドキュメント生成が動作
- [ ] 全テストが Pass する (`cargo test` で 0 failures)
- [ ] `cargo clippy` で警告 0 件

### 8.2 AI 自律実行可能性チェック

| 項目 | 状態 | 詳細 |
|------|------|------|
| 完了条件の明文化 | OK | 全 24 ステップに検証方法を記載 |
| 具体的アクション | OK | 各ステップに変更対象ファイル・変更内容を明記 |
| 判断基準の明示 | OK | TDD (RED -> GREEN) で判断基準を統一 |
| 成果物の定義 | OK | 全ファイル名・変更箇所を特定済み |
| 検証方法の明示 | OK | 各ステップに `cargo test` ベースの検証方法を記載 |

**判定結果**: Ready

### 8.3 曖昧表現チェック

以下の曖昧表現を検査した結果、全て排除済み:

| 曖昧表現 | 検出結果 |
|---------|---------|
| 「検討する」「考慮する」 | 不使用 |
| 「適宜」「必要に応じて」 | 1 箇所 -> 「wasi.rs が 500 行を超えた時点でモジュール分割する」に修正済み |
| 「等」「など」 | 不使用 (全項目を列挙済み) |
| 「可能であれば」 | 不使用 |
| 「適切に」「正しく」 | 不使用 |

### 8.4 完了条件チェックの実施タイミング

```
IMPLEMENTATION_PLAN.md 作成完了
    |
8.2 AI 自律実行可能性チェック実行
    +-- 全項目 OK -> Phase 5 (prompt) へ進む
```

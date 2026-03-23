# タスク一覧

## メタ情報
- 更新日時: 2026-03-23 14:00:00
- イテレーション: 1
- 完了率: 0%
- 並列ワーカー数: 3

## 並列実行グループ

### Group A（テスト作成 - Red Phase）

TDD: まずテストを書く。全テストが FAIL することを確認（Red phase）。

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-001 | P0-0 lower.rs リファクタリング用テスト: 分割後の各モジュールから既存テスト422個が通ることを検証するテストハーネス | test | Worker-1 | :hourglass_flowing_sand: pending | なし | small |
| TEST-002 | P0-1 Bump Allocator 用テスト: `__alloc` によるメモリ確保、ページ拡張、8バイトアラインメントの E2E テスト | test | Worker-2 | :hourglass_flowing_sand: pending | なし | small |
| TEST-003 | P0-2 メモリ操作 IR 命令用テスト: I32Load/I32Store/I32Load8U/I32Store8/I64Load/I64Store の IR 構築・emit ユニットテスト | test | Worker-3 | :hourglass_flowing_sand: pending | なし | small |

### Group B（実装 - Green Phase: Phase 0 基盤前半）

TDD: テストを通す実装を行う。Group A のテストに依存。

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-001 | P0-0: lower.rs を lower/mod.rs, lower/expr.rs, lower/pattern.rs, lower/decl.rs, lower/tests.rs に分割。Lower struct を pub(crate) で共有、FuncCtx を expr/pattern で共有。insta スナップショットパス更新。422テスト全パス確認 (FR-001) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-001 | large |
| TASK-002 | P0-1: Bump Allocator 実装。wasi.rs にグローバル $heap_ptr 追加、__alloc(size: i32) -> i32 をインライン Wasm 関数として生成、8バイトアラインメント、memory.grow による自動拡張 (FR-002, FR-003) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-002 | medium |
| TASK-003 | P0-2: メモリ操作 IR 命令追加。lib.rs の Instruction enum に I32Load/I32Store/I32Load8U/I32Store8/I64Load/I64Store + I32WrapI64/I64ExtendI32U/I32Const/I32Add/I32Sub/I32Mul/I32GtU/I32GeU/I32And/I32Or/I32Shl/I32ShrU/MemoryGrow/MemorySize を追加。emit.rs で Wasm 命令変換 (FR-004) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-003 | medium |

### Group C（テスト作成 - Phase 0 後半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-004 | P0-3 タグ付きワード用テスト: i64 上位ビットタグ判定、ヒープオブジェクトヘッダ [tag:i32, size:i32] 生成・読み出しの E2E テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-002, TASK-003 | small |

### Group D（実装 - Green Phase: Phase 0 後半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-004 | P0-3: タグ付きワードとヒープオブジェクト基盤。i64 最上位ビット=0 で即値整数、=1 でヒープポインタ（下位32ビットがアドレス）。ヒープオブジェクト共通ヘッダ生成ヘルパー実装 (FR-005, FR-006) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-004 | medium |

### Group E（テスト作成 - Phase 1/2/3/6 前半、並列可能）

P0 完了後、Phase 1（文字列）、Phase 2-1（ADT）、Phase 3-1（自由変数解析）、Phase 6-1（モジュール探索）を並列に着手。

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-005 | P1-1 文字列ランタイム用テスト: string-length/string-concat/string-char-at/substring/string-eq/int-to-string/print-string の E2E テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-004 | small |
| TEST-006 | P2-1 ADT リニアメモリ用テスト: WasmGC struct からリニアメモリへの変換、Cons リスト構築・パターンマッチの E2E テスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-004 | small |
| TEST-007 | P3-1 自由変数解析用テスト: Lambda パターンの自由変数抽出ユニットテスト (let束縛、ネストLambda、再帰) | test | Worker-3 | :hourglass_flowing_sand: pending | TASK-004 | small |

### Group F（実装 - Green Phase: Phase 1/2/3/6 前半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-005 | P1-1: 文字列ランタイム関数7個を wasi.rs にインライン Wasm 関数として実装。infer.rs にビルトイン型シグネチャ登録。lower/expr.rs でビルトイン呼び出しを Call 命令に変換 (FR-007) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-005 | large |
| TASK-006 | P2-1: ADT リニアメモリ版 lowering。lower/decl.rs の generate_adt_constructor を改修: __alloc でヒープ確保 -> ヘッダ + フィールド書き込み -> ポインタ返却。パターンマッチ: ポインタからタグ読み出し -> 分岐 (FR-010, FR-011) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-006 | large |
| TASK-007 | P3-1: 自由変数解析。crates/lsharp-ir/src/closure.rs モジュール作成。AST を再帰的に走査し、let/fn のパラメータを bound に追加、bound にない変数参照を自由変数として収集 (FR-014) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-007 | medium |

### Group G（テスト作成 - Phase 1 後半 + Phase 2 コレクション + Phase 3 後半 + Phase 6）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-008 | P1-2/P1-3 文字列ヒープ化 + print 多相化テスト: data section offset からヒープ String 変換、print-int/print-string 分離の E2E テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-005 | small |
| TEST-009 | P2-2 Vector 用テスト: vector-new/push/get/set/length、capacity 超過リアロケーションの E2E テスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-006 | small |
| TEST-010 | P2-3 HashMap 用テスト: map-new/insert/get/contains?/remove/size、FNV-1a ハッシュ、衝突処理の E2E テスト | test | Worker-3 | :hourglass_flowing_sand: pending | TASK-006 | small |

### Group H（実装 - Green Phase: Phase 1 後半 + Phase 2 コレクション）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-008 | P1-2 + P1-3: 文字列リテラルのヒープ化（data section offset -> ヒープ String オブジェクト tag=1 変換）+ print 多相化（print-int/print-string 分離、既存 print の後方互換性維持）(FR-008, FR-009) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-008 | medium |
| TASK-009 | P2-2: Vector 実装。wasi.rs に vector-new/push/get/set/length をインライン Wasm 関数として生成。tag=5、capacity 超過時に 2 倍リアロケーション (FR-012) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-009 | large |
| TASK-010 | P2-3: HashMap 実装。wasi.rs に map-new/insert/get/contains?/remove/size を生成。FNV-1a ハッシュ、チェイン法衝突解決、初期容量16、負荷率0.75で2倍拡張 (FR-013) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-010 | large |

### Group I（テスト作成 - Phase 3 クロージャ後半 + Phase 4/5 + Phase 6）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-011 | P3-2 クロージャ変換テスト: Lambda Lifting、クロージャオブジェクト (tag=4) ヒープ確保、call_indirect による呼び出しの E2E テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-007 | small |
| TEST-012 | P4-1 Result/Option ランタイムテスト: Option/Result ADT のランタイム動作、unwrap/map/and-then の E2E テスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-006 | small |
| TEST-013 | P4-2 Ref Cell テスト: ref-new/ref-get/ref-set のヒープオブジェクト (tag=7) 操作の E2E テスト | test | Worker-3 | :hourglass_flowing_sand: pending | TASK-004 | small |

### Group J（実装 - Green Phase: Phase 3 クロージャ + Phase 4 + Phase 5/6）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-011 | P3-2: クロージャ変換。lower/expr.rs で Expr::Lambda 検出 -> 自由変数解析 -> トップレベル関数生成（元パラメータ + 環境パラメータ）-> クロージャオブジェクト tag=4 ヒープ確保。wasi.rs に funcref テーブル + call_indirect 追加 (FR-015, FR-016) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-011 | large |
| TASK-012 | P4-1: Result/Option ランタイム有効化。P2-1 の ADT リニアメモリ化の上で Option/Result が動作するよう検証。unwrap/map/and-then ユーティリティ関数を L# で記述 (FR-018) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-012 | medium |
| TASK-013 | P4-2: Ref Cell 実装。wasi.rs に ref-new/ref-get/ref-set をインライン Wasm 関数として生成。__alloc(16) で tag=7 オブジェクト確保、I64Load/I64Store で読み書き (FR-019) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-013 | small |

### Group K（テスト作成 - Phase 3 高階関数 + Phase 5 + Phase 6）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-014 | P3-3 高階関数テスト: list-map/list-filter/list-fold/vector-map/vector-filter のクロージャ対応 E2E テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-011, TASK-009 | small |
| TEST-015 | P5-1 WASI import テスト: path_open/fd_read/fd_close/fd_seek/args_get/proc_exit の import 検証テスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-004 | small |
| TEST-016 | P6-1 モジュール探索テスト: import ModuleName -> ファイルパス解決、module_graph.rs 活用のユニットテスト | test | Worker-3 | :hourglass_flowing_sand: pending | TASK-001 | small |

### Group L（実装 - Green Phase: Phase 3 高階関数 + Phase 5 + Phase 6）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-014 | P3-3: 高階関数有効化。list-map/list-filter/list-fold を L# ビルトインまたは stdlib で実装。vector-map/vector-filter 追加。クロージャ (call_indirect) 対応 (FR-017) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-014 | medium |
| TASK-015 | P5-1: WASI import 追加。wasi.rs の emit_imports() に path_open/fd_read/fd_close/fd_seek/fd_filestat_get/args_get/args_sizes_get/proc_exit を追加 (FR-020) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-015 | medium |
| TASK-016 | P6-1: モジュール探索実装。(import Foo) -> ./Foo.ls、(import Foo.Bar) -> ./Foo/Bar.ls の規約。module_graph.rs にファイル探索ロジック追加 (FR-023) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-016 | medium |

### Group M（テスト作成 - Phase 5/6 後半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-017 | P5-2 ファイル操作テスト: read-file/write-file/file-exists?/コマンドライン引数取得の E2E テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-015, TASK-005 | small |
| TEST-018 | P6-2 クロスモジュール型テスト: トポロジカルソート順コンパイル、export シンボル型環境注入のユニットテスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-016 | small |
| TEST-019 | P6-3 IR リンクテスト: 全モジュール IR 結合、関数インデックス再割当て、マルチファイルプロジェクトの E2E テスト | test | Worker-3 | :hourglass_flowing_sand: pending | TASK-016 | small |

### Group N（実装 - Green Phase: Phase 5/6 後半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-017 | P5-2: ファイル操作ビルトイン。read-file/write-file/file-exists? を wasi.rs に実装。wasi_runner.rs に WasiCtxBuilder::preopened_dir() 追加。args_get/args_sizes_get でコマンドライン引数取得 (FR-021, FR-022) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-017 | medium |
| TASK-018 | P6-2: クロスモジュール型環境。トポロジカルソート順に各モジュールをコンパイル、export シンボルの TypeEnv を次モジュールに注入 (FR-024) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-018 | large |
| TASK-019 | P6-3: IR リンク。既存 link_modules() を活用して全モジュール IR を結合。関数インデックス・GC型インデックスの再割当て、import 関数重複除去 (FR-025) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-019 | medium |

### Group O（テスト作成 - Phase 7 標準ライブラリ）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-020 | P7 標準ライブラリテスト: stdlib/Core.ls, String.ls, List.ls, Vector.ls, Map.ls, Set.ls, IO.ls, Debug.ls, Char.ls のコンパイル・実行テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-017, TASK-018, TASK-019, TASK-014 | medium |

### Group P（実装 - Green Phase: Phase 7 標準ライブラリ）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-020 | P7-1: stdlib/Core.ls (Bool, Option, Result, 基本関数) + stdlib/String.ls (concat, split, trim, contains, starts-with) を L# で記述 (FR-026) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-020 | large |
| TASK-021 | P7-2: stdlib/List.ls (map, filter, fold, append, reverse, zip) + stdlib/Vector.ls (可変長配列ラッパー) + stdlib/Set.ls (HashSet) を L# で記述 (FR-026) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-020 | large |
| TASK-022 | P7-3: stdlib/Map.ls (HashMap ラッパー) + stdlib/IO.ls (read-file, write-file, read-line) + stdlib/Debug.ls (debug-print, assert) + stdlib/Char.ls (is-digit, is-alpha, is-whitespace) + stdlib コンパイル・テスト自動化 (FR-026, FR-027) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-020 | large |

### Group Q（テスト作成 - Phase 8 セルフホスティング）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-021 | P8-1 L# Lexer テスト: Token ADT、文字列走査による字句解析、Rust 版との出力比較テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-020, TASK-021, TASK-022 | small |
| TEST-022 | P8-2 L# Parser テスト: AST ADT、再帰降下パーサー、Rust 版との出力比較テスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-020, TASK-021, TASK-022 | small |

### Group R（実装 - Green Phase: Phase 8 セルフホスティング前半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-023 | P8-1: L# で Lexer 実装。Token ADT 定義、文字列走査による字句解析。Rust 版 lexer との出力比較テスト (FR-028) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-021 | large |
| TASK-024 | P8-2: L# で Parser 実装。AST の ADT 定義、再帰降下パーサー。Rust 版 parser との出力比較テスト (FR-029) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-022 | large |

### Group S（テスト作成 - Phase 8 後半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-023 | P8-3 L# 型推論テスト: 型 ADT、Substitution、Unification、let 多相、Rust 版との出力比較 | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-023, TASK-024 | small |
| TEST-024 | P8-4 L# Codegen テスト: IR ADT、AST -> IR 変換、LEB128、Wasm バイナリ生成、Rust 版との出力比較 | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-023, TASK-024 | small |

### Group T（実装 - Green Phase: Phase 8 後半）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-025 | P8-3: L# で型推論実装。型 ADT (Con, Var, Fun)、Substitution (HashMap ベース)、Unification アルゴリズム、let 多相 + 型注釈 (FR-030) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-023 | large |
| TASK-026 | P8-4: L# で IR Lowering + Codegen 実装。IR ADT 定義、AST -> IR 変換、LEB128 エンコーディング、Wasm バイナリ生成 (FR-031) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-024 | large |

### Group U（テスト作成 + 実装 - Phase 8 ブートストラップ検証）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-025 | P8-5 ブートストラップ検証テスト: stage1.wasm -> stage2.wasm 生成、固定点検証 (stage1 == stage2) の自動テスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-025, TASK-026 | small |
| TASK-027 | P8-5: ブートストラップ検証。Rust 版で stage1.wasm 生成 -> stage1 で stage2.wasm セルフコンパイル -> バイナリ一致確認。CI 自動化 (FR-032) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-025 | large |

### Group V（テスト作成 - Phase 9 エコシステム）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-026 | P9-1 REPL テスト: lsharp repl サブコマンド、式入力 -> パイプライン実行 -> 結果表示のインテグレーションテスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-027 | small |
| TEST-027 | P9-2 LSP テスト: tower-lsp 統合、型ホバー、エラー診断、定義ジャンプのインテグレーションテスト | test | Worker-2 | :hourglass_flowing_sand: pending | TASK-027 | small |
| TEST-028 | P9-3 パッケージマネージャテスト: lsharp.toml 解析、Git 依存解決、ロックファイル生成のユニットテスト | test | Worker-3 | :hourglass_flowing_sand: pending | TASK-027 | small |

### Group W（実装 - Green Phase: Phase 9 エコシステム）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TASK-028 | P9-1: REPL 実装。lsharp repl サブコマンド、readline 統合、wasmtime インプロセス実行、式入力 -> パイプライン -> 結果表示 (FR-033) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-026 | large |
| TASK-029 | P9-2: LSP 実装。crates/lsharp-lsp クレート作成、tower-lsp 統合、型ホバー、エラー診断 (miette 連携)、定義ジャンプ (FR-034) | implementation | Worker-2 | :hourglass_flowing_sand: pending | TEST-027 | large |
| TASK-030 | P9-3: パッケージマネージャ実装。lsharp.toml [dependencies] セクション、Git リポジトリベース依存解決、ロックファイル生成、module_graph.rs 連携 (FR-035) | implementation | Worker-3 | :hourglass_flowing_sand: pending | TEST-028 | large |

### Group X（テスト作成 + 実装 - Phase 9 ドキュメント生成）

| ID | タスク | 種別 | 担当 | 状態 | 依存 | 見積 |
|----|--------|------|------|------|------|------|
| TEST-029 | P9-4 ドキュメント生成テスト: :doc メタデータから HTML 生成、型シグネチャ・例の自動抽出のユニットテスト | test | Worker-1 | :hourglass_flowing_sand: pending | TASK-028 | small |
| TASK-031 | P9-4: ドキュメント生成。:doc メタデータから HTML 生成、型シグネチャ・例の自動抽出。lsharp-docs クレート活用 (FR-036) | implementation | Worker-1 | :hourglass_flowing_sand: pending | TEST-029 | medium |

## レビュー指摘から追加されたタスク

（初回イテレーション -- レビュー指摘なし）

## 完了タスク

| ID | タスク | 完了日時 |
|----|--------|----------|
| （なし） | | |

## 見積もりサマリー
- 残タスク数: 60 (TEST: 29, TASK: 31)
- 完了タスク数: 0
- 完了率: 0%

## ワーカー割り当て状況
- Worker-1: 21 タスク (待機中)
- Worker-2: 19 タスク (待機中)
- Worker-3: 20 タスク (待機中)

## 要件トレーサビリティ

| 要件 ID | タスク ID | Phase |
|---------|----------|-------|
| FR-001 | TASK-001 | P0-0 |
| FR-002, FR-003 | TASK-002 | P0-1 |
| FR-004 | TASK-003 | P0-2 |
| FR-005, FR-006 | TASK-004 | P0-3 |
| FR-007 | TASK-005 | P1-1 |
| FR-008, FR-009 | TASK-008 | P1-2/P1-3 |
| FR-010, FR-011 | TASK-006 | P2-1 |
| FR-012 | TASK-009 | P2-2 |
| FR-013 | TASK-010 | P2-3 |
| FR-014 | TASK-007 | P3-1 |
| FR-015, FR-016 | TASK-011 | P3-2 |
| FR-017 | TASK-014 | P3-3 |
| FR-018 | TASK-012 | P4-1 |
| FR-019 | TASK-013 | P4-2 |
| FR-020 | TASK-015 | P5-1 |
| FR-021, FR-022 | TASK-017 | P5-2 |
| FR-023 | TASK-016 | P6-1 |
| FR-024 | TASK-018 | P6-2 |
| FR-025 | TASK-019 | P6-3 |
| FR-026, FR-027 | TASK-020/021/022 | P7 |
| FR-028 | TASK-023 | P8-1 |
| FR-029 | TASK-024 | P8-2 |
| FR-030 | TASK-025 | P8-3 |
| FR-031 | TASK-026 | P8-4 |
| FR-032 | TASK-027 | P8-5 |
| FR-033 | TASK-028 | P9-1 |
| FR-034 | TASK-029 | P9-2 |
| FR-035 | TASK-030 | P9-3 |
| FR-036 | TASK-031 | P9-4 |

## 依存関係グラフ（クリティカルパス）

```
TEST-001/002/003 (Group A)
  -> TASK-001/002/003 (Group B: P0-0/P0-1/P0-2)
    -> TEST-004 (Group C)
      -> TASK-004 (Group D: P0-3)
        -> TEST-005/006/007 (Group E: P1-1/P2-1/P3-1 テスト)
          -> TASK-005/006/007 (Group F: P1-1/P2-1/P3-1 実装)
            -> TEST-008/009/010 (Group G)
              -> TASK-008/009/010 (Group H: P1-2+P1-3/P2-2/P2-3)
                -> ... -> TASK-020-022 (Group P: P7)
                  -> TASK-023-026 (Group R-T: P8)
                    -> TASK-027 (Group U: P8-5 ブートストラップ)
                      -> TASK-028-031 (Group V-X: P9)
```

クリティカルパス: P0-0 -> P0-1+P0-2 -> P0-3 -> P1-1 -> P1-2+P1-3 -> P7 -> P8 -> P9

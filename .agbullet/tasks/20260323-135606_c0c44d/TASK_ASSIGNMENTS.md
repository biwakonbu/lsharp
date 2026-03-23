# Task Assignments

## 概要
- 総タスク数: 60 (TEST: 29, TASK: 31)
- 最大並列数: 50
- グループ数: 24 (Group A ~ X)

## AI 割当サマリー
| AI | タスク数 | 特性 |
|----|---------|------|
| claude-code | 44 | 難易度重視 (コンパイラ中核、アルゴリズム、型推論、テスト設計) |
| gemini-cli | 16 | 物量重視 (stdlib 記述、定型ビルトイン、テンプレート的タスク) |

## タスク一覧

### Group A（依存なし -- P0 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-001 | P0-0 lower.rs リファクタリング用テスト: 分割後の各モジュールから既存テスト422個が通ることを検証するテストハーネス | claude-code | crates/lsharp-wasm/tests/e2e.rs, crates/lsharp-ir/src/lower/tests.rs | small |
| TEST-002 | P0-1 Bump Allocator 用テスト: `__alloc` によるメモリ確保、ページ拡張、8バイトアラインメントの E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-003 | P0-2 メモリ操作 IR 命令用テスト: I32Load/I32Store/I32Load8U/I32Store8/I64Load/I64Store の IR 構築・emit ユニットテスト | claude-code | crates/lsharp-ir/src/lib.rs, crates/lsharp-wasm/src/emit.rs | small |

### Group B（Group A 完了後 -- P0 基盤前半実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-001 | P0-0: lower.rs を lower/mod.rs, lower/expr.rs, lower/pattern.rs, lower/decl.rs, lower/tests.rs に分割。Lower struct を pub(crate) で共有。insta スナップショットパス更新。422テスト全パス確認 (FR-001) | claude-code | crates/lsharp-ir/src/lower.rs -> crates/lsharp-ir/src/lower/ (5 files) | large |
| TASK-002 | P0-1: Bump Allocator 実装。wasi.rs にグローバル $heap_ptr 追加、__alloc(size: i32) -> i32 をインライン Wasm 関数として生成、8バイトアラインメント、memory.grow による自動拡張 (FR-002, FR-003) | claude-code | crates/lsharp-wasm/src/wasi.rs | medium |
| TASK-003 | P0-2: メモリ操作 IR 命令追加。lib.rs の Instruction enum に I32Load/I32Store/I32Load8U/I32Store8/I64Load/I64Store + I32 系命令を追加。emit.rs で Wasm 命令変換 (FR-004) | claude-code | crates/lsharp-ir/src/lib.rs, crates/lsharp-wasm/src/emit.rs | medium |

### Group C（Group B 完了後 -- P0-3 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-004 | P0-3 タグ付きワード用テスト: i64 上位ビットタグ判定、ヒープオブジェクトヘッダ [tag:i32, size:i32] 生成・読み出しの E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |

### Group D（Group C 完了後 -- P0-3 実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-004 | P0-3: タグ付きワードとヒープオブジェクト基盤。i64 最上位ビット=0 で即値整数、=1 でヒープポインタ。ヒープオブジェクト共通ヘッダ生成ヘルパー実装 (FR-005, FR-006) | claude-code | crates/lsharp-ir/src/lower/expr.rs, crates/lsharp-ir/src/lower/mod.rs, crates/lsharp-wasm/src/wasi.rs | medium |

### Group E（Group D 完了後 -- P1/P2-1/P3-1 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-005 | P1-1 文字列ランタイム用テスト: string-length/string-concat/string-char-at/substring/string-eq/int-to-string/print-string の E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-006 | P2-1 ADT リニアメモリ用テスト: WasmGC struct からリニアメモリへの変換、Cons リスト構築・パターンマッチの E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-007 | P3-1 自由変数解析用テスト: Lambda パターンの自由変数抽出ユニットテスト (let束縛、ネストLambda、再帰) | claude-code | crates/lsharp-ir/src/closure.rs | small |

### Group F（Group E 完了後 -- P1-1/P2-1/P3-1 実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-005 | P1-1: 文字列ランタイム関数7個を wasi.rs にインライン Wasm 関数として実装。infer.rs にビルトイン型シグネチャ登録。lower/expr.rs でビルトイン呼び出しを Call 命令に変換 (FR-007) | claude-code | crates/lsharp-wasm/src/wasi.rs, crates/lsharp-types/src/infer.rs, crates/lsharp-ir/src/lower/expr.rs | large |
| TASK-006 | P2-1: ADT リニアメモリ版 lowering。lower/decl.rs の generate_adt_constructor を改修: __alloc でヒープ確保 -> ヘッダ + フィールド書き込み -> ポインタ返却。パターンマッチ: ポインタからタグ読み出し -> 分岐 (FR-010, FR-011) | claude-code | crates/lsharp-ir/src/lower/decl.rs, crates/lsharp-ir/src/lower/pattern.rs | large |
| TASK-007 | P3-1: 自由変数解析。crates/lsharp-ir/src/closure.rs モジュール作成。AST を再帰的に走査し、自由変数を収集 (FR-014) | claude-code | crates/lsharp-ir/src/closure.rs | medium |

### Group G（Group F 完了後 -- P1 後半/P2 コレクション テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-008 | P1-2/P1-3 文字列ヒープ化 + print 多相化テスト: data section offset からヒープ String 変換、print-int/print-string 分離の E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-009 | P2-2 Vector 用テスト: vector-new/push/get/set/length、capacity 超過リアロケーションの E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-010 | P2-3 HashMap 用テスト: map-new/insert/get/contains?/remove/size、FNV-1a ハッシュ、衝突処理の E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |

### Group H（Group G 完了後 -- P1 後半/P2 コレクション実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-008 | P1-2 + P1-3: 文字列リテラルのヒープ化 + print 多相化 (FR-008, FR-009) | claude-code | crates/lsharp-wasm/src/wasi.rs, crates/lsharp-ir/src/lower/expr.rs | medium |
| TASK-009 | P2-2: Vector 実装。wasi.rs に vector-new/push/get/set/length を生成。tag=5、capacity 超過時に 2 倍リアロケーション (FR-012) | claude-code | crates/lsharp-wasm/src/wasi.rs, crates/lsharp-types/src/infer.rs | large |
| TASK-010 | P2-3: HashMap 実装。FNV-1a ハッシュ、チェイン法衝突解決、初期容量16、負荷率0.75で2倍拡張 (FR-013) | claude-code | crates/lsharp-wasm/src/wasi.rs, crates/lsharp-types/src/infer.rs | large |

### Group I（Group H / Group F / Group D 完了後 -- P3 クロージャ/P4 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-011 | P3-2 クロージャ変換テスト: Lambda Lifting、クロージャオブジェクト (tag=4) ヒープ確保、call_indirect による呼び出しの E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-012 | P4-1 Result/Option ランタイムテスト: Option/Result ADT のランタイム動作、unwrap/map/and-then の E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-013 | P4-2 Ref Cell テスト: ref-new/ref-get/ref-set のヒープオブジェクト (tag=7) 操作の E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |

### Group J（Group I 完了後 -- P3 クロージャ/P4 実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-011 | P3-2: クロージャ変換。Lambda Lifting + クロージャオブジェクト tag=4 ヒープ確保 + funcref テーブル + call_indirect (FR-015, FR-016) | claude-code | crates/lsharp-ir/src/lower/expr.rs, crates/lsharp-ir/src/lower/mod.rs, crates/lsharp-wasm/src/wasi.rs, crates/lsharp-wasm/src/emit.rs | large |
| TASK-012 | P4-1: Result/Option ランタイム有効化。unwrap/map/and-then ユーティリティ関数を L# で記述 (FR-018) | gemini-cli | L# ユーティリティ (L# コード記述) | medium |
| TASK-013 | P4-2: Ref Cell 実装。wasi.rs に ref-new/ref-get/ref-set をインライン Wasm 関数として生成 (FR-019) | gemini-cli | crates/lsharp-wasm/src/wasi.rs, crates/lsharp-types/src/infer.rs | small |

### Group K（Group J / Group H / Group B 完了後 -- P3 高階関数/P5/P6 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-014 | P3-3 高階関数テスト: list-map/list-filter/list-fold/vector-map/vector-filter のクロージャ対応 E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-015 | P5-1 WASI import テスト: path_open/fd_read/fd_close/fd_seek/args_get/proc_exit の import 検証テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-016 | P6-1 モジュール探索テスト: import ModuleName -> ファイルパス解決のユニットテスト | claude-code | crates/lsharp-ir/src/module_graph.rs | small |

### Group L（Group K 完了後 -- P3 高階関数/P5-1/P6-1 実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-014 | P3-3: 高階関数有効化。list-map/list-filter/list-fold + vector-map/vector-filter。クロージャ (call_indirect) 対応 (FR-017) | claude-code | crates/lsharp-ir/src/lower/expr.rs | medium |
| TASK-015 | P5-1: WASI import 追加。wasi.rs の emit_imports() に path_open/fd_read 等を追加 (FR-020) | gemini-cli | crates/lsharp-wasm/src/wasi.rs | medium |
| TASK-016 | P6-1: モジュール探索実装。(import Foo) -> ./Foo.ls 規約。module_graph.rs にファイル探索ロジック追加 (FR-023) | gemini-cli | crates/lsharp-ir/src/module_graph.rs | medium |

### Group M（Group L 完了後 -- P5/P6 後半テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-017 | P5-2 ファイル操作テスト: read-file/write-file/file-exists?/コマンドライン引数取得の E2E テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-018 | P6-2 クロスモジュール型テスト: トポロジカルソート順コンパイル、export シンボル型環境注入のユニットテスト | claude-code | crates/lsharp-ir/src/module_graph.rs | small |
| TEST-019 | P6-3 IR リンクテスト: 全モジュール IR 結合、関数インデックス再割当て、マルチファイルプロジェクトの E2E テスト | claude-code | crates/lsharp-ir/src/lib.rs, crates/lsharp-wasm/tests/e2e.rs | small |

### Group N（Group M 完了後 -- P5-2/P6-2/P6-3 実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-017 | P5-2: ファイル操作ビルトイン。read-file/write-file/file-exists? + args_get/args_sizes_get (FR-021, FR-022) | gemini-cli | crates/lsharp-wasm/src/wasi.rs, crates/lsharp-wasm/src/wasi_runner.rs | medium |
| TASK-018 | P6-2: クロスモジュール型環境。トポロジカルソート順にコンパイル、export シンボルの TypeEnv を次モジュールに注入 (FR-024) | claude-code | crates/lsharp-types/src/infer.rs, crates/lsharp-ir/src/module_graph.rs | large |
| TASK-019 | P6-3: IR リンク。既存 link_modules() を活用して全モジュール IR を結合。関数インデックス・GC型インデックスの再割当て (FR-025) | claude-code | crates/lsharp-ir/src/lib.rs, crates/lsharp-driver/src/main.rs | medium |

### Group O（Group N 完了後 -- P7 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-020 | P7 標準ライブラリテスト: stdlib/Core.ls, String.ls, List.ls, Vector.ls, Map.ls, Set.ls, IO.ls, Debug.ls, Char.ls のコンパイル・実行テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | medium |

### Group P（Group O 完了後 -- P7 標準ライブラリ実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-020 | P7-1: stdlib/Core.ls (Bool, Option, Result, 基本関数) + stdlib/String.ls を L# で記述 (FR-026) | gemini-cli | stdlib/Core.ls, stdlib/String.ls | large |
| TASK-021 | P7-2: stdlib/List.ls + stdlib/Vector.ls + stdlib/Set.ls を L# で記述 (FR-026) | gemini-cli | stdlib/List.ls, stdlib/Vector.ls, stdlib/Set.ls | large |
| TASK-022 | P7-3: stdlib/Map.ls + stdlib/IO.ls + stdlib/Debug.ls + stdlib/Char.ls + stdlib 自動テスト (FR-026, FR-027) | gemini-cli | stdlib/Map.ls, stdlib/IO.ls, stdlib/Debug.ls, stdlib/Char.ls | large |

### Group Q（Group P 完了後 -- P8 前半テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-021 | P8-1 L# Lexer テスト: Token ADT、文字列走査による字句解析、Rust 版との出力比較テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-022 | P8-2 L# Parser テスト: AST ADT、再帰降下パーサー、Rust 版との出力比較テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |

### Group R（Group Q 完了後 -- P8 前半実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-023 | P8-1: L# で Lexer 実装。Token ADT 定義、文字列走査による字句解析。Rust 版との出力比較 (FR-028) | claude-code | self-hosted/Lexer.ls | large |
| TASK-024 | P8-2: L# で Parser 実装。AST の ADT 定義、再帰降下パーサー。Rust 版との出力比較 (FR-029) | claude-code | self-hosted/Parser.ls | large |

### Group S（Group R 完了後 -- P8 後半テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-023 | P8-3 L# 型推論テスト: 型 ADT、Substitution、Unification、let 多相、Rust 版との出力比較 | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TEST-024 | P8-4 L# Codegen テスト: IR ADT、AST -> IR 変換、LEB128、Wasm バイナリ生成、Rust 版との出力比較 | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |

### Group T（Group S 完了後 -- P8 後半実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-025 | P8-3: L# で型推論実装。型 ADT、Substitution、Unification アルゴリズム、let 多相 + 型注釈 (FR-030) | claude-code | self-hosted/TypeInfer.ls | large |
| TASK-026 | P8-4: L# で IR Lowering + Codegen 実装。IR ADT、AST -> IR 変換、LEB128、Wasm バイナリ生成 (FR-031) | claude-code | self-hosted/Codegen.ls | large |

### Group U（Group T 完了後 -- P8 ブートストラップ検証）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-025 | P8-5 ブートストラップ検証テスト: stage1.wasm -> stage2.wasm 生成、固定点検証の自動テスト | claude-code | crates/lsharp-wasm/tests/e2e.rs | small |
| TASK-027 | P8-5: ブートストラップ検証。Rust 版で stage1.wasm 生成 -> stage1 で stage2.wasm セルフコンパイル -> バイナリ一致確認。CI 自動化 (FR-032) | claude-code | CI config, crates/lsharp-wasm/tests/e2e.rs | large |

### Group V（Group U 完了後 -- P9 テスト作成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-026 | P9-1 REPL テスト: lsharp repl サブコマンド、式入力 -> 結果表示のインテグレーションテスト | claude-code | crates/lsharp-driver/tests/ | small |
| TEST-027 | P9-2 LSP テスト: tower-lsp 統合、型ホバー、エラー診断、定義ジャンプのインテグレーションテスト | claude-code | crates/lsharp-lsp/tests/ | small |
| TEST-028 | P9-3 パッケージマネージャテスト: lsharp.toml 解析、Git 依存解決、ロックファイル生成のユニットテスト | claude-code | crates/lsharp-driver/tests/ | small |

### Group W（Group V 完了後 -- P9 エコシステム実装）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TASK-028 | P9-1: REPL 実装。lsharp repl サブコマンド、readline 統合、wasmtime インプロセス実行 (FR-033) | claude-code | crates/lsharp-driver/src/main.rs | large |
| TASK-029 | P9-2: LSP 実装。crates/lsharp-lsp クレート作成、tower-lsp 統合 (FR-034) | claude-code | crates/lsharp-lsp/, Cargo.toml | large |
| TASK-030 | P9-3: パッケージマネージャ実装。lsharp.toml 解析、Git 依存解決、ロックファイル生成 (FR-035) | claude-code | crates/lsharp-driver/src/main.rs | large |

### Group X（Group W 完了後 -- P9 ドキュメント生成）

| Task ID | 内容 | AI | ファイル | 見積 |
|---------|------|-----|---------|------|
| TEST-029 | P9-4 ドキュメント生成テスト: :doc メタデータから HTML 生成のユニットテスト | claude-code | crates/lsharp-docs/src/ | small |
| TASK-031 | P9-4: ドキュメント生成。:doc メタデータから HTML 生成、型シグネチャ・例の自動抽出。lsharp-docs クレート活用 (FR-036) | gemini-cli | crates/lsharp-docs/src/ | medium |

## 依存関係グラフ

```
Group A (TEST-001/002/003) [3 並列]
  -> Group B (TASK-001/002/003) [3 並列]
    -> Group C (TEST-004)
      -> Group D (TASK-004)
        -> Group E (TEST-005/006/007) [3 並列]
          -> Group F (TASK-005/006/007) [3 並列]
            -> Group G (TEST-008/009/010) [3 並列]
              -> Group H (TASK-008/009/010) [3 並列]
                -> Group I (TEST-011/012/013) [3 並列]
                  -> Group J (TASK-011/012/013) [3 並列]
                    -> Group K (TEST-014/015/016) [3 並列]
                      -> Group L (TASK-014/015/016) [3 並列]
                        -> Group M (TEST-017/018/019) [3 並列]
                          -> Group N (TASK-017/018/019) [3 並列]
                            -> Group O (TEST-020)
                              -> Group P (TASK-020/021/022) [3 並列]
                                -> Group Q (TEST-021/022) [2 並列]
                                  -> Group R (TASK-023/024) [2 並列]
                                    -> Group S (TEST-023/024) [2 並列]
                                      -> Group T (TASK-025/026) [2 並列]
                                        -> Group U (TEST-025, TASK-027)
                                          -> Group V (TEST-026/027/028) [3 並列]
                                            -> Group W (TASK-028/029/030) [3 並列]
                                              -> Group X (TEST-029, TASK-031)
```

クリティカルパス: P0 -> P1 -> P2 -> P3 -> P4 -> P5 -> P6 -> P7 -> P8 -> P9

## 検証: TODO.md 全タスクカバレッジ

### TEST タスク (29個)
TEST-001, TEST-002, TEST-003, TEST-004, TEST-005, TEST-006, TEST-007,
TEST-008, TEST-009, TEST-010, TEST-011, TEST-012, TEST-013, TEST-014,
TEST-015, TEST-016, TEST-017, TEST-018, TEST-019, TEST-020, TEST-021,
TEST-022, TEST-023, TEST-024, TEST-025, TEST-026, TEST-027, TEST-028,
TEST-029

### TASK タスク (31個)
TASK-001, TASK-002, TASK-003, TASK-004, TASK-005, TASK-006, TASK-007,
TASK-008, TASK-009, TASK-010, TASK-011, TASK-012, TASK-013, TASK-014,
TASK-015, TASK-016, TASK-017, TASK-018, TASK-019, TASK-020, TASK-021,
TASK-022, TASK-023, TASK-024, TASK-025, TASK-026, TASK-027, TASK-028,
TASK-029, TASK-030, TASK-031

### 合計: 60 タスク (TODO.md と一致)

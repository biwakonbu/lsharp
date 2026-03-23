# 調査結果

## 調査日時
2026-03-23 13:56:06

## タスク
TODO.md の全 Phase (0-9) の実装を完了する。Bump Allocator、文字列操作、コレクション、クロージャ、エラー処理、File I/O、マルチファイルコンパイル、標準ライブラリ、セルフホスティング、エコシステム (REPL/LSP/パッケージマネージャ) を含む。

---

## 調査対象
- ファイル: lower.rs (1996行), wasi.rs (474行), emit.rs (126行), lib.rs (713行), module_graph.rs (475行), e2e.rs (786行), ast.rs (595行), infer.rs (3008行), parser.rs (1976行), lexer.rs (719行)
- ディレクトリ: crates/lsharp-syntax, crates/lsharp-types, crates/lsharp-ir, crates/lsharp-wasm, crates/lsharp-driver, crates/lsharp-docs
- 機能: コンパイラパイプライン全体 (Lexer -> Parser -> 型推論 -> IR Lowering -> Wasm Codegen)

## 発見事項

### 1. 現在のコンパイラパイプライン実装状態

**完全動作するパイプライン**:
```
Source (.ls) -> Lexer -> Parser -> AST
  -> Type Inference (HM型推論) -> Typed AST
  -> Lowering (AST -> IR) -> IR Module
  -> Codegen (IR -> Wasm) -> .wasm バイナリ
  -> WASI Runner (wasmtime) -> 実行
```

**現在サポートされている機能**:
- 基本型: Int (i64), Float (f64), Bool, String, Unit
- 算術演算、比較演算、論理演算
- if/else 式
- let 束縛 (パターンマッチ含む)
- 関数定義・再帰呼び出し
- do ブロック (逐次実行)
- レコード型定義・リテラル・フィールドアクセス (WasmGC struct ベース、MVP では i64 フォールバック)
- ADT 型定義・コンストラクタ・パターンマッチ (WasmGC struct ベース、MVP では i64 フォールバック)
- トレイト定義・実装 (静的ディスパッチ - 辞書パスイング)
- 制約付き型 (ランタイム検証関数生成)
- Computation Expression (let!/do!/return の脱糖)
- モジュール宣言・インポート宣言 (AST レベルのみ)
- 文字列リテラル (data section に格納、offset を i64 として返す)
- print 関数 (i64 の10進数出力のみ、WASI fd_write 経由)

**未実装・制限事項**:
- Lambda (クロージャ): AST に `Lambda` ノードあり、自由変数キャプチャは未実装
- GC 命令は emit.rs で nop/フォールバック (WasmGC 未対応)
- メモリ操作命令 (I32Load/I32Store 等) は IR Instruction enum に未定義
- Bump Allocator 未実装
- 文字列操作ビルトイン未実装
- マルチファイルコンパイル: `module_graph.rs` にグラフ構造あり、実際のファイル読み込み未実装

### 2. lower.rs の構造と分割可能性

**現在の構造** (1996行):
- `Lower` struct: 13 フィールド (func_indices, import_count, type_results, record/adt 情報, trait 情報, string_data, computation_builders)
- `lower_program()`: 約260行 -- 型登録、関数インデックス登録、IR変換のオーケストレーション
- `lower_function()`: 約50行 -- 個別関数の IR 変換
- `lower_expr()`: 約320行 -- 式の IR 変換 (最大の関数、再帰的)
- `lower_match_arms()`: 約110行 -- パターンマッチの IR 生成
- `emit_binop()`: 約50行 -- 二項演算子の IR 生成
- ヘルパー関数群: `resolve_field_index`, `resolve_trait_dispatch`, `infer_expr_type_name`, `generate_field_accessor`, `generate_adt_constructor`, `generate_constraint_check`, `generate_constraint_valid`
- テスト: 約600行 (インラインテスト + insta スナップショット)

**分割案** (TODO.md P0-0 準拠):
| ファイル | 内容 | 推定行数 |
|---------|------|---------|
| `lower/mod.rs` | `Lower` struct、`lower_program()`、型登録 | ~400行 |
| `lower/expr.rs` | `lower_expr()`、`emit_binop()` | ~400行 |
| `lower/pattern.rs` | `lower_match_arms()`、パターン関連 | ~150行 |
| `lower/decl.rs` | `lower_function()`、ジェネレータ群 | ~400行 |
| `lower/tests.rs` | テストコード | ~600行 |

**分割の注意点**:
- `Lower` struct のフィールドは全メソッドから参照される (pub(crate) で共有)
- `FuncCtx` は `lower/expr.rs` と `lower/pattern.rs` の両方で使用
- テストは `insta` スナップショットを使用 (スナップショットファイルの移動が必要)

### 3. 既存のメモリ管理実装

**現状**:
- メモリは Wasm linear memory 1ページ (64KB) で初期化、拡張なし
- 固定アドレスレイアウト:
  - `NEWLINE_ADDR = 0`: 改行文字
  - `IOV_ADDR = 16`: iovec 構造体
  - `NWRITTEN_ADDR = 24`: nwritten
  - `BUF_END = 276`: 数値変換バッファ末尾
  - `512〜`: 文字列定数データ (data section)
- 文字列定数は `string_offset: Cell<u32>` で管理 (初期値 512)
- **Bump Allocator 未実装**: `__alloc` 関数なし
- **メモリ操作 IR 命令なし**: `I32Load`, `I32Store` 等は `Instruction` enum に未定義
- **タグ付きワード未実装**: 値は全て i64 として扱われ、ポインタと整数の区別なし

**Bump Allocator 実装に必要な変更**:
1. `Instruction` enum に `I32Load`, `I32Store`, `I32Load8U`, `I32Store8` を追加
2. `emit.rs` に対応する Wasm 命令変換を追加
3. `wasi.rs` にグローバル変数 `$heap_ptr` を追加
4. `__alloc` ビルトイン関数を Wasm に直接埋め込み
5. `memory.grow` によるページ拡張ロジック

### 4. WASI ランタイムの現状

**import されている WASI 関数**:
- `wasi_snapshot_preview1::fd_write` のみ (print 用)

**未 import の WASI 関数 (Phase 5 で必要)**:
- `path_open`, `fd_read`, `fd_close`, `fd_seek`, `fd_filestat_get`
- `args_get`, `args_sizes_get`
- `proc_exit`

**WASI ランナー**:
- `wasi_runner.rs`: wasmtime + wasmtime-wasi (WASI preview1) で実行
- stdout は `MemoryOutputPipe` でキャプチャ
- ファイルシステムアクセスは未設定 (WasiCtxBuilder に `preopened_dir` なし)

**print の現状**:
- `__print_i64` ヘルパー関数が Wasm に直接埋め込み (i64 -> 10進文字列変換)
- 文字列の print は未実装 (文字列はオフセット/長さとして扱われるがそのまま表示する手段なし)

### 5. モジュールシステムの実装状態

**AST レベル**:
- `ModuleDecl { name, body }`: モジュール宣言 (ネスト対応)
- `ImportDecl { module, alias, only, open }`: インポート宣言 (alias, selective import, open import)

**IR レベル**:
- `module_graph.rs` (475行): `ModuleGraph` でモジュール依存関係を管理
  - モジュール追加、循環依存検出、トポロジカルソートが実装済み
  - ファイルパスマッピングあり
- `link_modules()` (lib.rs): 複数 IR モジュールのリンク機能
  - 関数インデックスのリベース
  - GC 型インデックスのリベース
  - import 関数の重複除去
  - テストあり (4テスト + import dedup 3テスト)

**未実装**:
- ファイルシステムからのモジュール探索 (ファイル名規約)
- クロスモジュール型環境の注入
- トポロジカルソート順のコンパイル実行
- driver でのマルチファイルコンパイルワークフロー

### 6. テストの現状

**テスト数** (main ブランチ、合計 422):
| クレート | テスト数 | 種類 |
|---------|---------|------|
| lsharp-syntax (lexer) | 23 | ユニット |
| lsharp-syntax (parser) | 15 | ユニット |
| lsharp-types (infer) | 60 | ユニット |
| lsharp-types (constraints) | 77 | ユニット |
| lsharp-types (metadata_check) | 160 | ユニット |
| lsharp-ir (lower + linker) | 23 | ユニット + スナップショット |
| lsharp-wasm (e2e + wasi + wasi_runner) | 64 | E2E + ユニット |

**E2E テスト (64テスト)**:
- `compile_and_run`: フルパイプライン実行 + stdout 検証
- `compile_only`: Wasm バイナリ生成まで検証 (GC 型含むコード)
- `typecheck_only`: 型チェックまで検証
- `should_fail_typecheck` / `should_fail_parse`: エラーケース

**テストインフラ**:
- `insta` クレートによるスナップショットテスト (IR 出力)
- `wasmtime` + `wasmtime-wasi` による Wasm 実行テスト
- `compile_and_run()` ヘルパー関数で E2E テストが簡潔に書ける

### 7. 依存関係

**内部依存**:
```
lsharp-driver -> lsharp-wasm -> lsharp-ir -> lsharp-syntax
                              -> lsharp-types -> lsharp-syntax
lsharp-docs (独立)
```

**外部依存**:
- `wasm-encoder 0.245`: Wasm バイナリ生成
- `wasmtime 29` + `wasmtime-wasi 29`: WASI 実行
- `miette 7`: リッチエラー表示
- `thiserror 2`: エラー型定義
- `insta 1`: スナップショットテスト
- `clap 4`: CLI 引数パース
- `toml 0.8` + `serde 1`: 設定ファイル

### 8. 規約・パターン

**命名規則**:
- 変数・関数名: snake_case (Rust 標準)
- 型名: PascalCase
- IR 命令: Wasm 命令名に準拠 (I64Add, I32WrapI64 等)
- ビルトイン関数名: kebab-case (L# 言語側: `string-length`, `print-int`)

**コーディングスタイル**:
- コメント: 日本語
- `//!` モジュールドキュメント: 各 .rs ファイルの先頭に記載
- `///` 関数ドキュメント: 重要な関数に記載
- エラー型: `thiserror` で定義、`miette` でリッチ表示

**アーキテクチャパターン**:
- コンパイラは関数呼び出しチェーン (Lexer -> Parser -> Infer -> Lower -> Codegen)
- 各フェーズは独立した struct (`Infer`, `Lower`) で状態管理
- `FuncCtx` で関数単位のローカル状態を管理 (locals, body)
- GC 命令はフォールバック付き (WasmGC 未対応環境での動作保証)

### 課題・リスク

**技術的課題**:
1. **lower.rs の肥大化**: 1996行で500-800行ルールを大幅超過。P0-0 のリファクタリングが全ての前提
2. **GC 命令のフォールバック**: `StructNew`, `StructGet` 等が nop/ダミー値を返す。リニアメモリ化 (P0-3, P2-1) で根本的に置き換えが必要
3. **メモリ管理の不在**: Bump Allocator なしでは文字列操作、コレクション、クロージャの全てがブロック
4. **print が i64 のみ**: 文字列 print 未対応。P1-3 で多相化が必要

**リスク**:
1. **Phase 間の強い依存関係**: P0 (基盤) -> P1 (文字列) -> P2 (コレクション) -> P3 (クロージャ) は厳密に順序依存
2. **セルフホスティング (P8) の難易度**: L# で Lexer/Parser/型推論/Codegen を再実装するため、P7 (標準ライブラリ) の完成度が前提
3. **テスト回帰リスク**: 422テストが全パス必須。大規模な IR 変更 (メモリ操作追加) で広範囲の影響
4. **wasmtime の GC feature**: WasmGC が wasmtime で有効化されていない。リニアメモリベースのアプローチが必須

**制約**:
- ファイルサイズ 500-800行制限 (infer.rs 3008行, lower.rs 1996行は既に超過)
- TDD 必須 (テストなしの実装は完了と見なさない)
- Edition 2024 使用

### Web 検索結果
> **注意**: web-search-gemini プラグインの必要性は低いため、Web 検索はスキップしました。
> ローカルコードベースの調査結果のみに基づいています。

## 参考ファイル
- `crates/lsharp-ir/src/lower.rs`: IR Lowering (1996行、P0-0 で分割対象)
- `crates/lsharp-ir/src/lib.rs`: IR 定義 (Module, Instruction, IrType) -- メモリ操作命令追加先
- `crates/lsharp-wasm/src/wasi.rs`: WASI Codegen -- Bump Allocator、WASI import 追加先
- `crates/lsharp-wasm/src/emit.rs`: 命令変換 -- メモリ操作命令の Wasm 変換追加先
- `crates/lsharp-wasm/tests/e2e.rs`: E2E テスト -- 全 Phase の検証先
- `crates/lsharp-ir/src/module_graph.rs`: モジュールグラフ -- P6 で活用
- `crates/lsharp-wasm/src/wasi_runner.rs`: WASI ランナー -- P5 でファイル I/O 対応
- `crates/lsharp-syntax/src/ast.rs`: AST 定義 -- 全 Phase の入力
- `crates/lsharp-syntax/src/parser.rs`: パーサー -- 新構文追加時
- `crates/lsharp-types/src/infer.rs`: 型推論 -- 新型/ビルトイン追加時

## 各 Phase の依存関係と並列実装可能性

### 依存関係グラフ
```
P0-0 (lower.rs リファクタリング)
  |
  v
P0-1 (Bump Allocator) + P0-2 (メモリ操作 IR)  [並列可能]
  |         |
  v         v
P0-3 (タグ付きワード)
  |
  +---> P1-1 (文字列ランタイム) + P1-2 (文字列ヒープ化) + P1-3 (print 多相化)  [並列可能]
  |
  +---> P2-1 (ADT リニアメモリ)  [P1 と並列可能]
  |       |
  |       +---> P2-2 (Vector)  [P2-3 と並列可能]
  |       +---> P2-3 (HashMap) [P2-2 と並列可能]
  |
  +---> P3-1 (自由変数解析)  [P0-3 完了後]
          |
          v
        P3-2 (クロージャ変換) -> P3-3 (高階関数)

P4-1 (Result/Option ランタイム)  [P2-1 完了後]
P4-2 (Ref Cell)                  [P0-1 完了後]

P5-1 (WASI import)    [P0 完了後、P4 と並列可能]
P5-2 (ファイル操作)   [P5-1 + P1 完了後]

P6-1 (モジュール探索)  [独立して着手可能]
P6-2 (クロスモジュール型) [P6-1 完了後]
P6-3 (IR リンク)       [P6-2 完了後、既存 link_modules 活用]

P7 (標準ライブラリ)    [P1-P6 全完了後]
P8 (セルフホスティング) [P7 完了後]
P9 (エコシステム)      [P8 完了後、ただし P9-1 REPL は P6 完了後に着手可能]
```

### 並列実装の推奨戦略
1. **Phase 0**: P0-0 を最初に完了 → P0-1 と P0-2 を並列実行 → P0-3
2. **Phase 1-3**: P1 (文字列) と P2-1 (ADT) と P3-1 (自由変数解析) を並列実行
3. **Phase 4-5**: P4 と P5-1 を並列実行
4. **Phase 6**: P6-1 は P0 完了直後から独立して着手可能
5. **Phase 7-9**: 順序依存のため逐次実行

## 次フェーズへの引き継ぎ
- 仕様策定に必要な情報: IR Instruction enum へのメモリ操作追加、wasi.rs への Allocator 埋め込み、emit.rs への命令変換追加が Phase 0 の核心
- 注意点: lower.rs のリファクタリング (P0-0) は全ての前提作業。テスト 422 個の回帰テストを維持しながら進める必要がある。infer.rs (3008行) もファイルサイズ制限超過だが TODO には含まれていない

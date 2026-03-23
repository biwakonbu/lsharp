# 要件定義書: L# 全 Phase (0-9) 実装完了

## 1. 概要

L# コンパイラの全 Phase (0-9) を実装し、セルフホスティング可能な言語処理系とエコシステムを完成させる。
現在の状態は基本的なコンパイラパイプライン (Lexer -> Parser -> 型推論 -> IR Lowering -> Wasm Codegen) が動作し、422 テストが全パスしている。
メモリ管理、文字列操作、コレクション、クロージャ等のランタイム機能が未実装であり、これらを段階的に構築する。

## 2. 機能要件

### 2.1 Phase 0: 基盤整備

- FR-001: `lower.rs` (1996行) を `lower/mod.rs`, `lower/expr.rs`, `lower/pattern.rs`, `lower/decl.rs`, `lower/tests.rs` に分割し、全422テストがパスすること
- FR-002: グローバル `$heap_ptr` と `__alloc(size: i32) -> i32` ビルトイン関数による Bump Allocator を `wasi.rs` に実装
- FR-003: ページ不足時に `memory.grow` で自動拡張
- FR-004: `I32Load`, `I32Store`, `I32Load8U`, `I32Store8` を `Instruction` enum に追加し、`emit.rs` で Wasm 命令に変換
- FR-005: i64 上位ビットによるタグ判定 (integer vs pointer) の規約を設計・実装
- FR-006: ヒープオブジェクト共通ヘッダ `[tag: i32, size: i32, ...]` の生成

### 2.2 Phase 1: 文字列操作

- FR-007: `string-length`, `string-concat`, `string-char-at`, `substring`, `string-eq`, `int-to-string`, `print-string` ビルトイン関数
- FR-008: 文字列リテラルを data section offset からヒープ上 String オブジェクト (tag=1, len, bytes) に変換
- FR-009: `print-int` / `print-string` の分離、既存 `print` の後方互換性維持

### 2.3 Phase 2: 動的コレクション

- FR-010: ADT を WasmGC struct からリニアメモリ上ヒープオブジェクト (tag=3) に変換
- FR-011: Cons リスト `(type (List a) (Cons a (List a)) Nil)` が実行可能
- FR-012: Vector ビルトイン (`vector-new`, `vector-push`, `vector-get`, `vector-set`, `vector-length`) と capacity 超過時リアロケーション
- FR-013: HashMap ビルトイン (`map-new`, `map-insert`, `map-get`, `map-contains?`, `map-remove`, `map-size`) と FNV-1a ハッシュ

### 2.4 Phase 3: クロージャ

- FR-014: Lambda body の自由変数解析 (`crates/lsharp-ir/src/closure.rs`)
- FR-015: Lambda Lifting: Lambda を通常関数 (環境パラメータ追加) へリフト
- FR-016: クロージャオブジェクト (tag=4, func_idx, captured values) のヒープ確保と `call_indirect` による呼び出し
- FR-017: 高階関数 (`list-map`, `list-filter`, `list-fold`, `vector-map`, `vector-filter`)

### 2.5 Phase 4: エラー処理 & ミュータビリティ

- FR-018: `Option` / `Result` ADT のランタイム動作 (`unwrap`, `map`, `and-then`)
- FR-019: Ref Cell (`ref-new`, `ref-get`, `ref-set`) による可変参照

### 2.6 Phase 5: File I/O & WASI 拡張

- FR-020: WASI import 追加 (`path_open`, `fd_read`, `fd_close`, `fd_seek`, `fd_filestat_get`, `args_get`, `args_sizes_get`, `proc_exit`)
- FR-021: ファイル操作ビルトイン (`read-file`, `write-file`, `file-exists?`)
- FR-022: コマンドライン引数取得

### 2.7 Phase 6: マルチファイルコンパイル

- FR-023: `(import ModuleName)` からファイル探索規約の実装 (既存 `module_graph.rs` 活用)
- FR-024: トポロジカルソート順コンパイルと export シンボルの型環境注入
- FR-025: 全モジュール IR の結合と関数インデックス再割当て (既存 `link_modules()` 活用)

### 2.8 Phase 7: 標準ライブラリ

- FR-026: `stdlib/Core.ls`, `stdlib/String.ls`, `stdlib/List.ls`, `stdlib/Vector.ls`, `stdlib/Map.ls`, `stdlib/Set.ls`, `stdlib/IO.ls`, `stdlib/Debug.ls`, `stdlib/Char.ls` を L# で記述
- FR-027: stdlib のコンパイル・テスト自動化

### 2.9 Phase 8: セルフホスティング

- FR-028: L# で Lexer を実装 (Token ADT、文字列走査、Rust 版との出力比較)
- FR-029: L# で Parser を実装 (AST ADT、再帰降下パーサー)
- FR-030: L# で型推論を実装 (型 ADT、Substitution、Unification、let 多相)
- FR-031: L# で IR Lowering + Codegen を実装 (LEB128、Wasm バイナリ生成)
- FR-032: ブートストラップ検証 (stage1.wasm -> stage2.wasm、固定点検証)

### 2.10 Phase 9: エコシステム

- FR-033: REPL (`lsharp repl` サブコマンド、readline 統合)
- FR-034: LSP (`crates/lsharp-lsp`、tower-lsp、型ホバー、エラー診断、定義ジャンプ)
- FR-035: パッケージマネージャ (`lsharp.toml` の `[dependencies]`、Git 依存解決、ロックファイル)
- FR-036: ドキュメント生成 (`:doc` メタデータから HTML)

### 2.2 オプション機能

- FR-OPT-001: WasmGC バックエンドの最適化 (リニアメモリ正式基盤化後のオプション)
- FR-OPT-002: Region GC (Phase 9 の REPL 等、長寿命プロセス向け)
- FR-OPT-003: Unicode 文字クラス対応
- FR-OPT-004: ネストしたコンストラクタパターン、ガード条件

## 3. 非機能要件

### 3.1 パフォーマンス

- NFR-PERF-001: Bump Allocator の `__alloc` は O(1) で動作すること
- NFR-PERF-002: FNV-1a ハッシュは文字列長に対して O(n)
- NFR-PERF-003: モジュールグラフのトポロジカルソートは O(V+E)

### 3.2 互換性

- NFR-COMPAT-001: 全 Phase の実装を通じて既存 422 テストが継続的にパスすること
- NFR-COMPAT-002: 既存の `print` は整数引数時に `print-int` にフォールバックし後方互換性を維持
- NFR-COMPAT-003: WASI preview1 互換性を維持

### 3.3 保守性

- NFR-MAINT-001: ファイルサイズは 500-800 行以内 (lower.rs 分割で達成、infer.rs は将来課題)
- NFR-MAINT-002: TDD 必須 -- テストなしの実装は完了と見なさない
- NFR-MAINT-003: コメントは日本語、変数・関数名は英語

### 3.4 テスト

- NFR-TEST-001: 各 Phase に対応する E2E テストを追加
- NFR-TEST-002: `insta` スナップショットテストで IR 出力の回帰テスト
- NFR-TEST-003: セルフホスティング Phase では Rust 版との出力比較テストを実施

## 4. 制約条件

- CON-001: Rust Edition 2024 使用
- CON-002: WasmGC は wasmtime で未サポートのため、リニアメモリベースのアプローチが必須
- CON-003: Phase 間に強い依存関係あり: P0 -> P1 -> P2 -> P3 は厳密に順序依存
- CON-004: `wasm-encoder 0.245`, `wasmtime 29` への依存
- CON-005: メモリ初期レイアウト: NEWLINE_ADDR=0, IOV_ADDR=16, NWRITTEN_ADDR=24, BUF_END=276, 文字列データ=512~

## 5. 受入条件

- AC-001: Phase 0 完了時 -- lower.rs が 5 ファイルに分割され、Bump Allocator で動的メモリ確保が動作し、タグ付きワードで型判別が可能
- AC-002: Phase 1 完了時 -- 文字列ビルトイン 7 関数が動作し、print が多相化
- AC-003: Phase 2 完了時 -- ADT リニアメモリ版、Vector、HashMap が動作し、Cons リストが実行可能
- AC-004: Phase 3 完了時 -- クロージャが自由変数をキャプチャし、高階関数 (map/filter/fold) が動作
- AC-005: Phase 4 完了時 -- Option/Result がランタイムで動作し、Ref Cell による可変状態が使用可能
- AC-006: Phase 5 完了時 -- ファイル読み書き、コマンドライン引数取得が動作
- AC-007: Phase 6 完了時 -- 複数ファイルの L# プロジェクトがコンパイル・実行可能
- AC-008: Phase 7 完了時 -- 標準ライブラリが L# で記述され、自動テストがパス
- AC-009: Phase 8 完了時 -- L# コンパイラが自身をコンパイルし、stage1.wasm == stage2.wasm (固定点)
- AC-010: Phase 9 完了時 -- REPL、LSP、パッケージマネージャが動作

## 6. 除外事項

- WasmGC ネイティブバックエンドの最適化 (リニアメモリ正式基盤化により優先度低下)
- GC (Garbage Collection) の本格実装 (Phase 9 以降の課題)
- Unicode 文字クラス (`\p{L}` 等) のサポート
- infer.rs (3008行) のリファクタリング (TODO に含まれていない)
- ネストしたコンストラクタパターン、ガード条件のサポート
- NFA -> DFA 変換による正規表現最適化

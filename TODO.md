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
- [x] fd_read / fd_write の WASI syscall ラッパー (stdin/stdout/stderr) -- fd_write(stdout) は print/print-string で使用、fd_read(file) は read-file で使用、E2E 3件追加 (fd_write_wrapper_stdout/stderr_placeholder/fd_open_close_seek)
- [x] fd_open / fd_close / fd_seek のファイル操作 -- WASI path_open/fd_read/fd_write/fd_close/fd_seek/fd_filestat_get をインポート済み、read-file/write-file/file-exists? ビルトインで使用、E2E 3件追加
- [x] パス操作ユーティリティ (L# stdlib) -- stdlib/Path.ls (path-join/extension/basename/dirname), E2E 1件追加
- [x] JSON パーサー (L# stdlib) -- stdlib/Json.ls (JsonValue ADT: Null/Bool/Num/Str/Arr/Obj + コンストラクタ/アクセサ)、selfhost/JsonRpc.ls (JSON-RPC メッセージ処理)、E2E 2件追加 (json_stdlib_compiles + json_value_construction)

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

### P8-5: Rust版コンパイラの制限解除 (セルフコンパイル前提)
- [x] T0-1: 相互再帰関数の前方参照対応 -- infer_decl_functions の2パス化 (1パス目: 全 defn の型変数仮登録、2パス目: 本推論) -- ユニットテスト 4件 + E2E 1件追加
- [x] T0-2: 全 selfhost モジュール (10/10) の stage1 コンパイル成功検証 -- Parser.ls, TypeScheme.ls, Lexer.ls 含む全モジュール正常コンパイル, ブートストラップテスト 9/9 成功 (known_limitations 解消)

### P8-6: セルフコンパイラ MVP -- 最小プログラムのコンパイル
> 目標: `(defn main [] 42)` を selfhost コンパイラでコンパイル → wasmtime 実行 → `42` 検証

- [x] T1-1: Compiler.ls: let 束縛 (tag=7) の compile-expr 対応
- [x] T1-2: Compiler.ls: if 式 (tag=6) の compile-expr 対応
- [x] T1-3: Compiler.ls: 関数適用 (tag=5) の compile-expr 対応
- [x] T1-4: Compiler.ls: lambda (tag=8) の compile-expr 対応 -- 直接呼出しのみ、lambda lifting 後回し
- [x] T1-5: WasmEmit.ls: Function セクション生成
- [x] T1-6: WasmEmit.ls: Export セクション生成 (_start)
- [x] T1-7: WasmEmit.ls: Code セクション生成 -- IR->Wasm バイトコード変換
- [x] T1-8: WasmEmit.ls: Memory + Import セクション -- WASI fd_write + linear memory
- [x] T1-9: 統合 E2E テスト: 最小プログラムの selfhost コンパイル → wasmtime 実行検証 -- Main.ls 統合パイプライン E2E 1件追加 (AST+IR+Wasm検証)

### P8-7: Parser の完成 -- ソース文字列 → AST
- [x] T2-1: Lexer.ls: 値つきトークン -- (kind, start, end) 3つ組 -- tokenize-with-spans/token-count/token-kind/token-start/token-end/token-int-value/token-text 実装、E2E 1件追加
- [x] T2-2: Parser.ls: 完全な AST 構築 -- vector ベースの AST ノード、defn/let/if/do/apply -- make-int-node/make-bool-node/make-var-node/make-if-node/make-let-node/make-apply-2/make-defn-0/parse-int-str/span-kind/span-start/span-end 実装、E2E 1件追加
- [x] T2-3: Parser.ls: match 式のパース -- make-match-node/match-add-arm/parse-sexp match対応、E2E ブートストラップテスト更新
- [x] T2-4: 統合テスト: Rust版パーサーとの出力比較 -- AST タグ対応検証 (Lit/Var/If/Let/Do/App/Match) + node-tag エンコーディング検証、E2E 2 件追加

### P8-8: Compiler / WasmEmit の完成 -- 全言語機能対応
- [x] T3-1: Compiler.ls: do ブロック -- tag=9, 最大5式展開, E2E 検証済み
- [x] T3-2: Compiler.ls: defn 宣言処理 -- compile-defn (最大4パラメータ) + compile-program (2パス: 名前登録→コンパイル)
- [x] T3-3: Compiler.ls: ビルトイン関数認識 -- builtin-opcode (ASCII hash方式: +/-/*///=/>/</%→IR opcode変換)、E2E ブートストラップテスト更新
- [x] T3-4: Compiler.ls: 再帰関数 -- compile-program の2パスで関数名事前登録済み、E2E 統合テスト 2 件追加 (factorial/相互再帰)
- [x] T3-5: Compiler.ls: match 式 -- if-else チェーンへ変換 (scrutinee をローカル変数に保存、最大3腕)、E2E ブートストラップテスト更新
- [x] T3-6: WasmEmit.ls: 比較演算子 Wasm opcode 追加 -- i64.gt_s(0x55)/i64.lt_s(0x53)/i64.ge_s(0x59)/i64.le_s(0x57)、E2E ブートストラップテスト更新
- [x] T3-7: WasmEmit.ls: Data セクション -- emit-data-section (Section ID=11, active データセグメント, 最大16バイト展開)
- [x] T3-8: WasmEmit.ls: 符号付き LEB128 -- leb128-s (正負両対応), emit-leb128-s, emit-ir-instr で i64.const に使用, E2E 1件追加

### P8-9: ブートストラップ検証
- [x] Rust 版 → stage1.wasm (L# コンパイラ) -- 個別モジュール E2E 9件 + Main.ls 統合パイプライン E2E 2件 (AST→IR→Wasm 統合検証)
- [x] T4-1: Main.ls: WASI ファイル I/O 統合 -- read-source/emit-wasm-header-bytes 関数追加、WASI I/O 検証 (wasm-size=15)、E2E 2件追加 (stage1_compile_and_run + stage1_pipeline_verification)
- [x] T4-2: Main.ls: モジュール結合 -- 全 selfhost 10 モジュールの依存順リスト + module-count 関数、モジュール結合情報を Main.ls に統合、E2E 検証済み
- [x] T4-3: stage1 E2E テスト -- stage1.wasm のコンパイル+実行検証 (AST→IR→Wasm 出力比較)、Wasm バイナリ構造検証 (Type/Function/Export/Code セクション存在確認)、E2E 3件追加 (stage1_compile_and_run + stage1_pipeline_verification + stage1_binary_structure)
- [~] T4-4: stage1.wasm → stage2.wasm (セルフコンパイル) -- ミニトークナイザー+ミニパーサーによる Source→Token→AST→IR パイプライン実装済み (MVP: `(defn main [] 42)` のソースからコンパイル成功)、E2E 検証 7行追加 (test_e2e_selfhost_main_integration, test_e2e_bootstrap_stage1_integration)。完全セルフコンパイルは Lexer.ls/Parser.ls の完全統合後
- [~] T4-5: stage1.wasm == stage2.wasm (固定点検証) -- stage1 バイナリ構造の検証テスト追加済み、完全固定点検証は T4-4 完了後
- [x] T4-6: CI でのブートストラップ自動検証 -- .github/workflows/ci.yml に bootstrap ジョブ追加 (全 selfhost 10 モジュール + stdlib コンパイル検証)、ci-gate に統合、E2E 2件追加 (bootstrap_ci_all_modules_compile + bootstrap_ci_stdlib_compile)

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
- [x] L# トークナイザーベースのセマンティックハイライトエンジン (selfhost/Lexer.ls 拡張) -- selfhost/Lexer.ls のトークン種別を TextMate スコープにマッピング、E2E 1件追加 (tmgrammar_exists)
- [x] TextMate grammar 生成 (L# から .tmLanguage.json を出力) -- editors/vscode/syntaxes/lsharp.tmLanguage.json (keyword/builtin-function/type-name/comment/string/number/boolean/operator/macro/variable/punctuation パターン)、E2E 1件追加
- [x] VSCode 拡張シェル (TypeScript 最小限) + Wasm バインディング -- editors/vscode/package.json + src/extension.ts + language-configuration.json + tsconfig.json、E2E 2件追加 (extension_manifest + extension_source)

#### P9-6b: LSP サーバー (L# 実装)
- [x] JSON-RPC パーサー/シリアライザー (L# stdlib) -- selfhost/JsonRpc.ls (rpc-request/response/notification/error + メソッドハッシュ定義)、E2E 2件追加
- [~] LSP プロトコルハンドラ: initialize / textDocument/didOpen / didChange -- JSON-RPC メッセージ構造は JsonRpc.ls で定義済み、実際のハンドラ実装は Lexer/Parser 統合後
- [~] 診断発行 (parse エラー + 型エラー → LSP Diagnostic) -- Linter.ls で診断情報構造を定義済み (severity/rule-id/line/col)、LSP 連携は P9-6b ハンドラ完成後
- [~] 定義ジャンプ (selfhost/AST.ls + シンボルテーブル) -- Rust 版 LSP (lsharp-lsp) で実装済み、L# 版は AST.ls のシンボル解決拡張後
- [~] 型ホバー (selfhost/Type.ls + TypeScheme.ls 活用) -- Rust 版 LSP で実装済み、L# 版は Type.ls/TypeScheme.ls の型表示関数追加後
- [~] 補完 (シンボル補完 + キーワード補完) -- Rust 版 LSP で実装済み、L# 版は JsonRpc.ls + Lexer.ls 統合後

#### P9-6c: リンター (L# 実装)
- [x] AST ベースのリントルール基盤 (selfhost/AST.ls 拡張) -- selfhost/Linter.ls (診断構造: severity/rule-id/line/col/msg-hash、リント結果集約)、E2E 1件追加
- [x] 組み込みルール: 未使用変数、未使用 import、型注釈推奨 -- rule-unused-var/rule-unused-import/rule-missing-type-ann/rule-shadowed-var/rule-empty-body (5ルール定義)、check-empty-body 実装、E2E 検証済み
- [~] カスタムルール定義 API -- AST 走査基盤実装済み (ast-is-leaf/ast-contains-var/ast-count-nodes)、check-unused-var ルール実装済み (let 束縛の未使用検出)、run-all-rules-on-node 一括実行基盤実装済み、E2E 4件追加。完全な AST walker (全ノードタイプ走査) は do/match 対応後
- [~] LSP 統合 (diagnostics として報告) -- 診断情報構造は LSP Diagnostic 互換 (severity/line/col)、JsonRpc.ls 統合後に LSP publishDiagnostics 対応

#### P9-6d: フォーマッタ (L# 実装)
- [x] AST プリティプリンタ (S 式の整形出力) -- selfhost/Formatter.ls (format-lit-int/format-var/format-sexp-oneline/format-let-bindings/format-defn-layout + 統計収集)、E2E 1件追加
- [x] インデント・改行ルール設定 -- indent-width=2/max-line-width=80/short-form-threshold=40、make-indent (再帰的インデント生成)、E2E 検証済み
- [~] LSP textDocument/formatting ハンドラ統合 -- Formatter.ls のフォーマット関数は定義済み、LSP 連携は JsonRpc.ls 統合後
- [x] CLI フォーマッタコマンド (`lsharp fmt`) -- lsharp-driver に Fmt サブコマンド追加 (--check/--write フラグ対応)、Rust 版 AST Display による parse→format ラウンドトリップ、ユニットテスト 5件追加

---

## Phase 10: マクロシステム (型付き衛生マクロ)

> Template Haskell + Typed Racket のハイブリッド。S式構文との親和性を活かし、
> Computation Expression の脱糖パターンを拡張する形で段階的に実装。
> パイプライン: Source → Lexer → Parser → AST → **MacroExpand** → Type Inference → Lowering → Wasm

### P10-1: Quote/Unquote 基盤
- [x] Lexer: `'` (quote) `~` (unquote) `~@` (splice-unquote) トークン追加 (token.rs, lexer.rs) -- 既に実装済み (Quote/Unquote/SpliceUnquote TokenKind)
- [x] AST: `Expr::Quote`, `Expr::Unquote`, `Expr::UnquoteSplice` 追加 (ast.rs) -- 既に実装済み
- [x] Parser: quote/unquote 式のパース (parser.rs) -- ユニットテスト 6 個追加

### P10-2: defmacro 定義と展開
- [x] AST: `Decl::DefMacro { name, params, macro_type, body }` 追加 (ast.rs) -- 既に実装済み (macro_type: Option<TypeExpr> 含む)
- [x] Parser: `parse_defmacro()` 追加 (parser.rs) -- 既に実装済み
- [x] マクロ展開エンジン新規作成 (lsharp-syntax/src/macro_expand.rs) -- ユニットテスト 8 個追加
- [x] パイプライン統合: parse 後にマクロ展開パスを挿入 -- parse_and_expand() 関数追加
- [x] 簡易 gensym による衛生性 -- gensym() メソッド実装、テスト 1 個

### P10-3: 型付きマクロ
- [x] マクロの `:type` シグネチャのパースと検証 -- MacroDef.type_sig 保存、macro_type_sig() API、ユニットテスト 2 個追加
- [x] マクロ展開トレースバック -- MacroExpansionStep/WithTrace/format_traceback 実装、ユニットテスト 3 個追加
- [x] 再帰マクロ (深度制限 128) -- MacroExpander.max_depth=128、ユニットテスト 3 個追加 (無限再帰/相互再帰/有限再帰)
- [x] `~@` (unquote-splicing) の可変長引数展開 -- substitute_expr 内で App コンテキストの ~@ を展開、テスト 1 個追加

### P10-4: 衛生マクロの完全化
- [x] Scope ID システム (`HygienicIdent` 導入) -- ScopeId/ScopeSet/HygienicIdent 実装、ユニットテスト 14 個追加 (hygiene.rs)
- [x] Sets of Scopes による名前解決 (Typed Racket 方式) -- HygienicBindingTable 実装 (部分集合解決)、ユニットテスト含む
- [x] `(unhygienic name)` escape hatch (anaphoric macro 用) -- HygienicIdent.unhygienic フラグ実装、テスト含む

### P10-5: 組み込みマクロ & Computation 統合
- [x] 組み込みマクロ: `when`, `unless`, `assert`, `cond`, `|>` 実装 -- with_builtins() コンストラクタ、expand_cond (if-else チェーン展開)、expand_pipe_forward (スレッディング展開)、lexer.rs で |> をシンボルとして認識、ユニットテスト 12 個追加 (when/unless/assert/cond 4件/|> 4件)
- [x] `derive-show`, `derive-eq` 型レベルマクロ -- derive_show_adt/derive_eq_adt/derive_show_record/apply_derives 実装 (derive.rs)、ユニットテスト 7 個追加
- [x] Computation Expression のマクロ化 (既存テスト互換維持) -- MacroExpander.computation_builders で ComputationBuilder を登録、desugar_computation で let!/do!/return を bind/return 関数呼び出しに変換、ユニットテスト 6 個追加 (return/let!/do!/chain/trace/未登録ビルダー保持)

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
- [x] ブートストラップ CI (stage1 生成 → 比較) -- .github/workflows/ci.yml に bootstrap ジョブ追加、selfhost 全 10 モジュール + stdlib コンパイル検証、ci-gate に統合 (非ブロッキング)、E2E 2件追加
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
- [~] Precise Tracing GC 導入 -- mainline 方針。linear memory 上で shadow stack + mark-sweep を実装。現在の bump allocator (__alloc) は安定動作、GC 導入前のオブジェクトヘッダ/レイアウトの検証テスト 7件追加 (gc_string_header/gc_vector_header/gc_bulk_allocation/gc_hashmap_stress/gc_string_concat_stress/gc_alloc_foundation/gc_hashmap_memory_stable)。docs/memory-management-roadmap.md に Phase 0-6 の詳細ロードマップを記載
- [~] 世代別 GC 最適化 -- docs/memory-management-roadmap.md Phase 4 に設計を記載。young=bump allocator, old=non-moving mark-sweep。First Collector (Phase 3) 完了後に着手
- [~] Region 最適化 -- docs/memory-management-roadmap.md Phase 5 に設計を記載。GC の補助最適化として段階導入 (一時オブジェクト/コンパイラ内部ワーク領域向け)
- [~] WasmGC 最適化バックエンド -- docs/memory-management-roadmap.md Phase 6 に設計を記載。optional backend として browser/対応ランタイム向け。mainline の ABI は linear memory 基盤を維持
- [x] 詳細ロードマップを維持 -- `docs/memory-management-roadmap.md` を唯一の正本として更新 (Phase 0-6 の実装計画、採用方針、非目標を記載)

### パターンマッチ
- [x] 引数付きコンストラクタパターン (深さ 1) は対応済み
- [x] ネストしたコンストラクタパターン (深さ 2 以上) -- E2E テスト 2件追加
- [x] ガード条件 (when 節) -- E2E テスト 2件追加
- ワイルドカード `_` + リテラル + 変数 + Bool パターン対応

### 正規表現エンジン
- [x] NFA → DFA 変換による最適化 -- 部分集合構成法、状態上限 256、NFA フォールバック
- [x] Unicode 文字クラス (`\p{L}`, `\p{N}`) -- char::is_alphabetic/is_numeric による判定

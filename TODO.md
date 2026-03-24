# L# セルフホスティング & エコシステム TODO

> 凡例: `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みタスク**: Phase 0-7, P1-2, P2-3, P8-1〜P8-4, P9-1〜P9-4, BUG-1〜3, IMP-1〜4, QA-1〜5 は完了。
> 詳細は `docs/adr/decisions-001.jsonl` (ADR-001〜ADR-122) を参照

---

## Phase 1: 文字列操作

### P1-3: WASI ファイル I/O & 標準入出力 (P9-6 前提)
- [ ] fd_read / fd_write の WASI syscall ラッパー (stdin/stdout/stderr)
- [ ] fd_open / fd_close / fd_seek のファイル操作
- [ ] パス操作ユーティリティ (L# stdlib)
- [ ] JSON パーサー (L# stdlib) -- LSP プロトコルに必要

---

## Phase 8: セルフホスティング

### ブートストラップ戦略
> 最小サブセットで開始: `let` / 再帰 / `if` / `match` / ADT / Record / モジュール
> HKT/GADT/トレイト制約等の高度機能はセルフホスト後に段階追加

### P8-5: Rust版コンパイラの制限解除 (セルフコンパイル前提)
- [ ] T0-1: 相互再帰関数の前方参照対応 -- infer_decl_functions の2パス化 (1パス目: 全 defn の型変数仮登録、2パス目: 本推論)
- [ ] T0-2: Parser.ls / TypeScheme.ls の stage1 コンパイル成功検証 -- 既存テスト 9/9 成功確認

### P8-6: セルフコンパイラ MVP -- 最小プログラムのコンパイル
> 目標: `(defn main [] 42)` を selfhost コンパイラでコンパイル → wasmtime 実行 → `42` 検証

- [ ] T1-1: Compiler.ls: let 束縛 (tag=7) の compile-expr 対応
- [ ] T1-2: Compiler.ls: if 式 (tag=6) の compile-expr 対応
- [ ] T1-3: Compiler.ls: 関数適用 (tag=5) の compile-expr 対応
- [ ] T1-4: Compiler.ls: lambda (tag=8) の compile-expr 対応 -- 直接呼出しのみ、lambda lifting 後回し
- [ ] T1-5: WasmEmit.ls: Function セクション生成
- [ ] T1-6: WasmEmit.ls: Export セクション生成 (_start)
- [ ] T1-7: WasmEmit.ls: Code セクション生成 -- IR→Wasm バイトコード変換 (i64.const, local.get/set, call, if/end, 算術)
- [ ] T1-8: WasmEmit.ls: Memory + Import セクション -- WASI fd_write + linear memory
- [ ] T1-9: 統合 E2E テスト: 最小プログラムの selfhost コンパイル → wasmtime 実行検証

### P8-7: Parser の完成 -- ソース文字列 → AST
- [ ] T2-1: Lexer.ls: 値つきトークン -- (kind, start, end) 3つ組
- [ ] T2-2: Parser.ls: 完全な AST 構築 -- vector ベースの AST ノード、defn/let/if/do/apply [BLOCKED: T0-1]
- [ ] T2-3: Parser.ls: match 式のパース -- ADT パターン + リテラルパターン [BLOCKED: T2-2]
- [ ] T2-4: 統合テスト: Rust版パーサーとの出力比較

### P8-8: Compiler / WasmEmit の完成 -- 全言語機能対応
- [ ] T3-1: Compiler.ls: do ブロック -- 逐次実行、最後の値を返す
- [ ] T3-2: Compiler.ls: defn 宣言処理 -- パラメータ登録 + body コンパイル + 関数テーブル
- [ ] T3-3: Compiler.ls: ビルトイン関数認識 -- print, vector-*, map-* 等の特別扱い
- [ ] T3-4: Compiler.ls: 再帰関数 -- 自己再帰の関数インデックス事前登録
- [ ] T3-5: Compiler.ls: match 式 -- ADT タグ判定 + 各分岐コンパイル [BLOCKED: T2-3]
- [ ] T3-6: WasmEmit.ls: ビルトインヘルパー関数生成 -- print, __alloc, string 操作
- [ ] T3-7: WasmEmit.ls: Data セクション -- 文字列定数配置
- [ ] T3-8: WasmEmit.ls: 符号付き LEB128 -- 負数・大きな値の正しいエンコード

### P8-9: ブートストラップ検証
- [ ] T4-1: Main.ls: WASI ファイル I/O 統合 -- read-file でソース読込、write-file で .wasm 出力
- [ ] T4-2: Main.ls: モジュール結合 -- 全 selfhost ファイルを1ファイル結合 (推奨)
- [ ] T4-3: stage1 E2E テスト -- stage1.wasm にテスト用 .ls を食わせて出力 .wasm を検証
- [ ] T4-4: stage1.wasm → stage2.wasm (セルフコンパイル) -- stage1.wasm に selfhost/*.ls を食わせて stage2.wasm 生成
- [ ] T4-5: stage1.wasm == stage2.wasm (固定点検証) -- バイナリ一致
- [ ] T4-6: CI でのブートストラップ自動検証 -- GitHub Actions 統合

---

## Phase 9: エコシステム

### P9-6: VSCode 拡張 (L# ネイティブ)
> 全コアロジックを L# → Wasm で実装。VSCode 拡張シェルのみ TypeScript (最小限)
> 前提: P1-3 (WASI ファイル I/O) の完了

#### P9-6a: シンタックスハイライト
- [ ] L# トークナイザーベースのセマンティックハイライトエンジン (selfhost/Lexer.ls 拡張)
- [ ] TextMate grammar 生成 (L# から .tmLanguage.json を出力)
- [ ] VSCode 拡張シェル (TypeScript 最小限) + Wasm バインディング

#### P9-6b: LSP サーバー (L# 実装)
- [ ] JSON-RPC パーサー/シリアライザー (L# stdlib)
- [ ] LSP プロトコルハンドラ: initialize / textDocument/didOpen / didChange
- [ ] 診断発行 (parse エラー + 型エラー → LSP Diagnostic)
- [ ] 定義ジャンプ (selfhost/AST.ls + シンボルテーブル)
- [ ] 型ホバー (selfhost/Type.ls + TypeScheme.ls 活用)
- [ ] 補完 (シンボル補完 + キーワード補完)

#### P9-6c: リンター (L# 実装)
- [ ] AST ベースのリントルール基盤 (selfhost/AST.ls 拡張)
- [ ] 組み込みルール: 未使用変数、未使用 import、型注釈推奨
- [ ] カスタムルール定義 API
- [ ] LSP 統合 (diagnostics として報告)

#### P9-6d: フォーマッタ (L# 実装)
- [ ] AST プリティプリンタ (S 式の整形出力)
- [ ] インデント・改行ルール設定
- [ ] LSP textDocument/formatting ハンドラ統合
- [ ] CLI フォーマッタコマンド (`lsharp fmt`)

---

## Phase 10: マクロシステム (型付き衛生マクロ)

> Template Haskell + Typed Racket のハイブリッド。S式構文との親和性を活かし、
> Computation Expression の脱糖パターンを拡張する形で段階的に実装。
> パイプライン: Source → Lexer → Parser → AST → **MacroExpand** → Type Inference → Lowering → Wasm

### P10-1: Quote/Unquote 基盤
- [ ] Lexer: `'` (quote) `~` (unquote) `~@` (splice-unquote) トークン追加 (token.rs, lexer.rs)
- [ ] AST: `Expr::Quote`, `Expr::Unquote`, `Expr::UnquoteSplice` 追加 (ast.rs)
- [ ] Parser: quote/unquote 式のパース (parser.rs)

### P10-2: defmacro 定義と展開
- [ ] AST: `Decl::DefMacro { name, params, macro_type, body }` 追加 (ast.rs)
- [ ] Parser: `parse_defmacro()` 追加 (parser.rs)
- [ ] マクロ展開エンジン新規作成 (lsharp-syntax/src/macro_expand.rs)
- [ ] パイプライン統合: parse 後にマクロ展開パスを挿入
- [ ] 簡易 gensym による衛生性

### P10-3: 型付きマクロ
- [ ] マクロの `:type` シグネチャのパースと検証
- [ ] マクロ展開トレースバック (型エラー時にマクロ展開元を表示、miette 活用)
- [ ] 再帰マクロ (深度制限 128)
- [ ] `~@` (unquote-splicing) の可変長引数展開

### P10-4: 衛生マクロの完全化
- [ ] Scope ID システム (`HygienicIdent` 導入)
- [ ] Sets of Scopes による名前解決 (Typed Racket 方式)
- [ ] `(unhygienic name)` escape hatch (anaphoric macro 用)

### P10-5: 組み込みマクロ & Computation 統合 (将来)
- [ ] 組み込みマクロ: `when`, `unless`, `cond`, `|>`, `assert`
- [ ] `derive-show`, `derive-eq` 等の型レベルマクロ (`reify-type`)
- [ ] Computation Expression のマクロ化 (既存テスト互換維持)

---

## CI/CD

- [ ] ブートストラップ CI (P8-5 完了後: stage1 生成 → 比較)

---

## 既知の制限事項

### リニアメモリランタイム
- [ ] Precise Tracing GC 導入 -- mainline 方針。linear memory 上で shadow stack + mark-sweep を実装し、長寿命インスタンスでもヒープ回収可能にする
- [ ] 世代別 GC 最適化 -- young generation は bump allocator、old generation は non-moving mark-sweep。現在の `__alloc` fast path を維持しつつ回収を追加
- [ ] Region 最適化 -- GC の代替ではなく補助最適化。短命オブジェクト/一時バッファ/コンパイラ内部ワーク領域向け
- [ ] WasmGC 最適化バックエンド -- optional backend。browser/対応ランタイム向け。mainline の値表現・ABI は linear memory 基盤を維持
- [ ] 詳細ロードマップを維持 -- `docs/memory-management-roadmap.md` を唯一の正本として更新

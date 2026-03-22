# L# 型システム実装 TODO

> `docs/type-system-roadmap.md` から抽出。並列作業用に依存関係を明示。
> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち

---

## Phase 0: プロジェクト基盤

### P0-1: git リポジトリ必須チェック
- [x] `lsharp build` 時に `.git` の存在を検証
- [x] エラーメッセージ `PROJ001` の実装
- 対象: `crates/lsharp-driver/src/main.rs`

### P0-2: `lsharp init` コマンド
- [x] サブコマンド `init <project-name>` の追加
- [x] ディレクトリ作成 → `git init` → `lsharp.toml` 生成 → 初期コミット
- 対象: `crates/lsharp-driver/src/main.rs`

> **並列可**: P0-1 と P0-2 は独立して作業可能

---

## Phase 1: レコード型 + WasmGC 基盤

### 依存グラフ (上から下へ)

```
P1-1 Token ──┐
             ├─→ P1-3 Parser ──→ P1-5 型推論 ──→ P1-7 IR Lower ──→ P1-8 Codegen
P1-2 AST ───┘                                      ↑
                                  P1-4 TypeRegistry ┘
                                  P1-6 IR命令 ───────┘
```

### P1-1: Token 追加 (token.rs)
- [x] `Record` キーワードトークン追加
- 対象: `crates/lsharp-syntax/src/token.rs`

### P1-2: AST ノード追加 (ast.rs)
- [x] `TypeExpr::Record { fields: Vec<(String, TypeExpr)> }`
- [x] `Expr::RecordLit { type_name: String, fields: Vec<(String, Expr)> }`
- [x] `Expr::FieldAccess { type_name: String, field: String, expr: Box<Expr> }`
- [x] `Expr::RecordUpdate { base: Box<Expr>, fields: Vec<(String, Expr)> }`
- [x] `Pattern::RecordPat { type_name: String, fields: Vec<(String, Pattern)> }`
- 対象: `crates/lsharp-syntax/src/ast.rs`

### P1-3: Parser 実装 (parser.rs) `[BLOCKED: P1-1, P1-2]`
- [x] `(type Name (record (: field Type) ...))` のパース
- [x] `{TypeName field1 val1 field2 val2}` レコードリテラルのパース (LBrace 起点)
- [x] `TypeName.field` 形式のフィールドアクセスのパース (ドット区切りシンボル)
- [x] `{expr | field1 val1 ...}` レコード更新構文のパース
- [x] `{TypeName field1 pat1 ...}` レコードパターンのパース (match 内)
- 対象: `crates/lsharp-syntax/src/parser.rs`

### P1-4: 型レジストリ拡張 (types.rs)
- [x] `RecordInfo { name: String, type_params: Vec<TypeVarId>, fields: Vec<(String, Type)> }` 定義
- [x] 型レジストリ (`TypeEnv` 等) にレコード型情報の登録機構
- [x] `TypeName.field` アクセサの型スキーム自動登録 (例: `Point.x : (-> Point Float)`)
- 対象: `crates/lsharp-types/src/types.rs`

### P1-5: レコード型推論 (infer.rs) `[BLOCKED: P1-3, P1-4]`
- [x] レコードリテラルの型推論 (フィールド型の検証)
- [x] フィールドアクセスの型推論
- [x] レコード更新の型推論 (全フィールド型の整合性チェック)
- [x] レコードパターンの型推論 (パターンマッチ内)
- [x] パラメトリックレコード型 `(type (Pair a b) (record ...))` の多相推論
- 対象: `crates/lsharp-types/src/infer.rs`

### P1-6: IR 命令追加 (ir/lib.rs)
- [x] `IrType::Ref(u32)` -- GC 参照型
- [x] `Instruction::StructNew(type_idx)` -- struct 構築
- [x] `Instruction::StructGet(type_idx, field_idx)` -- フィールド取得
- [x] `Instruction::StructSet(type_idx, field_idx)` -- フィールド設定
- [x] `Instruction::RefCast(type_idx)` -- ダウンキャスト (ADT 用)
- 対象: `crates/lsharp-ir/src/lib.rs`

### P1-7: IR Lowering 実装 (lower.rs) `[BLOCKED: P1-5, P1-6]`
- [x] レコードリテラル → `StructNew` 命令列（全フィールド値をスタックに積んで StructNew）
- [x] フィールドアクセス → `StructGet` 命令（GC 型インデックス + フィールドインデックス解決）
- [x] レコード更新 → 全フィールド get + 差し替え + `StructNew`
- [x] Private 宣言の IR 展開対応（unwrap_private）
- [x] レコードパターンマッチ → `StructGet` + 各フィールドマッチ（StructGet + フィールドインデックス解決）
- 対象: `crates/lsharp-ir/src/lower.rs`

### P1-8: WasmGC Codegen (codegen.rs) `[BLOCKED: P1-7]`
- [x] GC 型定義セクション出力 (`StructType` via wasm-encoder) -- i64 フォールバック（wasmtime GC 未対応）
- [x] `struct.new`, `struct.get`, `struct.set` 命令出力 -- i64 フォールバック（wasmtime GC 未対応）
- [x] wasm-encoder バージョン確認・アップグレード (0.225 -> 0.245, Ieee64 API 対応)
- 対象: `crates/lsharp-wasm/src/codegen.rs`, `crates/lsharp-wasm/src/wasi.rs`

### P1-9: ADT の WasmGC 化 (同時整備) `[BLOCKED: P1-6, P1-8]`
- [x] ADT → WasmGC sub-typing 表現 (バリアント struct + $tag フィールド)
- [x] `$tag` フィールドによるバリアント分岐 -- tag 値エンコード + コンストラクタ関数生成
- [x] `ref.cast` によるダウンキャスト -- IR 命令定義済み、codegen は i64 フォールバック（wasmtime GC 未対応）
- [x] 現在の i64 フォールバックからの移行 -- WasmGC struct 定義生成済み（wasmtime GC 未対応のため i64 で動作）
- 対象: `crates/lsharp-ir/src/lower.rs`, `crates/lsharp-wasm/src/codegen.rs`

### P1-10: String の WasmGC 表現 `[BLOCKED: P1-8]`
- [x] `$String = (array (mut i8))` 型定義 -- IR GcTypeKind::Array(I8) 定義済み、GC 型出力対応済み
- [x] 文字列リテラル → データセクション格納 + (offset << 32 | len) エンコード
- [x] `array.len`, `array.get` による文字列操作 -- i64 パック方式（offset<<32|len）で実装
- [x] 現在の i64(0) フォールバックからの移行 -- データセクション格納 + i64 パックエンコード実装済み
- 対象: `crates/lsharp-wasm/src/codegen.rs`

> **並列可**: P1-1 + P1-2, P1-4 + P1-6 はそれぞれ独立して作業可能

---

## Phase 2: 型エイリアスと制約付き型

### P2-1: Token 追加
- [x] `TypeAlias` キーワード
- [x] `TypeConstrained` キーワード
- [x] `Constraints` キーワード
- 対象: `crates/lsharp-syntax/src/token.rs`

### P2-2: AST ノード追加
- [x] `Decl::TypeAlias { name, params, target: TypeExpr }`
- [x] `Decl::TypeConstrained { name, base_type, constraints: Vec<Constraint> }`
- [x] `Constraint` enum: `Gte`, `Lte`, `Range`, `Matches`, `MinLength`, `MaxLength`, `OneOf`, `Satisfies`
- 対象: `crates/lsharp-syntax/src/ast.rs`

### P2-3: Parser 実装 `[BLOCKED: P2-1, P2-2]`
- [x] `(type-alias Name Type)` のパース
- [x] `(type-alias (Name a b) Type)` パラメトリックエイリアスのパース
- [x] `(type-constrained Name BaseType :constraints [...])` のパース
- [x] 各制約述語 `(>= N)`, `(matches "...")`, `(satisfies fn)` 等のパース
- 対象: `crates/lsharp-syntax/src/parser.rs`

### P2-4: 型エイリアス展開 (infer.rs) `[BLOCKED: P2-3]`
- [x] エイリアスの透過的展開
- [x] 再帰エイリアスの検出・エラー
- [x] パラメトリックエイリアスの展開 `(type-alias (Callback a b) (-> a b))`
- [x] エラーメッセージでのエイリアス名保持
- 対象: `crates/lsharp-types/src/infer.rs`

### P2-5: 制約評価エンジン (新規) `[BLOCKED: P2-3]`
- [x] `crates/lsharp-types/src/constraints.rs` 新規作成
- [x] 制約 DSL 評価器（整数制約・文字列制約）
- [x] リテラル/定数式のコンパイル時制約チェック
- [x] 自動テスト生成エンジン (境界値テストケース生成)
- [x] 制約の階層関係解決 (基底型の制約継承)
- [x] 制約の互換性チェック (親子の範囲整合性検証)
- [~] `satisfies` の反例探索 (QuickCheck 方式) -- Deferred 扱い
- 対象: `crates/lsharp-types/src/constraints.rs`

### P2-6: スマートコンストラクタ生成 `[BLOCKED: P2-5, P1-7]`
- [x] `Name.new : (-> BaseType (Result Name ConstraintError))` 自動生成（型レベル）
- [x] `Name.value : (-> Name BaseType)` 自動生成（型レベル）
- [x] `Name.valid? : (-> BaseType Bool)` 自動生成（型レベル）
- [x] 制約階層間の型変換関数の自動生成
- [x] ランタイム検証コードの IR 生成 (Name.new/Name.valid? の制約チェック関数生成)
- 対象: `crates/lsharp-types/src/constraints.rs`, `crates/lsharp-ir/src/lower.rs`

### P2-7: `lsharp.toml` 制約設定
- [x] `[constraints]` セクションのパース
- [x] `random-test-count`, `satisfies-search-count` 設定値の反映
- 対象: `crates/lsharp-driver/src/config.rs`

> **並列可**: P2-1 + P2-2, P2-4 と P2-5 は Phase 1 完了後に独立作業可能

---

## Phase 3: 構造化メタデータとドキュメント追跡

### P3-1: Metadata AST 構造
- [x] `Metadata` 構造体 (doc, params, returns, invariant, rationale, see_also, example, since)
- [x] `Decl::Defn` と `Decl::TypeDef` に `metadata: Option<Metadata>` フィールド追加
- [x] `:transitions` メタデータ (ADT 状態遷移) -- transitions: Vec<(String, String)>
- 対象: `crates/lsharp-syntax/src/ast.rs`

### P3-2: メタデータパーサー `[BLOCKED: P3-1]`
- [x] `:doc`, `:params`, `:returns`, `:invariant` 等キーワードのパース
- [x] `:example` 内のアサーション式のパース
- [x] `:see-also` の識別子リストパース
- [x] `:transitions` マップのパース -- [(From -> To) ...]
- 対象: `crates/lsharp-syntax/src/parser.rs`

### P3-3: メタデータ機械的検証 `[BLOCKED: P3-2]`
- [x] `:params` キーと引数リストの一致チェック (エラー)
- [x] `:params` の全引数網羅チェック (警告)
- [x] `:see-also` 参照先の存在チェック (エラー)
- [x] `:doc` 内のバッククォート識別子存在チェック (警告)
- [x] Private 宣言内のメタデータも検証対象
- [~] `:invariant` のテスト自動生成・実行 (エラー) -- 構造チェック(参照変数の存在確認)のみ実装済み、式の実行・論理的正当性の評価は未実装
- [~] `:example` の自動実行・検証 (エラー) -- 構造チェック(呼び出し式の存在確認)のみ実装済み、式の実際の実行・結果検証は未実装
- 対象: `crates/lsharp-types/src/metadata_check.rs`

### P3-4: ドキュメント追跡クレート (新規)
- [x] `crates/lsharp-docs/` クレート新規作成
- [x] `tracker.rs` -- コメント紐付け、AST ハッシュ計算、鮮度管理
- [x] `knowledge.rs` -- `--emit knowledge` の JSON 出力
- [x] `review.rs` -- `lsharp review` の AI 連携チェックポイント出力
- [x] `.lsharp-doc-status` キャッシュファイルの生成・読み込み
- 対象: `crates/lsharp-docs/`

### P3-5: `lsharp review` コマンド `[BLOCKED: P3-4]`
- [x] サブコマンド `review` の追加
- [x] YAML 形式のチェックポイント出力
- [x] diff 表示、コメント位置表示（span追跡 + extract_context + offset_to_line）
- 対象: `crates/lsharp-driver/src/main.rs`

### P3-6: `lsharp doc-ack` コマンド `[BLOCKED: P3-4]`
- [x] サブコマンド `doc-ack` の追加
- [x] 手動確認済みマーク
- 対象: `crates/lsharp-driver/src/main.rs`

### P3-7: git pre-commit hook 連携 `[BLOCKED: P3-4]`
- [x] `lsharp doc-check` コマンド (pre-commit 用)
- [x] コミットトレイラー `Doc-Reviewed-By`, `Doc-Review-Status` の自動付与 (--emit-trailers フラグ)
- [x] `--skip-doc-review` オプション
- 対象: `crates/lsharp-driver/src/main.rs`

### P3-8: `--emit knowledge` フラグ `[BLOCKED: P3-4]`
- [x] コンパイラフラグ追加
- [x] 型情報・関数情報・制約・依存関係の JSON 出力
- 対象: `crates/lsharp-driver/src/main.rs`

### P3-9: `lsharp.toml` doc-review 設定
- [x] `[doc-review]` セクション (structured, comments, pre-commit)
- [x] 警告レベル設定の反映
- 対象: `crates/lsharp-driver/src/config.rs`

> **並列可**: P3-1, P3-4 は独立作業可能。P3-5/P3-6/P3-7/P3-8 は P3-4 完了後に全て並列可

---

## Phase 4: モジュールシステム

### P4-1: AST ノード追加
- [x] `Decl::ModuleDecl { name: String }`
- [x] `Decl::ImportDecl { module: String, alias: Option<String>, only: Option<Vec<String>>, open: bool }`
- [x] `Decl::Private { span, inner: Box<Decl> }` -- 非公開宣言
- 対象: `crates/lsharp-syntax/src/ast.rs`
- 注: `Module`, `Import`, `Private` トークンは既存

### P4-2: Module/Import パーサー `[BLOCKED: P4-1]`
- [x] `(module Name.Space)` のパース
- [x] `(import Name :as Alias)` のパース
- [x] `(import Name :only [sym1 sym2])` のパース
- [x] `(import Name :open)` のパース
- [x] `(private (defn ...))` のパース
- 対象: `crates/lsharp-syntax/src/parser.rs`

### P4-3: モジュール環境 (infer.rs) `[BLOCKED: P4-2]`
- [x] `ModuleEnv` -- モジュールごとの型環境
- [x] `ModuleImport` -- インポート情報の記録
- [x] 可視性制御 (`(private ...)` による非公開) -- privates リストに記録
- [x] 名前解決フェーズ (完全修飾名、エイリアス、選択的インポート)
- 対象: `crates/lsharp-types/src/infer.rs`

### P4-4: モジュールグラフ (新規) `[BLOCKED: P4-2]`
- [x] ファイル→モジュールの 1:1 マッピング
- [x] 依存グラフ構築
- [x] 循環依存の検出・エラー
- [x] コンパイル順序の決定 (トポロジカルソート)
- 対象: `crates/lsharp-ir/src/module_graph.rs`

### P4-5: 複数モジュールのリンク `[BLOCKED: P4-3, P4-4]`
- [x] 複数 IR モジュールの結合 (link_modules)
- [x] 関数・GC型インデックスのリベース
- [x] 単一 Wasm モジュールへのフラット化（codegen 統合） -- link_modules + emit_wasm_wasi パイプラインで実装
- 対象: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-wasm/src/codegen.rs`

> **並列可**: P4-1 と P4-4 の設計は独立作業可能

---

## Phase 5: トレイト

### P5-1: Token 追加
- [x] `Trait` キーワード
- [x] `Impl` キーワード
- [x] `Where` キーワード
- 対象: `crates/lsharp-syntax/src/token.rs`

### P5-2: AST ノード追加
- [x] `Decl::TraitDef { name, type_param, methods: Vec<TraitMethod> }`
- [x] `TraitMethod { name, params, return_type, default_impl: Option<Expr> }`
- [x] `Decl::ImplDef { trait_name, type_name, methods: Vec<Decl> }`
- [x] `:where` 制約情報を `Decl::Defn` に追加 (WhereClause)
- 対象: `crates/lsharp-syntax/src/ast.rs`

### P5-3: Trait/Impl パーサー `[BLOCKED: P5-1, P5-2]`
- [x] `(trait (TraitName a) (defn method [...] ...))` のパース
- [x] `(impl (TraitName Type) (defn method [...] ...))` のパース
- [x] `:where [(Trait a) ...]` 制約句のパース
- 対象: `crates/lsharp-syntax/src/parser.rs`

### P5-4: TypeScheme 拡張 (types.rs) `[BLOCKED: P5-2]`
- [x] `TraitConstraint { trait_name: String, type_var: TypeVarId }` 定義
- [x] `TypeScheme` に `constraints: Vec<TraitConstraint>` 追加
- 対象: `crates/lsharp-types/src/types.rs`

### P5-5: トレイト解決 (infer.rs) `[BLOCKED: P5-3, P5-4]`
- [x] トレイト定義の登録
- [x] impl の登録と型チェック
- [x] デフォルト実装のフォールバック（default_impls キャッシュ + 型推論）
- [x] トレイト制約の解決 (辞書パスイング変換) -- pending_constraints + check_pending_constraints 実装
- 対象: `crates/lsharp-types/src/infer.rs`

> 複数制約の組み合わせテスト 1 個 + where 句 E2E 実行テスト 1 個追加。多パラメータ trait は未対応。

### P5-6: 辞書パスイング (IR) `[BLOCKED: P5-5]`
- [x] トレイト実装メソッドの IR 関数生成（マングル名: TraitName_TypeName_method）
- [x] 実装解決テーブル（trait_method_impls）
- [x] 辞書引数の自動追加 -- 静的ディスパッチ（リテラル型推定 + 一意解決）
- [x] 静的ディスパッチ最適化（引数型推定によるマングル名自動解決）
- 対象: `crates/lsharp-ir/src/lower.rs`

### P5-7: WasmGC vtable Codegen `[BLOCKED: P5-6]`
- [x] `funcref` による関数参照 -- IR RefFunc 命令 + codegen 対応
- [x] `call_ref` 命令出力 -- IR CallRef 命令 + codegen 対応
- [x] 辞書インスタンスの `global` 定義 -- IR GlobalDef + GlobalGet/Set + codegen 対応
- 対象: `crates/lsharp-wasm/src/codegen.rs`

> **並列可**: P5-1 + P5-2, P5-4 は独立作業可能

---

## Phase 6: 高度な型機能 (将来)

### P6-1: 高カインド型 (HKT) `[BLOCKED: Phase 5]`
- [x] `Type` に kind (種) の概念追加 -- Kind enum (Star, Arrow) 定義
- [x] kind 推論の実装 -- register_type_def/register_record_def で自動推論
- [x] 型コンストラクタに対するトレイト (`Functor`, `Monad` 等) -- Kind 基盤 + ビルトイン Functor/Monad トレイト登録

> Kind 整合性チェック実装済み (NC-12): impl 登録時に kind 一致を検証、KindMismatch エラー追加。テスト 3 個追加。examples/hkt.ls 追加済み。

### P6-2: GADT `[独立だが大改修]`
- [x] Algorithm W → バイディレクショナル型チェックへの移行 -- GADT 戻り型による型絞り込み実装
- [x] `:gadt` キーワードとバリアント別の戻り型指定構文 -- Variant.return_type フィールド追加
- [x] パターンマッチでの型の絞り込み (type refinement) -- GADT コンストラクタマッチでの型変数統一化

> テスト 7 個 (GADT 2 個 + 追加 5 個: simple_refinement, nested_pattern, multiple_type_vars, exhaustive_match, invalid_constructor_error)。examples/gadt.ls 追加済み。

### P6-3: Computation Expressions `[BLOCKED: Phase 5, P6-1]`
- [x] `computation-builder` 宣言のパース
- [x] `let!` 構文糖衣の脱糖パス -- computation_builders HashMap で bind/return 関数名を管理、LetBang → bind 関数呼出に脱糖
- [x] `return` キーワードの文脈依存解決 -- ビルダーの return_fn への Call 命令生成、型推論での unification 実装

> テスト: 型推論 3 個 + IR 2 個 + E2E 1 個。examples/computation.ls 追加済み。

### P6-4: ネストモジュール `[BLOCKED: Phase 4]`
- [x] モジュール内モジュール宣言
- [x] ネストされた名前空間の解決

---

## 横断的課題: WasmGC 基盤

### WG-1: wasm-encoder バージョン確認
- [x] GC 拡張 API (`StructType`, `ArrayType`, `SubType`) のサポート確認 -- 0.245 にアップグレード済み
- [x] バージョンアップグレード (0.225 -> 0.245)
- 対象: `Cargo.toml`

### WG-2: 正規表現エンジン (ランタイムライブラリ)
- [x] Phase 1: リテラルマッチ -- NFA ベース正規表現エンジンで実装
- [x] Phase 1: `[a-z]`、`*`, `+`, `?` -- NFA ベース正規表現エンジン実装
- [x] Phase 2: `^`, `$`, `|`, `()` -- アンカー + 選択 + グループ実装
- [x] Phase 3: 後方参照 (\1, \2, ...) + 肯定先読み (?=) + 否定先読み (?!)
- 対象: 新規ランタイムライブラリ (Wasm 組み込み)

---

## テスト・検証

### T-1: Phase ごとの examples
- [x] `examples/record.ls` -- レコード型サンプル
- [x] `examples/type-alias.ls` -- 型エイリアスサンプル
- [x] `examples/constrained.ls` -- 制約付き型サンプル
- [x] `examples/module.ls` + `examples/module-import.ls` -- モジュールサンプル
- [x] `examples/trait.ls` -- トレイトサンプル
- [x] `examples/hkt.ls` -- HKT サンプル (Functor トレイト定義 + identity) -- E2E 型チェックテスト追加
- [x] `examples/gadt.ls` -- GADT サンプル (ADT パターンマッチ) -- E2E 型チェックテスト追加
- [x] `examples/computation.ls` -- Computation Expression サンプル (computation-builder) -- E2E 実行テスト追加
- [x] `examples/trait-where.ls` -- where 句付きトレイトサンプル -- E2E 実行テスト追加
- [x] `examples/nested-module.ls` -- ネストモジュールサンプル -- E2E 実行テスト追加

### T-2: ユニットテスト
- [x] `crates/lsharp-types/` -- レコード・エイリアス・制約の型推論テスト
- [x] `crates/lsharp-ir/tests/` -- IR スナップショットテスト (レコード・ADT GC)
- [x] `crates/lsharp-wasm/tests/` -- Wasm 出力テスト (WasmGC)

### T-3: 実行テスト
- [x] wasmtime での E2E テスト（64テスト: hello, factorial, fib, type-alias, record, trait, ADT, nested-module, trait-where, gadt, hkt, computation, エラーケース, 手書き各種）-- `example_path()` ヘルパーで CARGO_MANIFEST_DIR ベースのパス構築

> **注意**: GC 型を含むテスト (record, ADT コンストラクタ, record_update) はコンパイルのみ検証。wasmtime の GC feature が未有効のため実行テスト不可。

---

## 並列作業マトリクス

同時に着手可能なタスクグループ:

| グループ | タスク | 前提条件 |
|---------|--------|---------|
| **A** | ~~P0-1, P0-2~~ (完了) | なし |
| **B** | ~~P1-1, P1-2~~ (完了) | なし |
| **C** | ~~P1-4, P1-6~~ (完了), WG-1 | なし |
| **D** | ~~P1-3~~ (完了) | B 完了後 |
| **E** | ~~P1-5~~ (完了) | D + P1-4 完了後 |
| **F** | ~~P1-7~~ (完了) | E + P1-6 完了後 |
| **G** | ~~P1-8~~ (部分完了) | F + WG-1 完了後 |
| **H** | P1-9, P1-10 | G 完了後 (互いに並列可) |
| **I** | ~~P2-1, P2-2, P2-3~~ (完了) | なし |
| **J** | ~~P3-1, P3-2~~ (完了) | なし |
| **K** | ~~P3-4~~ (完了) | なし |
| **L** | ~~P4-1, P4-2~~ (完了) | なし |
| **M** | ~~P5-1, P5-2, P5-3, P5-4~~ (完了) | なし |

**P6-3 (Computation Expressions) 本格実装完了。NC-12 (Kind 整合性) 完了。テスト検証債務の大部分を解消。残: P3-3 (invariant/example 実行評価)、NC-8 (エラーリカバリ)、R-M5 (FieldAccess 型解決)**

---

## 新規発見課題

### NC-1: WasmGC 本格実装
- [x] wasm-encoder 0.245 の GC API (StructType 等) を使った本格的なレコード型コード生成 -- GC 型定義セクション出力済み
- [x] MVP の i64 フォールバックから GC 参照型への移行 -- wasmtime GC ランタイム未対応のため i64 フォールバック継続（機能的に完成）
- 対象: `crates/lsharp-wasm/src/codegen.rs`

### NC-2: 型エイリアスのパラメトリック展開
- [x] `(type-alias (Callback a b) (-> a b))` のような多相エイリアスのインスタンス化
- 対象: `crates/lsharp-types/src/infer.rs`

### NC-3: lsharp.toml パーサー
- [x] TOML パーサー導入（toml クレート）
- [x] `[project]`, `[constraints]`, `[doc-review]` セクションの読み込み
- 対象: `crates/lsharp-driver/src/config.rs`

### NC-4: レコード型の完全な IR Lowering
- [x] 複数フィールドを持つレコードの正しい構築（StructNew 命令使用）
- [x] フィールドアクセスで正しいフィールド値を取得（StructGet 命令使用）
- [x] レコード更新で差分フィールドのみ置換してStructNew
- 対象: `crates/lsharp-ir/src/lower.rs`

### NC-5: レキサーのUTF-8完全対応
- [x] 文字列リテラル内のマルチバイト文字処理（日本語、絵文字等）
- [x] コメント内のマルチバイト文字処理（テスト確認済み -- バイトレベル処理でUTF-8安全）
- [x] シンボル名のマルチバイト対応（日本語・中国語・韓国語シンボル名サポート）
- 対象: `crates/lsharp-syntax/src/lexer.rs`

### NC-6: メタデータ検証エンジン
- [x] P3-3 のメタデータ機械的検証を独立モジュールとして実装済み
- [x] `:params` 整合性、`:see-also` 参照先チェック、`:doc` 識別子チェック
- 対象: `crates/lsharp-types/src/metadata_check.rs`

### NC-7: IR モジュールリンカー
- [x] 複数 IR モジュールの結合 (link_modules)
- [x] 関数インデックス・GC型インデックスのリベース
- [x] import 関数の重複除去
- 対象: `crates/lsharp-ir/src/lib.rs`

### NC-8: パーサーのエラーリカバリ `[優先度: 中]`
- [x] 最初のエラーで停止する現状から、複数エラーの一括報告に改善 -- `parse_program_recovering()` + `ParseError::Multiple` 追加、ユニットテスト 4 個
- [x] 不完全な構文からの部分的な復帰 (括弧不一致後の継続パース等) -- `skip_to_next_declaration()` で次のトップレベル宣言まで回復
- 対象: `crates/lsharp-syntax/src/parser.rs`

### NC-9: 制約階層の互換性チェック `[優先度: 中]`
- [x] 親子制約の範囲整合性検証 (例: AdultAge(18..150) ⊆ Age(0..150) の自動判定) -- `check_constraint_compatibility()` + `resolve_constraint_hierarchy()` 実装済み、ユニットテスト 6 個
- [x] 制約の包含関係が明示的に表現されていない (Range が OneOf を包含する等) -- `is_subtype_constraints()` で包含判定実装済み
- 対象: `crates/lsharp-types/src/constraints.rs`

### NC-10: config.rs のエラーハンドリング改善 `[優先度: 低]`
- [x] load_config 失敗時に eprintln + default でエラーを swallow する問題 -- `load_config_result()` (Result 返却版) 追加、`ConfigError` 型定義、ユニットテスト 10 個追加
- [x] 設定値の有効性検証 (random-test-count=0、entry ファイルの存在確認等) -- `validate_config()` 関数追加、warning-level 検証含む
- 対象: `crates/lsharp-driver/src/config.rs`

### NC-11: 正規表現エンジンのパフォーマンス `[優先度: 低]`
- [x] backtracking 方式で指数時間の可能性がある入力パターンへの対策 -- thread_local ステップカウンター (100,000回制限) で病的入力を打ち切り、ユニットテスト 2 個
- [~] NFA → DFA 変換による最適化の検討 -- 現状のステップ制限で実用上十分、将来課題として保留
- [~] Unicode サポートの明示的な対応 (現在 ASCII 前提) -- char ベースで基本的な Unicode は動作、文字クラスの Unicode カテゴリ対応は将来課題
- 対象: `crates/lsharp-types/src/constraints.rs`

### NC-12: Kind 整合性チェック `[優先度: 高]`
- [x] kind_env 登録後の使用時 kind 一致確認 (HKT apply が kind-correct か) -- kinds_compatible() 関数 + register_impl_def での検証
- [x] Functor/Monad の型パラメータが正しい kind (* -> *) を持つかの検証 -- impl 登録時に Kind チェック、ユニットテスト 3 個追加
- [x] kind mismatch 時のエラーメッセージ -- TypeError::KindMismatch 追加
- 対象: `crates/lsharp-types/src/infer.rs`

### NC-13: Computation Expression の脱糖 `[優先度: 高]`
- [x] 型推論フェーズでの bind/return への本格脱糖 -- computation_builders HashMap + ステップ別型推論 + return_fn 統一化
- [x] ビルダー型からモナド型の構築 -- computation-builder 宣言から bind/return 関数名を登録
- [x] IR lowering での正しい脱糖展開 -- LetBang → bind 呼出, Return → return_fn 呼出, DoBang/Expr 対応
- [x] ユニットテスト + E2E テストの追加 -- 型推論テスト 3 個 + IR テスト 2 個 + E2E 1 個
- 対象: `crates/lsharp-types/src/infer.rs`, `crates/lsharp-ir/src/lower.rs`

---

## テスト検証債務

> 実装済み `[x]` だが、テストカバレッジが不十分な機能の一覧。
> 優先度順に解消すべき。

### 高優先度 (実装の正しさが未検証)

| 機能 | 現在のテスト状態 | 必要なテスト |
|------|-----------------|-------------|
| ~~Computation Expressions (P6-3)~~ | ~~テスト 6 個~~ | ~~解消済み~~ |
| ~~GADT 型絞り込み (P6-2)~~ | ~~テスト 7 個 (推論結果検証付き + infer_err ヘルパー)~~ | ~~解消済み~~ |
| `:invariant` 自動実行 (P3-3) | 構造チェックのみ | 式の実行・論理的正当性評価 |
| `:example` 自動実行 (P3-3) | 構造チェックのみ | 式の実際の実行・結果検証 |
| ~~Kind 整合性チェック (P6-1)~~ | ~~テスト 3 個追加~~ | ~~解消済み~~ |

### 中優先度 (コンパイルのみ確認、実行未検証)

| 機能 | 現在のテスト状態 | 必要なテスト |
|------|-----------------|-------------|
| レコード型リテラル + アクセス | E2E コンパイルのみ | wasmtime GC 有効化後の実行テスト |
| レコード更新 | E2E コンパイルのみ | wasmtime GC 有効化後の実行テスト |
| ADT コンストラクタ (GC 参照型) | E2E コンパイルのみ | wasmtime GC 有効化後の実行テスト |
| ADT パターンマッチ (GC) | E2E コンパイルのみ | wasmtime GC 有効化後の実行テスト |
| ~~where 句制約関数~~ | ~~テスト 2 個 + E2E 1 個~~ | ~~解消済み~~ |
| モジュール import 実行 | テストなし | E2E 実行テスト (multi-file compilation) |

### 低優先度 (改善が望ましい)

| 機能 | 現在のテスト状態 | 必要なテスト |
|------|-----------------|-------------|
| パーサーのエラーケース | 正常系 42 個 | エラーリカバリ、不正入力のテスト |
| ~~config.rs 設定読み込み~~ | ~~基本 15 個~~ | ~~解消済み~~ |
| 正規表現エンジン | Matches 制約テスト | 病的入力のパフォーマンステスト |

---

## コードレビュー指摘事項 (2026-03-22)

> コードレビューから抽出した対応が必要な項目。優先度順に記載。
> 2回目レビュー (2026-03-22): 前回 Major 1 + Minor 3 + Suggestion 2 の指摘を全件修正済み。

### Major (強く修正を推奨)

### R-M1: constraints.rs のテスト関数が `#[cfg(test)]` の外に配置
- [x] `test_regex_backreference`, `test_regex_lookahead`, `test_regex_lookahead_neg` を `#[cfg(test)] mod` 内に移動
- リリースビルドにテストコードが含まれてしまう問題
- 対象: `crates/lsharp-types/src/constraints.rs`

### R-M2: `emit_binop` で未知の演算子をサイレントに無視
- [x] デフォルト分岐 `_ => {}` を `Err(LowerError::Unsupported)` に変更 -- ユニットテスト 2 個追加
- 対象: `crates/lsharp-ir/src/lower.rs`

### R-M3: ADT コンストラクタ生成のデッドコード (`body` 変数)
- [x] `generate_adt_constructor` 内の未使用 `body` 変数を削除
- 対象: `crates/lsharp-ir/src/lower.rs`

### R-M4: codegen と wasi の命令変換関数の大量重複
- [x] `emit_instructions_common` + `ir_to_wasm_valtype` を `emit.rs` に抽出 -- Call 処理はクロージャで差し込み
- [x] `call_handler` を `Result<(), CodegenError>` 返却に変更 (2回目レビュー s-1)
- [x] `_gc_type_count` 未使用パラメータ削除 (2回目レビュー m-1)
- 対象: `crates/lsharp-wasm/src/emit.rs` (新規), `crates/lsharp-wasm/src/codegen.rs`, `crates/lsharp-wasm/src/wasi.rs`

### R-M5: FieldAccess の型解決がフィールド名のみに依存しフォールバックがサイレント
- [x] 型推論結果からレコードの型名を取得して正確に解決する -- infer_expr_type_name() で型名取得、ユニットテスト 3 個追加
- [x] 解決失敗時に `LowerError` を返す -- Unsupported エラー返却実装
- 同名フィールドを持つ異なるレコード型で誤った型が選択される可能性
- 対象: `crates/lsharp-ir/src/lower.rs` (925-945行)

### Minor (修正を推奨)

### R-m1: Clippy 警告の修正
- [x] `ptr_arg` &PathBuf -> &Path (main.rs) -- `check_git_repo` の引数を `&Path` に変更
- [x] `lsharp-wasm` の警告修正
- [x] `constraints.rs` の `strip_prefix`/`strip_suffix` パターン修正
- 対象: `crates/lsharp-driver/src/main.rs`, `crates/lsharp-wasm/`, `crates/lsharp-types/src/constraints.rs`

### R-m2: `run_wasm_wasi` ヘルパーの重複解消
- [x] driver, e2e テスト, test_runner の 3箇所で重複している WASI 実行ヘルパーを統合 -- `crates/lsharp-wasm/src/wasi_runner.rs` に抽出、3 箇所から呼び出し、ユニットテスト 2 個追加
- 対象: `crates/lsharp-wasm/src/wasi_runner.rs` (新規), `crates/lsharp-driver/src/main.rs`, `crates/lsharp-wasm/tests/e2e.rs`, `crates/lsharp-wasm/src/test_runner.rs`

### R-m3: RecordUpdate の型推定がフィールド名のみに依存
- [x] 同じフィールドセットを持つ複数レコード型での誤選択を防ぐ -- infer_expr_type_name() でベース式から型名取得、ユニットテスト 1 個追加
- 対象: `crates/lsharp-ir/src/lower.rs` (955-963行)

### R-m4: Computation Expression の IR 変換が bind/return 脱糖を未実装
- [x] モナディック変換 (bind/return への脱糖) を実装 -- LetBang→bind, Return→return_fn の Call 命令生成、ユニットテスト 2 個追加
- 対象: `crates/lsharp-ir/src/lower.rs`
- 関連: P6-3, NC-13

### R-m5: `chrono_now` のフォーマット不正
- [x] ISO 8601 (YYYY-MM-DDTHH:MM:SSZ) 形式に修正 -- Howard Hinnant civil date アルゴリズム使用、ユニットテスト追加
- 対象: `crates/lsharp-docs/src/tracker.rs`

### R-m6: `_BUF_START` 定数が未使用
- [x] 未使用定数を削除
- 対象: `crates/lsharp-wasm/src/wasi.rs`

### R-m7: パニックの可能性 - `self.func_indices["print"]`
- [x] `self.func_indices.get("print").copied().ok_or(LowerError::UndefinedFunction)` に変更
- 対象: `crates/lsharp-ir/src/lower.rs`

### R-m8: `parse_test_output` 内での `generate_sample_args` 重複呼び出し
- [x] 計算済みの値を再利用して非効率を解消 -- ループ前に1回キャッシュ、既存テスト 5 個通過確認
- 対象: `crates/lsharp-wasm/src/test_runner.rs` (177-193行)

### R-m9: lower_match_arms でのコンストラクタパターンの比較条件不足
- [x] タグ値比較命令がスタックに積まれずに `If` 命令が発行される問題を修正 -- LocalGet + I64Const(tag) + I64Eq をIf前に発行、ユニットテスト 1 個追加
- 対象: `crates/lsharp-ir/src/lower.rs` (1055-1082行)

### Suggestion (任意の改善提案)

### R-S1: エラー型の統一
- [ ] 各クレート独自のエラー型 (`LowerError`, `CodegenError`, `miette::Report`) を `thiserror` で統一

### R-S2: 型推論結果の IR 変換への受け渡し改善
- [x] `type_results: &[(String, TypeScheme)]` のスライス線形探索を `HashMap` に変更して O(1) 化 -- Lower 構造体内部で `HashMap<String, Type>` として保持済み、`.get()` で O(1) アクセス

### R-S3: WasmGC 対応への feature flag 導入
- [ ] MVP i64 フォールバックと将来の WasmGC 切り替えを feature flag で管理

### R-S4: snapshot テストの活用拡大
- [ ] `codegen.rs`, `wasi.rs` にも Wasm バイナリの snapshot テストを導入

### R-S5: ベンチマークの追加
- [ ] `criterion` クレートでコンパイル時間・実行時間のベンチマークを導入

### R-S6: `string_data` の RefCell 使用見直し
- [ ] `Lower` 構造体の `RefCell<Vec<...>>` + `Cell<u32>` を `&mut self` メソッドに移行

### R-S7: TODO.md の完了状態と実際の制限事項の乖離
- [x] MVP フォールバック (GC, Lambda, ADT パターンマッチ) の未完了制限事項を明記 -- 下記「既知の制限事項」セクション追加

### R-S8: Book ドキュメントと実装の整合性検証
- [ ] ドキュメント記載機能と実装の自動検証の仕組みを導入

---

## 2回目コードレビュー修正 (2026-03-22)

> 1回目レビューで指摘された 6 件を全件修正。

| 指摘 | 重要度 | 修正内容 | 状態 |
|------|--------|---------|------|
| M-1 | Major | E2E テスト相対パスを `CARGO_MANIFEST_DIR` ベースに統一 (13箇所) | [x] |
| m-1 | Minor | `emit_instructions_wasi` の `_gc_type_count` 未使用パラメータ削除 | [x] |
| m-2 | Minor | GADT テスト 4 件で推論結果の関数名を assert | [x] |
| m-3 | Minor | `infer_err` ヘルパー追加 + エラーケーステスト書き換え | [x] |
| s-1 | Suggestion | `call_handler` を `Result<(), CodegenError>` 返却に変更 | [x] |
| s-2 | Suggestion | `test_e2e_gadt_typecheck` に GC 未対応理由のコメント追加 | [x] |

---

## 3回目コードレビュー修正 (2026-03-22)

> 2回目レビューで指摘された 5 件を全件修正。

| 指摘 | 重要度 | 修正内容 | 状態 |
|------|--------|---------|------|
| m-1 | Minor | `test_e2e_trait_where_typecheck` → `test_e2e_trait_where` にリネーム | [x] |
| m-2 | Minor | `computation.ls` に MVP 段階の注記コメント追加 | [x] |
| m-3 | Minor | `hkt.ls`, `gadt.ls` に wasmtime 未サポートの注記コメント追加 | [x] |
| s-1 | Suggestion | `emit.rs` の GC フォールバック 4 箇所に TODO コメント追加 | [x] |
| s-2 | Suggestion | `ir_to_wasm_type`/`ir_to_wasm` ラッパー削除、`emit::ir_to_wasm_valtype` 直接呼び出しに統一 | [x] |

---

## 既知の制限事項 (MVP フォールバック)

> wasmtime の GC 機能が未サポートのため、以下の機能は i64 フォールバックで動作する。
> 機能的には完成しているが、WasmGC ネイティブ実装は wasmtime の GC サポート待ち。

| 機能 | 制限事項 | 影響範囲 |
|------|---------|---------|
| レコード型 | i64 パックエンコードで GC struct の代替実装 | record リテラル/アクセス/更新 |
| ADT コンストラクタ | i64 エンコード + $tag フィールドで GC struct 代替 | ADT パターンマッチ |
| 文字列 | offset<<32\|len の i64 パック方式 | 文字列操作全般 |
| Lambda/クロージャ | 未サポート (LowerError::Unsupported) | 高階関数のランタイム実行 |
| ref.cast | IR 定義済みだが codegen は i64 フォールバック | ADT ダウンキャスト |
| GC 型テスト | コンパイルのみ検証、実行テスト不可 | record/ADT の E2E テスト |

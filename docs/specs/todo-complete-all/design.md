# TODO 残タスク一括完了 - 設計書

> 最終更新: 2026-03-24

## 概要

既存のコンパイラパイプラインを基盤とし、セルフホストコンパイラの完成、マクロシステムの新規追加、VSCode 拡張の構築、WASI I/O の補完、GC 検証基盤の整備を行う設計。パイプラインにマクロ展開パスを挿入し、型推論を2パス化することが主要な変更点。

## アーキテクチャ

### コンパイラパイプライン (拡張後)

```
Source (.ls)
  -> Lexer (token.rs + lexer.rs) [P10: quote/unquote トークン追加]
  -> Parser (parser.rs) [P10: quote/unquote/defmacro パース追加]
  -> AST (ast.rs) [P10: Quote/Unquote/DefMacro バリアント追加]
  -> MacroExpand (macro_expand.rs) [P10: 新規パス]
  -> Type Inference (infer.rs) [P8-5: 2パス化]
  -> IR Lowering (lower/) [変更なし]
  -> Wasm Codegen (wasi.rs + emit.rs) [GC: shadow stack/mark-sweep 予定]
```

セルフホスト系列 (P8-6 -- P8-9) は `selfhost/*.ls` に対する変更で、Rust コンパイラとは独立して開発。

## コンポーネント

### 型推論の2パス化 (P8-5)

- **対象ファイル**: `crates/lsharp-types/src/infer.rs`
- **改修箇所**: `infer_decl_functions` メソッド
- **設計**:
  - パス1: 全 defn の名前に対して新しい型変数 (TypeVar) を生成し TypeEnv に仮登録
  - パス2: 各 defn の body を推論。仮登録された型変数を通じて前方参照が可能
  - パス後: let-polymorphism のための generalize を適用
- **注意事項**:
  - 既存の非再帰 defn は動作を変えない
  - 全 defn を同一グループとして扱う単純な方式を採用 (相互再帰グループの検出は不要)
  - `TypeScheme` の generalize タイミング -- 2パス目完了後に一括 generalize

### セルフコンパイラ MVP (P8-6)

#### Compiler.ls 拡張

| タスク | tag | IR 生成内容 |
|--------|-----|-------------|
| let | 7 | env にバインディング追加、init-expr コンパイル、body コンパイル |
| if | 6 | cond コンパイル -> IR::If -> then コンパイル -> IR::Else -> else コンパイル -> IR::End |
| apply | 5 | args を順にコンパイル -> IR::Call(func-index) |
| lambda | 8 | パラメータを env 登録 -> body コンパイル (クロージャなし、直接呼出しのみ) |

#### WasmEmit.ls 拡張

| セクション | ID | 内容 |
|-----------|-----|------|
| Type (既存) | 1 | 関数シグネチャ |
| Import | 2 | WASI fd_write import |
| Function | 3 | funcidx -> typeidx マッピング |
| Memory | 5 | 1ページ (64KB) の linear memory |
| Export | 7 | `_start` 関数エクスポート |
| Code | 10 | IR 命令 -> Wasm opcodes 変換 |

**IR -> Wasm 変換テーブル (Code セクション)**:

```
IR::I64Const(n) -> 0x42 + signed LEB128(n)
IR::LocalGet(i) -> 0x20 + unsigned LEB128(i)
IR::LocalSet(i) -> 0x21 + unsigned LEB128(i)
IR::Call(i)     -> 0x10 + unsigned LEB128(i)
IR::If          -> 0x04 0x7E (block type i64)
IR::Else        -> 0x05
IR::End         -> 0x0B
IR::I64Add      -> 0x7C
IR::I64Sub      -> 0x7D
IR::I64Mul      -> 0x7E
```

### Parser の完成 (P8-7)

- **Lexer.ls 拡張**: トークンを (kind, start, end) 3つ組で返す。vector に3要素ずつ格納
- **Parser.ls 完全 AST**: vector ベースの AST ノード。各ノードは `[tag, ...fields]` の vector
  - defn: `[tag-defn, name-idx, params-vec, body-node]`
  - let: `[tag-let, name-idx, init-node, body-node]`
  - if: `[tag-if, cond-node, then-node, else-node]`
  - apply: `[tag-apply, func-node, args-vec]`
- **match 式パース**: pattern は `[tag-pattern, constructor-tag, bindings-vec]`

### Compiler/WasmEmit の完成 (P8-8)

- **Compiler.ls**: do ブロック (逐次実行)、defn (関数テーブル登録)、ビルトイン (名前テーブルで判定)、再帰 (事前登録)、match (タグ判定 + 分岐)
- **WasmEmit.ls**: ビルトインヘルパー (__print_i64, __print_string, __alloc)、Data セクション (文字列定数配置)、符号付き LEB128

### ブートストラップ検証 (P8-9)

- **Main.ls**: read-file でソース読込 -> tokenize -> parse -> compile -> emit-wasm -> write-file で .wasm 出力
- **モジュール結合戦略**: 全 selfhost/*.ls を1ファイルに concat (依存順: Token -> Lexer -> AST -> Parser -> Type -> TypeScheme -> IR -> Compiler -> WasmEmit -> Main)
- **固定点検証**: `sha256sum stage1.wasm stage2.wasm` で比較

### WASI ファイル I/O (P1-3)

- **stdin ラッパー**: `emit_read_line_func` を wasi.rs に追加。fd=0 に対して fd_read を呼び出し
- **パス操作** (stdlib/Path.ls): path-join, path-extension, path-basename, path-dirname
- **JSON パーサー** (stdlib/Json.ls): 再帰降下パーサー。ADT `(type JsonValue Null (Bool b) (Num n) (Str s) (Arr vs) (Obj kvs))` で出力

### VSCode 拡張 (P9-6)

- **ディレクトリ構成**:
  ```
  editors/vscode/
    package.json              # VSCode 拡張マニフェスト
    syntaxes/
      lsharp.tmLanguage.json  # TextMate grammar
    src/
      extension.ts            # 最小限の TypeScript シェル
      wasm-bridge.ts          # Wasm バインディング
  ```
- **TextMate grammar**: S式のキーワード (defn, let, if, match, do, type, module, import, open) + コメント (`;`) + 文字列 + 数値
- **リンター**: AST 走査で未使用変数/import を検出、Diagnostic として報告
- **フォーマッタ**: S式のインデント規則 (2スペース、closing paren は同行)

### マクロシステム (P10)

- **パイプライン挿入位置**: Parser -> AST -> **MacroExpand** -> Type Inference
- **Lexer 拡張**: `Quote` (`'`), `Unquote` (`~`), `UnquoteSplice` (`~@`) トークン追加
- **AST 拡張**:
  ```rust
  enum Expr {
      // ... existing variants ...
      Quote(Box<Expr>),
      Unquote(Box<Expr>),
      UnquoteSplice(Box<Expr>),
  }
  enum Decl {
      // ... existing variants ...
      DefMacro {
          name: String,
          params: Vec<String>,
          macro_type: Option<Type>,
          body: Expr,
      },
  }
  ```
- **マクロ展開エンジン** (macro_expand.rs):
  - `MacroEnv`: マクロ名 -> (params, body) のマッピング
  - `expand_program(program: Program) -> Program`: 全 Decl/Expr を再帰的に展開
  - 深度制限: 128 (無限ループ防止)
  - gensym: `__macro_N` 形式の一意名生成
- **衛生マクロ** (hygiene.rs):
  - `HygienicIdent { name: String, scope: ScopeId }`
  - Sets of Scopes 方式で名前解決
  - `(unhygienic name)` で衛生性を明示的に破壊可能

### GC/メモリ管理 (検証基盤)

- **オブジェクトヘッダ** (16 bytes):
  ```
  [0..4]   tag: i32          -- 型タグ
  [4..8]   size_or_words: i32 -- サイズ (バイト or ワード数)
  [8..12]  mark_state: i32    -- GC マークビット (0=white, 1=gray, 2=black)
  [12..16] next_free: i32     -- free list 用リンク / 予備
  ```
- 検証テスト 7件を追加済み。本体実装は memory-management-roadmap.md の Phase 1-6 に沿って段階的に進行予定

### CI/CD

- **対象ファイル**: `.github/workflows/ci.yml`
- **新規ジョブ**: `bootstrap` -- stage1 生成 -> stage2 生成 -> sha256sum 比較

## データ設計

### IR 命令の拡張 (selfhost/IR.ls)

| タグ | 名前 | オペランド |
|------|------|-----------|
| 1 | I64Const | value |
| 2 | LocalGet | index |
| 3 | LocalSet | index |
| 4 | Call | func-index |
| 5 | If | (none) |
| 6 | Else | (none) |
| 7 | End | (none) |
| 8 | I64Add | (none) |
| 9 | I64Sub | (none) |
| 10 | I64Mul | (none) |
| 11 | Drop | (none) |
| 12 | Return | (none) |

### GC オブジェクトレイアウト

```
String:     [header(16)] [length(4)] [bytes...]
ADT:        [header(16)] [field_count(4)] [field_0(8)] [field_1(8)] ...
Vector:     [header(16)] [length(4)] [capacity(4)] [elem_0(8)] [elem_1(8)] ...
HashMap:    [header(16)] [size(4)] [capacity(4)] [buckets...]
Closure:    [header(16)] [func_idx(4)] [capture_count(4)] [cap_0(8)] [cap_1(8)] ...
RefCell:    [header(16)] [value(8)]
```

## インターフェース

### API (新規 Builtin 関数)

| 名前 | シグネチャ | 説明 |
|------|-----------|------|
| `read-line` | `() -> String` | stdin から1行読み込み |
| `path-join` | `(String, String) -> String` | パス結合 |
| `json-parse` | `(String) -> JsonValue` | JSON パース |

### マクロ展開 API

```rust
pub fn expand_macros(program: Program) -> Result<Program, MacroError>;

pub struct MacroError {
    pub kind: MacroErrorKind,
    pub span: Span,
    pub expansion_trace: Vec<Span>,
}

pub enum MacroErrorKind {
    MaxDepthExceeded,
    UndefinedMacro(String),
    ArityMismatch { expected: usize, got: usize },
    InvalidPattern,
}
```

## エラーハンドリング

- **相互再帰の型推論エラー**: 既存の miette ベースのエラー報告を維持。前方参照先の位置情報を含める
- **マクロ展開エラー**: 展開トレースバック (どのマクロがどの位置で展開されたか) を miette label で表示
- **GC エラー**: GC 中の異常 (到達不能メモリへのアクセス等) は panic で即座に停止
- **selfhost コンパイルエラー**: exit code で報告 (0=成功, 1=パースエラー, 2=コンパイルエラー)

## テスト戦略

### ユニットテスト

| コンポーネント | テストファイル | テスト内容 |
|---------------|---------------|-----------|
| 2パス型推論 | infer.rs の tests | 相互再帰関数の型推論、既存の非再帰 defn の挙動維持 |
| マクロ展開 | macro_expand.rs の tests | quote/unquote 展開、defmacro 展開、深度制限、衛生性 |
| GC | gc.rs の tests | mark/sweep ロジック、free list 管理 |
| JSON パーサー | stdlib テスト | 各 JSON 型のパース、ネスト、エスケープ文字 |

### E2E テスト

| テスト | 検証内容 |
|--------|---------|
| 相互再帰 E2E | `(defn even? [n] ...)` + `(defn odd? ...)` の相互呼び出し |
| selfhost MVP | `(defn main [] 42)` の selfhost コンパイル -> wasmtime 実行 |
| stage1 E2E | stage1.wasm でテスト .ls をコンパイル -> 出力 .wasm を wasmtime 実行 |
| WASI stdin | `read-line` -> echo back テスト |
| マクロ E2E | `(defmacro when ...)` の展開 + 実行 |
| GC E2E | 大量アロケーション -> GC 起動 -> ヒープ回復の検証 |

### テスト実績

- テスト総数: 709 -> 817 (+108)
- 全テストパス (817/817)
- クレート別内訳: lsharp-wasm(E2E) 241, lsharp-syntax 164, lsharp-types(infer) 149, lsharp-types(constraints) 124, lsharp-ir 44, lsharp-lsp 29, lsharp-driver 27, lsharp-wasm(unit) 23, lsharp-docs 14, lsharp-types(regex) 2

## 実装優先順位

### Phase I: 基盤整備 (並列実行可能)

1. 相互再帰の前方参照 -- 最重要ブロッカー。P8-7/P8-8/P8-9 の全てがこれに依存
2. セルフコンパイラ MVP -- 前方参照と独立して進行可能
3. WASI I/O 補完 -- 独立して進行可能

### Phase II: セルフホスト完成 (Phase I 完了後)

4. stage1 コンパイル検証
5. Parser 完成
6. Compiler/WasmEmit 完成
7. ブートストラップ検証

### Phase III: エコシステム (独立進行可能)

8. マクロシステム -- 他タスクと独立
9. VSCode 拡張

### Phase IV: ランタイム改善 (独立進行可能、回帰リスク大)

10. GC (Phase 1-6)

### Phase V: CI (Phase II 完了後)

11. ブートストラップ CI

## 関連ドキュメント

- [要件定義書](./requirements.md)
- [TODO 全残タスク完了 (前回)](../todo-complete/design.md)

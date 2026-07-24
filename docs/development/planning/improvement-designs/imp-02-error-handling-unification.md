# imp-02: エラーハンドリング統一と LS#### エラーコード体系

> 対象 issue: [I-02](../../../../ISSUES.md#i-02) (エラーハンドリング不統一)、[DOC-06](../../../../ISSUES.md#doc-06) (エラーコード体系未定義)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase A-1

## 現状の正確な把握 (2026-06-12 コード検証済み)

### エラー型の現状一覧

| 型 | 場所 | バリアント | thiserror | span 保持 |
|----|------|-----------|-----------|-----------|
| `LexError` | `crates/lsharp-syntax/src/lexer.rs:6-15` | UnexpectedChar / UnterminatedString / InvalidNumber | あり | 全バリアント |
| `ParseError` | `crates/lsharp-syntax/src/parser.rs:7-23` | Unexpected / UnexpectedEof / UnknownForm / Multiple | あり | Unexpected, UnknownForm のみ |
| `TypeError` | `crates/lsharp-types/src/infer.rs:21-99` | Mismatch / InfiniteType / UndefinedVar / UndefinedConstructor / ArityMismatch / UndefinedRecord / UndefinedField / RecursiveAlias / UndefinedAlias / UndefinedTrait / MissingImpl / MismatchWithAlias / KindMismatch (13 個) | あり | ほぼ全バリアント |
| `LowerError` | `crates/lsharp-ir/src/lower/mod.rs:19-25` | Unsupported / UndefinedFunction | あり | **なし** |
| `ModuleGraphError` | `crates/lsharp-ir/src/module_graph.rs:84-102` | CyclicDependency / ModuleNotFound / ModuleNotExported / DuplicateModule | あり | **なし** |
| `CodegenError` | `crates/lsharp-wasm/src/codegen.rs:11-14` | Error(msg: String) の 1 個のみ | あり | **なし** |
| `LsharpError` | `crates/lsharp-driver/src/error.rs:10-39` | 上記を `#[from]` で集約 | あり | -- |

### フロントエンドの現状

- **CLI**: `main()` は `miette::Result<()>` (`main.rs:238`)。ただし変換は
  `.map_err(|e| miette::miette!("{}: {}", file.display(), e))?` (main.rs:283, 310, 356 等) の
  **文字列化**であり、span 情報が `miette::miette!` の時点で失われている
- **LSP**: `diagnostic_error` (`crates/lsharp-lsp/src/util.rs:356-364`) が
  固定 `Range::new(Position::new(0,0), Position::new(0,0))` を設定し、`Diagnostic.code` は未設定。
  ParseError/TypeError は `format!("{e}")` で文字列化されて渡る (util.rs:367, 430)。
  span → Range の変換ロジック自体は util.rs:349 付近に存在するが、エラー診断経路で使われていない
- **MCP**: `check_tool` (`mcp_server.rs:260-270`) は `{ok, diagnostics: [{message, severity}]}` を返す
  (コードなし)。`lsharp_errors` は E0001〜E0005 をハードコード (`mcp_server.rs:438-462`)
- **panic**: `crates/lsharp-ir/src/lib.rs:3609, :3611` がファイル I/O / parse 失敗で panic

## 設計

### 1. エラーコードの単一ソース

新規ファイル `crates/lsharp-syntax/src/error_codes.rs` ではなく、**依存の最下層に置けない**
(コードは全層に必要) ため、新クレートは作らず以下とする:

- 各エラー型の **バリアントに 1 コードを割り当て**、各クレートに
  `impl XxxError { pub fn code(&self) -> &'static str }` を実装する (依存追加なし)
- コード ↔ 説明 ↔ 対処の対応表は `crates/lsharp-driver/src/error_codes.rs` に
  `pub const ERROR_CODES: &[(&str, &str, &str, &str)]` (code, summary, detail, fix) として置き、
  MCP `lsharp_errors` / CLI `--explain` (任意) / docs 生成が共有する
- docs 側の正本は `docs/guides/error-reference.md` (imp-05)。
  対応表とドキュメントの一致は契約テスト (ERROR_CODES の全コードが md に出現する) で固定する

### 2. LS#### コード割り当て (初期セット)

| コード | バリアント |
|--------|-----------|
| LS0001-LS0003 | LexError::UnexpectedChar / UnterminatedString / InvalidNumber |
| LS0101-LS0104 | ParseError::Unexpected / UnexpectedEof / UnknownForm / Multiple |
| LS1001-LS1013 | TypeError の 13 バリアント (UndefinedVar=LS1001, Mismatch=LS1002, InfiniteType=LS1003, ArityMismatch=LS1004, UndefinedConstructor=LS1005, UndefinedRecord=LS1006, UndefinedField=LS1007, RecursiveAlias=LS1008, UndefinedAlias=LS1009, UndefinedTrait=LS1010, MissingImpl=LS1011, MismatchWithAlias=LS1012, KindMismatch=LS1013) |
| LS2001+ | 制約・メタデータ検証 (constraints.rs / metadata_check.rs のエラー) |
| LS3001-LS3002 | LowerError::Unsupported / UndefinedFunction |
| LS3101-LS3104 | ModuleGraphError の 4 バリアント |
| LS4001 | CodegenError::Error |
| LS4002 | GC 容量超過 (imp-03 の grow 失敗診断、新設) |
| LS4003 | GC root slot invariant failure (compiler-side safe-point spill の slot 不整合診断) |
| LS5001+ | driver 固有 (lsharp.toml 不正、パッケージ解決失敗 等) |

規則: 一度割り当てたコードの意味は変えない。欠番は再利用しない。
既存 MCP の E0001〜E0005 への対応: E0001→LS1001, E0002/E0003→LS1002 (if 系は Mismatch の
文脈表示で区別), E0004→LS1004, E0005→LS1003。1 リリースの間 `lsharp_errors` 応答に
`legacy_code` として併記する。

### 3. span の貫通と panic 排除

1. `LowerError` に `span: Option<Span>` を追加 (AST ノードは span を保持しているため
   lowering 時に引き渡すだけ)。`Unsupported` の発生箇所すべてで span を埋める
2. `LowerError::Io { path: String, source: std::io::Error }` バリアントを追加し、
   `lib.rs:3609, :3611` の panic を `Result` 伝播へ置換
   (呼び出し元 `compile_multi_file` 系は既に `Result<_, String>` のため伝播可能)
3. `CodegenError` は内部不変条件違反が主のため span なしを許容。ただし
   メッセージに関数名を含める
4. panic 許容基準: テストコード内は許容 (`expect("理由")` 推奨)。
   到達不能分岐は `unreachable!("不変条件の説明")` のみ許容。入力起因の失敗は禁止
5. 残数監視: `scripts/` に panic/unwrap カウントスクリプトを置き、現状値を固定して
   増加を CI で fail にする (段階的にゼロへ)

### 4. フロントエンドへの配線

- **CLI**: `miette::miette!("{e}")` の文字列化をやめ、各エラー型に
  `#[derive(miette::Diagnostic)]` + `#[diagnostic(code(...))]` を追加して
  `Into<miette::Report>` で渡す。`NamedSource` + span でソース抜粋付き表示になる
  (下層クレートに miette 依存が増えるが workspace 管理で統一)
- **LSP**: `diagnostic_error` を `diagnostic_from_error(err, source) -> Diagnostic` に置換。
  util.rs:349 付近の既存 span→Range 変換を使い、`Diagnostic.code = Some(NumberOrString::String(err.code()))`
  を設定する。**これにより固定 Range(0,0) 問題も同時に解消する**
- **MCP**: `check_tool` の diagnostics 要素に `code` フィールドを追加。
  `lsharp_errors` のハードコード表を `ERROR_CODES` 参照へ置換

### 5. 実装順序 (TDD)

1. RED: `XxxError::code()` のユニットテスト (全バリアント網羅、コード重複なし) →
   GREEN: code() 実装 (syntax → types → ir → wasm の順)
2. RED: ir/lib.rs:3609 相当の不正パス入力で Err が返るテスト → GREEN: panic 置換
3. RED: LowerError の span 保持テスト → GREEN: span 追加
4. LSP: 既知エラーソースで Diagnostic.range が実位置・code が LS#### になるテスト → 実装
5. MCP: lsharp_errors が ERROR_CODES 全件を返すテスト → 実装
6. CLI: スナップショットテストで診断表示に LS#### が含まれることを固定
7. panic 残数監視スクリプト + CI 組み込み

## 影響範囲

- 既存エラーメッセージ文言 (`#[error("...")]`) は不変。表示に code が追加されるため、
  CLI 出力のスナップショットは更新が必要 (insta review で一括)
- LSP の診断 range が 0,0 から実位置に変わる — クライアント側の見え方が改善する変更で、
  既存 LSP テスト (`crates/lsharp-lsp/` のテスト群) の期待値更新が必要

## ステータス

設計 (2026-06-12 起草、同日コード検証に基づき具体化)。着手時は TODO.md に Phase A-1 として項目を作成する。

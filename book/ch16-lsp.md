# LSP サーバー -- エディタ統合

## Language Server Protocol とは

Language Server Protocol (LSP) は、プログラミング言語のツール機能 (補完、定義ジャンプ、エラー表示等) をエディタから分離するためのプロトコルである。Microsoft が Visual Studio Code のために設計し、現在では多くのエディタとプログラミング言語が対応している。

LSP 以前は、各エディタが独自のプラグイン API を持ち、各言語がエディタごとにプラグインを開発する必要があった。M 個の言語と N 個のエディタがある場合、M x N 個のプラグインが必要だった。LSP はこの問題を M + N に削減する。

```
エディタ (クライアント)  ←  JSON-RPC  →  言語サーバー

VSCode    ─┐                          ┌─ L# Language Server
Vim/Neovim ─┤ ← LSP プロトコル →      ├─ Rust Analyzer
Emacs     ─┘                          └─ TypeScript Server
```

## L# の LSP 実装

L# の LSP サーバーは `crates/lsharp-lsp` クレートに実装されている。1,224 行の Rust コードで、以下のモジュール構成を持つ:

| ファイル | 行数 | 役割 |
|----------|------|------|
| lib.rs | 主要 | LsharpBackend + LanguageServer trait 実装 |
| util.rs | | parse_and_check, find_definition, find_type_at_position |
| references.rs | | シンボルの参照箇所検索 |
| rename.rs | | シンボルのリネーム |
| format.rs | | ソースコードフォーマット |

## tower-lsp 統合

L# は **tower-lsp** クレートを使って LSP サーバーを構築する。tower-lsp は Rust の非同期フレームワーク (tokio) 上に構築されており、JSON-RPC の通信処理を自動的に行う。

### LsharpBackend 構造体

```rust
pub struct LsharpBackend {
    client: Client,
    /// ソースコードキャッシュ (URI → ソース全文)
    source_cache: RwLock<HashMap<Url, String>>,
}
```

`client` は LSP クライアント (エディタ) への通信ハンドルで、診断メッセージの送信に使用する。`source_cache` はドキュメントのソースコードをメモリ上にキャッシュし、ユーザーの編集をリアルタイムに追跡する。

### サーバー能力の宣言

`initialize` ハンドラでサーバーが対応する機能を宣言する:

```rust
async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions { ... })),
            document_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        },
        ..Default::default()
    })
}
```

`TextDocumentSyncKind::FULL` は、ドキュメントの変更時に全文を送信する方式。差分同期 (`INCREMENTAL`) よりも実装が単純で、L# のソースファイルは通常小さいため十分な性能を発揮する。

## ドキュメント同期

ユーザーがファイルを開いたとき (`didOpen`) と編集したとき (`didChange`) に、ソースコードをキャッシュに保存し、診断を実行する:

```rust
async fn did_open(&self, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let text = params.text_document.text;
    let diagnostics = util::parse_and_check(&text);
    self.set_source(uri.clone(), text);
    self.client.publish_diagnostics(uri, diagnostics, None).await;
}
```

ファイルを開くたびに L# コンパイラのパーサーと型推論器を実行し、エラーがあれば診断メッセージとしてエディタに送信する。

## 診断 (Diagnostics)

`parse_and_check` 関数がパースと型チェックを行い、エラーを LSP の `Diagnostic` 形式に変換する:

```rust
pub fn parse_and_check(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // パース
    match parse(source) {
        Ok(program) => {
            // 型チェック
            match infer_program(&program) {
                Ok(_) => {}
                Err(type_error) => {
                    diagnostics.push(type_error_to_diagnostic(&type_error));
                }
            }
        }
        Err(parse_error) => {
            diagnostics.push(parse_error_to_diagnostic(&parse_error));
        }
    }

    diagnostics
}
```

エラーの `Span` (バイトオフセット) を LSP の `Position` (行・列番号) に変換する処理が必要になる。ソースコードの改行位置を走査して、バイトオフセットから行番号と列番号を計算する。

## 定義ジャンプ

カーソル位置のシンボルの定義箇所にジャンプする機能。`find_definition` 関数が AST を走査し、シンボルの定義位置を返す:

```rust
pub fn find_definition(
    source: &str,
    position: Position,
) -> Option<Location>
```

処理の流れ:

1. `position` (行・列) をバイトオフセットに変換
2. AST を走査し、オフセットに対応するシンボル名を特定
3. AST 内の全宣言を走査し、同名の `defn` や `type` を検索
4. 見つかった定義の `Span` を `Location` に変換して返す

## 型ホバー

カーソル位置のシンボルの型情報を表示する機能:

```rust
async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    if let Some(source) = self.get_source(uri) {
        if let Some(type_info) = util::find_type_at_position(&source, position) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```\n{}\n```", type_info),
                }),
                range: None,
            }));
        }
    }
    Ok(None)
}
```

`find_type_at_position` は型推論の結果を利用して、シンボルの型を文字列として返す。例えば `fib` 関数にカーソルを置くと `(Int) -> Int` と表示される。

## 補完

キーワードとスコープ内のシンボルを補完候補として提供する:

```rust
async fn completion(
    &self,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let mut items = Vec::new();

    // キーワード補完 (17種)
    for keyword in &["defn", "let", "if", "match", "type", "fn", "do",
                      "module", "import", "record", "trait", "impl",
                      "where", "type-alias", "type-constrained",
                      "private", "computation-builder"] {
        items.push(CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    // ソースからシンボルを収集
    if let Some(source) = self.get_source(&params.text_document_position.text_document.uri) {
        // 関数名・変数名をパースして補完候補に追加
        // ...
    }

    Ok(Some(CompletionResponse::Array(items)))
}
```

## 参照検索とリネーム

### 参照検索 (references.rs)

指定されたシンボルが使われている全ての箇所を返す:

```rust
pub fn find_references(source: &str, target: &str) -> Vec<Location>
```

AST を走査し、指定されたシンボル名と一致する全ての `Expr::Var` と `Decl::Function` の位置を収集する。

### リネーム (rename.rs)

シンボル名を一括変更する。定義と全ての参照箇所を同時に変更する:

```rust
pub fn rename_symbol(
    source: &str,
    position: Position,
    new_name: &str,
) -> Option<WorkspaceEdit>
```

リネーム処理の流れ:

1. カーソル位置のシンボル名を特定
2. 定義箇所と全参照箇所を検索
3. 各箇所の `TextEdit` を生成
4. `WorkspaceEdit` として返す

## フォーマット (format.rs)

L# のソースコードを整形する:

```rust
pub fn format_source(source: &str) -> String
```

S 式の自動インデント、一貫した空白の挿入を行う。S 式はインデントの深さが視覚的に重要であるため、フォーマッタは括弧のネストに応じてインデントを調整する。

## テスト

LSP サーバーのテストは 27 件のユニットテストで構成される:

```rust
// 定義ジャンプのテスト
#[test]
fn test_find_definition_simple() {
    let source = "(defn foo [] 42)\n(defn main [] (foo))";
    let def = find_definition(source, Position::new(1, 15));
    assert!(def.is_some());
}

// 型ホバーのテスト
#[test]
fn test_find_type_at_position() {
    let source = "(defn add [x y] (+ x y))";
    let ty = find_type_at_position(source, Position::new(0, 5));
    assert_eq!(ty, Some("(Int, Int) -> Int".to_string()));
}
```

## エディタでの使用

L# の LSP サーバーは以下のコマンドで起動する:

```bash
cargo run --bin lsharp-lsp
```

VSCode で使用する場合、`settings.json` に以下を追加:

```json
{
  "lsharp.serverPath": "/path/to/lsharp-lsp"
}
```

Neovim では `lspconfig` を使って設定:

```lua
require('lspconfig').lsharp.setup {
  cmd = { '/path/to/lsharp-lsp' },
  filetypes = { 'lsharp' },
  root_dir = function(fname)
    return require('lspconfig').util.find_git_ancestor(fname)
  end,
}
```

LSP サーバーにより、L# での開発体験がエディタの種類を問わず向上する。エラーの即時表示、型情報の確認、定義ジャンプといった機能は、言語の実用性を大幅に高める。

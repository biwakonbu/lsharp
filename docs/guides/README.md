# guides

**読者**: L# でアプリやライブラリを書く人（コンパイラ内部を深追いしない人向けのハウツー置き場）。

現時点で利用者向けに整備済みのページ:

- [`quick-start.md`](./quick-start.md) -- hello world から module までの 5 分チュートリアル
- [`language-reference.md`](./language-reference.md) -- 構文・型・module・metadata・stdlib の利用者向けリファレンス
- [`package-layout.md`](./package-layout.md) -- パッケージ標準レイアウトと `lsharp init` の生成物
- [`metadata-driven-development.md`](./metadata-driven-development.md) -- `:doc` / `:example` / `:invariant` をテストと docs へ使う手順
- [`ide-setup.md`](./ide-setup.md) -- `lsharp lsp` と editor / AI tool 連携
- [`deployment-targets.md`](./deployment-targets.md) -- `wasi-component` / `wasi-preview1` / `web-wasm` / `native` の選び方
- [`stdlib-guide.md`](./stdlib-guide.md) -- stdlib API の探し方と generated docs / MCP での参照
- [`examples.md`](./examples.md) -- `examples/*.ls` と言語機能、実行状態、関連ガイドの対応表

読む順序に迷う場合は、`quick-start.md`、`language-reference.md`、`examples.md`、必要な個別 guide の順で確認する。

言語を使う導線はこの directory を正面玄関にする。リポジトリ直下の [`book/`](../../book/) はコンパイラ実装を読む開発者向けの読み物であり、公開 CLI の通常手順では `parse` / `check` / `fmt` を直接案内しない。

コンパイラ実装や v1 契約は [`../language/`](../language/) と [`../development/`](../development/) の対象外（利用者向けではない）。

エラーコードリファレンスは `DOC-06` / `imp-02` の `LS####` 体系導入後に追加する。

公開サイトに載せる guide の一覧、表示順、出力先は [`../site.toml`](../site.toml) を正本とする。この directory に Markdown を追加しただけでは公開ページにはならない。

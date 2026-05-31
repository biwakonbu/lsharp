# guides

**読者**: L# でアプリやライブラリを書く人（コンパイラ内部を深追いしない人向けのハウツー置き場）。

現時点で利用者向けに整備済みのページ:

- [`quick-start.md`](./quick-start.md) -- hello world から module までの 5 分チュートリアル
- [`language-reference.md`](./language-reference.md) -- 構文・型・module・metadata・stdlib の利用者向けリファレンス
- [`package-layout.md`](./package-layout.md) -- パッケージ標準レイアウトと `lsharp init` の生成物

言語の読み物としてはリポジトリ直下の [`book/`](../book/) を参照する。

コンパイラ実装や v1 契約は [`../language/`](../language/) と [`../development/`](../development/) の対象外（利用者向けではない）。

公開サイトに載せる guide の一覧、表示順、出力先は [`../site.toml`](../site.toml) を正本とする。この directory に Markdown を追加しただけでは公開ページにはならない。

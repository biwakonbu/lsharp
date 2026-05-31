# Documentation Site Operations

L# の公開ドキュメントサイトは、既存の Markdown と stdlib metadata を正本として静的 HTML に変換する。生成物は `_site/` に出力し、GitHub Pages へ公開できる。

## SSOT

サイト構成の正本は `docs/site.toml` とする。

- ページ一覧、section、表示順、出力先は `docs/site.toml` にだけ定義する
- 本文の正本は各 `source` の Markdown ファイルとする
- 標準ライブラリ API の正本は `stdlib/*.ls` の `:doc` / `:params` / `:returns` / `:example` metadata とする
- `_site/` は生成物であり、通常は commit しない

新しい公開ページを追加する場合は、Markdown を追加したうえで `docs/site.toml` に `[[sections.pages]]` を追加する。`doc-site` は manifest に存在しない Markdown を自動公開しない。

## ローカル生成

```bash
cargo run -p lsharp-driver -- doc-site --output _site
```

CI と同じ確認を行う場合は次を使う。

```bash
bash scripts/ci/build-doc-site.sh _site
```

この script は出力先を作り直し、`index.html`、`.nojekyll`、`sitemap.xml`、`docs-site-manifest.json`、代表ページ、stdlib API JSON が生成されることを確認する。

## 公開フロー

`.github/workflows/docs.yml` は `main` push と手動実行で `_site/` を build し、GitHub Pages artifact として deploy する。Pull Request では build と artifact 作成まで行い、deploy は実行しない。

GitHub 側の Pages 設定では、Build and deployment の Source を GitHub Actions にする。公開 URL は `docs/site.toml` の `base_url` と合わせる。

## 変更時の確認

ドキュメントサイト関連を変更した場合は、最低限次を確認する。

```bash
cargo test -p lsharp-driver doc_site::tests
bash scripts/ci/build-doc-site.sh _site
bash scripts/audit_docs.sh
```

`docs/site.toml` の source / output が壊れている場合、`doc-site` の manifest validation で失敗する。

# docs

リポジトリ内ドキュメントの入口。**誰向けか**を先に決めてから読むと迷子になりにくい。

## 読者別の入口

| 誰向けか | まず読む場所 |
|----------|----------------|
| L# でプログラムを書きたい人 | リポジトリ直下の [`book/`](../book/)（言語の講義的ガイド）、将来は [`guides/`](./guides/) |
| コンパイラや backend を実装・変更する人 | [`language/`](./language/)（v1 契約）→ 必要に応じて [`development/`](./development/) |
| Phase / CI / リリースに関わる人 | [`development/`](./development/)（計画・検証・運用） |
| 意思決定の経緯を追いたい人 | [`adr/`](./adr/)（JSONL 形式の意思決定ログ） |
| ドキュメント生成・スキーマを触る人 | [`schemas/`](./schemas/)（JSON Schema。`lsharp-docs` 等から参照） |

## ディレクトリ構成

- `language/` -- 実装済み、または v1 契約として扱う言語 / runtime / backend の仕様
- `development/` -- Phase 11 の計画、検証、運用（CI / release / distribution / signing を含む）
- `adr/` -- 意思決定ログ（JSONL）
- `guides/` -- 利用者向けチュートリアル / マニュアル（現状はプレースホルダ）
- `schemas/` -- ドキュメント生成・レビュー管理用の JSON Schema

`book/` は `docs/` 外の独立導線として維持する（相対パスは [`../book/`](../book/)）。

自動生成される性能レポートは [`development/validation/BENCHMARK.md`](./development/validation/BENCHMARK.md) に出力される。内容を変える場合は `scripts/bench-report.sh` を編集してから再実行する。

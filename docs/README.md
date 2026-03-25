# docs

リポジトリ内ドキュメントの入口。**誰向けか**を先に決めてから読むと迷子になりにくい。

## 読者別の入口

| 誰向けか | まず読む場所 |
|----------|----------------|
| L# でプログラムを書きたい人 | リポジトリ直下の [`book/`](../book/)（言語の講義的ガイド）、将来は [`guides/`](./guides/) |
| コンパイラや backend を実装・変更する人 | [`language/`](./language/)（v1 契約）→ 必要に応じて [`development/`](./development/) |
| Phase / CI / リリース / 移行に関わる人 | [`development/`](./development/)（計画・検証・運用・タスク別 specs） |
| ドキュメント生成・スキーマを触る人 | [`schemas/`](./schemas/)（JSON Schema。`lsharp-docs` 等から参照） |

## ディレクトリ構成

- `language/` -- 実装済み、または v1 契約として扱う言語 / runtime / backend の仕様
- `development/` -- Phase 11 の計画、検証、運用、移行、タスク単位の要件・設計
- `guides/` -- 利用者向けチュートリアル / マニュアル（現状はプレースホルダ）

`development/specs/` の `requirements.md` / `design.md` は、実装タスク単位の文書として保持する。
そこで固まった恒久契約だけを `language/` へ要約・昇格し、要件定義書や設計書を丸ごと移設しない。
また、履歴性の強いタスク文書は `development/specs/archive/` に寄せ、主導線には現行の重要文書だけを残す。

`book/` は `docs/` 外の独立導線として維持する（相対パスは [`../book/`](../book/)）。

`adr/` は意思決定ログ（JSONL）。本ディレクトリの再編対象外。

自動生成される性能レポートは [`development/validation/BENCHMARK.md`](./development/validation/BENCHMARK.md) に出力される。内容を変える場合は `scripts/bench-report.sh` を編集してから再実行する。

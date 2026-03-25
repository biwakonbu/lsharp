# specs

実装タスクごとの要件定義書 / 設計書をサブディレクトリ単位で保持する。
現行の作業導線として読む文書を絞り、履歴性の強いものは `archive/` へ分けている。

**主な読者**: 当該タスクに着手する実装者、経緯を調べるメンテナー。恒久契約の参照は [`../../language/README.md`](../../language/README.md) を先に読む。

## 現在の主導線

まず読むべき文書は次の 3 つに絞る。

| ディレクトリ | ざっくりした内容 | 他との関係 |
|--------------|------------------|------------|
| `todo-complete-all/` | TODO を大カテゴリ・多数サブタスクに分解した「一括完了」系の要件・設計（表・メトリクスが太い） | タスク分解の **参照用の正本に近い** 1 本 |
| `todo-completion-p8-p11/` | P8 コードタスクと P11 仕様固定の **トラック分割**（並行実行の設計） | 運用・CI 束の計画層 |
| `selfhost-phase11-1/` | Phase 11 第 1 段（MacroExpand / TypeInfer / Main / 固定点 E2E 等）の **成果物境界** | `todo-completion-p8-p11` の実装詳細に近い。相互に参照 |

## アーカイブ

次の文書は履歴として保持しつつ、現行の主導線から外している。

- [`archive/todo-parallel-implementation/`](./archive/todo-parallel-implementation/) -- 初期の並列実装プラン。後続文書で吸収済み
- [`archive/todo-complete/`](./archive/todo-complete/) -- Phase A〜D の進行スナップショット
- [`archive/todo-completion/`](./archive/todo-completion/) -- 品質改善スプリントの履歴

アーカイブ化の理由と読み方は [`archive/README.md`](./archive/README.md) を参照。

## `docs/language` との分担

このディレクトリは、タスク単位の設計判断と実装工程を残すための場所である。
要件、受入条件、段階的な設計案、実績、レビュー結果のような履歴情報は、ここに残す。

一方で、実装の結果として確定した言語 / runtime / backend の恒久契約は、`docs/language/` へ抽出して反映する。
ただし、`requirements.md` や `design.md` を丸ごと `language/` へ移すのではなく、仕様として残すべき部分だけを要約・再構成して昇格させる。

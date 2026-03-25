# archive

`docs/development/specs/archive/` は、履歴として残すが主導線には置かないタスク文書をまとめる場所である。
現行の参照先ではなく、過去の経緯やスナップショットを確認したいときだけ読む。

## 収録方針

次の条件に当てはまる文書をこのディレクトリへ移す。

- 後続の文書で内容が上書き・吸収されている
- 現在の実装判断や作業導線の正本ではない
- 履歴として残す価値はあるが、通常の読者にはノイズになりやすい

## 現在の収録

| ディレクトリ | 理由 |
|--------------|------|
| `todo-parallel-implementation/` | 初期の並列実装プランであり、後続の TODO 完了系文書に吸収済み |
| `todo-complete/` | フェーズ A〜D の進行スナップショットであり、詳細は `../todo-complete-all/` に集約されている |
| `todo-completion/` | 品質改善スプリントの履歴であり、現行の主導線からは外す |

## 現行の主導線

現在の実装判断を追う場合は、まず次を参照する。

- [`../todo-complete-all/`](../todo-complete-all/)
- [`../todo-completion-p8-p11/`](../todo-completion-p8-p11/)
- [`../selfhost-phase11-1/`](../selfhost-phase11-1/)
- 恒久契約は [`../../../language/README.md`](../../../language/README.md)

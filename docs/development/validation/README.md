# validation

コンパイラの正しさ・性能をどう証明するか、および計測結果の置き場。

## 文書一覧

| 文書 | 主な読者 | 内容 |
|------|----------|------|
| [`verification-spec.md`](./verification-spec.md) | コンパイラ実装者、CI 設計者 | bootstrap 固定点、Wasm/native 差分、テスト行列、性能ゲートなど検証方針（P11-2d） |
| [`BENCHMARK.md`](./BENCHMARK.md) | メンテナー、リリース・性能の関係者 | ベンチマークのスナップショット。**手で直さない** — 更新は `scripts/bench-report.sh` を実行する |

## `BENCHMARK.md` について

- リポジトリにコミットされた版は「そのコミット時点の計測」であり、比較用の参考値である。
- 見出しや注記を変えたい場合は `scripts/bench-report.sh` のレポートテンプレートを編集し、スクリプトを再実行して再生成する。

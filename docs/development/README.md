# development

コンパイラの開発・移行・運用に関わる人向け。`docs/language/` の契約を **どう実装し、どう検証し、どう運用するか** を扱う（言語仕様そのものの正本は `language/`）。

## サブディレクトリ

| パス | 主な読者 | 内容の例 |
|------|----------|----------|
| `planning/` | ロードマップ担当、実装優先度を決める人 | Phase 11 計画、parity、完了条件、ギャップ分類 |
| `validation/` | CI・品質・性能を見る人 | 固定点・差分テスト方針、`BENCHMARK.md`（**スクリプト生成**） |
| `operations/` | リリース・インフラ担当 | CI 設定、緊急ロールバック手順 |

## `planning/` の主なファイル

- `phase11-implementation-plan.md` -- Phase 11 実装の全体計画
- `completion-criteria.md` / `compatibility-matrix.md` / `gap-classification.md` -- 完了判定と互換性
- `rust-parity-spec.md` / `toolchain-parity-spec.md` / `runtime-stability-spec.md` -- ツールチェーン・安定性
- `memory-management-roadmap.md` -- メモリ関連の長期方針

詳細は各ファイル先頭の概要に従う。

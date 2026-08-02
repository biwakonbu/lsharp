# development

コンパイラの開発・移行・運用に関わる人向け。`docs/language/` の契約を **どう実装し、どう検証し、どう運用するか** を扱う（言語仕様そのものの正本は `language/`）。

## サブディレクトリ

| パス | 主な読者 | 内容の例 |
|------|----------|----------|
| `planning/` | ロードマップ担当、実装優先度を決める人 | v0.2 設計、current milestone、parity、完了条件 |
| `validation/` | CI・品質・性能を見る人 | 固定点・差分テスト方針、`BENCHMARK.md`（**スクリプト生成**）、個別調査 bundle |
| `operations/` | リリース・インフラ担当 | CI 設定、配布 / 署名 / cross-build、緊急ロールバック手順 |

## `planning/` の主なファイル

- `README.md` -- current milestone と historical plans の入口
- `v0.2-evidence-contracts.md` -- 次版の evidence-driven contract 設計
- `v0.2-milestone-01.md` -- current milestone の task / RED / gate
- `completion-criteria.md` / `compatibility-matrix.md` / `gap-classification.md` -- 完了判定と互換性
- `rust-parity-spec.md` / `toolchain-parity-spec.md` / `runtime-stability-spec.md` -- ツールチェーン・安定性
- `memory-management-roadmap.md` -- メモリ関連の長期方針
- `improvement-roadmap.md` -- 問題台帳 ([`ISSUES.md`](../../ISSUES.md)) に対する改善フェーズと完了条件
- `improvement-designs/` -- 品質改善のテーマ別設計 (imp-01〜08。旧 V2 機能の設計は `v2-designs/`)

完了した Phase / task の判断と evidence pointer は `docs/adr/decisions-*.jsonl` に保存し、`TODO.md` には未完了項目だけを置く。

詳細は各ファイル先頭の概要に従う。

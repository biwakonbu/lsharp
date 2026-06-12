# improvement-designs: 品質改善の設計ドキュメント

[ISSUES.md](../../../../ISSUES.md) (問題台帳) と
[improvement-roadmap.md](../improvement-roadmap.md) (改善ロードマップ) に対応する、
テーマ別の改善設計ドキュメント集。

## v2-designs との役割の違い

| ディレクトリ | 役割 |
|-------------|------|
| [v2-designs/](../v2-designs/) | Phase 11 から Deferred された**新機能** (V2-01〜V2-10) の設計 |
| improvement-designs/ (本ディレクトリ) | 現バージョンの**品質問題** (ISSUES.md の D/I/DOC) に対する改善設計 |

両者が重なる領域 (WasmGC) は v2-designs 側 (v2-07) を正本とし、
imp-01 は補遺として現行コードからの移行手順のみを扱う。

## 索引

| 設計 | テーマ | 対象 issue | ロードマップ |
|------|--------|-----------|-------------|
| [imp-01](imp-01-wasmgc-full-migration.md) | WasmGC 完全移行 (v2-07 補遺) | D-01, D-02, D-03, D-04, D-06, D-09 | Phase B-1 |
| [imp-02](imp-02-error-handling-unification.md) | エラーハンドリング統一 + LS#### コード体系 | I-02, DOC-06 | Phase A-1 |
| [imp-03](imp-03-dynamic-memory-layout.md) | GC メモリレイアウト動的化・アロケータ改善 | I-03, I-04, D-10 | Phase A-3 / B-2 / B-5 |
| [imp-04](imp-04-module-system-strengthening.md) | モジュールシステム強化 (SCC 推論・キャッシュ) | D-07, I-05 | Phase C-1 / C-2 |
| [imp-05](imp-05-docs-restructure.md) | ドキュメント再構成 (ユーザー導線) | DOC-01〜DOC-05 | Phase D-1 / D-2 |
| [imp-06](imp-06-large-file-decomposition.md) | 大規模ファイル分割 (Rust 側) | I-01, I-08 (一部) | Phase A-2 / D-4 |

設計 doc を持たない issue (D-05, D-08, I-06, I-07, I-08 の主要部) の扱いは
improvement-roadmap.md のマッピング表を参照。

## 運用規則

- 設計に着手する際は TODO.md (タスク正本) に項目を作成し、本ディレクトリの設計 ID を記載する
- 設計内容が実装で変わった場合は設計 doc を更新し、必要なら ADR (decision-log) に記録する
- 全対象 issue が resolved になった設計 doc は冒頭に完了注記を付けて保持する (削除しない)

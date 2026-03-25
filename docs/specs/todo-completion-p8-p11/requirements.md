# TODO残タスク完了 (P8/P11) - 要件定義書

> 最終更新: 2026-03-25

## 概要

TODO.md に残っていた 27 件のタスクを全て完了させるプロジェクト。
部分実装 6 件はコード実装とテストで、未着手 21 件は仕様固定とドキュメント策定で完了した。

### 背景

- P11-3/4/5 は「仕様固定」パターンで完了済み
- P11-6 系 21 件も同様のアプローチが適用可能
- 部分実装 6 件は selfhost の import/module 解決とパイプライン統合が核心

## 機能要件

### カテゴリ A: コード実装 (6 件)

#### import/module 解決基盤 (P11-2a)

- **FR-001**: Main.ls を分割し、Lexer/Parser/MacroExpand/TypeInfer/Compiler/WasmEmit を import ベースで接続する
- **FR-002**: module graph 解決と複数入力のコンパイル順を topological sort で固定する

#### パイプライン統合 (P11-2)

- **FR-003**: selfhost compiler を完全パイプラインに統合する (Source -> Lexer -> Parser -> MacroExpand -> TypeInfer -> Lower -> WasmEmit)
- **FR-004**: Main.ls の暫定手動統合をやめ、実モジュール構成で selfhost 全モジュールをコンパイル可能にする

#### セルフコンパイル + 固定点 (P8-9)

- **FR-005**: stage1.wasm から stage2.wasm を生成するセルフコンパイルを実現する
- **FR-006**: stage1.wasm == stage2.wasm のバイト列一致による固定点検証を実現する

### カテゴリ B: CI/運用設定 -- 仕様固定 (13 件)

#### P11-6 CI 切替と Rust 撤去 (5 件)

- **FR-007**: CI 主経路を `cargo test` 中心から `stageN.wasm` 中心へ切替える仕様を固定する
- **FR-008**: bootstrap oracle (Rust 実装) を比較専用ジョブに一時隔離する方針を固定する
- **FR-009**: `Cargo.toml` workspace と `crates/` の削除手順と条件を仕様化する
- **FR-010**: native release artifact の生成/署名/配布/回帰テストの CI 組込み仕様を固定する
- **FR-011**: 完了条件 (L# のみでの bootstrap + ネイティブ配布) を文書化する

#### P11-6a CI 再編 (4 件)

- **FR-012**: CI job を bootstrap-wasm/bootstrap-native/golden-parity/release-smoke/packaging/docs に再編する仕様を固定する
- **FR-013**: legacy cargo test を shadow job として維持する方針を固定する
- **FR-014**: branch protection の required status 更新手順を仕様化する
- **FR-015**: CI artifact の保存対象を固定する

#### P11-6b legacy reference 隔離 (4 件)

- **FR-016**: Rust 実装の隔離ディレクトリ/ブランチ方針を固定する
- **FR-017**: L# 実装を正本化し legacy は比較専用とする方針を文書化する
- **FR-018**: 最終削除前の tag 確定ルールを仕様化する
- **FR-019**: feature parity 完了単位での順次削除手順を仕様化する

### カテゴリ C: ドキュメント/手順策定 -- 仕様固定 (8 件)

#### P11-6c リリース運用 (4 件)

- **FR-020**: semver/artifact naming/checksum/changelog/signing の release playbook を策定する
- **FR-021**: nightly/stable 2 チャネルの運用規則を策定する
- **FR-022**: crash report/diagnostic dump の収集方針を策定する
- **FR-023**: リリースごとの CLI/LSP/VSCode extension 互換表生成手順を策定する

#### P11-6d 最終撤去条件 (3 件)

- **FR-024**: bootstrap oracle / legacy reference 依存の完全除去チェックリストを策定する
- **FR-025**: fresh clone から native release 生成までの再現手順を文書化する
- **FR-026**: rollback 手順を文書化し、最後の legacy reference リリースへの復帰を保証する

## 非機能要件

### パフォーマンス

- **NFR-PERF-001**: 既存 E2E テスト (817 件以上) の回帰を発生させない
- **NFR-PERF-002**: Main.ls のファイルサイズは分割後に各ファイル 500-800 行以内に収める

### 保守性

- **NFR-MAINT-001**: P11-3/4/5 と同様の「仕様固定」パターンを P11-6 系に適用する
- **NFR-MAINT-002**: 仕様書は `docs/` 配下に配置し、TODO.md から参照する
- **NFR-MAINT-003**: 各仕様書には P11-6 の各サブタスクとの 1:1 対応を明記する

### 一貫性

- **NFR-CONS-001**: 既存の仕様固定パターン (`docs/native-backend-spec.md`, `docs/runtime-spec.md` 等) と同じフォーマットを踏襲する
- **NFR-CONS-002**: TODO.md の完了記載は P11-2b/2c/2d と同じ「仕様固定 docs/xxx.md」形式とする

### TDD

- **NFR-TDD-001**: カテゴリ A のコード実装はテスト駆動で進める (RED -> GREEN -> REFACTOR)
- **NFR-TDD-002**: テスト 0 個の項目は `[x]` にせず `[~]` で留める

## 制約条件

- **CON-001**: Main.ls は現在 780 行で上限付近。完全版 MacroExpand/TypeInfer のインライン統合は不可 (2000 行超になるため)
- **CON-002**: selfhost ファイルに `(module ...)` / `(import ...)` 宣言は当初ゼロ。import 解決は Rust コンパイラ側の機能に依存
- **CON-003**: P11-6 系は P8-9/P11-2 の実装完了が論理的前提だが、仕様固定は先行可能

## 受入条件

### カテゴリ A (コード実装)

- **AC-001**: selfhost/*.ls ファイルに `(module Name)` 宣言が追加されている
- **AC-002**: Main.ls が `(import ...)` で他モジュールを参照し、インライン定義が除去されている
- **AC-003**: module graph の topological sort が実装されている
- **AC-004**: compile-full-pipeline が完全版 MacroExpand/TypeInfer を使用している
- **AC-005**: stage1.wasm から stage2.wasm の生成が E2E テストで検証されている
- **AC-006**: stage1.wasm == stage2.wasm のバイト列一致が E2E テストで検証されている
- **AC-007**: 全ての新規実装に対応するテストが存在する

### カテゴリ B/C (仕様固定)

- **AC-008**: `docs/ci-migration-spec.md` が作成され、P11-6/6a の全タスクをカバーしている
- **AC-009**: `docs/legacy-isolation-spec.md` が作成され、P11-6b の全タスクをカバーしている
- **AC-010**: `docs/release-operations-spec.md` が作成され、P11-6c の全タスクをカバーしている
- **AC-011**: `docs/final-removal-spec.md` が作成され、P11-6d の全タスクをカバーしている
- **AC-012**: TODO.md の 21 件が `[x]` に更新され、各行に「仕様固定 docs/xxx.md」の参照が記載されている

## 除外事項

- 実際の CI ワークフロー (`.github/workflows/`) の変更はスコープ外 (仕様固定のみ)
- Rust `crates/` の実際の削除はスコープ外 (手順の仕様化のみ)
- native backend の実装はスコープ外 (既に仕様固定済み)

## 関連ドキュメント

- [設計書](./design.md)
- [CI 移行仕様](../../ci-migration-spec.md)
- [legacy 隔離仕様](../../legacy-isolation-spec.md)
- [リリース運用仕様](../../release-operations-spec.md)
- [最終撤去仕様](../../final-removal-spec.md)

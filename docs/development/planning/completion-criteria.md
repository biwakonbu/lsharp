# 完了条件 仕様 (P11-2e)

## 概要
Phase 11-2 (Native backend + bootstrap) の完了を判定するための条件群。
技術完了条件、ドキュメント完了条件、撤去前ゲートの 3 層で構成する。
全条件を満たした場合にのみ Phase 11-2 を完了とし、Rust 実装の段階的撤去に進む。

## 2026-03-25 現況メモ

- `scripts/ci/compile-phase11-inputs.sh` により fixed input set の blocking compile gate は導入済み。
- `cargo run -- compile selfhost/Main.ls` と `cargo run -- compile selfhost/MacroExpand.ls` は成功する。
- `selfhost/Lower.ls` / `LowerPattern.ls` の stage0 stack overflow は `lsharp-types` の `apply_subst` 改修で解消済み（compile gate に含める）。
- **OPS-05 第1段**: `scripts/ci/default-path-smoke.sh` + CI job `default-path-smoke` でビルド済み `lsharp` バイナリ経路を blocking 検証。
- true bootstrap (`stage1.wasm -> stage2.wasm`)、native self-regeneration、Wasm/native 観測差分ゼロ、**Rust workspace 物理撤去**、native-only RC は未完了のため、本書の該当 `pending` を維持する。

## 状態マーカー凡例

各完了条件には以下の 3 状態マーカーを付与する:

- [pending] -- 未着手
- [in-progress] -- 作業中
- [done] -- 完了

---

## P11-2e: 完了条件 (トップレベル方針)

### Rust 依存の境界
- Rust を使うのは stage0 生成だけに限定する
- stage1 以降の生成、検証、ネイティブ成果物生成は L# 単独で閉じる
- Rust crate への依存が stage0 以外に残っている場合は完了としない

### bootstrap + native の両立
- selfhost compiler が自分自身を native binary として再生成できる
- 同じ commit 上で bootstrap 経路 (Wasm) と native 経路の両方が CI を通る
- どちらか一方でも CI fail する場合は完了としない

### ドキュメント整合性
- AOT backend の仕様が README、book、TODO で矛盾なく説明されている
- 矛盾が検出された場合はドキュメント修正を完了条件に含める

---

## P11-2e-1: 技術完了条件

### 条件 1: stage1-native の単独コンパイル能力 [pending]
- stage1-native が以下を Rust compiler の助けなしに単独でコンパイルできること:
  - `selfhost/*.ls` -- selfhost compiler 本体
  - `stdlib/*.ls` -- 標準ライブラリ
  - `examples/fib.ls`, `examples/module.ls`, `examples/trait.ls` -- 代表例
- コンパイル結果が stage1.wasm の出力と観測的に同値であること

### 条件 2: stage1-native の自己再生成 [pending]
- stage1-native が自分自身のソースコード (selfhost/*.ls) から stage2-native を生成できること
- stage2-native が stage1-native と機能的に同値であること (同一入力に対して同一出力)
- 固定点検証: stage2-native で再度コンパイルした stage3-native が stage2-native と同値

### 条件 3: Wasm/native 差分ゼロ [pending]
- stageN.wasm と stageN-native の観測結果差分が allowlist なしでゼロになること
- 観測点は P11-2d-2 で定義した 5 点 (exit code, stdout, stderr, generated file bytes, diagnostics JSON)
- allowlist が残っている場合は、各エントリの解消を完了条件に含める

### 条件 4: 既存 Wasm backend の無回帰 [done]
- AOT backend 導入後も既存 Wasm backend の E2E テストが全件パスすること
- E2E テスト: `crates/lsharp-wasm/tests/e2e.rs` の全テストケース
- 新規テスト追加により E2E テスト数が減少していないこと
- **達成**: E2E 516 passed / 1 ignored（GC soak `#[ignore]`）。テスト数は単調増加を維持。

---

## P11-2e-2: ドキュメント完了条件

### 条件 1: README アーキテクチャ図の更新 [pending]
- README.md のアーキテクチャ図が Wasm 単一 backend 前提から multi-backend 前提へ更新されていること
- 更新内容:
  - コンパイラパイプライン図に native backend の分岐を追加
  - クレート構成表に native backend 関連クレート/モジュールを追加
  - ビルド手順に native backend のビルド方法を追加

### 条件 2: book の selfhosting 章の更新 [pending]
- `book/` の selfhosting 章が以下を反映していること:
  - native backend の設計と実装方針
  - bootstrap 手順 (stage0 -> stage1 -> stage2 -> stage3)
  - fixed-point 検証の方法と意味
  - Wasm backend との関係と使い分け

### 条件 3: CI/配布/署名/クロスビルド手順の一本化 [pending]
- 以下の手順が docs/ 配下に一本化されていること:
  - CI パイプライン構成と各 job の役割
  - リリースビルドの配布手順 (Wasm + native)
  - コード署名の手順 (macOS notarization, Windows signing)
  - クロスビルドの手順 (tier1/tier2 プラットフォーム向け)
- 手順間で矛盾や重複がないこと

---

## P11-2e-3: 撤去前ゲート

Rust 実装の撤去 (Phase 11-3 以降) に進む前に、以下のゲートを全て通過する必要がある。

### ゲート 1: Rust 無効化安定期間 [pending]
- Rust 実装を無効化 (feature flag or conditional compilation) した状態で mainline CI を実行する
- 2 週間以上連続で CI が安定すること (flaky test による単発失敗は除外)
- 安定期間中に発見された不具合は修正し、安定期間をリセットする
- 安定期間のカウント開始日と経過を CHANGELOG または ADR に記録する

### ゲート 2: native 配布物のみでのリリース候補作成 [pending]
- リリース候補 (RC) を少なくとも 1 回、native 配布物だけで作成する
- RC で以下が動作することを検証する:
  - CLI (`lsharp` コマンド) の全サブコマンド: parse, check, compile, test, fmt, doc
  - VSCode 拡張 (LSP 接続、diagnostics、hover、completion)
  - REPL (対話モード)
- RC の検証結果を release notes に記録する

### ゲート 3: rollback 手順の確定 [done]
- Rust ベースの最後の release tag を確定する (例: `v0.x.y-rust-final`)
- 以下を ADR に記録する:
  - 削除対象の Rust コード範囲 (クレート一覧、ファイル一覧)
  - rollback 手順 (tag からの復旧方法)
  - rollback が必要になるシナリオの列挙
  - rollback 後の CI 復旧手順
- ADR のレビューを少なくとも 1 名が完了していること
- **達成**: `docs/development/operations/adr-rust-removal.md`（撤去スコープ・9 段階削除順序・ロールバックシナリオ）、`docs/development/operations/rollback-procedure.md`（復旧手順）、`scripts/rollback.sh`（自動化スクリプト）を作成済み。

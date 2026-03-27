# 完了条件 仕様 (P11-2e)

## 概要
Phase 11-2 (Native backend + bootstrap) の完了を判定するための条件群。
技術完了条件、ドキュメント完了条件、撤去前ゲートの 3 層で構成する。
全条件を満たした場合にのみ Phase 11-2 を完了とし、Rust 実装の段階的撤去に進む。
本書では「証跡が文書化されていること」と「完了条件を閉じたこと」を分けて扱い、proxy/構造テストや補助 smoke test だけでは `done` に上げない。

## 2026-03-25 現況メモ

- `scripts/ci/compile-phase11-inputs.sh` により fixed input set の blocking compile gate は導入済み。
- `cargo run -- compile selfhost/Main.ls` と `cargo run -- compile selfhost/MacroExpand.ls` は成功する。
- `selfhost/Lower.ls` / `LowerPattern.ls` の stage0 stack overflow は `lsharp-types` の `apply_subst` 改修で解消済み（compile gate に含める）。
- **OPS-05 第1段**: `scripts/ci/default-path-smoke.sh` + CI job `default-path-smoke` でビルド済み `lsharp` バイナリ経路を blocking 検証。command surface 上の Rust built-in default / selfhost surface / `LSHARP_PATH` delegation の読み分けは `docs/development/operations/default-path-migration.md` と `docs/development/planning/compatibility-matrix.md` を正本とする。
- **OPS-07 暫定 gate**: `scripts/ci/test-fresh-clone.sh` + CI job `fresh-clone-smoke` で clean checkout 相当コピーからの `lsharp` 再ビルド、default-path smoke 再実行、`selfhost/Token.ls` / `stdlib/Core.ls` の代表 compile までは blocking 化された。**ただし** Rust 不要 `test-fresh-clone` ではない。
- **OPS-06 暫定 gate**: `scripts/release-playbook.sh` は release binary を用いて `compile-phase11-inputs.sh` / `default-path-smoke.sh` を再利用する。**ただし** tag push だけでの release 自動化、署名、checksum / note 生成の完全自動化は未完了。
- **監査整理 / bootstrap**: 現時点で完了証跡として確認できるのは stage0 による selfhost 再コンパイル、stage1 実行、および `test_e2e_bootstrap_stage1_emits_stage2_wasm_for_minimal_subset` による最小 subset `(defn main [] 42)` の `stage1.wasm -> stage2.wasm` 実生成までである。**ただし** full input set に対する `stage1.wasm -> stage2.wasm -> stage3.wasm` の実体生成・比較・固定点成立は BOOT-04 完了証跡として未提示。
- **監査整理 / native**: native 系の既存テストは stage chain の構造確認や 5 観測点比較フレームワークの存在確認として読む。true native self-regeneration と allowlist なし differential zero の完了証跡ではない。
- **監査整理 / runtime**: compile-and-run loop や短時間 REPL soak は runtime stability の補助証跡に留まる。S14/S15/S16 を閉じるには GC 有効の長寿命 stateful LSP/REPL と collector 有効 bootstrap fixed-point の証跡が別途必要。
- したがって true bootstrap、native self-regeneration、Wasm/native 観測差分ゼロ、GC 有効 long-lived runtime gate、**Rust workspace 物理撤去**、native-only RC は未完了のため、本書の該当 `pending` / `in-progress` を維持する。今回の ops 前進は cutover の暫定 gate を増やしたが、完了条件そのものはまだ閉じていない。

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

### runtime stability との接続
- compile-and-run loop、短時間 REPL soak、構造比較のみの differential test は補助証跡として扱い、単独では完了条件を閉じない
- `docs/development/planning/runtime-stability-spec.md` の S14/S15/S16 は、GC 有効の長寿命 stateful LSP/REPL workload と collector 有効 bootstrap fixed-point の双方が揃ったときにのみ満たしたとみなす
- 上記証跡が欠ける間は native-only RC や Rust 撤去判断に進まない

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
- **現況メモ**: 現在の compile gate は stage0 からの再コンパイル成功を示す証跡であり、`stage1-native` 単独で上記入力群を閉じた実行証跡ではない。

### 条件 2: stage1-native の自己再生成 [pending]
- stage1-native が自分自身のソースコード (selfhost/*.ls) から stage2-native を生成できること
- stage2-native が stage1-native と機能的に同値であること (同一入力に対して同一出力)
- 固定点検証: stage2-native で再度コンパイルした stage3-native が stage2-native と同値
- **現況メモ**: 既存の native stage chain テストは structural / observation-framework の確認に留まり、`stage1-native -> stage2-native -> stage3-native` の実体生成と functional fixed-point は未確認。

### 条件 3: Wasm/native 差分ゼロ [pending]
- stageN.wasm と stageN-native の観測結果差分が allowlist なしでゼロになること
- 観測点は P11-2d-2 で定義した 5 点 (exit code, stdout, stderr, generated file bytes, diagnostics JSON)
- allowlist が残っている場合は、各エントリの解消を完了条件に含める
- **現況メモ**: 5 観測点比較のハーネスや構造 parity テストが存在しても、native 実成果物に対する allowlist なし differential zero を継続的に示す証跡が揃うまでは完了扱いにしない。

### 条件 4: 既存 Wasm backend の無回帰 [done]
- AOT backend 導入後も既存 Wasm backend の E2E テストが全件パスすること
- E2E テスト: `crates/lsharp-wasm/tests/e2e.rs` の全テストケース
- 新規テスト追加により E2E テスト数が減少していないこと
- **達成**: E2E harness は `cargo test -p lsharp-wasm --test e2e -- --list` で 683 tests を列挙し、GC soak `#[ignore]` は 2 件（`test_e2e_gc_compile_run_loop_1000`, `test_e2e_gc_repl_soak_500_eval`）。テスト数は単調増加を維持。

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
- **現況メモ**: compile/run smoke や短い REPL 反復だけでは RC 完了証跡にならない。native 配布物のみで長寿命・stateful な LSP/REPL を GC 有効で運用した記録が必要。

### ゲート 3: rollback 手順の確定 [in-progress]
- Rust ベースの最後の release tag を確定する (例: `v0.x.y-rust-final`)
- 以下を ADR に記録する:
  - 削除対象の Rust コード範囲 (クレート一覧、ファイル一覧)
  - rollback 手順 (tag からの復旧方法)
  - rollback が必要になるシナリオの列挙
  - rollback 後の CI 復旧手順
- ADR のレビューを少なくとも 1 名が完了していること
- **現況**: `docs/development/operations/adr-rust-removal.md`（撤去スコープ・9 段階削除順序・ロールバックシナリオ）、`docs/development/operations/rollback-procedure.md`（復旧手順）、`scripts/rollback.sh`（自動化スクリプト）は作成済み。rollback 文書化と自動化は進んでいるが、`adr-rust-removal.md` は提案状態で、`v0.x.y-rust-final` tag の確定と ADR review 完了の証跡が揃うまでは `[done]` に上げない。

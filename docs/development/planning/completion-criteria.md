# 完了条件 仕様 (P11-2e)

## 概要
Phase 11-2 (Wasm bootstrap + toolchain parity) の完了を判定するための条件群。
技術完了条件、ドキュメント完了条件の 2 層で構成する。
全条件を満たした場合にのみ Phase 11-2 を完了とし、Phase 13 (Component Model) に進む。
本書では「証跡が文書化されていること」と「完了条件を閉じたこと」を分けて扱い、proxy/構造テストや補助 smoke test だけでは `done` に上げない。

> **2026-03-30 方針転換**: Wasmtime embedding + Component Model を正式配布モデルに据えることとし、native self-regeneration / Wasm-native differential zero / Rust workspace 物理撤去は Phase 11 の completion gate から外した。native 関連の条件 (旧条件 1-3) は Deferred/v2 (V2-08, V2-09) へ移動。Rust workspace は host launcher として残存する。

## 2026-03-25 現況メモ

- `scripts/ci/compile-phase11-inputs.sh` により fixed input set の blocking compile gate は導入済み。
- `cargo run -- compile selfhost/src/App/Main.ls` と `cargo run -- compile selfhost/src/Syntax/MacroExpand.ls` は成功する。
- `selfhost/src/IR/Lower.ls` / `LowerPattern.ls` の stage0 stack overflow は `lsharp-types` の `apply_subst` 改修で解消済み（compile gate に含める）。
- **OPS-05 第1段**: `scripts/ci/default-path-smoke.sh` + CI job `default-path-smoke` でビルド済み `lsharp` バイナリ経路を blocking 検証。command surface 上の Rust built-in default / selfhost surface / `LSHARP_PATH` delegation の読み分けは `docs/development/operations/default-path-migration.md` と `docs/development/planning/compatibility-matrix.md` を正本とする。
- **OPS-07 暫定 gate**: `scripts/ci/test-fresh-clone.sh` + CI job `fresh-clone-smoke` で clean checkout 相当コピーからの `lsharp` 再ビルド、default-path smoke 再実行、`selfhost/src/Syntax/Token.ls` / `stdlib/Core.ls` の代表 compile までは blocking 化された。**ただし** Rust 不要 `test-fresh-clone` ではない。
- **OPS-06 暫定 gate**: `scripts/release-playbook.sh` は release binary を用いて `compile-phase11-inputs.sh` / `default-path-smoke.sh` を再利用する。**ただし** tag push だけでの release 自動化、署名、checksum / note 生成の完全自動化は未完了。
- **監査整理 / bootstrap**: 現時点で完了証跡として確認できるのは stage0 による selfhost 再コンパイル、stage1 実行、および `test_e2e_bootstrap_stage1_emits_stage2_wasm_for_minimal_subset` による最小 subset `(defn main [] 42)` の `stage1.wasm -> stage2.wasm` 実生成までである。**ただし** full input set に対する `stage1.wasm -> stage2.wasm -> stage3.wasm` の実体生成・比較・固定点成立は BOOT-04 完了証跡として未提示。
- **監査整理 / native**: native 系の既存テストは stage chain の構造確認や 5 観測点比較フレームワークの存在確認として読む。true native self-regeneration と allowlist なし differential zero の完了証跡ではない。
- **監査整理 / runtime**: compile-and-run loop や短時間 REPL soak は runtime stability の補助証跡に留まる。S14/S15/S16 を閉じるには GC 有効の長寿命 stateful LSP/REPL と collector 有効 bootstrap fixed-point の証跡が別途必要。
- **Component Model pivot (2026-03-30)**: native self-regeneration / Wasm-native 観測差分ゼロ / Rust workspace 物理撤去 / native-only RC は completion gate から外した。残る未完了 gate は true bootstrap と GC 有効 long-lived runtime のみ。

## 状態マーカー凡例

各完了条件には以下の 3 状態マーカーを付与する:

- [pending] -- 未着手
- [in-progress] -- 作業中
- [done] -- 完了

---

## P11-2e: 完了条件 (トップレベル方針)

### Rust 依存の境界
- Rust は host launcher (Wasmtime embedding) として残存する
- stage0 生成に加え、Component Model adapter / host capability provision に Rust を使用する
- selfhost compiler は Wasm component として guest 側で動作し、host launcher から独立した semantic を持つ

### bootstrap の完結
- selfhost compiler が自分自身を Wasm として再生成できる (stage1 -> stage2 -> stage3 fixed-point)
- bootstrap 経路 (Wasm) が CI を安定的に通る

### runtime stability との接続
- compile-and-run loop、短時間 REPL soak、構造比較のみの differential test は補助証跡として扱い、単独では完了条件を閉じない
- `docs/development/planning/runtime-stability-spec.md` の S14/S15/S16 は、GC 有効の長寿命 stateful LSP/REPL workload と collector 有効 bootstrap fixed-point の双方が揃ったときにのみ満たしたとみなす
- 上記証跡が欠ける間は Phase 13 (Component Model) への移行判断に進まない

### ドキュメント整合性
- AOT backend の仕様が README、book、TODO で矛盾なく説明されている
- 矛盾が検出された場合はドキュメント修正を完了条件に含める

---

## P11-2e-1: 技術完了条件

### ~~条件 1: stage1-native の単独コンパイル能力~~ [deferred → V2-08]
> Component Model pivot により Deferred/v2 へ移動。native backend は将来の探求用に保持。

### ~~条件 2: stage1-native の自己再生成~~ [deferred → V2-08]
> Component Model pivot により Deferred/v2 へ移動。

### ~~条件 3: Wasm/native 差分ゼロ~~ [deferred → V2-09]
> Component Model pivot により Deferred/v2 へ移動。

### 条件 4: 既存 Wasm backend の無回帰 [done]
- AOT backend 導入後も既存 Wasm backend の E2E テストが全件パスすること
- E2E テスト: `crates/lsharp-wasm/tests/e2e.rs` の全テストケース
- 新規テスト追加により E2E テスト数が減少していないこと
- **達成**: E2E harness は `cargo test -p lsharp-wasm --test e2e -- --list` で 683 tests を列挙し、GC soak `#[ignore]` は 2 件（`test_e2e_gc_compile_run_loop_1000`, `test_e2e_gc_repl_soak_500_eval`）。テスト数は単調増加を維持。

---

## P11-2e-2: ドキュメント完了条件

### 条件 1: README アーキテクチャ図の更新 [pending]
- README.md のアーキテクチャ図が Wasm 単一 backend 前提から host launcher + guest component / dual target 前提へ更新されていること
- 更新内容:
  - コンパイラパイプライン図に `wasi-component` / `web-wasm` の分岐と host launcher の境界を追加
  - クレート構成表に host launcher / component tooling 関連クレート・モジュールを追加
  - ビルド手順に single-binary 配布と component build の流れを追加

### 条件 2: book の selfhosting 章の更新 [pending]
- `book/` の selfhosting 章が以下を反映していること:
  - guest component / host launcher 構成の設計と実装方針
  - bootstrap 手順 (stage0 -> stage1 -> stage2 -> stage3)
  - fixed-point 検証の方法と意味
  - `wasi-component` / `web-wasm` target の関係と使い分け

### 条件 3: CI/配布/署名/クロスビルド手順の一本化 [pending]
- 以下の手順が docs/ 配下に一本化されていること:
  - CI パイプライン構成と各 job の役割
  - リリースビルドの配布手順 (host launcher + embedded guest component / web-wasm)
  - コード署名の手順 (macOS notarization, Windows signing)
  - クロスビルドの手順 (tier1/tier2 プラットフォーム向け)
- 手順間で矛盾や重複がないこと

---

<a id="p11-2e-3-phase13-gate"></a>
## P11-2e-3: Phase 13 移行前ゲート

> **2026-03-30 方針転換**: 旧「撤去前ゲート」を「Phase 13 移行前ゲート」に改称。
> Rust workspace は host launcher として残存するため、Rust 物理撤去 / native-only RC は不要となった。

Phase 13 (Component Model) に進む前に、以下のゲートを全て通過する必要がある。

### ゲート 1: Wasm bootstrap fixed-point [pending]
- `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成でき、fixed-point が CI で安定すること
- full input set (selfhost/stdlib/examples) に対する `stage1 -> stage2 -> stage3` の実体生成・比較

### ゲート 2: GC 有効 runtime stability [pending]
- 長寿命 stateful LSP/REPL workload で GC 有効時にメモリが単調増加しないこと
- `docs/development/planning/runtime-stability-spec.md` の S14/S15/S16 を満たすこと

### ゲート 3: rollback 手順の確定 [in-progress]
- rollback 対象を「embedded compiler component の巻き戻し」として再定義する
- `docs/development/operations/rollback-procedure.md` を Component Model 構成に合わせて更新する
- **現況**: rollback 文書は host launcher + embedded guest component 基準へ更新済み。残る論点は last-known-good release tag / package を release gate にどう固定するかである。

---

## P13: Phase 13 完了条件 (Component Model)

### 条件 1: WASI Preview2 migration [pending]
- dual-mode WASI runner で preview1 / preview2 両方が動作すること
- preview2 codegen path が代表 E2E tests を通すこと

### 条件 2 (P13-B): WIT world definitions [pending]
- `wit/lsharp-compiler.wit` と `wit/lsharp-http-handler.wit` が定義され、Component Model adapter が動作すること
- core Wasm -> component 変換の post-processing が CI で安定すること

### 条件 3 (P13-C): Single binary distribution [pending]
- host launcher に compiler component が埋め込まれ、`lsharp compile` が動作すること
- `--target wasi-component` / `--target web-wasm` の 2 target が使えること

### 条件 4: HTTP handler model [pending]
- `wasi:http/incoming-handler` world の guest 実装が動作すること
- L# program が `(defn handle [request] response)` で HTTP handler を記述できること

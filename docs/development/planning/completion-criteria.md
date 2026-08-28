# 完了条件 仕様 (P11-2e)

## 概要
Phase 11-2 (Wasm bootstrap + toolchain parity) の完了を判定するための条件群。
技術完了条件、ドキュメント完了条件の 2 層で構成する。
全条件を満たした場合にのみ Phase 11-2 を完了とし、Phase 13 (Component Model) に進む。
本書では「証跡が文書化されていること」と「完了条件を閉じたこと」を分けて扱い、proxy/構造テストや補助 smoke test だけでは `done` に上げない。

> **2026-03-30 方針転換**: Wasmtime embedding + Component Model を正式配布モデルに据えることとし、native self-regeneration / Wasm-native differential zero / Rust workspace 物理撤去は Phase 11 の completion gate から外した。native 関連の条件 (旧条件 1-3) は Deferred/v2 (V2-08, V2-09) へ移動。Rust workspace は host launcher として残存する。
>
> **2026-07-25 状態更新**: V2-08〜V2-10 / V2-13〜V2-15 は完了済み。
> native completion evidence は `docs/language/native-backend-spec.md` と release 運用文書へ保持し、
> TODO.md には bootstrap provenance、rooting、public surface など未完 aggregate だけを置く。

## 2026-03-25 現況メモ

- `scripts/ci/compile-phase11-inputs.sh` により fixed input set の blocking compile gate は導入済み。
- `cargo run -- compile selfhost/src/App/Main.ls` と `cargo run -- compile selfhost/src/Syntax/MacroExpand.ls` は成功する。
- `selfhost/src/IR/Lower.ls` / `LowerPattern.ls` の stage0 stack overflow は `lsharp-types` の `apply_subst` 改修で解消済み（compile gate に含める）。
- **OPS-05 第1段**: `scripts/ci/default-path-smoke.sh` + CI job `default-path-smoke` でビルド済み `lsharp` バイナリ経路を blocking 検証。command surface 上の Rust built-in default / selfhost surface / `LSHARP_PATH` delegation の読み分けは `docs/development/operations/default-path-migration.md` と `docs/development/planning/compatibility-matrix.md` を正本とする。
- **OPS-07 現行 gate**: `scripts/ci/test-fresh-clone.sh` は CI 上で 2 系統に分かれる。`test-fresh-clone` は `fresh-clone-artifact` が同一 workflow 内で生成した release-style archive を download し、Rust toolchain 無しで `release-smoke.sh` / `default-path-smoke.sh` / `smoke_test_readme.sh` を通す **closest viable binary-only gate**。`fresh-clone-smoke` は clean checkout 相当コピーからの `lsharp` 再ビルド、default-path smoke 再実行、`selfhost/src/Syntax/Token.ls` / `stdlib/Core.ls` の代表 compile を継続検知する Rust-dependent 補完 gate。加えて手元検証用 scaffold として `scripts/fetch-stage0.sh` / `scripts/bootstrap.sh` / `scripts/release-bundle.sh` を追加し、release asset → stage1/stage2 compare → rebundle を operator path で試せる。**ただし** GitHub Releases 上の stage0 package を required gate から直接取得する true no-Rust end-state は mainline 未接続。
- **マルチファイル型検査 / Formatter (2026-07-25)**: `Tools.Text.FormatterExpr` /
  `FormatterDecl` / `Formatter` は明示 import と一般 SCC 推論を使い、旧 Formatter 専用 batch
  特例は除去済み。Rust host の compile / incremental / source override と Wasm validation は
  verified slice を持つが、canonical runtime、native stage0、両 supported target の parity は未完。
- **OPS-06 暫定 gate**: `scripts/release-playbook.sh` は release binary を用いて `compile-phase11-inputs.sh` / `default-path-smoke.sh` を再利用し、`.github/workflows/release.yml` は build job 内 smoke に加えて downloaded artifact を再検証する `release-smoke` job でも `scripts/ci/release-smoke.sh` を実行する。Supported product/release targets は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) の 2 つであり、macOS x86_64 / Windows / Linux aarch64 は out of support scope として release blocker に含めない。release asset には host launcher archive に加えて companion sidecar `lsharp-{version}-{target}.component.wasm` と release-level `dist/checksums.txt` も添付される。Mac Apple Silicon の macOS notarization workflow hook は secret-gated で接続済みだが、credential 未設定時は skip する。**ただし** 実署名完了と GitHub Releases 直結の true no-Rust end-state は未完了。
- **監査整理 / bootstrap**: stage0 による selfhost 再コンパイル、`test_e2e_bootstrap_fixed_point_stage2_stage3` による `Main.ls` の true fixed point、`test_e2e_bootstrap_stage2_self_feed_fixed_input_set` による fixed input set 54 件の stage2 self-feed 決定性、さらに `test_e2e_bootstrap_fixed_input_set_stage_chain_match` による同 54 件の `stage1.wasm -> stage2.wasm -> stage3.wasm` 実体生成・比較が提示済みであり、BOOT-04 完了証跡は full input set compare まで到達した。
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

### 条件 1: README アーキテクチャ図の更新 [done]
- README.md のアーキテクチャ図が Wasm 単一 backend 前提から host launcher + guest component / dual target 前提へ更新されていること
- 更新内容:
  - コンパイラパイプライン図に `wasi-component` / `web-wasm` の分岐と host launcher の境界を追加
  - クレート構成表に host launcher / component tooling 関連クレート・モジュールを追加
  - ビルド手順に single-binary 配布と component build の流れを追加
  - **達成**: `README.md` の `Architecture` / `Build / Use Paths` / `Current Status` を host launcher + embedded guest component / `wasi-component` + `web-wasm` 前提へ更新済み

### 条件 2: book の selfhosting 章の更新 [done]
- `book/` の selfhosting 章が以下を反映していること:
  - guest component / host launcher 構成の設計と実装方針
  - bootstrap 手順 (stage0 -> stage1 -> stage2 -> stage3)
  - fixed-point 検証の方法と意味
  - `wasi-component` / `web-wasm` target の関係と使い分け
  - **達成**: `book/ch15-selfhosting.md` に host launcher / guest component の役割分担、bootstrap fixed-point の意味、`wasi-component` / `web-wasm` の使い分けを反映済み

### 条件 3: CI/配布/署名/クロスビルド手順の一本化 [done]
- 以下の手順が docs/ 配下に一本化されていること:
  - CI パイプライン構成と各 job の役割
  - リリースビルドの配布手順 (host launcher + embedded guest component / web-wasm)
  - コード署名の手順 (Mac Apple Silicon notarization、Windows signing は archived / out of support scope)
  - クロスビルドの手順 (supported product/release targets 向け)
- 手順間で矛盾や重複がないこと
- **達成**: CI の正本は `docs/development/operations/ci-gate-v2-job-graph.md` / `CI.md`、配布・署名・cross-build の正本は `docs/development/operations/release-distribution-signing.md`、手元実行手順は `docs/development/operations/release-playbook.md` に集約済み。branch protection は `docs/development/operations/branch-protection-checklist.md` を参照する

---

<a id="p11-2e-3-phase13-gate"></a>
## P11-2e-3: Phase 13 移行前ゲート

> **2026-03-30 方針転換**: 旧「撤去前ゲート」を「Phase 13 移行前ゲート」に改称。
> Rust workspace は host launcher として残存するため、Rust 物理撤去 / native-only RC は不要となった。

Phase 13 (Component Model) に進む前に、以下のゲートを全て通過する必要がある。

### ゲート 1: Wasm bootstrap fixed-point [done]
- `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成でき、fixed-point が CI で安定すること
- full input set (selfhost/stdlib/examples) に対する `stage1 -> stage2 -> stage3` の実体生成・比較
- **現況**: `scripts/ci/compile-phase11-inputs.sh` は selfhost/stdlib/examples の fixed input set compile gate を通し、`RUN_BOOTSTRAP_FIXED_POINT=1` では `test_e2e_bootstrap_fixed_point_stage2_stage3`・`test_e2e_bootstrap_stage2_self_feed_fixed_input_set`・`test_e2e_bootstrap_fixed_input_set_stage_chain_match` を exact 実行する。`test_e2e_bootstrap_cli_fixed_input_compile_gate` で `App/Cli.ls` direct compile gate も固定され、historical compiler-mode memory.grow blocker は再現しない。ローカル再実行では script 全体が exit 0 となり、self-feed gate は fixed input set 54 件（selfhost 40 / stdlib 11 / examples 3）の stage2 self-feed 決定性を、stage-chain gate は同じ 54 件の `stage1 -> stage2` 出力と `stage2 -> stage3` 出力の bit-identical compare を `ci-artifacts/bootstrap-diff/{sha}/fixed-input-set-self-feed-report.txt` / `fixed-input-set-self-feed.json` / `fixed-input-set-stage-chain-report.txt` / `fixed-input-set-stage-chain.json` に保存する
- **達成**: full input set に対する `stage1 -> stage2 -> stage3` の実体生成・比較が CI 経路と artifact 付きで固定され、Gate 1 を close した

### ゲート 2: GC 有効 runtime stability [done]
- 長寿命 stateful LSP/REPL workload で GC 有効時にメモリが単調増加しないこと
- `docs/development/planning/runtime-stability-spec.md` の S14/S15/S16 を満たすこと
- **現況**: `test_e2e_alloc_metrics_ci_artifact_payload` と `bash scripts/ci/collect-gc-metrics.sh` は collector-backed `ci-artifacts/gc-metrics/{sha}/summary.json` / `collector-proof.json` を生成し、`allocator_mode = "collector"`、`s14_status = "pass"`、`s15_status = "pass"`、`s16_status = "pass"`、actual `s15_proof` / `s16_proof`、実 `heap_bytes_series`、`gc_collection_count` / `gc_freed_count` を required job `gc-metrics-artifact` から保存できる。`docs/development/planning/runtime-stability-spec.md` では G1 を documented limitation、G3 の 4 slice 棚卸しを GREEN として整理済みで、representative stateful REPL / actual `lsp --stdio` workload の session-internal collector telemetry も固定済み。
- **達成**: required CI artifact が S14/S15/S16 の machine-readable 証跡を直接保持し、GC 有効 runtime stability gate を close した

### ゲート 3: rollback 手順の確定 [done]
- rollback 対象を「embedded compiler component の巻き戻し」として再定義する
- `docs/development/operations/rollback-procedure.md` を Component Model 構成に合わせて更新する
- **現況**: rollback 文書・release playbook・配布運用 docs で、GitHub Release notes の `Rollback anchor` を last-known-good release tag / host launcher asset / guest component sidecar asset (`lsharp-{version}-{target}.component.wasm`) / checksum の正本として固定済み。

---

## P13: Phase 13 完了条件 (Component Model)

### 条件 1: WASI Preview2 migration [done]
- dual-mode WASI runner で preview1 / preview2 両方が動作すること
- preview2 codegen path が代表 E2E tests を通すこと
- **現況**: `crates/lsharp-wasm/src/wasi_runner.rs` は `WasiMode` ベースの `run_wasm_with_mode` / `run_wasm_with_mode_capture` / `run_wasm_with_mode_and_args_inherit_stdin_capture` を持ち、preview1 core Wasm と preview2 component を同じ dispatcher で起動できる。`test_run_wasm_with_mode_dispatches_preview1_core_module` / `test_run_wasm_with_mode_dispatches_preview2_component` / `test_run_wasm_with_mode_capture_preserves_preview2_stdin_and_stdout` が runner dispatch を固定し、`crates/lsharp-driver/src/main.rs` の `LSHARP_PATH=.wasm` / `.component.wasm` 委譲もこの共通 helper を通る。
- **達成**: preview2 codegen path も `test_emit_wasm_wasi_p2_runs_print_via_component_runner` / `test_emit_wasm_wasi_p2_supports_stdin_and_args` / `test_emit_wasm_wasi_p2_supports_file_roundtrip` に加え、public compile/build surface では `test_compile_without_lsharp_path_uses_embedded_component_default_path` / `test_build_without_lsharp_path_uses_embedded_component_default_path` / `test_compile_with_preview1_target_without_lsharp_path_writes_runnable_wasm_artifact` が Condition 1 の代表 E2E である。
- **訂正 (2026-08-28)**: この行はかつて actual selfhost CLI の
  `..._compile_target_and_output_path` / `..._compile_target_changes_wasm_size` も
  「通っており」と根拠に挙げていたが、**両者は 2026-07-12 (`2ba93d0a`) 以降ずっと赤**で、
  `ignored-lane-expected-failures.txt` にも載っていた。さらに guest は component target を
  capability boundary で拒否するので (`I-15` / `I-94`)、**そもそも preview2 codegen の
  根拠にはならない。** component 側の根拠は上記 host launcher 3 件が持つ。
  2 件は `I-94` の裁定で境界を期待する形へ書き換え、後者は
  `..._compile_target_preview1_ok_component_refused` へ改名した。
  同種の「赤い test を達成根拠にしている」記述は `I-104` が持つ

### 条件 2 (P13-B): WIT world definitions [done]
- `wit/lsharp-compiler.wit` と `wit/lsharp-http-handler.wit` が定義され、Component Model adapter が動作すること
- core Wasm -> component 変換の post-processing が CI で安定すること
- **現況**: `wit/lsharp-compiler.wit` と `wit/lsharp-http-handler.wit` はともに正本として存在し、`test_componentize_core_module_with_preview1_adapter` / `test_embed_component_metadata_for_http_handler_world_resolves_vendored_http_deps` が compiler world / HTTP handler world の metadata 埋め込みを固定している。さらに `test_http_handler_world_instantiates_dummy_component_against_synthetic_host` は staged WIT workspace から生成した dummy component を synthetic host で validate / instantiate できることを確認している。
- **達成**: `test_emit_wasm_wasi_p2_basic_program_compiles` / `test_compile_file_wasi_component_output_validates_as_component` が `lsharp-compiler` world の component 化を、`crates/lsharp-wasm/build.rs` の binding 生成と `test_compile_file_handle_only_emits_http_handler_component_export` が `lsharp-http-handler` world の post-processing を固定しており、Condition 2 を close した

### 条件 3 (P13-C): Single binary distribution [done]
- host launcher に compiler component が埋め込まれ、`lsharp compile` が動作すること
- `--target wasi-component` / `--target web-wasm` の 2 target が使えること
- **現況**: `crates/lsharp-driver/src/main.rs` は `embedded-lsharp.component.wasm` を `include_bytes!` で埋め込み、adjacent sidecar があればそちらを優先する host launcher として動作する。`test_compile_without_lsharp_path_uses_embedded_component_default_path` / `test_build_without_lsharp_path_uses_embedded_component_default_path` は `LSHARP_PATH` なしの `lsharp compile` / `lsharp build` が embedded guest component 経由で動くことを固定している。
- **達成**: `test_compile_with_preview1_target_without_lsharp_path_writes_runnable_wasm_artifact` / `test_compile_with_web_wasm_target_without_lsharp_path_uses_rust_builtin_path` / `test_cli_compile_target_accepts_wasi_component_alias_and_web_wasm` により single-binary launcher と `wasi-component` / `web-wasm` target surface が public path で成立しており、Condition 3 を close した

### 条件 4: HTTP handler model [done]
- `wasi:http/incoming-handler` world の guest 実装が動作すること
- L# program が `(defn handle [request] response)` で HTTP handler を記述できること
- **現況**: `emit_wasm_wasi_p2` は `main` を持たず単一引数の `handle` を持つ module を `emit_wasm_http_handler_p2` へ分岐させ、`test_compile_file_handle_only_emits_http_handler_component_export` は handle-only source が `wasi:http/incoming-handler@0.2.3` export を持つ component になることを確認している。
- **達成**: `test_http_handler_world_calls_lsharp_handle_and_sets_response_outparam` が `(defn handle [request] "ok")` を component 化し、HTTP world へ instantiate したうえで response outparam へ `"ok"` を書くところまで固定しており、Condition 4 を close した

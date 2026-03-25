# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P8, P9-1/2/3/4/6, P10。
> **Phase 11**: ADR-152〜ADR-157 で仕様固定済みだが、実装完了ではない。完了判定は `docs/development/planning/completion-criteria.md`, `docs/development/validation/verification-spec.md`, `docs/development/planning/compatibility-matrix.md` を優先する。
>
> P8-9 (T4-4/T4-5) → ADR-148, P9-6b → ADR-149, P9-6c → ADR-150, P9-6d → ADR-151

---

## Phase 11: Rust 完全撤去

> 2026-03-25 実測注記 (更新):
> - `selfhost/Main.ls` は import-only パイプラインへ寄せ済み (BOOT-01)。マルチファイル compile は `ModuleGraph::topological_sort` をモジュール名・import 名でソートし Wasm 出力の再現性を担保。
> - `Lower.ls` / `LowerPattern.ls` の stage0 stack overflow は `lsharp-types` の `Type::apply_subst` ループ化・サイクル打ち切りで解消。`compile-phase11-inputs.sh` に含める。
> - `test_e2e_bootstrap_stage1_stage2_match` 等は proxy のまま。加え `test_e2e_bootstrap_stage0_oracle_chain_four_way_identity` で Rust oracle 4 連一致を固定。
> - `scripts/ci/compile-phase11-inputs.sh` は known blocker なしで通過。残る大物は true bootstrap / native 実行差分 / toolchain parity / Rust 撤去。

> 目標: L# 製 compiler/toolchain をネイティブ配布の正式実装に昇格し、Rust workspace を段階的に撤去する
> 配布方針: ブートストラップと比較検証では Wasm/WASI を利用してよいが、エンドユーザー向け正式配布物は各プラットフォーム向けネイティブバイナリとする
> 正式完了条件:
> 1. `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成できる -- gate: test_e2e_bootstrap_stage1_deterministic, test_e2e_bootstrap_selfhost_modules_deterministic (E2E 5件)
> 2. `stageN.wasm == stageN+1.wasm` の固定点が CI で安定する -- gate: test_e2e_bootstrap_stage1_section_stability, test_e2e_bootstrap_stage1_symbol_stability (E2E 2件), docs/development/validation/verification-spec.md P11-2d-1
> 3. Rust CLI/LSP/docs 系の公開機能が L# 側で互換提供され、ネイティブ版 toolchain から利用できる -- gate: docs/development/planning/compatibility-matrix.md (CLI 13コマンド/LSP 10メソッド), docs/development/planning/toolchain-parity-spec.md (AC-001~AC-608)
> 4. 長寿命プロセス (LSP/REPL/server mode) で GC 有効時にメモリが単調増加しない -- gate: docs/development/planning/runtime-stability-spec.md S14-S16, docs/development/planning/memory-management-roadmap.md M1-M3
> 5. Rust workspace を削除しても開発・CI・ネイティブ配布が成立する -- gate: docs/development/planning/completion-criteria.md P11-2e-3, scripts/smoke_test_readme.sh
>
> 用語定義:
> - **bootstrap oracle**: Rust 実装を stage0 として使用する参照実装 (比較検証の基準)
> - **legacy reference**: 比較検証用に一時保持する旧 Rust 実装 (撤去対象)
> - **native release**: L# 製ネイティブバイナリの正式配布物 (最終成果物)

> **Phase 11 サブフェーズ** (全て仕様固定済み。実装完了ではない):
> - P11-1 (正本監査+互換マトリクス+差分判定+受入基準) → ADR-152
> - P11-2 (ブートストラップ閉路+Native backend+ランタイム+検証+完了条件) → ADR-153
> - P11-3 (Rust parity: syntax/types/IR/backend/移行順/完了条件) → ADR-154
> - P11-4 (ツールチェイン parity: CLI/LSP/formatter/linter/docs/配布、AC-001~AC-608) → ADR-155
> - P11-5 (ランタイム安定化: GC導入/長寿命ワークロード/観測/完了条件) → ADR-156
> - P11-6 (CI切替+legacy隔離+リリース運用+最終撤去) → ADR-157
>
> 仕様固定先ドキュメント一覧:
> `docs/development/planning/compatibility-matrix.md`, `docs/development/planning/gap-classification.md`, `docs/language/backend-boundary.md`,
> `docs/language/native-backend-spec.md`, `docs/language/runtime-spec.md`, `docs/development/validation/verification-spec.md`,
> `docs/development/planning/completion-criteria.md`, `docs/development/planning/rust-parity-spec.md`, `docs/development/planning/toolchain-parity-spec.md`,
> `docs/development/planning/runtime-stability-spec.md`, `docs/development/operations/ci-migration-spec.md`, `docs/development/operations/legacy-isolation-spec.md`,
> `docs/development/operations/release-operations-spec.md`, `docs/development/operations/final-removal-spec.md`,
> `scripts/audit_docs.sh`, `scripts/smoke_test_readme.sh`

*(ADR-159〜ADR-165 は Phase 11 の実装証跡ではなく、当時点の進捗記録として扱う。実装完了判定は `docs/development/planning/completion-criteria.md` / `docs/development/validation/verification-spec.md` / `docs/development/planning/compatibility-matrix.md` を優先する。)*

### Phase 11 ゴールまでの一本道

> 完了判定は以下の 6 段を依存順に閉じたときだけ行う。
> 前段が閉じていない間は、後段を完了扱いしない。

#### Step 1. `CP-01` Frontend unblock / bootstrap 入力集合を閉じる

- [x] [`BOOT-01 Main.ls import path consolidation`](docs/development/planning/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation) -- Evidence: `selfhost/Main.ls`, `test_e2e_selfhost_main_import_only_pipeline`, `test_e2e_selfhost_pipeline_complete_stages`（マルチファイル `compile_and_run_file`）。
- [x] [`IR-02 Lower split`](docs/development/planning/phase11-implementation-plan.md#ir-02-lower-split) / [`IR-04 Pattern lowering`](docs/development/planning/phase11-implementation-plan.md#ir-04-pattern-lowering) -- Evidence: `crates/lsharp-types/src/types.rs` `apply_subst` + `apply_subst_tests`、`selfhost/Lower*.ls` が `compile-phase11-inputs.sh` で通過。
- [x] [`BOOT-03 stdlib direct compile blockers`](docs/development/planning/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers) の残件 -- `scripts/ci/compile-phase11-inputs.sh` に Lower/LowerPattern を含め `KNOWN_BLOCKERS` 撤去済み。
- [x] Step 1 exit gate -- `scripts/ci/compile-phase11-inputs.sh` known blocker なしで通過。

#### Step 2. `CP-01` true bootstrap fixed point を成立させる

- [~] [`BOOT-04 True stage1-stage2-stage3 bootstrap`](docs/development/planning/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap) -- 真の self-compile 未接続。退行検知として **Rust stage0 oracle の 4 連一致** を追加 (`test_e2e_bootstrap_stage0_oracle_chain_four_way_identity`)。既存 proxy テストは維持。
- [x] [`WASM-03 Deterministic LEB emit`](docs/development/planning/phase11-implementation-plan.md#wasm-03-deterministic-leb-emit) -- マルチファイル決定性 (`ModuleGraph` ソート) + E2E: `test_e2e_wasm03_token_module_compile_deterministic`, 既存 `test_e2e_bootstrap_*deterministic*`。
- [ ] Step 2 exit gate -- `docs/development/validation/verification-spec.md` P11-2d-1 と `docs/development/planning/completion-criteria.md` P11-2e-1 条件 1/2 を満たす（**要: stage1.wasm による stage2 生成**）。

#### Step 3. `CP-03` Native parity を閉じる

- [ ] [`NATIVE-05 Stage1-native self-regeneration`](docs/development/planning/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration) -- `stage1-native -> stage2-native -> stage3-native` を functional equivalence で閉じる。
- [~] [`NATIVE-06 Wasm/native differential`](docs/development/planning/phase11-implementation-plan.md#native-06-wasmnative-differential) -- 同一ソースの **Wasm バイナリ連続一致** を `test_e2e_selfhost_wasm_native_differential` に追加。ネイティブ実行比較は未接続。
- [x] [`META-05 Differential allowlist registry`](docs/development/planning/phase11-implementation-plan.md#meta-05-differential-allowlist-registry) の完了 -- `tests/differential-allowlist.yaml` が `allowlist: []` であることを `test_e2e_meta05_differential_allowlist` で固定。
- [ ] Step 3 exit gate -- `docs/development/planning/completion-criteria.md` P11-2e-1 条件 1/2/3 を全て `[done]` にできる。

#### Step 4. `CP-04` Public toolchain parity を閉じる

- [ ] [`CLI-02 13 command implementations`](docs/development/planning/phase11-implementation-plan.md#cli-02-13-command-implementations) -- `selfhost/Cli.ls` の `run-*` stub を実処理へ置換する。
- [ ] [`LSP-02 10 method parity`](docs/development/planning/phase11-implementation-plan.md#lsp-02-10-method-parity) / [`LSP-03 Diagnostic ordering`](docs/development/planning/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots) -- JSON-RPC の観測可能応答を本実装にする。
- [~] [`FMT-01 Formatter roundtrip`](docs/development/planning/phase11-implementation-plan.md#fmt-01-formatter-roundtrip) / [`DOC-01 Schemas and snapshots`](docs/development/planning/phase11-implementation-plan.md#doc-01-schemas-and-snapshots) -- `format-program` が **空 program の vector-length ベース**で決定的・idempotent な出力に (`selfhost/Formatter.ls` + `test_e2e_selfhost_formatter`)。完全 roundtrip / DOC-01 は未達。
- [ ] Step 4 exit gate -- `docs/development/planning/compatibility-matrix.md` の active row が Rust default path 前提ではなくなり、`docs/development/planning/toolchain-parity-spec.md` AC-001~AC-608 の未達が消える。

#### Step 5. `CP-05` Runtime stability gate を閉じる

- [~] [`GC-05 LSP soak and REPL GC`](docs/development/planning/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc) -- 縮小版 `test_e2e_gc_light_compile_run_loop`（48 回 compile+run）を CI に追加。spec の 1,000 cycle / 500 eval REPL は未達。
- [ ] [`GC-06 Leak detection and metrics`](docs/development/planning/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics) -- heap / RSS / GC pause の観測を CI gate に上げる。
- [ ] Step 5 exit gate -- `docs/development/planning/runtime-stability-spec.md` S14-S16 を満たす。

#### Step 6. `CP-06` Ops cutover / Rust removal を閉じる

- [~] [`OPS-01 CI gate-v2 job graph`](docs/development/planning/phase11-implementation-plan.md#ops-01-ci-gate-v2-job-graph) / [`OPS-02 Artifact policy`](docs/development/planning/phase11-implementation-plan.md#ops-02-artifact-policy) -- `.github/workflows/ci.yml` に `ci-gate-v2`・artifact retention・`shadow-oracle` あり。E2E: `test_e2e_ops01_ci_gate_v2`, `test_e2e_ops02_artifact_policy`。spec 全文一致の再編は未完了。
- [ ] [`OPS-05 Default path migration`](docs/development/planning/phase11-implementation-plan.md#ops-05-default-path-migration) -- default path を Rust から L# / native へ切り替える。
- [ ] [`OPS-06 Release playbook`](docs/development/planning/phase11-implementation-plan.md#ops-06-release-playbook) / [`OPS-07 Fresh clone without Rust`](docs/development/planning/phase11-implementation-plan.md#ops-07-fresh-clone-without-rust) -- native-only RC と Rust 未導入 fresh clone を成立させる。
- [ ] [`OPS-08 Final removal and rollback`](docs/development/planning/phase11-implementation-plan.md#ops-08-final-removal-and-rollback) -- rollback ADR と最終撤去手順を確定する。
- [ ] Step 6 exit gate -- `docs/development/planning/completion-criteria.md` P11-2e-2 / P11-2e-3 を全て `[done]` にし、Rust workspace 依存を mainline から外す。

### Phase 11 クリティカルパス現況

- [~] `CP-01 Frontend/bootstrap` -- Step 1 完了 + WASM-03 / oracle 4 連一致テスト。真 bootstrap は未接続。Evidence: `crates/lsharp-wasm/tests/e2e.rs`（`test_e2e_bootstrap_stage0_oracle_chain_four_way_identity` 等）
- [~] `CP-02 Syntax/types parity` -- syntax/type 系テストは増えているが、完了判定に必要な parity table は未充足。Evidence: `docs/development/planning/compatibility-matrix.md`, `docs/development/planning/completion-criteria.md`
- [~] `CP-03 IR/backend/native` -- Lower/LowerPattern の stage0 compile は通過。native parity / Wasm 実行差分は structure 〜 Wasm 側のみ。Evidence: `selfhost/Lower.ls`, `test_e2e_selfhost_wasm_native_differential`, `tests/differential-allowlist.yaml`
- [~] `CP-04 Public toolchain` -- `selfhost/Cli.ls`, `selfhost/LspServer.ls`, `selfhost/Formatter.ls`, `selfhost/TestRunner.ls` は骨格実装に留まる。Evidence: `selfhost/Cli.ls`, `selfhost/LspServer.ls`, `selfhost/Formatter.ls`, `selfhost/TestRunner.ls`
- [~] `CP-05 Runtime stability` -- `test_e2e_gc_light_compile_run_loop` で短ループ回帰。1,000 cycle / メトリクス CI は未達。Evidence: `crates/lsharp-wasm/tests/e2e.rs`
- [~] `CP-06 CI cutover` -- `scripts/ci/compile-phase11-inputs.sh` と `audit-docs` gate は blocking 化したが、Rust default path / native-only RC / rollback ADR は未達。Evidence: `scripts/ci/compile-phase11-inputs.sh`, `.github/workflows/ci.yml`, `docs/development/planning/completion-criteria.md`

### Phase 11 実装状態

- [x] [META-02 Completion marker sync](docs/development/planning/phase11-implementation-plan.md#meta-02-completion-marker-sync) -- `TODO.md`, `docs/development/planning/compatibility-matrix.md`, `docs/development/planning/completion-criteria.md` を実装実態へ同期。
- [x] [META-03 Audit-docs gate](docs/development/planning/phase11-implementation-plan.md#meta-03-audit-docs-gate) -- `scripts/audit_docs.sh`, `.github/workflows/ci.yml` で Phase 11 完了矛盾とエビデンス欠落を fail-fast 化。
- [x] [BOOT-03 stdlib direct compile blockers](docs/development/planning/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers) -- `scripts/ci/compile-phase11-inputs.sh` を追加し、bootstrap job で selfhost/stdlib/examples の fixed input set を blocking 化。
- [x] [BOOT-01 Main.ls import path consolidation](docs/development/planning/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation) -- Evidence: `selfhost/Main.ls` import-only コメント・パイプライン、`crates/lsharp-wasm/tests/e2e.rs`（`compile_and_run_file` / `selfhost_main_path`）。
- [~] [BOOT-04 True stage1-stage2-stage3 bootstrap](docs/development/planning/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap) -- proxy 維持 + `test_e2e_bootstrap_stage0_oracle_chain_four_way_identity`。
- [ ] [NATIVE-05 Stage1-native self-regeneration](docs/development/planning/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration) -- `test_e2e_selfhost_native_self_regeneration` は structure test に留まる。
- [~] [NATIVE-06 Wasm/native differential](docs/development/planning/phase11-implementation-plan.md#native-06-wasmnative-differential) -- Wasm バイナリ連続一致を `test_e2e_selfhost_wasm_native_differential` に追加。native 実行比較は未。
- [ ] [CLI-02 13 command implementations](docs/development/planning/phase11-implementation-plan.md#cli-02-13-command-implementations) -- `selfhost/Cli.ls` の `run-*` は success code を返すだけ。
- [ ] [LSP-02 10 method parity](docs/development/planning/phase11-implementation-plan.md#lsp-02-10-method-parity) -- `selfhost/LspServer.ls` の主要ハンドラは `0` / 空 vector の骨格のみ。
- [~] [FMT-01 Formatter roundtrip](docs/development/planning/phase11-implementation-plan.md#fmt-01-formatter-roundtrip) -- `format-program` を `vector-length` ベースの決定版に。完全 roundtrip は未。
- [~] [GC-05 LSP soak and REPL GC](docs/development/planning/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc) -- `test_e2e_gc_light_compile_run_loop`（48 回）。1,000 cycle / REPL は未。
- [ ] [OPS-05 Default path migration](docs/development/planning/phase11-implementation-plan.md#ops-05-default-path-migration) -- default path は依然 Rust で、native-only RC / rollback ADR も未達。

### Deferred / v2

> Gate 外タスク。Phase 11 完了判定には含めない。各項目の受入・Evidence は `phase11-implementation-plan.md` の V2-01〜V2-07 節を正とし、着手時に個別ブランチ／PR で切る。

- [ ] [V2-01 LSP incremental sync](docs/development/planning/phase11-implementation-plan.md#v2-01-lsp-incremental-sync)
- [ ] [V2-02 Formatter/linter custom rule API](docs/development/planning/phase11-implementation-plan.md#v2-02-formatterlinter-custom-rule-api)
- [ ] [V2-03 Package manager distribution](docs/development/planning/phase11-implementation-plan.md#v2-03-package-manager-distribution)
- [ ] [V2-04 Linux aarch64 tier2 distribution](docs/development/planning/phase11-implementation-plan.md#v2-04-linux-aarch64-tier2-distribution)
- [ ] [V2-05 Windows Authenticode signing](docs/development/planning/phase11-implementation-plan.md#v2-05-windows-authenticode-signing)
- [ ] [V2-06 Region optimization](docs/development/planning/phase11-implementation-plan.md#v2-06-region-optimization)
- [ ] [V2-07 WasmGC optional backend](docs/development/planning/phase11-implementation-plan.md#v2-07-wasmgc-optional-backend)

---

## 既知の制限事項

### リニアメモリランタイム
> 全項目仕様固定済み → ADR-158
> 詳細: `docs/development/planning/memory-management-roadmap.md` (Phase 0-6) + `docs/development/planning/runtime-stability-spec.md` (P11-5接続)

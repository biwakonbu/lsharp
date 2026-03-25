# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P8, P9-1/2/3/4/6, P10, P11, BUG/IMP/QA/CI は完了。
> 詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-132) および `docs/adr/decisions-002.jsonl` (ADR-133〜ADR-165) を参照
>
> P8-9 (T4-4/T4-5) → ADR-148, P9-6b → ADR-149, P9-6c → ADR-150, P9-6d → ADR-151

---

## Phase 11: Rust 完全撤去

> 2026-03-25 実測注記:
> - `crates/lsharp-wasm/tests/e2e.rs` の selfhost ignored は 0 件。MacroExpand/TypeInfer/pipeline の stale Red Phase テストは通常テスト化済み。
> - `test_e2e_bootstrap_stage1_stage2_match` / `test_e2e_bootstrap_fixed_point_stage2_stage3` は現時点では proxy 検証であり、真の self-compile fixed-point ではない。
> - `cargo run -- compile selfhost/Main.ls` はまだ失敗する。原因は `selfhost/MacroExpand.ls` の構文が Rust 側 parser の受理範囲を超えている点で、bootstrap CI を完全 blocking にする前の残課題。

> 目標: L# 製 compiler/toolchain をネイティブ配布の正式実装に昇格し、Rust workspace を段階的に撤去する
> 配布方針: ブートストラップと比較検証では Wasm/WASI を利用してよいが、エンドユーザー向け正式配布物は各プラットフォーム向けネイティブバイナリとする
> 正式完了条件:
> 1. `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成できる -- gate: test_e2e_bootstrap_stage1_deterministic, test_e2e_bootstrap_selfhost_modules_deterministic (E2E 5件)
> 2. `stageN.wasm == stageN+1.wasm` の固定点が CI で安定する -- gate: test_e2e_bootstrap_stage1_section_stability, test_e2e_bootstrap_stage1_symbol_stability (E2E 2件), docs/verification-spec.md P11-2d-1
> 3. Rust CLI/LSP/docs 系の公開機能が L# 側で互換提供され、ネイティブ版 toolchain から利用できる -- gate: docs/compatibility-matrix.md (CLI 13コマンド/LSP 10メソッド), docs/toolchain-parity-spec.md (AC-001~AC-608)
> 4. 長寿命プロセス (LSP/REPL/server mode) で GC 有効時にメモリが単調増加しない -- gate: docs/runtime-stability-spec.md S14-S16, docs/memory-management-roadmap.md M1-M3
> 5. Rust workspace を削除しても開発・CI・ネイティブ配布が成立する -- gate: docs/completion-criteria.md P11-2e-3, scripts/smoke_test_readme.sh
>
> 用語定義:
> - **bootstrap oracle**: Rust 実装を stage0 として使用する参照実装 (比較検証の基準)
> - **legacy reference**: 比較検証用に一時保持する旧 Rust 実装 (撤去対象)
> - **native release**: L# 製ネイティブバイナリの正式配布物 (最終成果物)

> **完了済みサブフェーズ** (全て仕様固定済み):
> - P11-1 (正本監査+互換マトリクス+差分判定+受入基準) → ADR-152
> - P11-2 (ブートストラップ閉路+Native backend+ランタイム+検証+完了条件) → ADR-153
> - P11-3 (Rust parity: syntax/types/IR/backend/移行順/完了条件) → ADR-154
> - P11-4 (ツールチェイン parity: CLI/LSP/formatter/linter/docs/配布、AC-001~AC-608) → ADR-155
> - P11-5 (ランタイム安定化: GC導入/長寿命ワークロード/観測/完了条件) → ADR-156
> - P11-6 (CI切替+legacy隔離+リリース運用+最終撤去) → ADR-157
>
> 仕様固定先ドキュメント一覧:
> `docs/compatibility-matrix.md`, `docs/gap-classification.md`, `docs/backend-boundary.md`,
> `docs/native-backend-spec.md`, `docs/runtime-spec.md`, `docs/verification-spec.md`,
> `docs/completion-criteria.md`, `docs/rust-parity-spec.md`, `docs/toolchain-parity-spec.md`,
> `docs/runtime-stability-spec.md`, `docs/ci-migration-spec.md`, `docs/legacy-isolation-spec.md`,
> `docs/release-operations-spec.md`, `docs/final-removal-spec.md`,
> `scripts/audit_docs.sh`, `scripts/smoke_test_readme.sh`

*(Phase 11 全66タスク完了。詳細は ADR-159〜ADR-165 および `docs/phase11-implementation-plan.md` を参照)*

> Phase 11 実装完了 ADR:
> - CP-01 Frontend unblock (BOOT-01〜04, 4タスク, E2E 4件) → ADR-159
> - CP-02 Syntax/types parity (SYNTAX-01〜06, TYPE-01〜08, 14タスク, テスト10件) → ADR-160
> - CP-03 IR/backend/native (IR-01〜06, WASM-01〜06, NATIVE-01〜06, 18タスク, テスト14件) → ADR-161
> - CP-04 Public toolchain (CLI/LSP/FMT/LINT/DOC/PKG, 10タスク, テスト12件) → ADR-162
> - CP-05 Runtime stability (GC-01〜06, 6タスク, テスト6件) → ADR-163
> - CP-06 CI cutover (OPS-01〜08, 8タスク, テスト9件) → ADR-164
> - WS-META Evidence/hygiene (META-01〜05, 5タスク) → ADR-165

### Deferred / v2

- [ ] [V2-01 LSP incremental sync](docs/phase11-implementation-plan.md#v2-01-lsp-incremental-sync)
- [ ] [V2-02 Formatter/linter custom rule API](docs/phase11-implementation-plan.md#v2-02-formatterlinter-custom-rule-api)
- [ ] [V2-03 Package manager distribution](docs/phase11-implementation-plan.md#v2-03-package-manager-distribution)
- [ ] [V2-04 Linux aarch64 tier2 distribution](docs/phase11-implementation-plan.md#v2-04-linux-aarch64-tier2-distribution)
- [ ] [V2-05 Windows Authenticode signing](docs/phase11-implementation-plan.md#v2-05-windows-authenticode-signing)
- [ ] [V2-06 Region optimization](docs/phase11-implementation-plan.md#v2-06-region-optimization)
- [ ] [V2-07 WasmGC optional backend](docs/phase11-implementation-plan.md#v2-07-wasmgc-optional-backend)

---

## 既知の制限事項

### リニアメモリランタイム
> 全項目仕様固定済み → ADR-158
> 詳細: `docs/memory-management-roadmap.md` (Phase 0-6) + `docs/runtime-stability-spec.md` (P11-5接続)

# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P8, P9-1/2/3/4/6, P10, BUG/IMP/QA/CI は完了。
> 詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-132) および `docs/adr/decisions-002.jsonl` (ADR-133〜ADR-158) を参照
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

*(Phase 11 の仕様固定は完了。以下は実行順の master checklist。各 task ID の詳細な実装方針・依存・受入条件は `docs/phase11-implementation-plan.md` を正本とする)*

> Phase 11 backlog の運用ルール:
> - `TODO.md` は実行順と workstream の索引だけを持つ
> - `docs/phase11-implementation-plan.md` は task ID ごとの `Goal / Current state / Rust source / L# target / Implementation direction / Dependencies / Acceptance / Evidence` を持つ
> - `META-*` は継続タスクであり、いずれの `CP-*` を閉じる場合も同時更新を必須とする

### Critical Path

- [ ] CP-01 Frontend unblock
  - [ ] [BOOT-01 Main.ls import path consolidation](docs/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation)
  - [ ] [BOOT-02 MacroExpand parser-compat cleanup](docs/phase11-implementation-plan.md#boot-02-macroexpand-parser-compat-cleanup)
  - [ ] [BOOT-03 stdlib direct compile blockers](docs/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers)
  - [ ] [BOOT-04 true stage1-stage2-stage3 bootstrap](docs/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap)

- [ ] CP-02 Syntax / types parity foundation
  - [ ] [SYNTAX-01 Span model](docs/phase11-implementation-plan.md#syntax-01-span-model)
  - [ ] [SYNTAX-02 Full AST coverage](docs/phase11-implementation-plan.md#syntax-02-full-ast-coverage)
  - [ ] [SYNTAX-03 Parser recovery and diagnostics](docs/phase11-implementation-plan.md#syntax-03-parser-recovery-and-diagnostics)
  - [ ] [SYNTAX-04 Hygiene gensym and expansion trace](docs/phase11-implementation-plan.md#syntax-04-hygiene-gensym-and-expansion-trace)
  - [ ] [SYNTAX-05 Derive expansion](docs/phase11-implementation-plan.md#syntax-05-derive-expansion)
  - [ ] [SYNTAX-06 Syntax golden fixtures](docs/phase11-implementation-plan.md#syntax-06-syntax-golden-fixtures)
  - [ ] [TYPE-01 Type API normalization](docs/phase11-implementation-plan.md#type-01-type-api-normalization)
  - [ ] [TYPE-02 Unify generalize instantiate](docs/phase11-implementation-plan.md#type-02-unify-generalize-instantiate)
  - [ ] [TYPE-03 Match inference](docs/phase11-implementation-plan.md#type-03-match-inference)
  - [ ] [TYPE-04 Constraints trait where](docs/phase11-implementation-plan.md#type-04-constraints-trait-where)
  - [ ] [TYPE-05 Metadata check](docs/phase11-implementation-plan.md#type-05-metadata-check)
  - [ ] [TYPE-06 HKT GADT alias record update](docs/phase11-implementation-plan.md#type-06-hkt-gadt-alias-record-update)
  - [ ] [TYPE-07 Type error parity](docs/phase11-implementation-plan.md#type-07-type-error-parity)
  - [ ] [TYPE-08 Deterministic ordering](docs/phase11-implementation-plan.md#type-08-deterministic-ordering)

- [ ] CP-03 IR / backend / native / bootstrap parity
  - [ ] [IR-01 Module graph](docs/phase11-implementation-plan.md#ir-01-module-graph)
  - [ ] [IR-02 Lower split](docs/phase11-implementation-plan.md#ir-02-lower-split)
  - [ ] [IR-03 Closure conversion](docs/phase11-implementation-plan.md#ir-03-closure-conversion)
  - [ ] [IR-04 Pattern lowering](docs/phase11-implementation-plan.md#ir-04-pattern-lowering)
  - [ ] [IR-05 Trait dispatch lowering](docs/phase11-implementation-plan.md#ir-05-trait-dispatch-lowering)
  - [ ] [IR-06 IR snapshot serializer](docs/phase11-implementation-plan.md#ir-06-ir-snapshot-serializer)
  - [ ] [WASM-01 Backend boundary](docs/phase11-implementation-plan.md#wasm-01-backend-boundary)
  - [ ] [WASM-02 Section builders](docs/phase11-implementation-plan.md#wasm-02-section-builders)
  - [ ] [WASM-03 Deterministic LEB emit](docs/phase11-implementation-plan.md#wasm-03-deterministic-leb-emit)
  - [ ] [WASM-04 WASI helpers](docs/phase11-implementation-plan.md#wasm-04-wasi-helpers)
  - [ ] [WASM-05 Test runner](docs/phase11-implementation-plan.md#wasm-05-test-runner)
  - [ ] [WASM-06 Wasm golden](docs/phase11-implementation-plan.md#wasm-06-wasm-golden)
  - [ ] [NATIVE-01 Target descriptors](docs/phase11-implementation-plan.md#native-01-target-descriptors)
  - [ ] [NATIVE-02 Object emitter](docs/phase11-implementation-plan.md#native-02-object-emitter)
  - [ ] [NATIVE-03 Linker response](docs/phase11-implementation-plan.md#native-03-linker-response)
  - [ ] [NATIVE-04 Deterministic codegen](docs/phase11-implementation-plan.md#native-04-deterministic-codegen)
  - [ ] [NATIVE-05 Stage1-native self-regeneration](docs/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration)
  - [ ] [NATIVE-06 Wasm/native differential](docs/phase11-implementation-plan.md#native-06-wasmnative-differential)

- [ ] CP-04 Public toolchain parity
  - [ ] [CLI-01 Command contracts](docs/phase11-implementation-plan.md#cli-01-command-contracts)
  - [ ] [CLI-02 13 command implementations](docs/phase11-implementation-plan.md#cli-02-13-command-implementations)
  - [ ] [LSP-01 Full-sync skeleton](docs/phase11-implementation-plan.md#lsp-01-full-sync-skeleton)
  - [ ] [LSP-02 10 method parity](docs/phase11-implementation-plan.md#lsp-02-10-method-parity)
  - [ ] [LSP-03 Diagnostic ordering and JSON snapshots](docs/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots)
  - [ ] [FMT-01 Formatter roundtrip](docs/phase11-implementation-plan.md#fmt-01-formatter-roundtrip)
  - [ ] [LINT-01 Rule IDs and CLI/LSP parity](docs/phase11-implementation-plan.md#lint-01-rule-ids-and-clilsp-parity)
  - [ ] [DOC-01 Schemas and snapshots](docs/phase11-implementation-plan.md#doc-01-schemas-and-snapshots)
  - [ ] [DOC-02 Trailer and deterministic HTML](docs/phase11-implementation-plan.md#doc-02-trailer-and-deterministic-html)
  - [ ] [PKG-01 Archives checksums and Quick Start](docs/phase11-implementation-plan.md#pkg-01-archives-checksums-and-quick-start)

- [ ] CP-05 Runtime stability and long-lived workloads
  - [ ] [GC-01 M1 object model](docs/phase11-implementation-plan.md#gc-01-m1-object-model)
  - [ ] [GC-02 M2 mark-sweep MVP](docs/phase11-implementation-plan.md#gc-02-m2-mark-sweep-mvp)
  - [ ] [GC-03 M3 generational pass](docs/phase11-implementation-plan.md#gc-03-m3-generational-pass)
  - [ ] [GC-04 Longevity benchmarks](docs/phase11-implementation-plan.md#gc-04-longevity-benchmarks)
  - [ ] [GC-05 LSP soak and REPL GC](docs/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc)
  - [ ] [GC-06 Leak detection and metrics](docs/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics)

- [ ] CP-06 CI cutover and final Rust removal
  - [ ] [OPS-01 CI gate-v2 job graph](docs/phase11-implementation-plan.md#ops-01-ci-gate-v2-job-graph)
  - [ ] [OPS-02 Artifact policy](docs/phase11-implementation-plan.md#ops-02-artifact-policy)
  - [ ] [OPS-03 Shadow/oracle lifecycle](docs/phase11-implementation-plan.md#ops-03-shadoworacle-lifecycle)
  - [ ] [OPS-04 Legacy isolation](docs/phase11-implementation-plan.md#ops-04-legacy-isolation)
  - [ ] [OPS-05 Default path migration](docs/phase11-implementation-plan.md#ops-05-default-path-migration)
  - [ ] [OPS-06 Release playbook](docs/phase11-implementation-plan.md#ops-06-release-playbook)
  - [ ] [OPS-07 Fresh clone without Rust](docs/phase11-implementation-plan.md#ops-07-fresh-clone-without-rust)
  - [ ] [OPS-08 Final removal and rollback](docs/phase11-implementation-plan.md#ops-08-final-removal-and-rollback)

### Workstream Index

- [ ] WS-META Evidence / backlog hygiene
  - [ ] [META-01 Compatibility matrix evidence enrichment](docs/phase11-implementation-plan.md#meta-01-compatibility-matrix-evidence-enrichment)
  - [ ] [META-02 Completion marker sync](docs/phase11-implementation-plan.md#meta-02-completion-marker-sync)
  - [ ] [META-03 Audit-docs gate](docs/phase11-implementation-plan.md#meta-03-audit-docs-gate)
  - [ ] [META-04 Gap backlog classification](docs/phase11-implementation-plan.md#meta-04-gap-backlog-classification)
  - [ ] [META-05 Differential allowlist registry](docs/phase11-implementation-plan.md#meta-05-differential-allowlist-registry)

- [ ] WS-BOOTSTRAP Frontend unblock
  - [ ] [BOOT-01 Main.ls import path consolidation](docs/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation)
  - [ ] [BOOT-02 MacroExpand parser-compat cleanup](docs/phase11-implementation-plan.md#boot-02-macroexpand-parser-compat-cleanup)
  - [ ] [BOOT-03 stdlib direct compile blockers](docs/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers)
  - [ ] [BOOT-04 true stage1-stage2-stage3 bootstrap](docs/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap)

- [ ] WS-SYNTAX Frontend syntax parity
  - [ ] [SYNTAX-01 Span model](docs/phase11-implementation-plan.md#syntax-01-span-model)
  - [ ] [SYNTAX-02 Full AST coverage](docs/phase11-implementation-plan.md#syntax-02-full-ast-coverage)
  - [ ] [SYNTAX-03 Parser recovery and diagnostics](docs/phase11-implementation-plan.md#syntax-03-parser-recovery-and-diagnostics)
  - [ ] [SYNTAX-04 Hygiene gensym and expansion trace](docs/phase11-implementation-plan.md#syntax-04-hygiene-gensym-and-expansion-trace)
  - [ ] [SYNTAX-05 Derive expansion](docs/phase11-implementation-plan.md#syntax-05-derive-expansion)
  - [ ] [SYNTAX-06 Syntax golden fixtures](docs/phase11-implementation-plan.md#syntax-06-syntax-golden-fixtures)

- [ ] WS-TYPES HM / constraints / metadata parity
  - [ ] [TYPE-01 Type API normalization](docs/phase11-implementation-plan.md#type-01-type-api-normalization)
  - [ ] [TYPE-02 Unify generalize instantiate](docs/phase11-implementation-plan.md#type-02-unify-generalize-instantiate)
  - [ ] [TYPE-03 Match inference](docs/phase11-implementation-plan.md#type-03-match-inference)
  - [ ] [TYPE-04 Constraints trait where](docs/phase11-implementation-plan.md#type-04-constraints-trait-where)
  - [ ] [TYPE-05 Metadata check](docs/phase11-implementation-plan.md#type-05-metadata-check)
  - [ ] [TYPE-06 HKT GADT alias record update](docs/phase11-implementation-plan.md#type-06-hkt-gadt-alias-record-update)
  - [ ] [TYPE-07 Type error parity](docs/phase11-implementation-plan.md#type-07-type-error-parity)
  - [ ] [TYPE-08 Deterministic ordering](docs/phase11-implementation-plan.md#type-08-deterministic-ordering)

- [ ] WS-IR-BACKEND Lowering / Wasm parity
  - [ ] [IR-01 Module graph](docs/phase11-implementation-plan.md#ir-01-module-graph)
  - [ ] [IR-02 Lower split](docs/phase11-implementation-plan.md#ir-02-lower-split)
  - [ ] [IR-03 Closure conversion](docs/phase11-implementation-plan.md#ir-03-closure-conversion)
  - [ ] [IR-04 Pattern lowering](docs/phase11-implementation-plan.md#ir-04-pattern-lowering)
  - [ ] [IR-05 Trait dispatch lowering](docs/phase11-implementation-plan.md#ir-05-trait-dispatch-lowering)
  - [ ] [IR-06 IR snapshot serializer](docs/phase11-implementation-plan.md#ir-06-ir-snapshot-serializer)
  - [ ] [WASM-01 Backend boundary](docs/phase11-implementation-plan.md#wasm-01-backend-boundary)
  - [ ] [WASM-02 Section builders](docs/phase11-implementation-plan.md#wasm-02-section-builders)
  - [ ] [WASM-03 Deterministic LEB emit](docs/phase11-implementation-plan.md#wasm-03-deterministic-leb-emit)
  - [ ] [WASM-04 WASI helpers](docs/phase11-implementation-plan.md#wasm-04-wasi-helpers)
  - [ ] [WASM-05 Test runner](docs/phase11-implementation-plan.md#wasm-05-test-runner)
  - [ ] [WASM-06 Wasm golden](docs/phase11-implementation-plan.md#wasm-06-wasm-golden)

- [ ] WS-NATIVE Native backend / bootstrap parity
  - [ ] [NATIVE-01 Target descriptors](docs/phase11-implementation-plan.md#native-01-target-descriptors)
  - [ ] [NATIVE-02 Object emitter](docs/phase11-implementation-plan.md#native-02-object-emitter)
  - [ ] [NATIVE-03 Linker response](docs/phase11-implementation-plan.md#native-03-linker-response)
  - [ ] [NATIVE-04 Deterministic codegen](docs/phase11-implementation-plan.md#native-04-deterministic-codegen)
  - [ ] [NATIVE-05 Stage1-native self-regeneration](docs/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration)
  - [ ] [NATIVE-06 Wasm/native differential](docs/phase11-implementation-plan.md#native-06-wasmnative-differential)

- [ ] WS-TOOLCHAIN CLI / LSP / formatter / linter / docs / packaging
  - [ ] [CLI-01 Command contracts](docs/phase11-implementation-plan.md#cli-01-command-contracts)
  - [ ] [CLI-02 13 command implementations](docs/phase11-implementation-plan.md#cli-02-13-command-implementations)
  - [ ] [LSP-01 Full-sync skeleton](docs/phase11-implementation-plan.md#lsp-01-full-sync-skeleton)
  - [ ] [LSP-02 10 method parity](docs/phase11-implementation-plan.md#lsp-02-10-method-parity)
  - [ ] [LSP-03 Diagnostic ordering and JSON snapshots](docs/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots)
  - [ ] [FMT-01 Formatter roundtrip](docs/phase11-implementation-plan.md#fmt-01-formatter-roundtrip)
  - [ ] [LINT-01 Rule IDs and CLI/LSP parity](docs/phase11-implementation-plan.md#lint-01-rule-ids-and-clilsp-parity)
  - [ ] [DOC-01 Schemas and snapshots](docs/phase11-implementation-plan.md#doc-01-schemas-and-snapshots)
  - [ ] [DOC-02 Trailer and deterministic HTML](docs/phase11-implementation-plan.md#doc-02-trailer-and-deterministic-html)
  - [ ] [PKG-01 Archives checksums and Quick Start](docs/phase11-implementation-plan.md#pkg-01-archives-checksums-and-quick-start)

- [ ] WS-RUNTIME Long-lived runtime stability
  - [ ] [GC-01 M1 object model](docs/phase11-implementation-plan.md#gc-01-m1-object-model)
  - [ ] [GC-02 M2 mark-sweep MVP](docs/phase11-implementation-plan.md#gc-02-m2-mark-sweep-mvp)
  - [ ] [GC-03 M3 generational pass](docs/phase11-implementation-plan.md#gc-03-m3-generational-pass)
  - [ ] [GC-04 Longevity benchmarks](docs/phase11-implementation-plan.md#gc-04-longevity-benchmarks)
  - [ ] [GC-05 LSP soak and REPL GC](docs/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc)
  - [ ] [GC-06 Leak detection and metrics](docs/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics)

- [ ] WS-OPS CI / release / removal
  - [ ] [OPS-01 CI gate-v2 job graph](docs/phase11-implementation-plan.md#ops-01-ci-gate-v2-job-graph)
  - [ ] [OPS-02 Artifact policy](docs/phase11-implementation-plan.md#ops-02-artifact-policy)
  - [ ] [OPS-03 Shadow/oracle lifecycle](docs/phase11-implementation-plan.md#ops-03-shadoworacle-lifecycle)
  - [ ] [OPS-04 Legacy isolation](docs/phase11-implementation-plan.md#ops-04-legacy-isolation)
  - [ ] [OPS-05 Default path migration](docs/phase11-implementation-plan.md#ops-05-default-path-migration)
  - [ ] [OPS-06 Release playbook](docs/phase11-implementation-plan.md#ops-06-release-playbook)
  - [ ] [OPS-07 Fresh clone without Rust](docs/phase11-implementation-plan.md#ops-07-fresh-clone-without-rust)
  - [ ] [OPS-08 Final removal and rollback](docs/phase11-implementation-plan.md#ops-08-final-removal-and-rollback)

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

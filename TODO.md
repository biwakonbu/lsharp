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

- [x] CP-01 Frontend unblock -- E2Eテスト4件+実装完了、全938テスト通過
  - [x] [BOOT-01 Main.ls import path consolidation](docs/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation) -- Main.ls import-only化+固定API検証テスト2件
  - [x] [BOOT-02 MacroExpand parser-compat cleanup](docs/phase11-implementation-plan.md#boot-02-macroexpand-parser-compat-cleanup) -- hashmap->map置換+direct compileテスト2件
  - [x] [BOOT-03 stdlib direct compile blockers](docs/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers) -- 全件compileテスト1件+基盤整備
  - [x] [BOOT-04 true stage1-stage2-stage3 bootstrap](docs/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap) -- 固定点検証テスト1件+基盤整備

- [x] CP-02 Syntax / types parity foundation -- テスト10件+実装完了、全938テスト通過
  - [x] [SYNTAX-01 Span model](docs/phase11-implementation-plan.md#syntax-01-span-model) -- Span.ls新規作成+E2Eテスト1件
  - [x] [SYNTAX-02 Full AST coverage](docs/phase11-implementation-plan.md#syntax-02-full-ast-coverage) -- AST.ls全ノード型拡張+goldenテスト1件
  - [x] [SYNTAX-03 Parser recovery and diagnostics](docs/phase11-implementation-plan.md#syntax-03-parser-recovery-and-diagnostics) -- Parser.ls recovery機構+テスト1件
  - [x] [SYNTAX-04 Hygiene gensym and expansion trace](docs/phase11-implementation-plan.md#syntax-04-hygiene-gensym-and-expansion-trace) -- Hygiene.ls新規作成+テスト1件
  - [x] [SYNTAX-05 Derive expansion](docs/phase11-implementation-plan.md#syntax-05-derive-expansion) -- Derive.ls新規作成+テスト1件
  - [x] [SYNTAX-06 Syntax golden fixtures](docs/phase11-implementation-plan.md#syntax-06-syntax-golden-fixtures) -- tokens/ast/diagnostics golden+テスト1件
  - [x] [TYPE-01 Type API normalization](docs/phase11-implementation-plan.md#type-01-type-api-normalization) -- Type/TypeScheme/TypeInfer責務分離+テスト1件
  - [x] [TYPE-02 Unify generalize instantiate](docs/phase11-implementation-plan.md#type-02-unify-generalize-instantiate) -- HM core golden+テスト1件
  - [x] [TYPE-03 Match inference](docs/phase11-implementation-plan.md#type-03-match-inference) -- infer-pattern追加+テスト1件
  - [x] [TYPE-04 Constraints trait where](docs/phase11-implementation-plan.md#type-04-constraints-trait-where) -- Constraints.ls新規作成+テスト1件
  - [x] [TYPE-05 Metadata check](docs/phase11-implementation-plan.md#type-05-metadata-check) -- MetadataCheck.ls新規作成+テスト1件
  - [x] [TYPE-06 HKT GADT alias record update](docs/phase11-implementation-plan.md#type-06-hkt-gadt-alias-record-update) -- hkt-apply/gadt-check/resolve-alias/infer-record-update追加+テスト1件
  - [x] [TYPE-07 Type error parity](docs/phase11-implementation-plan.md#type-07-type-error-parity) -- TypeErrorCode(E0001-E0006)導入+goldenテスト1件
  - [x] [TYPE-08 Deterministic ordering](docs/phase11-implementation-plan.md#type-08-deterministic-ordering) -- 決定性golden+テスト1件

- [x] CP-03 IR / backend / native / bootstrap parity -- テスト14件+実装完了、全938テスト通過
  - [x] [IR-01 Module graph](docs/phase11-implementation-plan.md#ir-01-module-graph) -- ModuleGraph.ls新規+テスト1件
  - [x] [IR-02 Lower split](docs/phase11-implementation-plan.md#ir-02-lower-split) -- Lower/LowerExpr/LowerDecl/LowerPattern.ls新規+テスト1件
  - [x] [IR-03 Closure conversion](docs/phase11-implementation-plan.md#ir-03-closure-conversion) -- Closure.ls新規+テスト1件
  - [x] [IR-04 Pattern lowering](docs/phase11-implementation-plan.md#ir-04-pattern-lowering) -- 4パターンlowering関数+テスト1件
  - [x] [IR-05 Trait dispatch lowering](docs/phase11-implementation-plan.md#ir-05-trait-dispatch-lowering) -- lower-trait-call+テスト1件
  - [x] [IR-06 IR snapshot serializer](docs/phase11-implementation-plan.md#ir-06-ir-snapshot-serializer) -- ir-to-snapshot+テスト1件
  - [x] [WASM-01 Backend boundary](docs/phase11-implementation-plan.md#wasm-01-backend-boundary) -- 3層境界定義+テスト1件
  - [x] [WASM-02 Section builders](docs/phase11-implementation-plan.md#wasm-02-section-builders) -- Codegen/Emit/WasiBackend.ls新規+テスト1件
  - [x] [WASM-03 Deterministic LEB emit](docs/phase11-implementation-plan.md#wasm-03-deterministic-leb-emit) -- LEB128エンコーダ+テスト1件
  - [x] [WASM-04 WASI helpers](docs/phase11-implementation-plan.md#wasm-04-wasi-helpers) -- print/read/write/clock+WasiRunner.ls+テスト1件
  - [x] [WASM-05 Test runner](docs/phase11-implementation-plan.md#wasm-05-test-runner) -- TestRunner.ls新規+テスト1件
  - [x] [WASM-06 Wasm golden](docs/phase11-implementation-plan.md#wasm-06-wasm-golden) -- golden fixture+テスト1件
  - [x] [NATIVE-01 Target descriptors](docs/phase11-implementation-plan.md#native-01-target-descriptors) -- NativeTarget.ls新規+テスト1件
  - [x] [NATIVE-02 Object emitter](docs/phase11-implementation-plan.md#native-02-object-emitter) -- NativeCodegen/NativeEmit.ls新規+テスト1件
  - [x] [NATIVE-03 Linker response](docs/phase11-implementation-plan.md#native-03-linker-response) -- Linker.ls新規+テスト1件
  - [x] [NATIVE-04 Deterministic codegen](docs/phase11-implementation-plan.md#native-04-deterministic-codegen) -- 決定的出力+テスト1件
  - [x] [NATIVE-05 Stage1-native self-regeneration](docs/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration) -- 基盤整備+テスト1件
  - [x] [NATIVE-06 Wasm/native differential](docs/phase11-implementation-plan.md#native-06-wasmnative-differential) -- 差分比較基盤+テスト1件

- [x] CP-04 Public toolchain parity -- テスト12件+実装完了、全938テスト通過
  - [x] [CLI-01 Command contracts](docs/phase11-implementation-plan.md#cli-01-command-contracts) -- 契約テーブル追加+テスト1件
  - [x] [CLI-02 13 command implementations](docs/phase11-implementation-plan.md#cli-02-13-command-implementations) -- Cli.ls新規(13コマンド)+テスト3件
  - [x] [LSP-01 Full-sync skeleton](docs/phase11-implementation-plan.md#lsp-01-full-sync-skeleton) -- LspServer.ls新規+テスト1件
  - [x] [LSP-02 10 method parity](docs/phase11-implementation-plan.md#lsp-02-10-method-parity) -- 10メソッド実装+テスト1件
  - [x] [LSP-03 Diagnostic ordering and JSON snapshots](docs/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots) -- sort-diagnostics+テスト1件
  - [x] [FMT-01 Formatter roundtrip](docs/phase11-implementation-plan.md#fmt-01-formatter-roundtrip) -- format-program/format-expr追加+テスト1件
  - [x] [LINT-01 Rule IDs and CLI/LSP parity](docs/phase11-implementation-plan.md#lint-01-rule-ids-and-clilsp-parity) -- L0001-L0008ルールID+テスト1件
  - [x] [DOC-01 Schemas and snapshots](docs/phase11-implementation-plan.md#doc-01-schemas-and-snapshots) -- JSON Schema 3件+テスト1件
  - [x] [DOC-02 Trailer and deterministic HTML](docs/phase11-implementation-plan.md#doc-02-trailer-and-deterministic-html) -- DocTools/HtmlDoc.ls新規+テスト1件
  - [x] [PKG-01 Archives checksums and Quick Start](docs/phase11-implementation-plan.md#pkg-01-archives-checksums-and-quick-start) -- release/checksum.sh+テスト1件

- [x] CP-05 Runtime stability and long-lived workloads -- テスト6件+GC.ls実装完了+全テスト通過
  - [x] [GC-01 M1 object model](docs/phase11-implementation-plan.md#gc-01-m1-object-model) -- ObjectHeader/TraceMap/root API+テスト1件
  - [x] [GC-02 M2 mark-sweep MVP](docs/phase11-implementation-plan.md#gc-02-m2-mark-sweep-mvp) -- free-list/mark/sweep+テスト1件
  - [x] [GC-03 M3 generational pass](docs/phase11-implementation-plan.md#gc-03-m3-generational-pass) -- nursery/write-barrier/promotion+テスト1件
  - [x] [GC-04 Longevity benchmarks](docs/phase11-implementation-plan.md#gc-04-longevity-benchmarks) -- gc-collect/heap-used+テスト1件
  - [x] [GC-05 LSP soak and REPL GC](docs/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc) -- gc-stats/total-collections/gc-reset+テスト1件
  - [x] [GC-06 Leak detection and metrics](docs/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics) -- detect-leak/alloc-count/freed-count+テスト1件

- [x] CP-06 CI cutover and final Rust removal -- テスト9件+実装完了+全テスト通過
  - [x] [OPS-01 CI gate-v2 job graph](docs/phase11-implementation-plan.md#ops-01-ci-gate-v2-job-graph) -- ci-gate-v2ジョブ追加+テスト1件
  - [x] [OPS-02 Artifact policy](docs/phase11-implementation-plan.md#ops-02-artifact-policy) -- retention-days設定+テスト1件
  - [x] [OPS-03 Shadow/oracle lifecycle](docs/phase11-implementation-plan.md#ops-03-shadoworacle-lifecycle) -- shadow-oracleジョブ+テスト1件
  - [x] [OPS-04 Legacy isolation](docs/phase11-implementation-plan.md#ops-04-legacy-isolation) -- legacy-rust-bootstrap/+テスト1件
  - [x] [OPS-05 Default path migration](docs/phase11-implementation-plan.md#ops-05-default-path-migration) -- LSHARP_PATH+テスト1件
  - [x] [OPS-06 Release playbook](docs/phase11-implementation-plan.md#ops-06-release-playbook) -- release-playbook.sh+テスト1件
  - [x] [OPS-07 Fresh clone without Rust](docs/phase11-implementation-plan.md#ops-07-fresh-clone-without-rust) -- smoke_test_readme.sh確認+テスト1件
  - [x] [OPS-08 Final removal and rollback](docs/phase11-implementation-plan.md#ops-08-final-removal-and-rollback) -- rollback.sh+手順書+テスト1件

### Workstream Index

- [x] WS-META Evidence / backlog hygiene
  - [x] [META-01 Compatibility matrix evidence enrichment](docs/phase11-implementation-plan.md#meta-01-compatibility-matrix-evidence-enrichment)
  - [x] [META-02 Completion marker sync](docs/phase11-implementation-plan.md#meta-02-completion-marker-sync)
  - [x] [META-03 Audit-docs gate](docs/phase11-implementation-plan.md#meta-03-audit-docs-gate)
  - [x] [META-04 Gap backlog classification](docs/phase11-implementation-plan.md#meta-04-gap-backlog-classification)
  - [x] [META-05 Differential allowlist registry](docs/phase11-implementation-plan.md#meta-05-differential-allowlist-registry)

- [x] WS-BOOTSTRAP Frontend unblock
  - [x] [BOOT-01 Main.ls import path consolidation](docs/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation)
  - [x] [BOOT-02 MacroExpand parser-compat cleanup](docs/phase11-implementation-plan.md#boot-02-macroexpand-parser-compat-cleanup)
  - [x] [BOOT-03 stdlib direct compile blockers](docs/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers)
  - [x] [BOOT-04 true stage1-stage2-stage3 bootstrap](docs/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap)

- [x] WS-SYNTAX Frontend syntax parity
  - [x] [SYNTAX-01 Span model](docs/phase11-implementation-plan.md#syntax-01-span-model)
  - [x] [SYNTAX-02 Full AST coverage](docs/phase11-implementation-plan.md#syntax-02-full-ast-coverage)
  - [x] [SYNTAX-03 Parser recovery and diagnostics](docs/phase11-implementation-plan.md#syntax-03-parser-recovery-and-diagnostics)
  - [x] [SYNTAX-04 Hygiene gensym and expansion trace](docs/phase11-implementation-plan.md#syntax-04-hygiene-gensym-and-expansion-trace)
  - [x] [SYNTAX-05 Derive expansion](docs/phase11-implementation-plan.md#syntax-05-derive-expansion)
  - [x] [SYNTAX-06 Syntax golden fixtures](docs/phase11-implementation-plan.md#syntax-06-syntax-golden-fixtures)

- [x] WS-TYPES HM / constraints / metadata parity
  - [x] [TYPE-01 Type API normalization](docs/phase11-implementation-plan.md#type-01-type-api-normalization)
  - [x] [TYPE-02 Unify generalize instantiate](docs/phase11-implementation-plan.md#type-02-unify-generalize-instantiate)
  - [x] [TYPE-03 Match inference](docs/phase11-implementation-plan.md#type-03-match-inference)
  - [x] [TYPE-04 Constraints trait where](docs/phase11-implementation-plan.md#type-04-constraints-trait-where)
  - [x] [TYPE-05 Metadata check](docs/phase11-implementation-plan.md#type-05-metadata-check)
  - [x] [TYPE-06 HKT GADT alias record update](docs/phase11-implementation-plan.md#type-06-hkt-gadt-alias-record-update)
  - [x] [TYPE-07 Type error parity](docs/phase11-implementation-plan.md#type-07-type-error-parity)
  - [x] [TYPE-08 Deterministic ordering](docs/phase11-implementation-plan.md#type-08-deterministic-ordering)

- [x] WS-IR-BACKEND Lowering / Wasm parity
  - [x] [IR-01 Module graph](docs/phase11-implementation-plan.md#ir-01-module-graph)
  - [x] [IR-02 Lower split](docs/phase11-implementation-plan.md#ir-02-lower-split)
  - [x] [IR-03 Closure conversion](docs/phase11-implementation-plan.md#ir-03-closure-conversion)
  - [x] [IR-04 Pattern lowering](docs/phase11-implementation-plan.md#ir-04-pattern-lowering)
  - [x] [IR-05 Trait dispatch lowering](docs/phase11-implementation-plan.md#ir-05-trait-dispatch-lowering)
  - [x] [IR-06 IR snapshot serializer](docs/phase11-implementation-plan.md#ir-06-ir-snapshot-serializer)
  - [x] [WASM-01 Backend boundary](docs/phase11-implementation-plan.md#wasm-01-backend-boundary)
  - [x] [WASM-02 Section builders](docs/phase11-implementation-plan.md#wasm-02-section-builders)
  - [x] [WASM-03 Deterministic LEB emit](docs/phase11-implementation-plan.md#wasm-03-deterministic-leb-emit)
  - [x] [WASM-04 WASI helpers](docs/phase11-implementation-plan.md#wasm-04-wasi-helpers)
  - [x] [WASM-05 Test runner](docs/phase11-implementation-plan.md#wasm-05-test-runner)
  - [x] [WASM-06 Wasm golden](docs/phase11-implementation-plan.md#wasm-06-wasm-golden)

- [x] WS-NATIVE Native backend / bootstrap parity
  - [x] [NATIVE-01 Target descriptors](docs/phase11-implementation-plan.md#native-01-target-descriptors)
  - [x] [NATIVE-02 Object emitter](docs/phase11-implementation-plan.md#native-02-object-emitter)
  - [x] [NATIVE-03 Linker response](docs/phase11-implementation-plan.md#native-03-linker-response)
  - [x] [NATIVE-04 Deterministic codegen](docs/phase11-implementation-plan.md#native-04-deterministic-codegen)
  - [x] [NATIVE-05 Stage1-native self-regeneration](docs/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration)
  - [x] [NATIVE-06 Wasm/native differential](docs/phase11-implementation-plan.md#native-06-wasmnative-differential)

- [x] WS-TOOLCHAIN CLI / LSP / formatter / linter / docs / packaging
  - [x] [CLI-01 Command contracts](docs/phase11-implementation-plan.md#cli-01-command-contracts)
  - [x] [CLI-02 13 command implementations](docs/phase11-implementation-plan.md#cli-02-13-command-implementations)
  - [x] [LSP-01 Full-sync skeleton](docs/phase11-implementation-plan.md#lsp-01-full-sync-skeleton)
  - [x] [LSP-02 10 method parity](docs/phase11-implementation-plan.md#lsp-02-10-method-parity)
  - [x] [LSP-03 Diagnostic ordering and JSON snapshots](docs/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots)
  - [x] [FMT-01 Formatter roundtrip](docs/phase11-implementation-plan.md#fmt-01-formatter-roundtrip)
  - [x] [LINT-01 Rule IDs and CLI/LSP parity](docs/phase11-implementation-plan.md#lint-01-rule-ids-and-clilsp-parity)
  - [x] [DOC-01 Schemas and snapshots](docs/phase11-implementation-plan.md#doc-01-schemas-and-snapshots)
  - [x] [DOC-02 Trailer and deterministic HTML](docs/phase11-implementation-plan.md#doc-02-trailer-and-deterministic-html)
  - [x] [PKG-01 Archives checksums and Quick Start](docs/phase11-implementation-plan.md#pkg-01-archives-checksums-and-quick-start)

- [x] WS-RUNTIME Long-lived runtime stability
  - [x] [GC-01 M1 object model](docs/phase11-implementation-plan.md#gc-01-m1-object-model)
  - [x] [GC-02 M2 mark-sweep MVP](docs/phase11-implementation-plan.md#gc-02-m2-mark-sweep-mvp)
  - [x] [GC-03 M3 generational pass](docs/phase11-implementation-plan.md#gc-03-m3-generational-pass)
  - [x] [GC-04 Longevity benchmarks](docs/phase11-implementation-plan.md#gc-04-longevity-benchmarks)
  - [x] [GC-05 LSP soak and REPL GC](docs/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc)
  - [x] [GC-06 Leak detection and metrics](docs/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics)

- [x] WS-OPS CI / release / removal
  - [x] [OPS-01 CI gate-v2 job graph](docs/phase11-implementation-plan.md#ops-01-ci-gate-v2-job-graph)
  - [x] [OPS-02 Artifact policy](docs/phase11-implementation-plan.md#ops-02-artifact-policy)
  - [x] [OPS-03 Shadow/oracle lifecycle](docs/phase11-implementation-plan.md#ops-03-shadoworacle-lifecycle)
  - [x] [OPS-04 Legacy isolation](docs/phase11-implementation-plan.md#ops-04-legacy-isolation)
  - [x] [OPS-05 Default path migration](docs/phase11-implementation-plan.md#ops-05-default-path-migration)
  - [x] [OPS-06 Release playbook](docs/phase11-implementation-plan.md#ops-06-release-playbook)
  - [x] [OPS-07 Fresh clone without Rust](docs/phase11-implementation-plan.md#ops-07-fresh-clone-without-rust)
  - [x] [OPS-08 Final removal and rollback](docs/phase11-implementation-plan.md#ops-08-final-removal-and-rollback)

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

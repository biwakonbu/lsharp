# Rust 側ファイル分割の完了記録 (2026-08-22 時点)

- **正本の位置づけ**: 実測値と、完了した分割の一次記録。判断は各 ADR、残作業は
  [imp-06](../planning/improvement-designs/imp-06-large-file-decomposition.md) と
  `TODO.md` の `LEGACY-MAINT-01` が持つ。
- **なぜここへ移したか**: 完了記録が imp-06 の「## ステータス」節へ追記され続け、
  20,594 バイトの列挙になって**残作業が読めなくなっていた** (`ISSUES.md` `DOC-10`)。
  記録そのものは必要なので破棄せず、`.claude/rules/docs-organization.md` の
  「作業報告・進捗ログを設計ドキュメントに置かない」に従ってここへ移した。

## 判断の一次記録は ADR にある

分割 1 件ごとに ADR がある。2026-08-22 時点で **166 本**。

```bash
ls docs/adr/ | grep -c split          # => 166
ls docs/adr/decisions-legacy-*split*.md
```

以下に archive する文章は、その 166 本の要約が imp-06 へ累積したものである。
**個別の分割について調べるときは ADR を読むこと。** ここは「何がいつ完了したか」を
一覧するためだけに残す。

## 現在の超過ファイル (2026-08-22 実測)

取得条件を固定する。比較するときは同じコマンドで測り直すこと。

```bash
find crates -path "*/src/*"   -name "*.rs" | xargs wc -l | grep -v total | awk '$1>800' | sort -rn
find crates -path "*/tests/*" -name "*.rs" | xargs wc -l | grep -v total | awk '$1>800' | sort -rn
```

`src/` は **6 件**。

| ファイル | 行数 |
|---|---|
| `crates/lsharp-driver/src/main.rs` | 3254 |
| `crates/lsharp-driver/src/main_tests.rs` | 3086 |
| `crates/lsharp-driver/src/mcp_tests.rs` | 1949 |
| `crates/lsharp-types/src/infer_tests.rs` | 1384 |
| `crates/lsharp-driver/src/mcp_review_registry_tests.rs` | 1223 |
| `crates/lsharp-types/src/validation.rs` | 825 |

`tests/` は **33 件**。上位のみ挙げる。

| ファイル | 行数 |
|---|---|
| `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` | 62990 |
| `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` | 19412 |
| `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` | 6334 |
| `crates/lsharp-wasm/tests/e2e/strings_patterns_compiler_integration.rs` | 5354 |
| `crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs` | 3061 |
| `crates/lsharp-wasm/tests/e2e/selfhost_typeinfer_basic_errors.rs` | 2843 |
| `crates/lsharp-driver/tests/default_path_delegation.rs` | 2750 |
| `crates/lsharp-wasm/tests/e2e/support.rs` | 1951 |

**重心は `src/` から `tests/` へ移った。** `src/` の 6 件のうち 4 件は `*_tests.rs` で、
production から test を切り出した先が今度は超過している。以後の分割対象は
`crates/**/tests/**` である。件数の正本は `ISSUES.md` の `I-01`。

## archive: imp-06 の旧「検証済み部分実装」「ステータス」節

以下は 2026-08-22 に imp-06 から移した文章をそのまま置いたものである。
**追記しないこと。** 新しい分割の記録は ADR に書き、ここには足さない。

---

## 検証済み部分実装 (2026-07-25)

`module_graph.rs` の inline unit test 43 件を `src/module_graph/` 配下の 4 test module へ移動し、続けて `ModuleSearchPaths`、path resolver、entry graph builder を `src/module_graph/resolve.rs` へ分離した。さらに Rust parser の inline test 61 件を `src/parser/` 配下の 3 test module、constraints の inline test 43 件を `src/constraints/` 配下の 4 test module、macro expand の inline test 35 件を `src/macro_expand/` 配下の 3 test module、regex の inline test 25 件を `src/regex/tests.rs`、lexer の inline test 37 件を `src/lexer/` 配下の 3 test module、metadata_check の inline test 31 件を `src/metadata_check/` 配下の 2 test moduleへ移動し、macro expand production を `error.rs` / `builtins.rs` / `expand.rs`、constraints production を `eval.rs` / `hierarchy.rs` / `conversion.rs` / `runtime.rs`、regex production を `node.rs` / `parser.rs` / `matcher.rs` へ分離した。さらに `lsharp-ir/src/lower/tests.rs` の inline test 143 件を helper/定数だけの親 133 行と、WasmGC/root・core・allocating call・self-TCO・language/trait・record/ADT・module/lambda・closure call・heap/ADT の9 test module（129/692/414/531/531/228/598/290/390 行）へ移動した。加えて `lsharp-syntax/src/hygiene.rs` の inline test 17 件を production parent 297 行と `hygiene/tests.rs` 249 行、`lsharp-types/src/regex/dfa.rs` の inline test 13 件を production parent 594 行と `regex/dfa_tests.rs` 107 行へ分離した。公開 API・production semantics・test body は変更していない。focused lower 143 tests、Self-TCO 8 tests、hygiene 17 tests、DFA 13 tests、large-stack `lsharp-ir` 257 tests、`lsharp-syntax` package 175 tests、`lsharp-types` package 258 tests、該当 crate の clippy/rustfmt、`git diff --check` が passし、default stack の Formatter incremental fixture overflow は imp-04 C-1n の既知境界として分離した。詳細は各 `decisions-legacy-*split.md` ADR に記録する。

`wasi.rs` は parent 4568 行 + `wasi_tests.rs` 647 行へ分離した。production の責務分割と I-01 / I-08 aggregate は未完了である。

`metadata_check.rs` は参照収集・scope・`:doc` identifier・builtin 判定 helper を `metadata_check/references.rs` (255 行) へ
移動し、parent を 601 行へ縮小した。metadata checker の公開 API と production semantics は変更していない。

`host_bridge.rs` は inline test 7 件と synthetic HTTP fixture を `host_bridge/tests/mod.rs`、
`operations.rs`、`synthetic_http_state.rs` へ移動し、parent を 126 行へ縮小した。production の
host capability/linker semantics と `host_bridge::tests` namespace は維持した。full `lsharp-wasm`/
native gate、production の追加責務分割、I-01 / I-08 aggregate は未完了である。

## ステータス

`derive.rs`、`codegen.rs`、`component_adapter.rs`、`wasi_runner.rs`、`wasi.rs`、`host_bridge.rs`、`artifact_cache.rs`、`closure.rs`、`tracker.rs`、`review.rs`、`lockfile.rs`、`resolver.rs`、`knowledge.rs`、`claude_plugin.rs`、`config.rs`、`api_doc.rs`、`util.rs`、`types.rs`、`main.rs`、`infer.rs`、`completion.rs`、`analysis.rs`、`format.rs`、`references.rs`、`rename.rs`、`lsharp-tooling/fmt.rs` の test-only 分離を含む verified partial slice は、公開 API と production semantics を変更せず、各 focused/package gate と docs audit で確認済みである。metadata_check の references helper production split も verified partial slice に含める。derive/codegen/component_adapter/wasi_runner/wasi/host_bridge/artifact_cache/closure/tracker/review/lockfile/resolver/knowledge/claude_plugin/config/api_doc/util/types/main/infer/completion/analysis/format/references/rename/tooling-fmt の production 責務分割、metadata checker の追加責務分割、I-01 / I-08 aggregate は未完了である。

設計 + module graph のテスト/path-resolution 分離、parser・constraints・macro expand・regex・lexer・metadata_check・hygiene・lower のテスト分離、lower/decl Self-TCO production split、doc_site test split、regex DFA/codegen/component_adapter/wasi_runner/main/infer/artifact_cache/closure/tracker/review/lockfile/resolver/knowledge/claude_plugin/config/api_doc/util/types/completion/analysis/format/references/rename/tooling-fmt test split、macro expand・constraints・regex production split、lexer/metadata_check test split verified partial slice (2026-07-25)。lower tests は parent 133 行 + 9 module（最大 692 行）、lower/decl は parent 692 行 + `decl/self_tco.rs` 139 行、doc_site は parent 575 行 + `doc_site/tests.rs` 266 行、hygiene は parent 297 行 + `hygiene/tests.rs` 249 行、regex DFA は parent 594 行 + `regex/dfa_tests.rs` 107 行、codegen は parent 237 行 + `codegen_tests.rs` 279 行、closure は parent 140 行 + `closure_tests.rs` 164 行、tracker は parent 135 行 + `tracker_tests.rs` 140 行、review は parent 296 行 + `review_tests.rs` 97 行 + `review_context_tests.rs` 48 行、lockfile は parent 143 行 + `lockfile_tests.rs` 133 行、resolver は parent 184 行 + `resolver_tests.rs` 49 行、knowledge は parent 120 行 + `knowledge_tests.rs` 87 行、claude_plugin は parent 110 行 + `claude_plugin_tests.rs` 126 行、config は parent 291 行 + `config_tests.rs` 316 行、api_doc は parent 326 行 + `api_doc_tests.rs` 207 行、util は parent 653 行 + `util_tests.rs` 206 行、types は parent 479 行 + `types_tests.rs` 48 行、artifact cache は parent 222 行 + `artifact_cache_tests.rs` 246 行、wasi runner は parent 581 行 + `wasi_runner_tests.rs` 450 行、main は parent 2438 行 + `main_tests.rs` 2271 行、infer は parent 2789 行 + `infer_tests.rs` 1268 行、component adapter は parent 377 行 + `component_adapter_tests.rs` 283 行、completion は parent 97 行 + `completion_tests.rs` 33 行、analysis は parent 80 行 + `analysis_tests.rs` 23 行、format は parent 159 行 + `format_tests.rs` 77 行、references は parent 59 行 + `references_tests.rs` 84 行、rename は parent 32 行 + `rename_tests.rs` 96 行、tooling fmt は parent 76 行 + `fmt_tests.rs` 75 行となった。lower expr/mod の production 分割は未着手である。parser の expr/decl production 分割、lexer/metadata production の責務分割、doc_site/tracker/review/lockfile/resolver/knowledge/claude_plugin/config/api_doc/util/types/component_adapter/artifact_cache/wasi_runner/main/infer/completion/analysis/format/references/rename/tooling fmt production の責務分割、regex parser と matcher algorithm の追加分割、codegen/closure production の追加分割、graph/SCC core の追加分割、`wasi.rs` / `main.rs` / `infer.rs` / `ir/lib.rs` の責務分割、I-01 / I-08 aggregate 完了は未着手または未完了である。着手時は TODO.md に Phase A-2 / D-4 としてファイル単位の項目を作成する。

`regex/matcher.rs` の capture/backreference と capture-aware bounded repeat を `regex/matcher_advanced.rs`（270 行）へ移動し、通常 NFA matcher との内部 seam を `pub(super)` で固定した。parent は 699 行から 443 行へ縮小し、既存 `simple_pattern_match` / `has_advanced_features` の crate-private API、DFA fast path、backreference・lookahead・bounded-repeat の判定契約を維持した。capture seam RED (`E0583`) → GREEN、regex 40 tests、`lsharp-types` package、clippy、対象 files の rustfmt、`git diff --check` を passした。regex parser の追加分割、matcher algorithm の改善、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-regex-matcher-advanced-split.md`。

`crates/lsharp-types/src/canonical_contract_check.rs` の非空性/vacuity 判定を `canonical_contract_check/non_vacuity.rs`、canonical contract の synthetic HM probe と型診断を `canonical_contract_check/types.rs` へ移動し、parent を 794 行から 11 行へ縮小した。crate-private re-export、metadata checker の diagnostics と lexical scope、production semantics は変更していない。empty-program seam RED (`E0583`) → GREEN、metadata contract 30 tests、`lsharp-types` package 214 unit + 117 integration、clippy、workspace check、対象 files の Rust 2024 rustfmt、`git diff --check`、docs audit が passした。canonical checker の追加分割、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-canonical-contract-check-split.md`。

`crates/lsharp-tooling/src/metadata_test_tests.rs` の 36 tests を shared fixture/basic diagnostics、canonical assertion/case、deterministic property profile の三 fragmentへ移動し、parent を 742 行から 24 行へ縮小した。`include!` で既存 `metadata_test::tests` namespace、test body、production API/runtime semanticsを維持した。RED (`E0583`) → GREEN、metadata test focused 36件、`lsharp-tooling` package 134 unit + doc-test 0、clippy、workspace check、対象 files の Rust 2024 rustfmt、`git diff --check`、docs audit が passした。metadata runner production split、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-metadata-test-tests-split.md`。

`crates/lsharp-wasm/src/wasi/compiler_world.rs` の Code Section helper emission を `wasi/compiler_world/code.rs` へ移動し、parent を 761 行から 693 行、child を 166 行とした。private な `WasiCodegenIndices` context を seam に置き、section order、runtime import/ABI、function body、`_start`、optional component runner の semantics は維持した。RED (`E0583`) → GREEN、compiler_world focused 1件、WASI filtered 48 passed / 1 existing root-lifetime failure、`cargo clippy -p lsharp-wasm --lib -- -D warnings`、workspace check、対象 Rust 2024 rustfmt、`git diff --check`、docs audit が passした。all-targets clippy の既存 test warnings、full Rust/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-compiler-world-code-split.md`。

`crates/lsharp-types/src/infer.rs` の `Infer::builtin_env`（472 行）を `infer/builtin_env.rs`（475 行）へ移動し、parent を 2789 行から 2319 行へ縮小した。組み込み演算子、string/ref/vector/map/file/argv/root helper、Functor/Monad kind/trait 登録の scheme と登録順序を維持し、`Infer` の既存内部 API を変更していない。RED (`E0583`) → GREEN、builtin scheme focused 1件、`lsharp-types` package 全テスト、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check` が passした。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-builtin-env-split.md`。

`crates/lsharp-types/src/infer.rs` の `TypeError` / `TypeErrorCode` と stable code/span 実装（153 行）を `infer/error.rs`（155 行）へ移動し、parent を 2319 行から 2168 行へ縮小した。`infer::TypeError` / `infer::TypeErrorCode` の公開 re-export、診断 variant、error code、span、Display、Error trait の semantics を維持した。RED (`E0583`) → GREEN、公開 re-export focused 1件、`lsharp-types` package 216 unit + 117 integration + doc-test 0、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check` が passした。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-error-split.md`。

`crates/lsharp-types/src/infer.rs` の unification / `int_heap_compatible` / occurs-check 付き `bind_var`（108 行）を `infer/unify.rs`（118 行）へ移動し、parent を 2168 行から 2061 行へ縮小した。既存の `Infer` 内部呼び出しと `unify` test seam は `pub(super)` で維持し、関数・型適用・record・Int/heap compatibility、代入合成、`TypeError` variant、`global_subst` 更新の semantics を変更していない。RED (`E0583`) → GREEN、`unify_property_tests` 2件、`lsharp-types` package 217 unit + 117 integration + doc-test 0、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check` が passした。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-unify-split.md`。

`crates/lsharp-types/src/infer.rs` の `Infer::generalize`（14 行）を `infer/generalize.rs`（19 行）へ移動し、parent を 2061 行から 2047 行へ縮小した。`TypeEnv` / `Type` の free variable 集合から environment-bound variable を除外して `TypeScheme` を構築する既存 semantics と、親の内部呼び出しを `pub(super)` seam で維持した。RED (`E0583`) → GREEN、generalize focused 1件、`lsharp-types` package 218 unit + 117 integration + doc-test 0、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check` が passした。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-generalize-split.md`。

`crates/lsharp-types/src/infer.rs` の `infer_expr` と record/pattern helper 群（約 637 行）を `infer/expr.rs`（644 行）へ移動し、parent を 2047 行から 1414 行へ縮小した。`infer_expr` の親呼び出しは `pub(super)`、共有 resolver/diagnostic/instantiate helpers は child seam のため `pub(super)` にした。式/let/lambda/application/match/do/annotation/computation、record literal/access/update、constructor/record pattern、binding/literal typing、unification/generalization/diagnostic semantics は変更していない。RED (`E0583`) → GREEN、expr focused 1件、`lsharp-types` package 219 unit + 117 integration + doc-test 0、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check` が passした。infer の `infer_program`/宣言責務、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-expr-split.md`。

`crates/lsharp-types/src/infer.rs` の `infer_program`、`register_nested_module_types`、`infer_decl_functions`、signature helper/`infer_defn` を `infer/decl.rs`（458 行）へ、ADT/record/type alias/constrained/trait/impl registration を `infer/registration.rs`（489 行）へ移動し、parent を 1414 行から 474 行へ縮小した。`infer_program` の公開 API、宣言順序、nested module の修飾名、2-pass defn inference、constructor/accessor/type scheme、trait/default impl、constraint registration と `pub(super)` internal seams は維持した。RED (`E0583`) → GREEN、declaration focused 1件、registration focused 1件、`lsharp-types` package 221 unit + 117 integration + doc-test 0、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check` が passした。selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-decl-registration-split.md`。

`crates/lsharp-lsp/src/lib.rs` の tower-lsp `params_normalizer`（128 行）を `crates/lsharp-lsp/src/params_normalizer.rs`（134 行）へ移動し、parent を 1397 行から 1270 行へ縮小した。`ParamsNormalizer` の param-less method に対する `null` / 空 params stripping、non-empty params preservation、request id/method forwarding、`Service` の readiness / call semantics は変更していない。RED (`E0583`) → GREEN、focused 1件、`lsharp-lsp` package 62 unit + main 0 + doc-test 0、clippy、workspace check、対象 Rust 2024 rustfmt、`git diff --check`、docs audit が passした。LSP backend handler の追加 production 分割、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-lsp-params-normalizer-split.md`。

`crates/lsharp-lsp/src/lib.rs` の inline `tests` module（788 行）を `crates/lsharp-lsp/src/lib_tests.rs` へ移動し、parent を 1270 行から 504 行へ縮小した。`include!` によって既存 `tests::*` module path、fixture、LSP backend behavior を維持し、incremental sync と formatting capability の protocol contract test を追加した。RED (`E0583`) → GREEN、`lsharp-lsp` package 63 unit + main 0 + doc-test 0、clippy、workspace check、対象 Rust 2024 rustfmt、`git diff --check`、docs audit が passした。LSP production backend の追加分割、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-lsp-inline-tests-split.md`。

`crates/lsharp-wasm/src/wasi_runner.rs` の Preview1 core Wasm 実行経路と Preview2 Component Model 実行経路を、それぞれ `wasi_runner/preview1.rs`（163 行）と `wasi_runner/preview2.rs`（243 行）へ移動し、mode routing/共通 helper と既存 public re-export を parent 196 行へ集約した。WasiMode、public function path、stdin/argv/directory、component run export、error text の semantics は変更していない。RED (`E0583`) → GREEN、`wasi_runner::tests` 26件、WASI lib 110 pass / 1 existing root-lifetime failure、lib clippy、対象 rustfmt、`git diff --check` が passした。all-tests clippy の既存 closure lint と root-lifetime failure、selfhost/native parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-runner-mode-split.md`。

`crates/lsharp-wasm/src/wasmgc.rs` の IR instruction lowering と Component output の linear-memory copy helper（約 207 行）を `wasmgc/instructions.rs`（206 行）へ移動し、parent を 628 行から 429 行へ縮小した。`WasmGcEmitOptions` と `ComponentOutputLocals` を親限定の `pub(super)` seam とし、runtime import index、typed funcref offset、GC struct/array opcode、packed-byte `ArrayGetU`、canonical `lsharp:wasmgc-output/stdout@0.1.0#write` 呼び出しの semantics を維持した。RED (`E0583`) → GREEN、Component output import/memory/export contract test 1件、`wasmgc_probe` 101件、`lsharp-wasm --lib` 110 pass / 1 existing root-lifetime failure、lib clippy、対象 Rust 2024 rustfmt、`git diff --check` が passした。WasmGC の全 language/native/selfhost parity、advanced runtime handoff、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasmgc-instructions-split.md`。

`crates/lsharp-wasm/src/wasmgc_runner_component_output.rs` の Preview2/CLI Component 実行、preopen rights、WASI stdout stream、CLI exit decoder（約 367 行）を `wasmgc_runner_component_preview2.rs`（375 行）へ移動し、parent を 543 行から 189 行へ縮小した。`wasmgc_runner_component_output::*` の public functions/types と `decode_wasmgc_component_run_result` の test-only path を再 export し、Preview2 rights、preopen、WASI stdout check-write/write/flush、CLI `wasi:cli/run` exit mapping の semantics を維持した。RED (`E0583`) → GREEN、Preview2 rights/decoder focused 2件、`wasmgc_probe` 101件、`lsharp-wasm --lib` 111 pass / 1 existing root-lifetime failure、lib clippy、workspace check、対象 Rust 2024 rustfmt、`git diff --check` が passした。WasmGC の全 language/native/selfhost parity、advanced runtime handoff、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasmgc-runner-preview2-split.md`。

`crates/lsharp-wasm/src/wasi/http_handler.rs` の HTTP handler Component core emitter（約 566 行）を `wasi/http_handler_core.rs` へ移動し、parent を 585 行から 23 行へ縮小した。`http_handler::emit_wasm_http_handler_p2` の既存 module path と Component 化 boundary を維持し、core emitter は `pub(super)` seam だけで親へ接続した。RED (`E0583`) → GREEN、HTTP handler Component compatibility test、Preview2 5件、host bridge 7件、lib clippy、workspace check、対象 Rust 2024 rustfmt、`git diff --check` が passした。HTTP/native/selfhost parity、advanced runtime handoff、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-http-handler-core-split.md`。

`crates/lsharp-wasm/src/wasi/gc_collect.rs` の GC mark/sweep collector emitter（約 626 行）を `wasi/gc_collect_core.rs` へ移動し、parent を 629 行から 8 行へ縮小した。`gc_collect::emit_gc_collect_func` の既存 module path と `CollectorGlobals` ABI を `pub(super)` seam で維持し、root seed、fixed-point mark、free-list growth、collector metrics の semantics を変更していない。RED (`E0583`) → GREEN、GC collector focused 2件、`lsharp-wasm --lib` 113 pass / 1 existing root-lifetime failure、lib clippy、workspace check、対象 Rust 2024 rustfmt、`git diff --check` が passした。GC/native/selfhost parity、advanced runtime handoff、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-gc-collect-core-split.md`。

`crates/lsharp-wasm/src/wasi_tests.rs` の Preview1/core inline tests（474 行）を `wasi_tests/core.rs` へ移動し、parent を（shared fixture contract test 追加後の）568 行から 94 行へ縮小した。共有 `compile_wasi` / `run_wasi` helper、`wasi::tests` namespace、既存 test names、Preview1/Preview2 fixture semantics は `include!` seam で維持した。新しい shared fixture contract test を先に追加し、RED (`core.rs` include の E0583) → GREEN、focused fixture 1件、`lsharp-wasm --lib` 114 pass / 1 existing root-lifetime failure、lib clippy、workspace check、対象 Rust 2024 rustfmt、`git diff --check`、docs audit が passした。WASI/native/selfhost parity、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-tests-core-split.md`。

IR compile/incremental orchestration の production seam として、`crates/lsharp-ir/src/lib.rs` の parse/cache helper、multi-file merged/modular pipeline、SCC surface inference、segment patch、source-override analysis、incremental compile を `compile_support.rs`（524 行）、`compile_pipeline.rs`（597 行）、`compile_entrypoints.rs`（122 行）、`compile_incremental.rs`（697 行）へ移動し、`compile.rs` は順序付き include seam、parent は 80 行へ縮小した。`compile_multi_file`、`compile_multi_file_with_cache`、`analyze_*`、`compile_multi_file_incremental` の公開 path、cache key、SCC 順序、IR segment reuse、既存 error semantics は変更していない。RED (`E0583`) → GREEN、compile/cache entrypoint contract 1件、`lsharp-ir` 289 unit tests、clippy、workspace check、対象 rustfmt、`git diff --check` が passした。native/selfhost parity、残る large production files、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-ir-compile-orchestration-split.md`。

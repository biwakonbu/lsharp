# imp-06: 大規模ファイル分割 (Rust 側)

> 対象 issue: [I-01](../../../../ISSUES.md#i-01) (ファイルサイズ規約超過)、[I-08](../../../../ISSUES.md#i-08) (テスト配置の偏り、一部)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase A-2 / D-4
>
> **先行事例**: selfhost 側は ADR-168 (STR-01〜03) で同種の分割を完了している
> (TypeInfer.ls 1093 → 290 行 + Apply/Block/Pattern/Record 切り出し、LspServer.ls 1303 → 80 行、
> Formatter.ls 930 → 84 行、全ファイル 800 行未満)。Rust 側も同じ「責務軸での切り出し +
> 親は再エクスポートと統合のみ」方式を踏襲する。

## 概要

プロジェクト規約 (1 ファイル 500-800 行、CLAUDE.md) を大幅に超えるソースが多数ある
(2026-06-12 実測、詳細は [ISSUES.md I-01](../../../../ISSUES.md#i-01) の表を参照)。
最大の `crates/lsharp-wasm/src/wasi.rs` は 4175 行で規約の 5.2 倍。
インラインテスト (`#[cfg(test)]`) の肥大が行数の 15-20% を占めるファイルもあり、
テスト分離だけでも効果がある。

## 設計

### 1. 分割の一般規則

- 親ファイルは「モジュール宣言 + 再エクスポート + 横断ロジックの統合」のみ残し、800 行未満にする
- 公開 API (pub シグネチャ) は不変。`pub use` で従来パスを維持し、利用側の変更をゼロにする
- インラインテストは `src/<module>/tests.rs` (または `tests/` 配下) へ移し、本体と分離する
- 1 ファイルの分割 = 1 PR 相当の独立変更とし、各分割後に `cargo test` / `cargo clippy` 全件 green を確認
- 分割は「移動のみ」とし、ロジック変更 (リファクタリング) を同一コミットに混ぜない

### 2. 主要ファイルの分割軸

セクション境界は 2026-06-12 に実測済み。分割線はこの境界に沿って引く:

| 対象 | 行数 | 実測セクション境界 | 分割案 |
|------|------|--------------------|--------|
| `crates/lsharp-wasm/src/wasi.rs` | 5208 | :1-4561 WASI production / :4562-5208 inline test 28件を `wasi_tests.rs` (647行) へ分離 | `wasi/layout.rs` (メモリレイアウト定数・ヘッダ)、`wasi/gc_runtime.rs` (root_push/pop/set・mark・sweep・free list の emit)、`wasi/io.rs` (fd_write 系 WASI 連携)、`wasi/emit_p1.rs` / `wasi/emit_p2.rs` (preview1 / component 入口)、production 分割後も `wasi_tests.rs` を維持 |
| `crates/lsharp-driver/src/main.rs` | 4715 | :1-2438 CLI/driver production / test-only helper + inline test 132 件を `main_tests.rs` (2271 行) へ分離 | test-only 分離後に `cli.rs` / `commands/<command>.rs` へ command 単位の production 責務を分割し、`main.rs` はディスパッチのみ |
| `crates/lsharp-types/src/infer.rs` | 4055 | :1-2789 type inference production / test-only 11 module・92 件を `infer_tests.rs` (1268 行) へ分離 | `infer/error.rs` (TypeError)、`infer/unify.rs` (unify + occur check)、`infer/generalize.rs`、`infer/expr.rs` (式推論)、`infer/decl.rs` (`infer_program` と宣言処理) へ production 責務を後続分割 |
| `crates/lsharp-types/src/types.rs` | 527 | :1-478 type schema/substitution/env production / :479-527 inline tests | inline tests を `types_tests.rs` へ移し、type schema/substitution production と regression fixture の ownership を分離 |
| `crates/lsharp-ir/src/lib.rs` | 5462 | :1-3078 production + cfg(test) tracking helper / :3079-5462 tail test 7 module・61件を `lib_tests.rs` (2000行) + `lib_tests/linker.rs` (383行) へ分離 | `ir.rs` (Module / Function / Instruction / IrType 定義)、`linker.rs` (`link_modules`)、`compile.rs` (compile_multi_file 系 + incremental 系、imp-04 と接続)、production 分割後も `lib_tests.rs` を維持 |
| `crates/lsharp-syntax/src/parser.rs` | 2242 | `parser/expr.rs` / `parser/decl.rs` / `parser/tests.rs` |
| `crates/lsharp-syntax/src/hygiene.rs` | 546 | :1-294 Sets of Scopes production API / :295-546 inline tests | inline tests を `hygiene/tests.rs` へ移し、production API と回帰 fixture の ownership を分離 |
| `crates/lsharp-syntax/src/derive.rs` | 507 | :1-333 derive production builders / :334-507 inline tests | inline tests を `derive/tests.rs` へ移し、derive builder と AST fixture の ownership を分離 |
| `crates/lsharp-types/src/constraints.rs` | 1961 | `constraints/def.rs` (定義・登録)、`constraints/eval.rs` (評価)、`constraints/pattern.rs` (簡易正規表現)、`constraints/tests.rs`。まず inline tests を責務別 test files へ分離し、production 分割は後続 |
| `crates/lsharp-types/src/regex/dfa.rs` | 699 | :1-580 NFA/DFA production + cache / :581-699 inline tests | inline tests を `regex/dfa_tests.rs` へ移し、DFA backend の test-only fixture を分離 |
| `crates/lsharp-types/src/metadata_check.rs` | 846 | :1-231 / :481-842 metadata 診断・legacy invariant・test 生成 production、:232-480 参照収集 helper | `metadata_check/references.rs` (参照収集・scope・doc identifier・builtin 判定) へ helper を分離し、parent は統合と metadata checker 本体を担当 |
| `crates/lsharp-wasm/src/codegen.rs` | 520 | :1-234 Wasm codegen production / :235-520 inline tests + Wasmtime stub harness | inline tests を `codegen_tests.rs` へ移し、codegen production と runtime fixture の ownership を分離 |
| `crates/lsharp-wasm/src/wasi_runner.rs` | 1033 | :1-578 WASI/Component runner production / :579-1033 inline tests | inline tests を `wasi_runner_tests.rs` へ移し、runner production と WASI/Component runtime fixture の ownership を分離 |
| `crates/lsharp-wasm/src/host_bridge.rs` | 1032 | :1-125 host capability/linker production / :126-1032 inline tests + synthetic HTTP fixture | inline tests を `host_bridge/tests/` へ移し、host bridge production と HTTP/WIT fixture の ownership を分離 |
| `crates/lsharp-wasm/src/component_adapter.rs` | 657 | :1-373 Component adapter production / :374-657 inline tests | inline tests を `component_adapter_tests.rs` へ移し、Component/WIT runtime fixture と adapter production の ownership を分離 |
| `crates/lsharp-ir/src/closure.rs` | 304 | :1-137 free-variable analysis production / :138-304 inline tests | inline tests を `closure_tests.rs` へ移し、解析 production と AST fixture の ownership を分離 |
| `crates/lsharp-docs/src/tracker.rs` | 280 | :1-131 document tracking production / :133-280 inline tests | inline tests を `tracker_tests.rs` へ移し、tracking production と hash/freshness fixture の ownership を分離 |
| `crates/lsharp-docs/src/review.rs` | 441 | :1-289 review production / :290-441 inline tests | inline tests を `review_tests.rs` / `review_context_tests.rs` へ移し、review production と DocTools fixture の ownership を分離 |
| `crates/lsharp-driver/src/lockfile.rs` | 276 | :1-139 lockfile generation/read-write production / :141-276 inline tests | inline tests を `lockfile_tests.rs` へ移し、lockfile production と dependency fixture の ownership を分離 |
| `crates/lsharp-driver/src/resolver.rs` | 233 | :1-180 semver/cache resolver production / :182-233 inline tests | inline tests を `resolver_tests.rs` へ移し、resolver production と version-selection fixture の ownership を分離 |
| `crates/lsharp-docs/src/knowledge.rs` | 207 | :1-117 knowledge schema/serialization production / :118-207 inline tests | inline tests を `knowledge_tests.rs` へ移し、knowledge production と JSON fixture の ownership を分離 |
| `crates/lsharp-driver/src/claude_plugin.rs` | 215 | :1-107 Claude plugin production / :108-215 inline tests | inline tests を `claude_plugin_tests.rs` へ移し、plugin installation production と CLI fixture の ownership を分離 |
| `crates/lsharp-driver/src/config.rs` | 607 | :1-288 config schema/loader/validation production / :289-607 inline tests | inline tests を `config_tests.rs` へ移し、config production と TOML fixture の ownership を分離 |
| `crates/lsharp-tooling/src/api_doc.rs` | 533 | :1-322 API doc production / :323-533 inline tests | inline tests を `api_doc_tests.rs` へ移し、API document builder production と metadata/file fixture の ownership を分離 |
| `crates/lsharp-tooling/src/artifact_cache.rs` | 468 | :1-219 artifact cache production / :220-468 inline tests | inline tests を `artifact_cache_tests.rs` へ移し、cache production と filesystem regression fixture の ownership を分離 |
| `crates/lsharp-lsp/src/completion.rs` | 130 | :1-94 completion production / :95-130 inline tests | inline tests を `completion_tests.rs` へ移し、completion production と LSP fixture の ownership を分離 |
| `crates/lsharp-lsp/src/analysis.rs` | 103 | :1-77 hover/analysis production / :78-103 inline tests | inline tests を `analysis_tests.rs` へ移し、analysis production と hover fixture の ownership を分離 |
| `crates/lsharp-lsp/src/format.rs` | 236 | :1-156 formatter production / :157-236 inline tests | inline tests を `format_tests.rs` へ移し、format production と formatter fixture の ownership を分離 |
| `crates/lsharp-lsp/src/references.rs` | 143 | :1-56 references production / :57-143 inline tests | inline tests を `references_tests.rs` へ移し、references production と LSP fixture の ownership を分離 |
| `crates/lsharp-lsp/src/rename.rs` | 128 | :1-30 rename production / :31-128 inline tests | inline tests を `rename_tests.rs` へ移し、rename production と LSP fixture の ownership を分離 |
| `crates/lsharp-tooling/src/fmt.rs` | 151 | :1-73 formatter/CLI production / :74-151 inline tests | inline tests を `fmt_tests.rs` へ移し、tooling formatter production と CLI fixture の ownership を分離 |
| `crates/lsharp-ir/src/lower/expr.rs` | 1897 | パターンマッチ lowering / 計算式 lowering を切り出し |
| `crates/lsharp-ir/src/lower/decl.rs` | 823 | :1-690 宣言 lowering / :691 Self-TCO helper | Self-TCO helper を `lower/decl/self_tco.rs` へ移し、親は 800 行未満へする。残る宣言 lowering の責務分割は後続 |
| `crates/lsharp-driver/src/doc_site.rs` | 840 | :1-573 production doc-site generation / :574-840 inline tests | inline tests を `doc_site/tests.rs` へ移し、production parent を 800 行未満へする。サイト生成責務の分割は後続 |
| `crates/lsharp-syntax/src/macro_expand.rs` | 1681 | 展開器本体 / 組み込みマクロ / tests |
| `crates/lsharp-ir/src/module_graph.rs` | 1597 | グラフ構築 / 解決 / (imp-04 の SCC・キャッシュ導入前に分割) |
| `crates/lsharp-lsp/src/lib.rs` | 1397 | ハンドラ単位 (hover / completion / definition...) は既に別ファイルがあるため、残る統合部から診断変換等を切り出し |
| `crates/lsharp-lsp/src/util.rs` | 862 | :1-650 LSP utility production / :651-862 inline tests | inline tests を `util_tests.rs` へ移し、utility production と診断/incremental fixture の ownership を分離 |

`crates/lsharp-ir/src/lower/tests.rs` (3913 行) は production 変更と分離した test-only slice として、
helper/定数だけを親へ残し、WasmGC/root、core、allocating call、self-TCO、language/trait、record/ADT、
module/lambda、closure call、heap/ADT の9 moduleへ分割する (I-08 の切り分け性改善)。

`crates/lsharp-ir/src/lower/decl.rs` (823 行) は Self-TCO helper を `decl/self_tco.rs` (139 行) へ分離し、親を 692 行へ縮小した。

`crates/lsharp-driver/src/doc_site.rs` (840 行) は inline test 8 件を `doc_site/tests.rs` (266 行) へ移動し、production parent を 575 行へ縮小した。`doc_site::tests` focused 8 件、driver unit 132 件、clippy、rustfmt、`git diff --check` が pass した。full driver integration lane の `default_path_delegation` 12 件は embedded component / selfhost artifact の今回の差分外 failure boundary として残る。

`crates/lsharp-syntax/src/hygiene.rs` (546 行) は inline test 17 件を `hygiene/tests.rs` (249 行) へ移動し、production parent を 297 行へ縮小した。`hygiene::tests` focused 17 件、`lsharp-syntax` package 175 件、clippy、rustfmt、`git diff --check` が pass した。

`crates/lsharp-syntax/src/derive.rs` (507 行) は inline test 7 件を `derive/tests.rs` (175 行) へ移動し、production parent を 335 行へ縮小した。`derive::tests` focused 7 件、`lsharp-syntax` package 175 件、clippy、rustfmt、`git diff --check` が pass した。

`crates/lsharp-types/src/regex/dfa.rs` (699 行) は inline test 13 件を `regex/dfa_tests.rs` (107 行) へ移動し、production parent を 594 行へ縮小した。`regex::dfa::tests` focused 13 件、`lsharp-types` package 258 件、clippy、rustfmt、`git diff --check` が pass した。

`crates/lsharp-wasm/src/codegen.rs` (520 行) は inline test 8 件と Wasmtime stub harness を `codegen_tests.rs` (279 行) へ移動し、production parent を 237 行へ縮小した。`codegen::tests` focused 8 件、対象 files の rustfmt、`git diff --check` が pass した。`lsharp-wasm` package の root-lifetime test 1 件と clippy warning は既存 failure boundary として残る。

`crates/lsharp-ir/src/closure.rs` (304 行) は inline test 10 件と Span fixture helper を `closure_tests.rs` (164 行) へ移動し、production parent を 140 行へ縮小した。`closure::tests` focused 10 件、large-stack `lsharp-ir` package 257 件、clippy、rustfmt、`git diff --check` が pass した。

`crates/lsharp-docs/src/tracker.rs` (280 行) は inline test 9 件を `tracker_tests.rs` (140 行) へ移動し、production parent を 135 行へ縮小した。`tracker::tests` focused 9 件、`lsharp-docs` package 23 件、clippy、Rust 2024 rustfmt、`git diff --check` が pass した。

`crates/lsharp-docs/src/review.rs` (441 行) は inline test 11 件を `review_tests.rs` (97 行) / `review_context_tests.rs` (48 行) へ移動し、production parent を 296 行へ縮小した。`review::tests` focused 7 件、`review::context_tests` focused 4 件、`lsharp-docs` package 23 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-driver/src/lockfile.rs` (276 行) は inline test 5 件を `lockfile_tests.rs` (133 行) へ移動し、production parent を 143 行へ縮小した。`lockfile::tests` focused 5 件、driver unit 132 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。driver の `default_path_delegation` 12 件は embedded component / selfhost artifact の今回の差分外 failure boundary として残る。

`crates/lsharp-driver/src/resolver.rs` (233 行) は inline test 4 件を `resolver_tests.rs` (49 行) へ移動し、production parent を 184 行へ縮小した。`resolver::tests` focused 4 件、driver unit 132 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-docs/src/knowledge.rs` (207 行) は inline test 3 件を `knowledge_tests.rs` (87 行) へ移動し、production parent を 120 行へ縮小した。`knowledge::tests` focused 3 件、`lsharp-docs` package 23 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-lsp/src/completion.rs` (130 行) は inline test 2 件を `completion_tests.rs` (33 行) へ移動し、production parent を 97 行へ縮小した。`completion::tests` focused 2 件、`lsharp-lsp` package 61 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-lsp/src/analysis.rs` (103 行) は inline test 1 件を `analysis_tests.rs` (23 行) へ移動し、production parent を 80 行へ縮小した。`analysis::tests` focused 1 件、`lsharp-lsp` package 61 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-lsp/src/format.rs` (236 行) は inline test 7 件を `format_tests.rs` (77 行) へ移動し、production parent を 159 行へ縮小した。`format::tests` focused 7 件、`lsharp-lsp` package 61 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-lsp/src/references.rs` (143 行) は inline test 7 件を `references_tests.rs` (84 行) へ移動し、production parent を 59 行へ縮小した。`references::tests` focused 7 件、`lsharp-lsp` package 61 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-lsp/src/rename.rs` (128 行) は inline test 6 件を `rename_tests.rs` (96 行) へ移動し、production parent を 32 行へ縮小した。`rename::tests` focused 6 件、`lsharp-lsp` package 61 件、doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。

`crates/lsharp-tooling/src/fmt.rs` (151 行) は inline test 6 件を `fmt_tests.rs` (75 行) へ移動し、production parent を 76 行へ縮小した。`fmt::tests` focused 6 件、`lsharp-tooling` doc-tests 0 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。tooling package 全体の metadata property `LS2005` failure 2 件は既知の差分外 failure boundary として残る。

`crates/lsharp-driver/src/claude_plugin.rs` (215 行) は inline test 5 件を `claude_plugin_tests.rs` (126 行) へ移動し、production parent を 110 行へ縮小した。`claude_plugin::tests` focused 5 件、driver unit 132 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit が pass した。`default_path_delegation` の既知 embedded component / selfhost artifact failure boundary 12 件は別問題として残る。

`crates/lsharp-tooling/src/api_doc.rs` (533 行) は inline test 7 件を `api_doc_tests.rs` (207 行) へ移動し、production parent を 326 行へ縮小した。`api_doc::tests` focused 7 件、doc-tests 0/0、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit は pass。tooling package は 130 passed / 2 failed で、既存 metadata property LS2005 vacuity boundary（Bool property binder / 3-case rejection）を再確認した。今回の分離は metadata_test を変更していない。API doc production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-api-doc-test-split.md`。

`crates/lsharp-driver/src/config.rs` (607 行) は inline test 21 件を `config_tests.rs` (316 行) へ移動し、production parent を 291 行へ縮小した。`config::tests` focused 21 件、driver unit 132 件、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit は pass。`default_path_delegation` の既存 embedded component / selfhost artifact failure boundary（stack overflow を含む）は今回の差分外として残り、driver は binary-only package のため doc-test は適用対象外。config production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-config-test-split.md`。

`crates/lsharp-lsp/src/util.rs` (862 行) は inline test 12 件を `util_tests.rs` (206 行) へ移動し、production parent を 653 行へ縮小した。`util::tests` focused 12 件、`lsharp-lsp` package 61 件、doc-tests 0/0、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit は pass。util production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-lsp-util-test-split.md`。

`crates/lsharp-types/src/types.rs` (527 行) は inline test 4 件を `types_tests.rs` (48 行) へ移動し、production parent を 479 行へ縮小した。`types::apply_subst_tests` focused 4 件、types package unit 209 件 / integration 49 件、doc-tests 0/0、clippy、Rust 2024 rustfmt、`git diff --check`、docs audit は pass。types production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-types-test-split.md`。

`crates/lsharp-tooling/src/artifact_cache.rs` (468 行) は inline test 6 件と cache filesystem fixture helper を `artifact_cache_tests.rs` (246 行) へ移動し、production parent を 222 行へ縮小した。`artifact_cache::tests` focused 6 件、tooling package 130 passed / 2 failed、clippy、doc-tests 0/0、Rust 2024 rustfmt、`git diff --check`、docs audit は確認済み。package の既存 metadata property LS2005 vacuity boundary 2 件は今回の差分外として残る。artifact cache production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-artifact-cache-test-split.md`。

`crates/lsharp-wasm/src/component_adapter.rs` (657 行) は inline test 8 件と Component/WIT runtime fixture helper を `component_adapter_tests.rs` (283 行) へ移動し、production parent を 377 行へ縮小した。`component_adapter::tests` focused 8 件、Rust 2024 rustfmt、`git diff --check` は pass。`lsharp-wasm` package の既存 root-lifetime test 1 件と clippy lint debt は差分外 failure boundary として残る。component adapter production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-component-adapter-test-split.md`。

`crates/lsharp-wasm/src/wasi_runner.rs` (1033 行) は inline test 25 件と WASI/Component runtime fixture helper を `wasi_runner_tests.rs` (450 行) へ移動し、production parent を 581 行へ縮小した。`wasi_runner::tests` focused 25 件、`lsharp-wasm` package 86 passed / 1 failed、clippy、Rust 2024 rustfmt、`git diff --check` は確認済み。package の既存 root-lifetime test 1 件と clippy lint debt は今回の差分外 failure boundary として残る。wasi runner production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-runner-test-split.md`。

`crates/lsharp-driver/src/main.rs` (4715 行) は test-only helper と inline test 132 件を `main_tests.rs` (2271 行) へ移動し、production parent を 2438 行へ縮小した。`main::tests` focused 132 件、driver package unit 132 件、clippy、対象 files の Rust 2024 rustfmt、`git diff --check` は pass。`default_path_delegation` は embedded component / selfhost artifact の既存 failure boundary 12 件を含み 34 passed / 12 failed。main command production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-main-test-split.md`。

`crates/lsharp-types/src/infer.rs` (4055 行) は test-only 11 module / 92 件を `infer_tests.rs` (1268 行) へ移動し、production parent を 2789 行へ縮小した。`include!` で既存 `infer::tests` 等の module path を維持し、focused infer 92 件、`lsharp-types` package 258 件、doc-tests 0/0、clippy、Rust 2024 rustfmt、`git diff --check` は pass。infer production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-infer-test-split.md`。

`crates/lsharp-wasm/src/wasi.rs` (5208 行) は inline test module 28 件と WASI/Wasmtime fixture helper を `wasi_tests.rs` (647 行) へ移動し、production parent を 4568 行へ縮小した。`include!` で既存 `wasi::tests` module path と private helper access を維持した。focused wasi 28 件は 27 pass / 1 件が既存 `RootLifetime::RootSetWithoutActiveSlot` failure、`lsharp-wasm --lib` は 86 pass / 1 fail、doc-tests 0/0、production clippy、Rust 2024 rustfmt、`git diff --check` は確認済み。`--all-targets -D warnings` は既存 test lint debt（移動した test block の unit closure lint と `native_cli_output` / E2E の既存 lint）で fail。wasi production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-wasi-test-split.md`。

`crates/lsharp-ir/src/lib.rs` (5462 行) は末尾の test 7 module / 61 件を `lib_tests.rs` へ移動し、production/helper parent を 3080 行へ縮小した。その後 `linker_tests` を `lib_tests/linker.rs` へ再分離し、`lib_tests.rs` は 2000 行、linker module は 383 行になった。`include!` / `#[path]` で既存の test module path と private helper access を維持した。`RUST_MIN_STACK=33554432 cargo test -p lsharp-ir` は 257 pass、doc-tests 0/0、clippy、Rust 2024 rustfmt、`git diff --check` は pass。default stack の formatter incremental fixture overflow は既存 boundary として残る。ir production split、I-01 / I-08 aggregate は未完了である。Evidence: `docs/adr/decisions-legacy-ir-lib-test-split.md`、`docs/adr/decisions-legacy-ir-linker-test-split.md`。

### 3. 優先順位

1. **imp-02 (エラー統一) の対象になるファイルを先に分割しない** — A-1 のエラー型変更を
   先に済ませ、診断が安定した後に分割する (コンフリクト最小化)。例外として
   テスト分離 (`#[cfg(test)]` の別ファイル化) はいつでも安全に実施できるため先行可
2. wasi.rs (imp-03 の改修対象) と module_graph.rs (imp-04 の改修対象) は、
   それぞれの機能改修**前**に分割しておく — 改修の diff が読める粒度になる
3. main.rs はユーザー影響がなく独立性が高いため、並行作業の隙間で進める

### 4. 機械検査

CI (または契約テスト) に「`crates/**/src/**/*.rs` の行数が 800 を超えるファイル一覧」を
出す検査を追加し、現状の超過リストを許容リストとして固定 → 分割の進捗に応じて
許容リストを縮め、最終的に空にする (リグレッション防止)。

## 影響範囲

- 公開 API 不変・ロジック移動のみのため、機能リスクは低い
- git blame の追跡性が下がるため、分割コミットは「移動のみ」と明記する
- 進行中ブランチとのコンフリクトが最大のコスト。各ファイルの分割タイミングは
  当該ファイルを触る作業の合間を選ぶ

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

設計 + module graph のテスト/path-resolution 分離、parser・constraints・macro expand・regex・lexer・metadata_check・hygiene・lower のテスト分離、lower/decl Self-TCO production split、doc_site test split、regex DFA/codegen/component_adapter/wasi_runner/main/infer/artifact_cache/closure/tracker/review/lockfile/resolver/knowledge/claude_plugin/config/api_doc/util/types/completion/analysis/format/references/rename/tooling-fmt test split、macro expand・constraints・regex production split、lexer/metadata_check test split verified partial slice (2026-07-25)。lower tests は parent 133 行 + 9 module（最大 692 行）、lower/decl は parent 692 行 + `decl/self_tco.rs` 139 行、doc_site は parent 575 行 + `doc_site/tests.rs` 266 行、hygiene は parent 297 行 + `hygiene/tests.rs` 249 行、regex DFA は parent 594 行 + `regex/dfa_tests.rs` 107 行、codegen は parent 237 行 + `codegen_tests.rs` 279 行、closure は parent 140 行 + `closure_tests.rs` 164 行、tracker は parent 135 行 + `tracker_tests.rs` 140 行、review は parent 296 行 + `review_tests.rs` 97 行 + `review_context_tests.rs` 48 行、lockfile は parent 143 行 + `lockfile_tests.rs` 133 行、resolver は parent 184 行 + `resolver_tests.rs` 49 行、knowledge は parent 120 行 + `knowledge_tests.rs` 87 行、claude_plugin は parent 110 行 + `claude_plugin_tests.rs` 126 行、config は parent 291 行 + `config_tests.rs` 316 行、api_doc は parent 326 行 + `api_doc_tests.rs` 207 行、util は parent 653 行 + `util_tests.rs` 206 行、types は parent 479 行 + `types_tests.rs` 48 行、artifact cache は parent 222 行 + `artifact_cache_tests.rs` 246 行、wasi runner は parent 581 行 + `wasi_runner_tests.rs` 450 行、main は parent 2438 行 + `main_tests.rs` 2271 行、infer は parent 2789 行 + `infer_tests.rs` 1268 行、component adapter は parent 377 行 + `component_adapter_tests.rs` 283 行、completion は parent 97 行 + `completion_tests.rs` 33 行、analysis は parent 80 行 + `analysis_tests.rs` 23 行、format は parent 159 行 + `format_tests.rs` 77 行、references は parent 59 行 + `references_tests.rs` 84 行、rename は parent 32 行 + `rename_tests.rs` 96 行、tooling fmt は parent 76 行 + `fmt_tests.rs` 75 行となった。lower expr/mod の production 分割は未着手である。parser の expr/decl production 分割、lexer/metadata production の責務分割、doc_site/tracker/review/lockfile/resolver/knowledge/claude_plugin/config/api_doc/util/types/component_adapter/artifact_cache/wasi_runner/main/infer/completion/analysis/format/references/rename/tooling fmt production の責務分割、regex parser/matcher の追加分割、codegen/closure production の追加分割、graph/SCC core の追加分割、`wasi.rs` / `main.rs` / `infer.rs` / `ir/lib.rs` の責務分割、I-01 / I-08 aggregate 完了は未着手または未完了である。着手時は TODO.md に Phase A-2 / D-4 としてファイル単位の項目を作成する。

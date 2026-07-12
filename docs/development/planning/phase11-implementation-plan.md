# Phase 11 実装バックログ詳細設計

## 位置づけ

- `TODO.md` は実行順の master checklist。
- この文書は task ID ごとの implementation bridge。仕様変更の正本ではなく、既存 spec を実コードへ落とすための実装方針・依存・受入条件を固定する。
- 正本 spec は `docs/development/planning/rust-parity-spec.md`, `docs/development/planning/toolchain-parity-spec.md`, `docs/development/validation/verification-spec.md`, `docs/development/planning/completion-criteria.md`, `docs/development/planning/runtime-stability-spec.md`, `docs/development/operations/ci-gate-v2-job-graph.md`, `docs/development/operations/artifact-policy.md`, `docs/development/operations/default-path-migration.md`, `docs/development/operations/release-playbook.md`, `docs/development/operations/fresh-clone-spec.md`, `docs/development/operations/adr-rust-removal.md`, `docs/development/operations/rollback-procedure.md` を優先する。

## 運用ルール

- task を閉じる条件は `Acceptance` を満たし、`Evidence` のファイル・テスト・snapshot 名が埋まること。
- `Evidence` は概念名ではなく、実ファイル名または実テスト名で記録する。
- test 分類は `unit`, `golden`, `e2e`, `bootstrap`, `release-smoke` の 5 種で固定する。
- `META-*` は継続タスクであり、他 workstream の PR でも同時更新を必須とする。
- `Gate 外 / v2` は Phase 11 完了判定に含めない。

## Critical Path Summary

| ID | 目的 | 閉じる条件 |
|----|------|-----------|
| `CP-01` | selfhost frontend を Rust fallback なしで回す | `BOOT-01`〜`BOOT-04` 完了 |
| `CP-02` | syntax/type parity の土台を固める | `SYNTAX-01`〜`SYNTAX-06`, `TYPE-01`〜`TYPE-08` 完了 |
| `CP-03` | IR/backend/Wasm bootstrap parity を成立させる（native parity は Deferred/v2） | `IR-01`〜`IR-06`, `WASM-01`〜`WASM-06` 完了。`NATIVE-01`〜`NATIVE-06` は Gate 外 / v2 |
| `CP-04` | 公開 toolchain を L# 実装へ移す | `CLI-01`〜`PKG-01` 完了 |
| `CP-05` | 長寿命 runtime を gate 化する | `GC-01`〜`GC-06` 完了 |
| `CP-06` | CI/release/default path を L# 正本へ切り替え、host launcher + guest component 配布へ収束させる | `OPS-01`〜`OPS-08` 完了（Rust 物理撤去は含まない） |

## Workstream Details

## WS-META Evidence / backlog hygiene

<a id="meta-01-compatibility-matrix-evidence-enrichment"></a>
### META-01 Compatibility matrix evidence enrichment

- Goal: `docs/development/planning/compatibility-matrix.md` を削除可否判断に使える evidence table に引き上げる。
- Current state: status と parity test の有無はあるが、Rust/L# 実装箇所と具体的 evidence が足りない。
- Rust source: `crates/lsharp-syntax/src/*`, `crates/lsharp-types/src/*`, `crates/lsharp-ir/src/*`, `crates/lsharp-wasm/src/*`
- L# target: `docs/development/planning/compatibility-matrix.md`
- Implementation direction: CLI/LSP/selfhost 各表を 8 列へ拡張し、`Rust source` と `L# source` を追加する。パス表記は `crate::module` と canonical な `selfhost/src/Namespace/File.ls` に固定し、`Parity test` には件数ではなく concrete test 名または snapshot 名を列挙する。
- Dependencies: なし。
- Acceptance: active row すべてに `Rust source`, `L# source`, `Parity test`, `Default path`, `Deletion gate` が埋まり、`-` が残るのは未着手機能だけ。
- Evidence: `docs/development/planning/compatibility-matrix.md`, `scripts/audit_docs.sh`

<a id="meta-02-completion-marker-sync"></a>
### META-02 Completion marker sync

- Goal: TODO/README/book/docs 間の完了表示を実装状況と一致させる。
- Current state: 仕様固定済みの `[x]` と、実装完了の `[x]` が混在している。
- Rust source: `README.md`, `book/`, `docs/*.md`, `docs/adr/*.jsonl`
- L# target: `TODO.md`, `README.md`, `book/`, `docs/*.md`
- Implementation direction: `仕様固定済み`, `部分実装`, `完了` の 3 状態を明示し、実装完了ではない `[x]` を `仕様固定済み` 注記へ置換する。以後、実装完了の記法は acceptance/evidence を持つ task だけに使う。
- Dependencies: `META-01`
- Acceptance: Phase 11 関連文書に「仕様固定済みだが未実装」を `[x]` で表す箇所が残らない。
- Evidence: `TODO.md`, `README.md`, `book/`, `scripts/audit_docs.sh`

<a id="meta-03-audit-docs-gate"></a>
### META-03 Audit-docs gate

- Goal: 文書の整合性崩れを CI で検出する。
- Current state: `scripts/audit_docs.sh` は存在するが Phase 11 の blocking gate に組み込まれていない。
- Rust source: `.github/workflows/ci.yml`, `scripts/audit_docs.sh`
- L# target: `.github/workflows/ci.yml`, `scripts/audit_docs.sh`
- Implementation direction: `docs` job で `scripts/audit_docs.sh` を必須実行し、互換マトリクス未更新・壊れたリンク・完了表記の矛盾を fail にする。
- Dependencies: `META-01`, `META-02`
- Acceptance: selfhost/toolchain/runtime/CI に触れる PR で audit が required check になる。
- Evidence: `.github/workflows/ci.yml`, CI job `docs`

<a id="meta-04-gap-backlog-classification"></a>
### META-04 Gap backlog classification

- Goal: 未完了項目を差分種別で分類し、優先順位を固定する。
- Current state: TODO は機能別に並んでいるが、仕様差分と実装差分が分離されていない。
- Rust source: `docs/development/planning/gap-classification.md`
- L# target: `TODO.md`, `docs/development/planning/phase11-implementation-plan.md`
- Implementation direction: 各 task に `Gap class` を暗黙的に持たせ、`仕様差分` は spec 更新、`実装欠落` はコード追加、`出力差分` は golden 修正、`性能差分` は GC/benchmark、`運用差分` は CI/release へ振り分ける。
- Dependencies: `META-01`
- Acceptance: Phase 11 task のいずれも `仕様差分` と `実装欠落` を混同しない。
- Evidence: この文書の各 task 記述, `docs/development/planning/gap-classification.md`

<a id="meta-05-differential-allowlist-registry"></a>
### META-05 Differential allowlist registry

- Goal: Wasm/native 既知差分の一時退避先を 1 つに固定する。
- Current state: allowlist ファイルが未作成で、差分が TODO や会話ログに散っている。
- Rust source: `docs/development/validation/verification-spec.md`
- L# target: `tests/differential-allowlist.yaml`
- Implementation direction: allowlist は `id`, `category`, `observation`, `reason`, `resolve_condition`, `tracking_issue` の YAML 配列に固定し、追加時は TODO の該当 task へ逆リンクを張る。
- Dependencies: `NATIVE-06`, `OPS-01`
- Acceptance: 差分例外はすべて `tests/differential-allowlist.yaml` に集約され、件数 0 を完了条件にできる。
- Evidence: `tests/differential-allowlist.yaml`, `docs/development/validation/verification-spec.md`

## WS-BOOTSTRAP Frontend unblock

<a id="boot-01-mainls-import-path-consolidation"></a>
### BOOT-01 Main.ls import path consolidation

- Goal: `selfhost/src/App/Main.ls` を統合パイプラインだけに縮退させ、暫定インライン定義を除去する。
- Current state: [Main.ls](/Users/biwakonbu/github/lsharp/selfhost/src/App/Main.ls) は `import` を宣言しつつ、Token/AST/IR/Compiler/WasmEmit を再定義している。
- Rust source: `crates/lsharp-syntax/src/*`, `crates/lsharp-ir/src/*`, `crates/lsharp-wasm/src/*`
- L# target: `selfhost/src/App/Main.ls`, `selfhost/src/Syntax/Token.ls`, `selfhost/src/Syntax/AST.ls`, `selfhost/src/IR/IR.ls`, `selfhost/src/Backend/Wasm/Compiler.ls`, `selfhost/src/Backend/Wasm/WasmEmit.ls`
- Implementation direction: `Main.ls` の責務を `pipeline orchestration + CLI entry` に限定する。各モジュールは最低 1 つの public entrypoint を持つ。`Lexer.tokenize`, `Parser.parse-program`, `MacroExpand.expand-program`, `TypeInfer.infer-program`, `Compiler.lower-program`, `WasmEmit.emit-module` を固定 API にする。
- Dependencies: `SYNTAX-01`, `TYPE-01`, `IR-02`, `WASM-01`
- Acceptance: `Main.ls` に Token/AST/IR/Compiler/WasmEmit の再定義が残らず、selfhost core modules import だけでパイプラインが組み上がる。
- Evidence: `selfhost/src/App/Main.ls`, `test_e2e_selfhost_pipeline_complete_stages`

<a id="boot-02-macroexpand-parser-compat-cleanup"></a>
### BOOT-02 MacroExpand parser-compat cleanup

- Goal: `selfhost/src/Syntax/MacroExpand.ls` を現行 parser が受理できる構文へ揃える。
- Current state: [MacroExpand.ls](/Users/biwakonbu/github/lsharp/selfhost/src/Syntax/MacroExpand.ls) は direct compile blocker になっている。
- Rust source: `crates/lsharp-syntax/src/macro_expand.rs`, `crates/lsharp-syntax/src/parser.rs`
- L# target: `selfhost/src/Syntax/MacroExpand.ls`
- Implementation direction: 第 1 段階では parser widening ではなく `MacroExpand.ls` を parser-compatible subset に書き換える。reader shorthand や未対応 syntactic sugar は使わず、quote/unquote は explicit AST constructor 経由で表す。
- Dependencies: `BOOT-01`, `SYNTAX-02`
- Acceptance: `cargo run -- compile selfhost/src/Syntax/MacroExpand.ls` と `cargo run -- compile selfhost/src/App/Main.ls` が成功する。
- Evidence: `cargo run -- compile selfhost/src/Syntax/MacroExpand.ls`, `cargo run -- compile selfhost/src/App/Main.ls`

<a id="boot-03-stdlib-direct-compile-blockers"></a>
### BOOT-03 stdlib direct compile blockers

- Goal: `stdlib/*.ls` を selfhost compiler で個別に direct compile できるようにする。
- Current state: stdlib compile は監視扱いで、`stdlib/Map.ls` などの blocker が残っている。
- Rust source: `stdlib/*.ls`, `crates/lsharp-wasm/tests/e2e.rs`
- L# target: `stdlib/*.ls`, `scripts/ci/compile-phase11-inputs.sh`
- Implementation direction: 固定入力集合を shell script で列挙し、`selfhost/src/**/*.ls`, `stdlib/*.ls`, `examples/*.ls` を 1 ファイルずつ compile する。失敗は module/type/macro/backend のどこで落ちたかに分類して TODO へ戻す。
- Dependencies: `BOOT-02`, `TYPE-02`, `IR-01`
- Acceptance: fixed input set 全件が individual compile で pass し、CI の stdlib compile を `continue-on-error` から blocking に上げられる。
- Evidence: `scripts/ci/compile-phase11-inputs.sh`, `test_e2e_bootstrap_ci_stdlib_compile`, `test_e2e_bootstrap_stage1_compile_selfhost_sources`

<a id="boot-04-true-stage1-stage2-stage3-bootstrap"></a>
### BOOT-04 True stage1-stage2-stage3 bootstrap

- Goal: proxy bootstrap を本物の 3 段固定点検証へ置換する。
- Current state: `test_e2e_bootstrap_four_layer_comparison` と `test_e2e_bootstrap_stage_chain_verification` により、stage0(Rust) の決定性と stage1.wasm の実行可能性は確認済み。さらに minimal subset / extended `do` / zero-arg multi-fn call / single-param call / let local / 5 defn loop / memory-only builtin / `vector-*` / `ref-*` / `map-*` / string literal data section / `read-file` / `command-line-arg` / `__fnv1a_hash` / generalized helper pair-triple-quad sliceまで stage1→stage2 実生成と repeatability が通っている。`scripts/ci/compile-phase11-inputs.sh` は selfhost/stdlib/examples の fixed input set compile gate を持ち、`RUN_BOOTSTRAP_FIXED_POINT=1` 時に `test_e2e_bootstrap_fixed_point_stage2_stage3`・`test_e2e_bootstrap_stage2_self_feed_fixed_input_set`・`test_e2e_bootstrap_fixed_input_set_stage_chain_match` を exact 実行する。`test_e2e_bootstrap_cli_fixed_input_compile_gate` により `App/Cli.ls` import 面も `Types.TypeInferCore` / `Tools.Text.FormatterDecl` / `Tools.Lsp.JsonRpc` / `Tools.Lsp.LspServerCore` / `Tools.Lsp.LspServerNav` で同期され、historical compiler-mode memory.grow blocker は再現しない。self-feed gate は fixed input set 54 件（selfhost 40 / stdlib 11 / examples 3）を stage2 self compiler で 2 回ずつ self-feed し、single-module recovery / valid wasm / byte-identical determinism を `ci-artifacts/bootstrap-diff/{sha}/fixed-input-set-self-feed-report.txt` と `fixed-input-set-self-feed.json` に保存する。さらに stage-chain gate は同じ 54 件について `stage1.wasm -> stage2-target.wasm` と `stage2.wasm -> stage3-target.wasm` を full-set compare し、bit-identical fixed point を `fixed-input-set-stage-chain-report.txt` と `fixed-input-set-stage-chain.json` へ保存する。これにより true `stage1.wasm -> stage2.wasm -> stage3.wasm` self-bootstrap fixed point の full-set 実体比較まで到達した。
- Rust source: `docs/development/validation/verification-spec.md`, `crates/lsharp-wasm/tests/e2e.rs`
- L# target: `crates/lsharp-wasm/tests/e2e.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_acceptance.rs`, `scripts/ci/compile-phase11-inputs.sh`, `.github/workflows/ci.yml`, `ci-artifacts/bootstrap-diff/`
- Implementation direction: stage0(Rust) -> stage1.wasm -> stage2.wasm -> stage3.wasm を必ず実体生成し、比較層は `raw wasm bytes`, `exported symbol list`, `data section bytes`, `compiler diagnostics` の 4 つに固定する。失敗時 diff は artifact へ保存する。
- Dependencies: `BOOT-01`, `BOOT-03`, `WASM-03`
- Acceptance: `test_e2e_bootstrap_stage1_stage2_match`, `test_e2e_bootstrap_fixed_point_stage2_stage3`, `test_e2e_bootstrap_stage2_self_feed_fixed_input_set`, `test_e2e_bootstrap_fixed_input_set_stage_chain_match`, `test_e2e_bootstrap_stage1_section_stability`, `test_e2e_bootstrap_stage1_symbol_stability` が実体比較で pass する。
- Evidence: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_acceptance.rs`, `crates/lsharp-wasm/tests/e2e/strings_patterns_compiler_integration.rs`, `scripts/ci/compile-phase11-inputs.sh`, `ci-artifacts/bootstrap-diff/`

## WS-SYNTAX Frontend syntax parity

<a id="syntax-01-span-model"></a>
### SYNTAX-01 Span model

- Goal: Rust `Span` と等価な selfhost span/token モデルを導入する。
- Current state: parser/lexer は 3 つ組 span 配列を扱うが、独立した `Span.ls` がなく AST の正本にもなっていない。
- Rust source: `crates/lsharp-syntax/src/span.rs`, `crates/lsharp-syntax/src/token.rs`
- L# target: `selfhost/src/Syntax/Span.ls`, `selfhost/src/Syntax/Token.ls`, `selfhost/src/Syntax/Lexer.ls`, `selfhost/src/Syntax/Parser.ls`
- Implementation direction: `Span` は Rust と同じ `[start end]` に固定し、line/column は diagnostics 層で導出する。`Token` は `[kind payload span]` に正規化し、literal payload は token 側で保持する。
- Dependencies: `BOOT-01`
- Acceptance: lexer/parser/type error が全て span を返し、span merge と dummy が `Span.ls` に集約される。
- Evidence: `selfhost/src/Syntax/Span.ls`, `test_e2e_selfhost_lexer_basic`, `test_e2e_selfhost_parser_basic`

<a id="syntax-02-full-ast-coverage"></a>
### SYNTAX-02 Full AST coverage

- Goal: Rust AST の Decl/Expr/Pattern/Literal/Metadata を selfhost AST に 1:1 対応させる。
- Current state: `Parser.ls` と `AST.ls` は最小 subset だけを扱い、quote/record/annotation/computation/trait metadata 等が欠けている。
- Rust source: `crates/lsharp-syntax/src/ast.rs`
- L# target: `selfhost/src/Syntax/AST.ls`, `selfhost/src/Syntax/Parser.ls`
- Implementation direction: 全 AST ノードを `[tag span ...payload]` 形式に統一し、constructor/accessor を `AST.ls` へ集約する。`Pattern`, `Metadata`, `TypeExpr`, `WhereClause`, `MatchArm`, `Variant`, `Param` も tagged vector で正規化する。
- Dependencies: `SYNTAX-01`
- Acceptance: parser が Rust AST に存在する全ノード型を lossless に表現できる。
- Evidence: `selfhost/src/Syntax/AST.ls`, `test_e2e_selfhost_parser_full_sexp`, `test_e2e_selfhost_module_declarations`

<a id="syntax-03-parser-recovery-and-diagnostics"></a>
### SYNTAX-03 Parser recovery and diagnostics

- Goal: parser recovery と複数診断収集を parity 条件に入れる。
- Current state: parser は fail-fast に近く、診断の severity/code/order が固定されていない。
- Rust source: `crates/lsharp-syntax/src/parser.rs`
- L# target: `selfhost/src/Syntax/Parser.ls`
- Implementation direction: parser の戻り値を `program + diagnostics` に拡張し、diagnostic は `[severity code span message-hash]` へ固定する。recovery point は `)`・`]`・`}`・次トップレベル宣言キーワードの 4 種に固定する。
- Dependencies: `SYNTAX-01`, `SYNTAX-02`
- Acceptance: 1 回の parse で複数エラーを収集し、diagnostics が source order で安定する。
- Evidence: `test_golden_syntax_recovery_*`, `test_e2e_selfhost_parser_basic`

<a id="syntax-04-hygiene-gensym-and-expansion-trace"></a>
### SYNTAX-04 Hygiene gensym and expansion trace

- Goal: 衛生マクロの最小完全モデルを selfhost へ導入する。
- Current state: `MacroExpand.ls` は引数置換中心で、gensym と expansion trace が未整備。
- Rust source: `crates/lsharp-syntax/src/hygiene.rs`, `crates/lsharp-syntax/src/macro_expand.rs`
- L# target: `selfhost/src/Syntax/Hygiene.ls`, `selfhost/src/Syntax/MacroExpand.ls`, `selfhost/src/Syntax/AST.ls`
- Implementation direction: `Hygiene.ls` に `scope-id`, `gensym-counter`, `expansion-trace` を集約する。binder/var は raw hash ではなく hygiene symbol を保持し、マクロ展開時に generated symbol へ fresh gensym を割り当てる。
- Dependencies: `SYNTAX-02`, `BOOT-02`
- Acceptance: nested macro 展開で衝突しない生成名を得られ、エラー時に expansion trace を表示できる。
- Evidence: `test_e2e_selfhost_macro_nested_expansion`, `test_e2e_selfhost_macro_defmacro_with_args`

<a id="syntax-05-derive-expansion"></a>
### SYNTAX-05 Derive expansion

- Goal: derive expansion を MacroExpand から分離して selfhost へ実装する。
- Current state: `Derive.ls` が存在せず、derive は未移植。
- Rust source: `crates/lsharp-syntax/src/derive.rs`
- L# target: `selfhost/src/Syntax/Derive.ls`, `selfhost/src/App/Main.ls`
- Implementation direction: pipeline 順を `Parser -> Derive -> MacroExpand -> TypeInfer` に固定する。derive は top-level `Decl` を受け取り helper decl 群へ展開し、一般 macro 展開前に実行する。
- Dependencies: `SYNTAX-02`
- Acceptance: derive 由来の helper decl が parser と type infer の入力として安定生成される。
- Evidence: `selfhost/src/Syntax/Derive.ls`, `test_golden_syntax_derive_*`

<a id="syntax-06-syntax-golden-fixtures"></a>
### SYNTAX-06 Syntax golden fixtures

- Goal: Rust parser/lexer の挙動を fixture 化し、L# syntax parity の gate にする。
- Current state: syntax golden fixture の正本がない。
- Rust source: `crates/lsharp-syntax/src/*`
- L# target: `tests/golden/syntax/`, `crates/lsharp-wasm/tests/e2e.rs`
- Implementation direction: Rust 側 exporter で `input`, `tokens`, `ast`, `diagnostics` を JSON fixture 化する。L# 側は同 fixture を読み、token/ast/diagnostics を deep-equal 比較する。
- Dependencies: `SYNTAX-01`〜`SYNTAX-05`
- Acceptance: syntax fixture 差分が CI gate になり、golden 更新は Rust 由来 fixture のみで行う。
- Evidence: `tests/golden/syntax/*`, `test_golden_syntax_*`

## WS-TYPES HM / constraints / metadata parity

<a id="type-01-type-api-normalization"></a>
### TYPE-01 Type API normalization

- Goal: Type 層の責務を `Type.ls`, `TypeScheme.ls`, `TypeInfer.ls` へ整理する。
- Current state: [TypeInfer.ls](/Users/biwakonbu/github/lsharp/selfhost/src/Types/TypeInfer.ls) が type constructor, substitution, instantiate を再定義している。
- Rust source: `crates/lsharp-types/src/types.rs`
- L# target: `selfhost/src/Types/Type.ls`, `selfhost/src/Types/TypeScheme.ls`, `selfhost/src/Types/TypeInfer.ls`
- Implementation direction: `Type.ls` は type representation + substitution + occurs-check のみ、`TypeScheme.ls` は `mono/poly/free-type-vars/generalize/instantiate` のみ、`TypeInfer.ls` は inference state と algorithm のみに限定する。
- Dependencies: `BOOT-01`
- Acceptance: `TypeInfer.ls` に type representation 再定義が残らない。
- Evidence: `selfhost/src/Types/Type.ls`, `selfhost/src/Types/TypeScheme.ls`, `test_e2e_selfhost_type_system`

<a id="type-02-unify-generalize-instantiate"></a>
### TYPE-02 Unify generalize instantiate

- Goal: HM 推論の核を Rust 実装と同じ公開挙動へ揃える。
- Current state: unify と instantiate は最小版で、program-level inference と let-polymorphism が限定的。
- Rust source: `crates/lsharp-types/src/infer.rs`, `crates/lsharp-types/src/types.rs`
- L# target: `selfhost/src/Types/TypeInfer.ls`, `selfhost/src/Types/TypeScheme.ls`
- Implementation direction: `InferState = [subst counter diagnostics]` を導入し、`infer-expr`, `generalize`, `instantiate`, `apply-subst-env` を state threading で実装する。let-bound value は必ず generalize し、application は arity と function type を明示的に unify する。
- Dependencies: `TYPE-01`
- Acceptance: literal/variable/function/let-polymorphism/unification の既存 E2E が stateful 実装へ置換されても通る。
- Evidence: `test_e2e_selfhost_typeinfer_literal`, `test_e2e_selfhost_typeinfer_variable`, `test_e2e_selfhost_typeinfer_function`, `test_e2e_selfhost_typeinfer_let_poly`, `test_e2e_selfhost_typeinfer_unification`

<a id="type-03-match-inference"></a>
### TYPE-03 Match inference

- Goal: `match` の型推論とパターン束縛を selfhost 側で完結させる。
- Current state: pattern match は E2E 1 本あるが、arm ごとの binder/type refinement が限定的。
- Rust source: `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/src/Types/TypeInfer.ls`, `selfhost/src/Syntax/AST.ls`
- Implementation direction: `infer-pattern` を追加し、pattern が生成する binder env と scrutinee expected type を返す。各 arm body は共通 result type へ unify し、guard は Bool に固定する。
- Dependencies: `SYNTAX-02`, `TYPE-02`
- Acceptance: wildcard/var/literal/constructor/record pattern の binder と arm return type が正しく推論される。
- Evidence: `test_e2e_selfhost_typeinfer_pattern_match`, `test_golden_types_match_*`

<a id="type-04-constraints-trait-where"></a>
### TYPE-04 Constraints trait where

- Goal: trait/where/constraint solving を `Constraints.ls` へ分離する。
- Current state: `Constraints.ls` が存在せず、trait 解決ロジックを置く場所がない。
- Rust source: `crates/lsharp-types/src/constraints.rs`
- L# target: `selfhost/src/Types/Constraints.ls`, `selfhost/src/Types/TypeInfer.ls`
- Implementation direction: `Constraints.ls` は trait registry, impl registry, pending constraint queue, solver を持つ。`TypeInfer.ls` は constraint を生成するだけにし、解決は `Constraints.solve-all` へ委譲する。
- Dependencies: `TYPE-01`, `TYPE-02`
- Acceptance: where clause と trait constraint が pending queue で解決され、未解決時は deterministic な error code を返す。
- Evidence: `selfhost/src/Types/Constraints.ls`, `test_golden_types_constraints_*`

<a id="type-05-metadata-check"></a>
### TYPE-05 Metadata check

- Goal: metadata validation を type inference とは別モジュールへ隔離する。
- Current state: `MetadataCheck.ls` が存在せず、metadata の構造検証も未移植。
- Rust source: `crates/lsharp-types/src/metadata_check.rs`
- L# target: `selfhost/src/Types/MetadataCheck.ls`
- Implementation direction: `MetadataCheck.validate-program` を導入し、doc/params/returns/invariant/example/transitions の構造制約を検証する。type checker は metadata expression の type だけを返し、メタデータ schema 検証はここへ移す。
- Dependencies: `SYNTAX-02`, `TYPE-02`
- Acceptance: metadata 不正が type error と混ざらず、独立した validation error として返る。
- Evidence: `selfhost/src/Types/MetadataCheck.ls`, `test_golden_types_metadata_*`

<a id="type-06-hkt-gadt-alias-record-update"></a>
### TYPE-06 HKT GADT alias record update

- Goal: 高度型機能の最小完了集合を selfhost 側へ揃える。
- Current state: HKT/GADT/alias/record update はほぼ未移植。
- Rust source: `crates/lsharp-types/src/types.rs`, `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/src/Types/Type.ls`, `selfhost/src/Types/TypeInfer.ls`, `selfhost/src/Types/Constraints.ls`
- Implementation direction: kind environment を `Type.ls` に追加し、type constructor application で kind check を行う。GADT は constructor return type の specialization を match inference に渡し、record update は original record type と field patch を unify する。
- Dependencies: `TYPE-03`, `TYPE-04`
- Acceptance: type alias 展開、record update、GADT constructor match、HKT kind error が自動検証で閉じる。
- Evidence: `test_golden_types_hkt_*`, `test_golden_types_gadt_*`, `test_e2e_type_alias`

<a id="type-07-type-error-parity"></a>
### TYPE-07 Type error parity

- Goal: error code / span / primary message の parity を固定する。
- Current state: selfhost 側はエラーマーカー中心で、Rust と同じエラー構造を返していない。
- Rust source: `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/src/Types/TypeInfer.ls`, `selfhost/src/Types/Constraints.ls`
- Implementation direction: type error は `[code span primary secondary* help*]` に固定し、primary message の意味一致を必須にする。secondary/help は byte-perfect ではなく meaning-preserving でよいが、`code` と `span` は一致させる。
- Dependencies: `SYNTAX-01`, `TYPE-02`, `TYPE-04`
- Acceptance: representative type mismatch/undefined var/arity/trait/kind errors が golden fixture と一致する。
- Evidence: `tests/golden/types/errors/*`, `test_e2e_type_error_rejected`

<a id="type-08-deterministic-ordering"></a>
### TYPE-08 Deterministic ordering

- Goal: type variable naming と diagnostics/display 順序を安定化する。
- Current state: hash map 由来の順序揺れ余地が残る。
- Rust source: `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/src/Types/TypeInfer.ls`, `selfhost/src/Types/TypeScheme.ls`
- Implementation direction: type var id は source order 発番に固定し、env/constraint/diagnostic の走査順は ordered map or insertion order vector へ統一する。
- Dependencies: `TYPE-02`, `TYPE-07`
- Acceptance: 同一ソースの 2 回 typecheck で type variable 名と diagnostics 順が一致する。
- Evidence: `test_e2e_selfhost_all_modules_deterministic`, `test_golden_types_deterministic_*`

## WS-IR-BACKEND Lowering / Wasm parity

<a id="ir-01-module-graph"></a>
### IR-01 Module graph

- Goal: multi-file compile の依存解決を `ModuleGraph.ls` へ切り出す。
- Current state: `ModuleGraph.ls` は存在せず、module compile order は ad-hoc。
- Rust source: `crates/lsharp-ir/src/module_graph.rs`
- L# target: `selfhost/src/IR/ModuleGraph.ls`
- Implementation direction: import decl から adjacency list を生成し、topological sort と cycle diagnostic を提供する。入力は file path list、出力は ordered module plan。
- Dependencies: `SYNTAX-02`
- Acceptance: module graph の順序と循環依存エラーが deterministic である。
- Evidence: `selfhost/src/IR/ModuleGraph.ls`, `test_e2e_selfhost_module_graph_topological_sort`

<a id="ir-02-lower-split"></a>
### IR-02 Lower split

- Goal: lowering を `Lower.ls`, `LowerExpr.ls`, `LowerDecl.ls`, `LowerPattern.ls` に分割する。
- Current state: lowering は `Main.ls` と `Compiler.ls` に混在している。
- Rust source: `crates/lsharp-ir/src/lower/mod.rs`, `crates/lsharp-ir/src/lower/expr.rs`, `crates/lsharp-ir/src/lower/decl.rs`, `crates/lsharp-ir/src/lower/pattern.rs`
- L# target: `selfhost/src/IR/Lower.ls`, `selfhost/src/IR/LowerExpr.ls`, `selfhost/src/IR/LowerDecl.ls`, `selfhost/src/IR/LowerPattern.ls`, `selfhost/src/Backend/Wasm/Compiler.ls`
- Implementation direction: `Compiler.ls` は orchestration 層へ縮退し、decl lowering と expr lowering を分離する。`lower-program` は `FrontendResult -> LoweredModule` の唯一の入口にする。
- Dependencies: `BOOT-01`, `TYPE-02`
- Acceptance: `Main.ls` から lowering 実装が剥がれ、`Compiler.ls` は `Lower.ls` を呼ぶだけになる。
- Evidence: `selfhost/src/IR/Lower.ls`, `test_e2e_selfhost_ir`, `test_e2e_selfhost_compiler`

<a id="ir-03-closure-conversion"></a>
### IR-03 Closure conversion

- Goal: 自由変数解析と環境キャプチャを独立段階にする。
- Current state: closure conversion 専用モジュールがない。
- Rust source: `crates/lsharp-ir/src/closure.rs`
- L# target: `selfhost/src/IR/Closure.ls`, `selfhost/src/IR/LowerExpr.ls`
- Implementation direction: lambda lowering 前に free var set を計算し、environment record を明示生成する。 direct call と closure call を別 IR 命令にし、table slot 生成は codegen へ渡す。
- Dependencies: `IR-02`
- Acceptance: nested lambda と higher-order function が closure conversion 後も deterministic IR を出す。
- Evidence: `test_golden_ir_closure_*`, `crates/lsharp-ir/src/lower/snapshots/*lower_closure.snap`

<a id="ir-04-pattern-lowering"></a>
### IR-04 Pattern lowering

- Goal: pattern match を IR の branch sequence へ落とす。
- Current state: pattern lowering は限定的または未分離。
- Rust source: `crates/lsharp-ir/src/lower/pattern.rs`
- L# target: `selfhost/src/IR/LowerPattern.ls`
- Implementation direction: literal/constructor/record/wildcard pattern をタグ比較・field extraction・guard branch へ展開する。 match arm 順は source order を維持し、failure path を次 arm へ接続する。
- Dependencies: `TYPE-03`, `IR-02`
- Acceptance: ADT match と nested pattern が stable IR snapshot を生成する。
- Evidence: `test_golden_ir_pattern_*`, `crates/lsharp-ir/src/lower/snapshots/*lower_adt_match.snap`

<a id="ir-05-trait-dispatch-lowering"></a>
### IR-05 Trait dispatch lowering

- Goal: trait method call を辞書引数付き call へ lowering する。
- Current state: trait dispatch lowering の専用段階がない。
- Rust source: `crates/lsharp-ir/src/lower/mod.rs`
- L# target: `selfhost/src/IR/LowerDecl.ls`, `selfhost/src/IR/LowerExpr.ls`
- Implementation direction: solved trait constraints から dictionary value を materialize し、method call site に hidden first arg として挿入する。 impl selection は type phase の結果だけを使い、lowering では分岐しない。
- Dependencies: `TYPE-04`, `IR-02`
- Acceptance: trait dispatch IR が impl order ではなく solved constraint order で安定する。
- Evidence: `crates/lsharp-ir/src/lower/snapshots/*lower_trait_dispatch.snap`, `test_golden_ir_trait_dispatch_*`

<a id="ir-06-ir-snapshot-serializer"></a>
### IR-06 IR snapshot serializer

- Goal: Rust/L# IR を line-based snapshot で比較できるようにする。
- Current state: IR snapshot の L# 側 serializer がない。
- Rust source: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/lower/tests.rs`
- L# target: `selfhost/src/IR/IR.ls`, `tests/golden/ir/`
- Implementation direction: snapshot format は `; module`, `; function`, `instruction` の 3 階層に固定し、type var と temp id は正規化する。 map 由来順序は serializer 側で stable sort する。
- Dependencies: `IR-02`〜`IR-05`
- Acceptance: examples/stdlib/selfhost の representative set で Rust/L# snapshot diff が空になる。
- Evidence: `tests/golden/ir/*`, `test_golden_ir_*`

<a id="wasm-01-backend-boundary"></a>
### WASM-01 Backend boundary

- Goal: backend 境界を `FrontendResult -> LoweredModule -> CodegenArtifact` の 3 層に固定する。
- Current state: `Main.ls` が pipeline 全段の暫定データを vector で持っている。
- Rust source: `docs/language/backend-boundary.md`, `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-wasm/src/lib.rs`
- L# target: `selfhost/src/App/Main.ls`, `selfhost/src/IR/IR.ls`, `selfhost/src/Backend/Wasm/WasmEmit.ls`
- Implementation direction: `FrontendResult`, `LoweredModule`, `CodegenArtifact` の tagged record を `IR.ls` と `WasmEmit.ls` で定義し、Main はこの 3 層を受け渡すだけにする。
- Dependencies: `IR-02`, `BOOT-01`
- Acceptance: Main の中間値は raw vector 配列ではなく named stage artifact に置き換わる。
- Evidence: `docs/language/backend-boundary.md`, `test_e2e_selfhost_full_pipeline`

<a id="wasm-02-section-builders"></a>
### WASM-02 Section builders

- Goal: Wasm backend を section builder 単位に分割する。
- Current state: [WasmEmit.ls](/Users/biwakonbu/github/lsharp/selfhost/src/Backend/Wasm/WasmEmit.ls) は 1 ファイルで LEB, header, section 生成を抱えている。
- Rust source: `crates/lsharp-wasm/src/codegen.rs`, `crates/lsharp-wasm/src/emit.rs`, `crates/lsharp-wasm/src/wasi.rs`
- L# target: `selfhost/src/Backend/Wasm/WasmEmit.ls`, `selfhost/src/Backend/Wasm/Codegen.ls`, `selfhost/src/Backend/Wasm/Emit.ls`, `selfhost/src/Backend/Wasm/WasiBackend.ls`
- Implementation direction: `Codegen.ls` は IR -> Wasm op sequence, `Emit.ls` は section bytes builder, `WasiBackend.ls` は imports/exports/memory/runtime wiring を担当する。`WasmEmit.ls` は façade に縮退させる。
- Dependencies: `WASM-01`
- Acceptance: type/import/function/memory/export/code/data section が独立 builder になり、unit test 可能になる。
- Evidence: `selfhost/src/Backend/Wasm/WasmEmit.ls`, `test_e2e_selfhost_wasm_emit`, `test_e2e_selfhost_wasmemit`

<a id="wasm-03-deterministic-leb-emit"></a>
### WASM-03 Deterministic LEB emit

- Goal: Wasm バイナリ出力の非決定性をなくす。
- Current state: LEB encode と section 生成が ad-hoc 実装で、ordering と padding の保証が弱い。
- Rust source: `crates/lsharp-wasm/src/emit.rs`
- L# target: `selfhost/src/Backend/Wasm/Emit.ls`, `selfhost/src/Backend/Wasm/WasmEmit.ls`
- Implementation direction: signed/unsigned LEB, section order, symbol order, data order を spec 固定順で出力する。ハッシュやホストパスや時刻を埋め込まない。
- Dependencies: `WASM-02`, `BOOT-04`
- Acceptance: 同一入力 2 回 compile で bytes/hash/export/data section が一致する。
- Evidence: `test_e2e_bootstrap_stage1_deterministic`, `test_e2e_bootstrap_deterministic_output`, `test_e2e_bootstrap_selfhost_modules_deterministic`

<a id="wasm-04-wasi-helpers"></a>
### WASM-04 WASI helpers

- Goal: runtime boundary を selfhost backend 側に移植する。
- Current state: helper 群の責務分離が不完全。
- Rust source: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi_runner.rs`
- L# target: `selfhost/src/Backend/Wasm/WasiBackend.ls`, `selfhost/src/Backend/Wasm/WasiRunner.ls`
- Implementation direction: `print`, `read_file`, `write_file`, `clock_now` などの helper import を固定し、WASI runner 側で path/time normalization を行う。
- Dependencies: `WASM-02`
- Acceptance: file I/O と time を含む program が deterministic な観測値を返す。
- Evidence: `test_golden_wasm_wasi_*`, `docs/development/validation/verification-spec.md`

<a id="wasm-05-test-runner"></a>
### WASM-05 Test runner

- Goal: `:example` / `:invariant` から自動テストを生成する。
- Current state: `selfhost/src/Tools/Test/TestRunner.ls` と `test_e2e_selfhost_test_runner_extracts_supported_metadata_suite`, `test_e2e_selfhost_test_runner_executes_examples_only`, `test_e2e_selfhost_test_runner_executes_invariant_only`, `test_e2e_selfhost_test_runner_executes_supported_metadata_suite` により、`:example` / `:invariant` の抽出、example 実行、invariant materialize までは追加済み。CLI 側も `test_e2e_selfhost_cli_test_source_metadata_pass` / `test_e2e_selfhost_cli_test_source_metadata_fail` で failing example -> `runtime-error` を確認できる。**ただし** 実行対象は算術/比較/if/let/do/トップレベル `defn` 呼び出しの supported subset に限られ、full metadata semantics / multi-file parity は未達。
- Rust source: `crates/lsharp-wasm/src/test_runner.rs`
- L# target: `selfhost/src/Tools/Test/TestRunner.ls`
- Implementation direction: metadata を走査し、example は compile-and-run assertion、invariant は compile-time property check へ変換する。生成 test 名は `test_generated_example_<symbol>` 形式に固定する。
- Dependencies: `TYPE-05`, `WASM-04`
- Acceptance: example/invariant 付き宣言から deterministic な generated test suite が得られる。
- Evidence: `selfhost/src/Tools/Test/TestRunner.ls`, `test_e2e_selfhost_test_runner_extracts_supported_metadata_suite`, `test_e2e_selfhost_test_runner_executes_examples_only`, `test_e2e_selfhost_test_runner_executes_invariant_only`, `test_e2e_selfhost_test_runner_executes_supported_metadata_suite`, `test_e2e_selfhost_cli_test_source_metadata_pass`, `test_e2e_selfhost_cli_test_source_metadata_fail`

<a id="wasm-06-wasm-golden"></a>
### WASM-06 Wasm golden

- Goal: Wasm output を section hash + runtime result で golden 比較する。
- Current state: Wasm backend の parity gate は E2E 中心で、golden fixture が不足している。
- Rust source: `crates/lsharp-wasm/src/*`
- L# target: `tests/golden/wasm/`, `crates/lsharp-wasm/tests/e2e.rs`
- Implementation direction: fixture には `exports`, `section_hashes`, `stdout`, `stderr`, `exit_code` を保存する。L# 側は byte-by-byte ではなく section-aware diff で mismatch を報告する。
- Dependencies: `WASM-03`, `WASM-04`, `WASM-05`
- Acceptance: representative inputs の Wasm parity が golden test 化され、Rust fixture 差分ゼロが gate になる。
- Evidence: `tests/golden/wasm/*`, `test_golden_wasm_*`

## WS-NATIVE Native backend / bootstrap parity

<a id="native-01-target-descriptors"></a>
### NATIVE-01 Target descriptors

- Goal: supported product/release target ごとの差分を descriptor に閉じ込める。
- Current state: `selfhost/src/Backend/Native/NativeTarget.ls` に skeleton はあるが、descriptor は `arch`, `os`, `obj-format`, `triple-id` の narrow slice に留まる。
- Rust source: `docs/language/native-backend-spec.md`
- L# target: `selfhost/src/Backend/Native/NativeTarget.ls`
- Implementation direction: supported product/release targets (`aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`) の target descriptor を `abi`, `section names`, `relocation types`, `linker flavor`, `runtime artifact policy` の 5 項目で定義する。`x86_64-apple-darwin` は internal diagnostic descriptor としてのみ残す。
- Dependencies: `IR-06`
- Acceptance: target 固有条件分岐が codegen 本体ではなく descriptor 参照だけになる。
- Evidence: `selfhost/src/Backend/Native/NativeTarget.ls`

<a id="native-02-object-emitter"></a>
### NATIVE-02 Object emitter

- Goal: LoweredModule から relocation 付き object を生成する。
- Current state: `selfhost/src/Backend/Native/NativeCodegen.ls` / `selfhost/src/Backend/Native/NativeEmit.ls` の skeleton と narrow execution slice はあるが、relocation 付き object の product path と `runtime.o` 分離契約は未固定。
- Rust source: `docs/language/native-backend-spec.md`
- L# target: `selfhost/src/Backend/Native/NativeCodegen.ls`, `selfhost/src/Backend/Native/NativeEmit.ls`
- Implementation direction: codegen は machine-instr list を生成し、emit は `program.o` を中心に artifact 契約を閉じる。shadow path を早く閉じるため、必要なら補助的な external object-generation path を許容するが、product path では `program.o`, `runtime.o`, `linker-response.txt`, `program.native` の契約と determinism を固定する。
- Dependencies: `NATIVE-01`, `IR-02`
- Acceptance: tier1 target で object file が生成され、linker 手前で停止できる。
- Evidence: `build/native/*`, `test_unit_native_emit_*`

<a id="native-03-linker-response"></a>
### NATIVE-03 Linker response

- Goal: linker invocation を deterministic response file 化する。
- Current state: `selfhost/src/Backend/Native/Linker.ls` に response file skeleton はあるが、tier1 linker flavor (`ld64`, `ld.lld`, `ld`) と product path の引数契約は未固定。
- Rust source: `docs/language/native-backend-spec.md`, `docs/development/operations/ci-migration-spec.md`
- L# target: `selfhost/src/Backend/Native/Linker.ls`
- Implementation direction: linker 呼び出しは command line 直打ちではなく response file 経由に固定する。response file には object 順、runtime object、output path、system libs を source order で記載し、Darwin は `ld64`、Linux は `ld.lld` 優先で固定する。
- Dependencies: `NATIVE-02`
- Acceptance: 2 回の native build で response file と linked binary hash が一致する。
- Evidence: `build/native/linker-response.txt`, `test_unit_native_linker_*`

<a id="native-04-deterministic-codegen"></a>
### NATIVE-04 Deterministic codegen

- Goal: native backend を fixed-point を壊さない deterministic backend にする。
- Current state: narrow deterministic slices はあるが、`program.o`, `runtime.o`, `linker-response.txt`, `program.native` の end-to-end determinism は未証明。
- Rust source: `docs/language/native-backend-spec.md`, `docs/development/planning/completion-criteria.md`
- L# target: `selfhost/src/Backend/Native/NativeCodegen.ls`, `selfhost/src/Backend/Native/NativeEmit.ls`
- Implementation direction: function order, static data order, relocation order, symbol numbering は source order + stable sort に固定する。debug info は v1 では出さない。
- Dependencies: `NATIVE-01`〜`NATIVE-03`
- Acceptance: 同一 commit 同一 target の 2 回 native build で binary hash が一致する。
- Evidence: `test_bootstrap_native_deterministic_*`, `docs/language/native-backend-spec.md`

<a id="native-05-stage1-native-self-regeneration"></a>
### NATIVE-05 Stage1-native self-regeneration

- Goal: stage1-native が selfhost compiler を自分で再生成できるようにする。
- Current state: Component Model pivot 後も native backend の canonical artifact / response file / deterministic stage observation / actual gap report を維持し、`test_e2e_stage23_actual_native_self_regeneration_harness_stage2_stage3_match` で representative `stage1-native -> stage2-native -> stage3-native` の artifact observation と transport payload 一致まで完了した。Darwin arm64 の representative actual self-regeneration は完了している。Linux x86_64 の `V2-13a-5` は完了し、local Lima VM の stage2/stage3 byte-for-byte compare を証跡とする。Linux x86_64 の `V2-15` は完了し、actual `App.Cli` bundle、rollback compatibility archive、2 target の local official archive/download smoke を固定した。NATIVE-05 を Phase 11 blocker として reopen しない。
- Rust source: `docs/development/planning/completion-criteria.md`
- L# target: `scripts/ci/build-native.sh`, `.github/workflows/ci.yml`
- Implementation direction: `stage1-native` の build entry は `selfhost/src/App/Main.ls` compile で固定し、`stage1-native -> stage2-native -> stage3-native` の functional equivalence を gate にする。
- Dependencies: `BOOT-04`, `NATIVE-04`
- Acceptance: representative input set に対して stage2-native と stage3-native の観測値が一致する。
- Evidence: `test_bootstrap_native_stage2_stage3_*`, `.github/workflows/ci.yml`

<a id="native-06-wasmnative-differential"></a>
### NATIVE-06 Wasm/native differential

- Goal: Wasm と native の観測差分をゼロにする。
- Current state: differential harness、`tests/differential-allowlist.yaml`、5 観測点 proxy test は追加済み。加えて `test_wasm_native_execution_parity_double`, `test_native_codegen_processes_multiple_ir_instructions` などの narrow execution parity slice があり、`NativeCodegen.ls` は 1 命令 3 byte の placeholder ではなく複数 IR 命令を順に full byte 列へ落とせる。**ただし** 現状の比較は file structure / diagnostics / 限定入力での proxy parity が中心で、tier1 native artifact の zero diff と実 native regeneration は未証明。
- Rust source: `docs/development/validation/verification-spec.md`
- L# target: `tests/differential-allowlist.yaml`, `.github/workflows/ci.yml`, `scripts/ci/compare-differential.sh`
- Implementation direction: 7 カテゴリ入力に対し `exit code`, `stdout`, `stderr`, `generated file bytes`, `diagnostics JSON` を比較する。差分は allowlist へ退避できるが、Phase 11 完了前に 0 件へ戻す。
- Dependencies: `BOOT-04`, `WASM-06`, `NATIVE-05`, `META-05`
- Acceptance: differential job が tier1 で green になり、allowlist 件数 0 を達成する。
- Evidence: `tests/differential-allowlist.yaml`, `golden-parity` job

## WS-TOOLCHAIN CLI / LSP / formatter / linter / docs / packaging

<a id="cli-01-command-contracts"></a>
### CLI-01 Command contracts

- Goal: 13 CLI command の入力・出力・終了コード契約を固定する。
- Current state: `compatibility-matrix` では多数が `なし` で、CLI contract table がない。
- Rust source: Rust driver, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `selfhost/src/App/Cli.ls`, `docs/development/planning/toolchain-parity-spec.md`
- Implementation direction: command contract は `args`, `stdin`, `stdout`, `stderr`, `exit code`, `artifacts` の 6 項目で表にし、`--help`, `--version`, `-o/--output` もこの表に含める。
- Dependencies: `META-01`
- Acceptance: 13 command すべてに contract table があり、help/version snapshot の正本になる。
- Evidence: `docs/development/planning/toolchain-parity-spec.md`, `test_snapshot_cli_help_*`

<a id="cli-02-13-command-implementations"></a>
### CLI-02 13 command implementations

- Goal: `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` を L# 実装で提供する。
- Current state: 13 サブコマンド名、終了コード API、stdout/stderr 分離、help/version smoke は揃っている。さらに `parse` は `decls:N` / `first-decl:<name>` / `first-body:<name>` の deterministic text に加えて `diagnostics:0` / `diagnostics:1,P0001@1:1,first-body:unexpected token )` / `unexpected token ]` 形式の token-aware summary text を返し、`parse-diagnostics-count` は `parse-with-recovery` 経由で recovery 対象の `)` / `]` を 1 件として数えられる。`test` command は `TestRunner` と接続され、supported subset の metadata suite を抽出・実行し、failing example を `runtime-error` で返せる。さらに `run-test-source` / `run-test` は `examples:N` / `invariants:N` / `failures:N` の labeled summary text を返せる。`check` は builtin 型名 text と `diagnostics:0` / `diagnostics:1,T0001@1:1,first-body:if condition must be Bool` / `undefined symbol` summary text を返し、`repl` は `type:Int` / `evals:1` / `input-bytes:17` の warmup session summary を返せ、`check-diagnostics-count` は top-level `defn` の型エラーを数えられる。`compile` / `build` は guest 側で `wasm-size:<n>` text を返しつつ、host launcher default path では output file に実 Wasm / Component bytes を書けるようになり、`scripts/ci/default-path-smoke.sh` / `scripts/ci/release-smoke.sh` / `scripts/smoke_test_readme.sh` でも magic bytes と実行可能性を確認できる。file-path compile/build は `App.ModuleResolver` / `App.CompilerMode` を介する import-aware path で nested import fixture と actual `main` の `compile/build <file>` / `-o` / `--output` / `--target` parity まで前進し、`install` は deterministic な dry-run plan text、`lsp` は `sync:full` / `hover:true` / `completion:true` / `definition:true` / `references:true` / `rename:true` / `formatting:true` に加えて `requests:1` / `documents:0` / `source-bytes:0` の shared-state summary text を返し、completion item も text label / insertText を返せる。`fmt` / `review` / `doc` は deterministic な実テキスト surface を返せ、`doc-ack` は `ack:recorded` + title/body + comment-form trailer を返し、`--trailer` では trailer 行だけを返せる。`doc-check` は `status:ok` + title/body + comment-form trailer を返し、`--strict` では末尾の `Doc-Review-Status` / `Doc-Reviewed-By` comment trailer を検証して invalid 時に compile-error text を返せる。`review` は count/title/body に加えて `warning` や `L0001@1:1` 形式の code/location text を stdout に出せ、summary body には `first-body:` detail も含められる。`generate-review` 由来の diagnostics も `severity` / `line` / `column` / `code` slot まで持て、unused-let body は binder 名を含められる。さらに `review --json` / `review --format json` は `DocJson.ls` 経由の schema-object review JSON text を返せ、`doc --json` / `doc --format json` も schema-object doc JSON text を返せる。source helper / actual `main` argv slice でも representative snapshots に固定し、invalid な `review/doc --format` value は actual `main` でも `error: unsupported option: ...` + non-zero exit を返せる。加えて `selfhost/src/App/EmbeddedCli.ls` と host launcher delegation gate も `review` text surface に加えて `review --json` / `review --format json`、`doc-ack --trailer`、`doc-check --strict` を embedded default path で処理でき、strict missing-trailer failure と runtime disable 下の option-aware hint 復帰まで `default_path_delegation.rs` と `scripts/ci/default-path-smoke.sh` で固定した。LSP stdio body parser も JSON string escape を decode するようになり、spec 寄り `textDocument.text` / `contentChanges[0].text` に含まれる escaped quote / newline と unicode escaped quote / newline (`\u0022` / `\u000a`) が actual `lsp --stdio` の formatting / completion でそのまま最新 source に収束する。**ただし** `doc` / host-backed subset を含む multi-command ownership、broader stateful transport parity、self-bootstrap fixed-point は未完。
- Rust source: Rust CLI 実装群
- L# target: `selfhost/src/App/Cli.ls`, `selfhost/src/App/Main.ls`, `selfhost/src/Tools/Lsp/JsonRpc.ls`, `selfhost/src/Tools/Text/Formatter.ls`, `selfhost/src/Tools/Text/Linter.ls`, `selfhost/src/Tools/Test/TestRunner.ls`
- Implementation direction: `selfhost/src/App/Cli.ls` を新設し、arg parse と subcommand dispatch を一元化する。`Main.ls` は compiler core only にし、CLI entry はここへ移す。
- Dependencies: `CLI-01`, `BOOT-01`, `LSP-01`, `FMT-01`, `DOC-01`
- Acceptance: 13 command すべてが help/version/exit code contract を満たし、host launcher が `compatibility-matrix.md` に定義した embedded guest / host-backed subset / delegation hook を選択できる。
- Evidence: `test_e2e_selfhost_cli_help_output`, `test_e2e_selfhost_cli_version_output`, `test_e2e_selfhost_test_runner_executes_supported_metadata_suite`, `test_e2e_selfhost_cli_test_source_metadata_pass`, `test_e2e_selfhost_cli_test_source_metadata_fail`, `test_e2e_selfhost_cli_review_source_json_snapshot`, `test_e2e_selfhost_cli_doc_source_json_snapshot`, `test_e2e_selfhost_cli_main_with_args_review_json_file`, `test_e2e_selfhost_cli_main_with_args_doc_json_file`, `test_review_json_without_lsharp_path_uses_embedded_component_default_path`, `test_review_format_json_without_lsharp_path_uses_embedded_component_default_path`, `test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_escaped_quote`, `test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_unicode_escaped_quote`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_escaped_newline`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_unicode_escaped_newline`, `cargo test -q -p lsharp-wasm --test e2e escaped_ -- --nocapture`, `cargo test -q -p lsharp-wasm --test e2e spec_params -- --nocapture`, `scripts/ci/default-path-smoke.sh`, `scripts/smoke_test_readme.sh`

<a id="lsp-01-full-sync-skeleton"></a>
### LSP-01 Full-sync skeleton

- Goal: JSON-RPC PoC を full-sync LSP server の骨格へ引き上げる。
- Current state: [JsonRpc.ls](/Users/biwakonbu/github/lsharp/selfhost/src/Tools/Lsp/JsonRpc.ls) は整数タグベースの PoC だが、bootstrap helper としては initialize/didOpen/didChange/shutdown の skeleton response を返せる。initialize result は 7 slot capability vector、shutdown は response-wrapped sentinel まで持ち、さらに initialize/shutdown については deterministic な JSON-RPC response text (`{"jsonrpc":"2.0","id":...,"result":...}`) も生成できる。一方で true JSON schema 互換ではない。
- Rust source: Rust LSP server, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `selfhost/src/Tools/Lsp/JsonRpc.ls`, `selfhost/src/Tools/Lsp/LspServer.ls`
- Implementation direction: `JsonRpc.ls` は codec のみに限定し、`LspServer.ls` を新設して session state, full document sync, request dispatch を担当させる。v1 は `TextDocumentSyncKind.Full` に固定する。
- Dependencies: `CLI-01`
- Acceptance: initialize/didOpen/didChange/shutdown の skeleton が JSON-RPC 2.0 / LSP 3.17 互換で通る。
- Evidence: `test_snapshot_lsp_initialize`, `test_snapshot_lsp_full_sync`

<a id="lsp-02-10-method-parity"></a>
### LSP-02 10 method parity

- Goal: 10 LSP method の公開挙動を Rust 版互換にする。
- Current state: 10 メソッド名、dispatch、helper 群は揃っている。hover/definition/references/rename/completion は source param がある場合、top-level `defn` に対する source-driven subset (`test_e2e_selfhost_lsp_real_shapes_*`) まで動き、hover は `defn <name>` / `symbol <name>` の text contents、completion は `[label, kind, insertText]` の text item、rename は単一 URI の `WorkspaceEdit` 風 `[uri, edits]` を返せる。`initialize` capability vector も sync/hover/completion/definition に加えて references/rename/formatting まで宣言でき、initialized/shutdown flag も getter で観測できる。formatting も `parse-program` → `format-program` を使う canonical full-document edit まで前進し、shared-state `server-loop-step` / `server-loop-sequence` により multi-request sequence を同一 state で dispatch できる。同一 URI への repeated `didOpen` でも `server-state-doc-count` が増殖しないよう state update を整理した。さらに `App/Cli.ls` transport では `didOpen` / `didChange` 後に parse/type/lint diagnostics が変化した場合だけ `publishDiagnostics` frame を後置する narrow slice を追加し、parse error open と `error -> clear` change に加え、type/lint error open と `error -> clear` change も representative actual `lsp --stdio` snapshot で固定した。stdio body parser も didOpen の `textDocument.text` と didChange の `contentChanges[0].text` を legacy `source` と同様に読めるようになり、parse/type/lint diagnostics refresh でも spec 寄り params が actual stdio snapshot に収束する。さらに JSON string decode も escape-aware にしたため、spec 寄り `textDocument.text` / `contentChanges[0].text` に含まれる escaped quote / newline と unicode escaped quote / newline (`\u0022` / `\u000a`) が actual `lsp --stdio` formatting / completion でも最新 source として扱われる。shared nav parser では hover / definition / references / completion の `position.character` を `col` に写像できるため、`textDocument.uri` + `position.character` の open-document hover / definition / references / completion も actual stdio で既存 response に収束する。hover / definition / references / completion では didChange 後 / same-URI repeated didOpen 後の latest-source fallback も actual stdio と changed/latest-reopened snapshot に収束する。専用 rename parser も `textDocument.uri` + `position.character` + `newName` を legacy `[uri,line,col,source,newName]` と同様に扱えるため、spec 寄り open-document rename も既存 response と snapshot に収束し、rename でも didChange 後 / same-URI repeated didOpen 後の latest-source workspace edit を actual stdio と changed/latest-reopened snapshot で固定した。bootstrap `JsonRpc.ls` 側では initialize/shutdown response の deterministic text も生成できる。さらに shutdown 済み state に対する後続 request は `App/Cli.ls` transport が `Invalid Request (-32600)` frame で拒否し、transport helper / actual `lsp --stdio` snapshot (`shutdown-request-after-error.json`) の両方で固定した。open document params が `path` を持つ場合は filesystem-backed direct/nested import も hover / completion / definition / references / rename まで actual/stateful parity で解決でき、未 open import target の definition / references / rename では path-hash virtual URI を返せる。representative actual `lsp --stdio` filesystem snapshots (`hover` / `completion` / `definition` / `references` / `rename` / `filesystem-document-sequence`) も追加済みで、spec 寄り request shape でも同じ `filesystem-document-sequence` snapshot に収束する actual regression を固定した。`cargo test -q -p lsharp-wasm --test e2e filesystem_document_sequence_ -- --nocapture` も 2 passed となり、representative long-lived filesystem-backed sequence の broad gate を確認した。stateful parity suite でも didChange 後の completion / hover / definition / references / rename と same-URI repeated `didOpen` 後の latest-source completion を固定し、`cargo test -q -p lsharp-wasm --test lsp_stateful_parity -- --nocapture` は 42 tests green。さらに actual `lsp --stdio` repeated-sequence soak (`test_e2e_gc_lsp_actual_stdio_repeated_sequence_soak`) まで通過し、公開 10 method parity gate を閉じる representative transport/session evidence が揃った。
- Rust source: Rust LSP 実装, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `selfhost/src/Tools/Lsp/LspServer.ls`, `selfhost/src/Tools/Text/Formatter.ls`, `selfhost/src/Tools/Text/Linter.ls`
- Implementation direction: method 実装順は `initialize`, `shutdown`, `didOpen`, `didChange`, `hover`, `goto_definition`, `references`, `rename`, `formatting`, `completion` に固定する。レスポンス shape は JSON snapshot を正本にする。
- Dependencies: `LSP-01`, `FMT-01`, `LINT-01`, `TYPE-07`
- Acceptance: 10 method が JSON schema 互換レスポンスを返し、VSCode extension から spawn できる。
- Evidence: `test_e2e_selfhost_lsp_10_methods`, `test_e2e_selfhost_lsp_real_shapes_hover_uses_source_symbol`, `test_e2e_selfhost_lsp_real_shapes_definition_and_references_use_source`, `test_e2e_selfhost_lsp_real_shapes_rename_returns_workspace_edit`, `test_e2e_selfhost_lsp_real_shapes_completion_uses_prefix_and_symbols`, `test_e2e_selfhost_lsp_real_shapes_formatting_returns_document_edit`, `test_e2e_selfhost_cli_lsp_transport_request_after_shutdown_error`, `test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_type_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_lint_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_type_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_lint_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_hover_spec_position_character_params`, `test_e2e_selfhost_cli_lsp_stdio_body_rename_spec_position_character_params`, `test_e2e_selfhost_cli_main_with_lsp_stdio_request_after_shutdown_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_type_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_lint_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_type_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_lint_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_open_document_spec_params`, `test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_changed_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_latest_reopened_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_open_document_spec_params`, `test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_changed_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_latest_reopened_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_open_document_spec_params`, `test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_changed_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_latest_reopened_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_open_document_spec_params`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_changed_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_repeated_didopen_keeps_latest_source`, `test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_open_document_spec_params`, `test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_changed_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_latest_reopened_document`, `test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_escaped_quote`, `test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_unicode_escaped_quote`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_escaped_newline`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_unicode_escaped_newline`, `cargo test -q -p lsharp-wasm --test e2e escaped_ -- --nocapture`, `cargo test -q -p lsharp-wasm --test e2e spec_params -- --nocapture`, `test_e2e_selfhost_cli_main_with_lsp_stdio_filesystem_document_sequence_spec_style_snapshot`, `cargo test -q -p lsharp-wasm --test e2e filesystem_document_sequence_ -- --nocapture`, `test_e2e_lsp_stateful_hover_resolves_filesystem_import_from_document_path`, `test_e2e_lsp_actual_stdio_hover_resolves_filesystem_import_from_document_path`, `test_e2e_lsp_stateful_completion_resolves_filesystem_import_from_document_path`, `test_e2e_lsp_actual_stdio_completion_resolves_filesystem_import_from_document_path`, `test_e2e_lsp_stateful_goto_definition_resolves_filesystem_import_from_document_path`, `test_e2e_lsp_actual_stdio_goto_definition_resolves_filesystem_import_from_document_path`, `test_e2e_lsp_stateful_references_find_filesystem_import_occurrences_from_document_path`, `test_e2e_lsp_actual_stdio_references_find_filesystem_import_occurrences_from_document_path`, `test_e2e_lsp_stateful_rename_returns_filesystem_import_workspace_edit_from_document_path`, `test_e2e_lsp_actual_stdio_rename_returns_filesystem_import_workspace_edit_from_document_path`, `test_e2e_lsp_stateful_completion_uses_changed_document`, `test_e2e_lsp_stateful_hover_uses_changed_document`, `test_e2e_lsp_stateful_definition_uses_changed_document`, `test_e2e_lsp_stateful_references_uses_changed_document`, `test_e2e_lsp_stateful_rename_uses_changed_document`, `test_e2e_lsp_stateful_repeated_didopen_keeps_latest_source`, `cargo test -q -p lsharp-wasm --test lsp_stateful_parity -- --nocapture`

<a id="lsp-03-diagnostic-ordering-and-json-snapshots"></a>
### LSP-03 Diagnostic ordering and JSON snapshots

- Goal: diagnostics の順序と schema を安定化する。
- Current state: `sort-diagnostics` / `dedup-diagnostics` helper と関連 E2E に加え、`render-diagnostic-json` / `render-diagnostics-json` で diagnostics を deterministic JSON text へ落とせるようになった。sort/dedup 後の固定順 JSON array まで targeted E2E で確認済みで、`tests/snapshots/lsp/diagnostics/` に representative snapshot files も配置した。さらに actual `lsp --stdio` についても initialize / initialize→shutdown / unknown method / shutdown 後 request error / didOpen→didChange / completion / hover / references / rename / open-document definition / open-document formatting / publishDiagnostics / didOpen parse-error→didChange fix の parse diagnostics refresh / didOpen→didChange type diagnostics refresh / didOpen→didChange lint diagnostics refresh / didChange 後 hover / definition / references / completion / rename fallback / same-URI repeated didOpen 後 hover / definition / references / completion / rename fallback の framed response body を schema snapshot として固定し、Content-Length transport を通した valid JSON 応答を確認する slice を追加した。didOpen/didChange の body parser は legacy `source` だけでなく spec 寄り `textDocument.text` / `contentChanges[0].text` でも parse/type/lint diagnostics refresh snapshot を返せる。representative actual stdio snapshots と changed/latest-reopened/document-sequence 系 snapshot 群で ordering/schema gate を固定し、公開 diagnostics parity gate を閉じる証跡が揃った。
- Rust source: Rust LSP diagnostics path
- L# target: `selfhost/src/Tools/Lsp/LspServer.ls`, `tests/snapshots/lsp/`
- Implementation direction: diagnostics は `source(parse/type/lint)`, `severity`, `line`, `column` の順で sort し、同一 span は最も severity の高いものだけを残す。
- Dependencies: `SYNTAX-03`, `TYPE-07`, `LINT-01`
- Acceptance: 同一ファイルを再パースしても diagnostics JSON diff が空になる。
- Evidence: `selfhost/src/Tools/Lsp/LspServer.ls`, `test_e2e_selfhost_lsp_runtime_sort_diagnostics`, `test_e2e_selfhost_lsp_diagnostic_dedup`, `test_e2e_selfhost_lsp_render_diagnostic_json`, `test_e2e_selfhost_lsp_render_sorted_deduped_diagnostics_json`, `test_e2e_selfhost_lsp_render_diagnostic_json_snapshot`, `test_e2e_selfhost_lsp_render_sorted_deduped_diagnostics_json_snapshot`, `test_e2e_selfhost_cli_lsp_transport_request_after_shutdown_error`, `test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_type_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_lint_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_type_diagnostics_refresh`, `test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_lint_diagnostics_refresh`, `test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_shutdown_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_unknown_method_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_request_after_shutdown_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_type_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_lint_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_type_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_lint_diagnostics_refresh_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_changed_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_completion_latest_reopened_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_hover_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_hover_changed_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_hover_latest_reopened_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_references_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_references_changed_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_references_latest_reopened_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_rename_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_rename_changed_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_rename_latest_reopened_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_definition_open_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_definition_changed_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_definition_latest_reopened_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_open_document_schema_snapshot`, `test_e2e_selfhost_cli_main_with_lsp_stdio_publish_diagnostics_schema_snapshot`, `tests/snapshots/lsp/diagnostics/*.json`, `tests/snapshots/lsp/stdio/*.json`, `tests/snapshots/lsp/stdio/document-sequence-diagnostics-refresh.json`, `tests/snapshots/lsp/stdio/document-sequence-type-diagnostics-refresh.json`, `tests/snapshots/lsp/stdio/document-sequence-lint-diagnostics-refresh.json`, `tests/snapshots/lsp/stdio/completion-changed-document.json`, `tests/snapshots/lsp/stdio/completion-latest-reopened.json`, `tests/snapshots/lsp/stdio/definition-changed-document.json`, `tests/snapshots/lsp/stdio/hover-latest-reopened.json`, `tests/snapshots/lsp/stdio/references-changed-document.json`, `tests/snapshots/lsp/stdio/references-latest-reopened.json`, `tests/snapshots/lsp/stdio/rename-changed-document.json`, `tests/snapshots/lsp/stdio/rename-latest-reopened.json`, `tests/snapshots/lsp/stdio/shutdown-request-after-error.json`

<a id="fmt-01-formatter-roundtrip"></a>
### FMT-01 Formatter roundtrip

- Goal: formatter を AST 全体対応 + roundtrip/idempotency gate へ引き上げる。
- Current state: [Formatter.ls](/Users/biwakonbu/github/lsharp/selfhost/src/Tools/Text/Formatter.ls) は roundtrip/idempotency test に加え、`format-program` 自体が `defn/int/bool/unit/var/apply/if/let/fn/do/match/recordlit/fieldaccess/recordupdate/computation` を canonical な実テキストへ整形し、decl 側でも `defn/module/impl/computation-builder` を deterministic text へ整形できる。`module body` / `impl` / `computation-builder` も parser→formatter 経路で canonical text を維持できる。未対応ノードは fallback へ退避できる。CLI `run-fmt-source` / `run-fmt` はこの canonical text を stdout へ返し、LSP formatting も `parse-program` → `format-program` 経由で `TextEdit.newText` に canonical text を積める。さらに source-aware string literal の representative CLI stdout を text snapshot (`tests/snapshots/cli/fmt-source-string-literal.txt`) へ固定した。今回、parser が trailing vector として保持する defn metadata (`:params` / `:returns` / `:doc` / `:example`) も source-aware formatting で canonical 順に再構成できるようになり、CLI fmt / LSP formatting の両経路で同じ canonical text へ収束する回帰を追加した。representative CLI snapshot と defn metadata formatting regression も CLI/LSP 双方で固定でき、public formatter parity gate を閉じる証跡が揃った。
- Rust source: Rust formatter path, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `selfhost/src/Tools/Text/Formatter.ls`
- Implementation direction: formatter の public API は `format-program ast -> text` に固定し、parser と対で `parse(format(parse(src))) == parse(src)` を gate にする。短形式/長形式/let 整列ルールは spec に従う。
- Dependencies: `SYNTAX-02`, `SYNTAX-06`
- Acceptance: AST 全 node coverage, roundtrip, idempotency が CI gate に入る。
- Evidence: `test_e2e_selfhost_formatter_roundtrip_v2`, `test_e2e_selfhost_formatter_format_expr_lit_int`, `test_e2e_selfhost_formatter_format_expr_apply`, `test_e2e_selfhost_formatter_format_expr_let`, `test_e2e_selfhost_formatter_format_expr_if`, `test_e2e_selfhost_formatter_format_expr_lambda`, `test_e2e_selfhost_formatter_format_expr_do`, `test_e2e_selfhost_formatter_format_expr_match`, `test_e2e_selfhost_formatter_format_expr_recordlit`, `test_e2e_selfhost_formatter_format_expr_fieldaccess`, `test_e2e_selfhost_formatter_format_expr_recordupdate`, `test_e2e_selfhost_formatter_format_expr_computation`, `test_e2e_selfhost_formatter_format_decl_defn`, `test_e2e_selfhost_formatter_format_decl_module_with_body`, `test_e2e_selfhost_formatter_format_decl_impl`, `test_e2e_selfhost_formatter_format_decl_computation_builder`, `test_e2e_selfhost_formatter_format_program_recordupdate_expr`, `test_e2e_selfhost_formatter_format_program_computation_expr`, `test_e2e_selfhost_formatter_format_program_module_decl`, `test_e2e_selfhost_formatter_format_program_impl_decl`, `test_e2e_selfhost_formatter_format_program_computation_builder_decl`, `test_e2e_selfhost_formatter_format_program_with_source_defn_metadata`, `test_e2e_selfhost_cli_fmt_source_defn_metadata`, `test_e2e_selfhost_lsp_real_shapes_formatting_preserves_defn_metadata`, `test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_preserves_defn_metadata`, `test_e2e_selfhost_cli_fmt_source_string_literal_snapshot`, `tests/snapshots/cli/fmt-source-string-literal.txt`

<a id="lint-01-rule-ids-and-clilsp-parity"></a>
### LINT-01 Rule IDs and CLI/LSP parity

- Goal: linter を stable rule ID と CLI/LSP 共通 diagnostics に揃える。
- Current state: [Linter.ls](/Users/biwakonbu/github/lsharp/selfhost/src/Tools/Text/Linter.ls) は簡易 rule 実装で AST coverage が狭い。
- Rust source: Rust lint path, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `selfhost/src/Tools/Text/Linter.ls`
- Implementation direction: builtin rule は `L0001` 形式の ID へ固定し、出力を `[rule-id severity span message]` に正規化する。CLI と LSP は同じ core API を使う。
- Dependencies: `SYNTAX-02`, `LSP-03`
- Acceptance: same source に対する CLI lint と LSP diagnostics の rule/severity/span が一致する。
- Evidence: `test_snapshot_lint_*`, `test_e2e_selfhost_linter`, `test_e2e_selfhost_linter_lsp_integration`

<a id="doc-01-schemas-and-snapshots"></a>
### DOC-01 Schemas and snapshots

- Goal: knowledge/review/doc output の schema を固定する。
- Current state: `docs/schemas/` と `selfhost/src/Tools/Doc/DocTools.ls` の helper 群、deterministic/no-timestamp test は追加済み。DocTools は function/type entry の決定的 sort に加えて hash から再構成した name text を保持した `[title, body, functions, types]` / `[module-id, functions, types]` slice を返し、body summary も `functions:N,types:M,first-fn:...,first-type:...` まで返せる。module decl がある場合、`generate` / `generate-doc-output` の title も `module-<name>` へ寄せられる。`generate-knowledge` は function entry に `params` (`name:type`) / `returns` / `doc` / `example` を含む richer metadata を返せるようになり、`knowledge.schema.json` は `name` / `arity` に加えて `params` / `returns` を必須化しつつ `doc` / `example` string property も定義した。`selfhost/src/Syntax/Parser.ls` も defn trailing metadata として `:doc` / `:example` / `:params` / `:returns` を保持できる。typed `generate-doc-output` の function entry も `params[{name,type,doc}]` / `returns{type,doc}` / `doc` / `example` を含み、`html.sections` は `[id, count]` metadata vector を返せるようになり、`doc-output.schema.json` は function/type entry array と structured section metadata に加えて function `params` / `returns` object property を定義する。`generate-review` も unused-let / empty-do に対する deterministic diagnostics vector `[rule-id, title, body, severity, line, column, code]` を返せ、`review.schema.json` も `title` / `severity` / `message` / `line` / `column` / `code` を必須化した。CLI `run-doc-source` / `run-doc` はこの name-aware な title/body を、CLI `run-doc-ack` / `run-doc-check` は status line 付き title/body を、CLI `run-review-source` / `run-review` は count/title/body に加えて severity と `code@line:column`、さらに first diagnostic body を stdout へ返せる。さらに representative CLI doc title/body output を text snapshot (`tests/snapshots/cli/doc-source-basic.txt`) へ固定した。HtmlDoc/HtmlTemplate/HtmlLayout はその typed entry を使う実 HTML section/layout を返せ、module page では function signature / params / returns / doc / example を HTML escape して描画できるだけでなく、parameter/return doc も host truth 寄り HTML として描画できる。さらに representative doc-site HTML snapshot として `render-guide-page.html` / `render-doc-site-index.html` も追加し、guide/doc-site index surface の regression gate を `doctools_parity.rs` へ取り込んだ。新規 `DocJson.ls` も knowledge/review/doc-output vector payload を schema-object JSON text に変換でき、selfhost CLI `review --json` / `review --format json` / `doc --json` / `doc --format json` は source helper / actual `main` argv の representative snapshot まで固定し、invalid な `review/doc --format` value も actual `main` で明示的に拒否できる。加えて host-backed `doc` distribution ownership として `scripts/ci/release-smoke.sh` が packaged binary の `doc` / `doc --json` を、`scripts/smoke_test_readme.sh` が inline Quick Start fixture に対する `doc` smoke を行うようになり、release/readme 導線の verify も追加した。さらに driver 側の doc-site generation focused gate (`test_cmd_doc_site_generates_guides_and_api_site`) も green となり、public docs schema/HTML/distribution gate を閉じる representative evidence が揃った。
- Rust source: Rust docs/review path, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `docs/schemas/knowledge.schema.json`, `docs/schemas/review.schema.json`, `docs/schemas/doc-output.schema.json`, `selfhost/src/Tools/Doc/DocTools.ls`, `selfhost/src/Tools/Doc/HtmlDoc.ls`, `selfhost/src/Tools/Doc/HtmlTemplate.ls`, `selfhost/src/Tools/Doc/HtmlLayout.ls`
- Implementation direction: docs 系は structured JSON schema を先に固定し、CLI/LSP/extension はその schema だけを見る。snapshot test は JSON canonicalization 後に比較する。
- Dependencies: `META-01`
- Acceptance: 3 schema が配布物に同梱され、knowledge/review/doc 出力が snapshot gate を持つ。
- Evidence: `docs/schemas/*.json`, `test_e2e_knowledge_schema_json_contract`, `test_e2e_review_schema_json_contract`, `test_e2e_doc_output_schema_json_contract`, `test_e2e_selfhost_doc_deterministic_html`, `test_e2e_selfhost_doctools_extract_public_functions_runtime`, `test_e2e_selfhost_doctools_extract_type_definitions_runtime`, `test_e2e_selfhost_doctools_extract_module_public_functions_runtime`, `test_e2e_selfhost_doctools_extract_module_type_definitions_runtime`, `test_e2e_selfhost_doctools_generate_structured_doc_payload`, `test_e2e_selfhost_doctools_module_title_uses_name`, `test_e2e_selfhost_doctools_schema_knowledge`, `test_e2e_selfhost_doctools_schema_doc_output`, `test_e2e_selfhost_doctools_schema_doc_output_module_title_name`, `test_e2e_selfhost_doctools_schema_doc_output_function_metadata`, `test_e2e_selfhost_doctools_generate_html_basic`, `test_e2e_selfhost_doctools_generate_html_idempotent`, `test_e2e_selfhost_doctools_schema_review`, `test_e2e_selfhost_doctools_schema_review_empty_do`, `test_e2e_selfhost_htmldoc_render_function_signature`, `test_e2e_selfhost_htmldoc_render_type_definition`, `test_e2e_selfhost_htmldoc_render_module_page_structure`, `test_e2e_selfhost_htmldoc_render_module_page_function_metadata`, `test_e2e_htmldoc_render_guide_page_snapshot`, `test_e2e_htmldoc_render_doc_site_index_snapshot`, `test_e2e_selfhost_cli_doc_source_snapshot`, `test_e2e_selfhost_cli_review_source_json_snapshot`, `test_e2e_selfhost_cli_doc_source_json_snapshot`, `test_e2e_selfhost_cli_main_with_args_review_json_file`, `test_e2e_selfhost_cli_main_with_args_doc_json_file`, `tests/snapshots/cli/doc-source-basic.txt`, `tests/snapshots/doctools/*`, `cargo test -p lsharp-wasm --test doctools_parity -- --nocapture`, `cargo test -p lsharp-wasm --test e2e selfhost_doctools_ -- --nocapture`

<a id="doc-02-trailer-and-deterministic-html"></a>
### DOC-02 Trailer and deterministic HTML

- Goal: doc-ack/doc-check trailer と HTML doc 生成の deterministic 性を揃える。
- Current state: `DocTools.ls` は `; Doc-Review-Status:` / `; Doc-Reviewed-By:` の comment-form trailer を生成でき、`doc-check-trailer-valid?` で source 末尾 2 行の non-empty trailer comment を検証できる。`App/Cli.ls` も `doc-ack --trailer` と `doc-check --strict` の narrow CLI surface を file handler / actual `main`、および strict failure 時の non-zero process exit まで備えた。HTML 側も deterministic/no-timestamp slice は維持されているが、full HTML payload parity と richer trailer semantics は未達。
- Rust source: Rust doc tooling, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `selfhost/src/Tools/Doc/DocTools.ls`, `selfhost/src/Tools/Doc/HtmlDoc.ls`
- Implementation direction: trailer syntax は Rust 版互換の comment form に固定し、HTML は timestamp/hostname/absolute path を埋め込まない。environment-dependent metadata は opt-in flag がある時だけ出す。
- Dependencies: `DOC-01`
- Acceptance: `doc-ack`, `doc-check`, `doc` が deterministic output を返し、2 回実行 diff が空になる。
- Evidence: `test_e2e_selfhost_doctools_doc_ack_trailer_payload`, `test_e2e_selfhost_doctools_doc_check_trailer_payload`, `test_e2e_selfhost_doctools_doc_check_trailer_validation_accepts_comment_form`, `test_e2e_selfhost_doctools_doc_check_trailer_validation_rejects_missing_status`, `test_e2e_selfhost_doctools_doc_check_trailer_validation_rejects_empty_reviewer`, `test_e2e_selfhost_cli_doc_ack_file_handler_trailer_only`, `test_e2e_selfhost_cli_doc_check_file_handler_strict_success`, `test_e2e_selfhost_cli_doc_check_file_handler_strict_missing_trailer_fails`, `test_e2e_selfhost_cli_main_with_args_doc_ack_trailer_only`, `test_e2e_selfhost_cli_main_with_args_doc_check_strict_file`, `test_e2e_selfhost_cli_main_with_args_doc_check_strict_missing_trailer`, `test_e2e_selfhost_doc_deterministic_html`, `test_e2e_selfhost_doctools_generate_html_idempotent`

<a id="pkg-01-archives-checksums-and-quick-start"></a>
### PKG-01 Archives checksums and Quick Start

- Goal: host launcher + embedded guest component の single binary 配布で Quick Start が完走する配布形を固定する。
- Current state: release archive 契約は固定済みで、payload は `lsharp`, `lsharp-lsp`, `README.md`, `LICENSE`, `checksums.txt` に揃えた。`README.md` Quick Start は packaged artifact だけで `checksum verify -> compile -> test -> doc` を辿れるように更新され、`scripts/release.sh` は required payload を fail-fast で検証する。`scripts/ci/release-smoke.sh` と `scripts/smoke_test_readme.sh` も packaged archive / inline fixture の両経路で smoke を行い、`test_e2e_selfhost_pkg_archives`, `test_e2e_pkg01_readme_quick_start_uses_release_artifact_only`, `test_e2e_ops06_release_smoke_contract`, `test_e2e_ops06_release_smoke_script_runs_fixture_archive`, `test_e2e_ops07_fresh_clone_no_rust`, `test_e2e_ops07_test_fresh_clone_script_runs_binary_only_fixture_archive` が green、ローカルでも `VERSION=local-pkg-contract bash scripts/release.sh` → `bash scripts/ci/release-smoke.sh dist/lsharp-local-pkg-contract-aarch64-apple-darwin.tar.gz` → `bash scripts/ci/test-fresh-clone.sh dist/lsharp-local-pkg-contract-aarch64-apple-darwin.tar.gz` を通過した。
- Rust source: `docs/development/operations/release-distribution-signing.md`, `docs/development/planning/toolchain-parity-spec.md`
- L# target: `scripts/ci/release-smoke.sh`, `scripts/smoke_test_readme.sh`, release workflow
- Implementation direction: 配布アーカイブ内容は host launcher としての `lsharp`, `lsharp-lsp`, `README.md`, `LICENSE`, `checksums.txt` に固定する。`lsharp` host launcher は guest component を内包し、`lsharp-lsp` は editor integration 用の companion binary として同梱する。Quick Start smoke は展開後 `checksum verify -> compile -> test -> doc` を自動化する。
- Dependencies: `CLI-02`, `LSP-02`, `OPS-06`
- Acceptance: single-binary release artifact 展開だけで README Quick Start が通り、checksum 検証も自動化される。
- Evidence: `test_e2e_selfhost_pkg_archives`, `test_e2e_pkg01_readme_quick_start_uses_release_artifact_only`, `test_e2e_ops06_release_smoke_contract`, `test_e2e_ops06_release_smoke_script_runs_fixture_archive`, `test_e2e_ops07_fresh_clone_no_rust`, `test_e2e_ops07_test_fresh_clone_script_runs_binary_only_fixture_archive`, `scripts/smoke_test_readme.sh`, `scripts/ci/release-smoke.sh`, `scripts/ci/test-fresh-clone.sh`

## WS-RUNTIME Long-lived runtime stability

<a id="gc-01-m1-object-model"></a>
### GC-01 M1 object model

- Goal: collector 前提の object header / trace map / root API を導入する。
- Current state: Rust/WASI path の `root_push` / `root_pop` / `root_set` helper 自体は実装済みで、Rust lowering も `string-concat` / `substring` の allocating safe point に対する narrow な auto-root insertion を持つ。ただし共通 object header / trace map と、general call site / selfhost parity まで含む precise root set は未実装。
- Rust source: `docs/language/runtime-spec.md`, `docs/development/planning/memory-management-roadmap.md`, `docs/development/planning/runtime-stability-spec.md`
- L# target: runtime layer, builtins, `selfhost/src/**`
- Implementation direction: object header は `[tag size-or-words mark-state aux]` で固定し、既存の `root_push`, `root_pop`, `root_set` helper と narrow auto-root insertion を general call site / selfhost parity まで拡張する。trace map は string/adt/record/vector/hashmap/closure/ref-cell ごとに定義する。
- Dependencies: なし。
- Acceptance: all heap object kinds に trace 規約があり、GC 未導入でも root API を呼べる。
- Evidence: `test_unit_runtime_object_header_*`, `test_unit_runtime_root_api_*`

<a id="gc-02-m2-mark-sweep-mvp"></a>
### GC-02 M2 mark-sweep MVP

- Goal: mark-sweep collector の最小版を導入する。
- Current state: free list, mark bit, sweep loop が未実装。
- Rust source: `docs/development/planning/memory-management-roadmap.md`, `docs/development/planning/runtime-stability-spec.md`
- L# target: runtime allocator / collector
- Implementation direction: allocator は fast path と slow path に分け、slow path で `mark -> sweep -> free list reuse` を実行する。collector reentry は global state で禁止する。
- Dependencies: `GC-01`
- Acceptance: collector 有効でも既存 E2E 群が落ちず、free list reuse が確認できる。
- Evidence: `test_unit_gc_mark_sweep_*`, `test_e2e_runtime_gc_mark_sweep_*`

<a id="gc-03-m3-generational-pass"></a>
### GC-03 M3 generational pass

- Goal: nursery + write barrier + promotion policy を導入する。
- Current state: performance pass が未着手。
- Rust source: `docs/development/planning/memory-management-roadmap.md`, `docs/development/planning/runtime-stability-spec.md`
- L# target: runtime allocator / write barrier
- Implementation direction: young generation を bump allocator、old generation を non-moving mark-sweep に固定し、minor GC と full GC を分離する。promotion は survival count or size threshold の 2 条件で決める。
- Dependencies: `GC-02`
- Acceptance: generational mode が mark-sweep mode と同じ意味論を保ち、minor GC metrics を収集できる。
- Evidence: `test_unit_gc_generational_*`, `benchmark-results/*.json`

<a id="gc-04-longevity-benchmarks"></a>
### GC-04 Longevity benchmarks

- Goal: 1000x format/hover と 100x self-compile を標準 longevity benchmark に固定する。
- Current state: 長寿命 workload の標準セットがコード化されていない。
- Rust source: `docs/development/planning/runtime-stability-spec.md`
- L# target: `scripts/bench/longevity.sh`, CI benchmark harness
- Implementation direction: format 1000 回, hover 1000 回, self-compile 100 回を同一プロセスで回し、CI では 1/10 スケール版を使う。対象プロジェクトは selfhost source に固定する。
- Dependencies: `GC-02`
- Acceptance: benchmark harness が CI 用簡易モードと手元用詳細モードを両方持つ。
- Evidence: `benchmark-results/*.json`, `scripts/bench/longevity.sh`

<a id="gc-05-lsp-soak-and-repl-gc"></a>
### GC-05 LSP soak and REPL GC

- Goal: LSP と REPL の長寿命挙動を GC 前提で検証する。
- Current state: compile+run loop と REPL eval loop の soak testに加え、`test_e2e_gc_repl_stateful_single_session_metrics`, `test_e2e_gc_repl_session_batch_metrics`, `test_e2e_gc_repl_stateful_long_session_metrics`, `test_e2e_gc_lsp_stateful_session_sequence_metrics`, `test_e2e_gc_lsp_stateful_repeated_sequence_metrics` で単一 Wasm セッション上の REPL 継続評価メトリクス、REPL batch helper、200-step single-session REPL soak、`didOpen -> hover -> didChange -> completion -> formatting` の stateful / repeated sequence は固定済み。さらに `test_e2e_selfhost_lsp_runtime_server_loop_stateful_sequence` で `server-loop-step` の shared-state dispatch を固定し、same-URI repeated `didOpen` でも doc-count が増殖しないことを確認した。今回 `test_e2e_gc_lsp_actual_stdio_repeated_sequence_soak` を追加し、actual `lsp --stdio` でも 12 回の open/hover/change/completion/formatting repeated sequence を long-lived session で固定した。あわせて `emit_read_stdin_func` は 4KiB chunk を EOF まで連結する実装に更新し、soak-sized stdio wire を最後まで読めるようにした。さらに `test_e2e_alloc_metrics_ci_artifact_payload` は `proxy_workloads` に light compile+run / REPL 50 eval / stateful long-session REPL / stateful single-session REPL / actual `lsp --stdio` repeated sequence の representative result を格納するようになり、`gc-metrics-artifact` required job から機械可読な proxy workload 証跡を回収できる。加えて `crates/lsharp-wasm/src/wasi.rs` は `_start` return / `proc_exit(0)` 成功終了経路で actual `__lsharp_gc_collect` を自動実行するため、`test_e2e_gc_repl_stateful_long_session_in_session_collector_telemetry` と `test_e2e_gc_lsp_actual_stdio_repeated_sequence_in_session_collector_telemetry` で session-internal collector telemetry (`gc_collection_count > 0` / `gc_freed_count > 0` / `root_stack_top = 0`) まで固定した。今回さらに runtime 側でも `test_e2e_runtime_collector_preserves_string_reachable_through_rooted_closure_capture` と `test_e2e_runtime_collector_ignores_legacy_zero_root_slot_sentinel` により closure capture transitive trace と root-stack `0` sentinel ignore path を固定し、selfhost compiler の source-aware `string-concat` / `substring` / `ref-new` / `vector-push` / `map-insert` / `map-get` / direct user call も local spill + `root_push` / `root_pop` へ広げ、`test_e2e_selfhost_compiler_string_concat_*` / `test_e2e_selfhost_compiler_substring_*` / `test_e2e_selfhost_compiler_ref_new_*` / `test_e2e_selfhost_compiler_vector_push_*` / `test_e2e_selfhost_compiler_map_*` / `test_e2e_selfhost_compiler_user_call_*` で IR parity を固定した。**ただし** S14-S16 を閉じるには selfhost 側の broader closure/indirect heap 値判定と compiler-side GC-safe point spill / shadow stack 完全列挙がなお必要。
- Rust source: `docs/development/planning/runtime-stability-spec.md`
- L# target: LSP server, REPL implementation, test harness
- Implementation direction: LSP は `open -> edit -> diagnostics -> hover -> completion` を 1000 サイクル、REPL は single-session 500 eval を固定 workload にする。両方とも同じ root API を使用する。
- Dependencies: `LSP-02`, `GC-02`
- Acceptance: GC 有効で soak test を完走し、crash/trap/unreachable が出ない。
- Evidence: `test_runtime_lsp_soak_*`, `test_runtime_repl_gc_*`

<a id="gc-06-leak-detection-and-metrics"></a>
### GC-06 Leak detection and metrics

- Goal: leak suspect 検知と metrics 出力を CI gate にする。
- Current state: metrics API / leak suspect test / CI gate spec 文書に加えて、`test_e2e_alloc_metrics_ci_artifact_payload` と `scripts/ci/collect-gc-metrics.sh` により collector-backed `ci-artifacts/gc-metrics/{sha}/summary.json` を生成し、required CI job `gc-metrics-artifact` から `gc-metrics-{sha}` artifact と `collector-proof.json` sidecar を保存できる。payload schema は required 21 keys に加えて actual `gc_collection_count` / `gc_freed_count` / `gc_free_list_count` / `gc_live_alloc_count` を持ち、`heap_bytes_series` と `proxy_workloads` を通じて S14 monotonic-trend evaluator を Rust test と Python validator の両方で機械化する。`s15_proof` は `stage2` / `stage3` fixed-point compare、`s16_proof` は required workload 完走と crash-free counters を保持し、validate-only fixture path (`LSHARP_GC_METRICS_INPUT=/path/to/summary.json bash scripts/ci/collect-gc-metrics.sh`) と `gc_metrics_contract.rs` が bump / blocked / proof-overlay 契約も回帰検知する。collector-backed required path は S14-S16 を機械判定できる状態まで到達した。
- Rust source: `docs/development/planning/runtime-stability-spec.md`
- L# target: runtime metrics collector, CI jobs
- Implementation direction: CI では `peak RSS` と `full GC count` だけを fail threshold に使い、手元実行では live object count と pause histogram まで出す。 leak suspect は tag ごと単調増加を検出して stderr 出力する。
- Dependencies: `GC-03`, `GC-04`, `GC-05`
- Acceptance: runtime metrics が CI artifact 化され、S14-S16 を機械判定できる。
- Evidence: `scripts/ci/collect-gc-metrics.sh`, `.github/workflows/ci.yml`, `ci-artifacts/gc-metrics/`, `test_e2e_alloc_metrics_ci_artifact_payload`, `crates/lsharp-wasm/tests/gc_metrics_contract.rs`

## WS-OPS CI / release / removal

<a id="ops-01-ci-gate-v2-job-graph"></a>
### OPS-01 CI gate-v2 job graph

- Goal: CI の主経路を L# ベース job graph へ切り替える。
- Current state: `ci-gate` / `ci-gate-v2` に `default-path-smoke` / `test-fresh-clone` / `fresh-clone-smoke` / `gc-metrics-artifact` / `editor-extension-build` の job graph を導入済み。**Temporary policy (2026-07-12): CI 自動実行は停止**しており、`.github/workflows/ci.yml` は `workflow_dispatch` だけを許可する。したがって `CI Gate v2` は branch protection の required check にせず、通常 release は Mac + Lima VM の local manual gate と手動 GitHub Release 公開を正本とする。native/release job graph の再編と release signing は引き続き未完。
- Rust source: `docs/development/operations/ci-gate-v2-job-graph.md`, `.github/workflows/ci.yml`
- L# target: `.github/workflows/ci.yml`
- Implementation direction: job graph は `bootstrap-wasm -> bootstrap-native -> golden-parity -> release-smoke -> packaging`, `docs` 独立、`ci-gate-v2` 集約に固定する。required checks もこの名前に合わせる。
- Dependencies: `BOOT-04`, `NATIVE-06`, `PKG-01`
- Acceptance: branch protection の required check が `ci-gate-v2` に移行できる。
- Evidence: `.github/workflows/ci.yml`, GitHub branch protection, `docs/development/operations/CI.md`, `docs/development/operations/ci-gate-v2-job-graph.md`

<a id="ops-02-artifact-policy"></a>
### OPS-02 Artifact policy

- Goal: bootstrap/native/differential/release の artifact 保存規則を固定する。
- Current state: active workflow は `bootstrap-diff-${github.sha}` / `fresh-clone-archive-${github.sha}` / `gc-metrics-${github.sha}` / `ci-gate-v2-results` / `shadow-oracle-results` を CI artifact 名として使い、release workflow も workflow-local artifact 名 `lsharp-${github.ref_name}-${matrix.target}` と GitHub Release asset `lsharp-{version}-{target}.{ext}` の区別まで docs / test と同期済み。placeholder 名 (`bootstrap-stages-*`, `native-binaries-*`, `benchmark-*`) は未実装 workflow のため active contract から外した。
- Rust source: `docs/development/operations/artifact-policy.md`
- L# target: `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- Implementation direction: active workflow が emit する artifact 名だけを正本化し、workflow-local artifact 名と GitHub Release asset 名を混同しない。新しい artifact 名は workflow と回帰テストが入った時点で追加する。
- Dependencies: `OPS-01`
- Acceptance: active CI / release workflow が artifact 名と retention rule を一貫して使い、`test_e2e_ops02_artifact_policy` で drift を検知できる。
- Evidence: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `docs/development/operations/artifact-policy.md`, `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs`

<a id="ops-03-shadoworacle-lifecycle"></a>
### OPS-03 Shadow/oracle lifecycle

- Goal: legacy Rust を shadow/oracle として計画的に縮退させる。
- Current state: Rust path はまだ主経路と shadow の境界が曖昧。
- Rust source: `docs/development/operations/ci-migration-spec.md`, `docs/development/operations/legacy-isolation-spec.md`
- L# target: `.github/workflows/ci.yml`, `legacy-rust-bootstrap/`
- Implementation direction: `shadow-legacy` は PR 2 週間 -> main push 2 週間 -> 削除、`oracle-parity` は exit code mismatch 0 を条件に warning から fail へ上げた後に撤去する。
- Dependencies: `OPS-01`, `META-05`
- Acceptance: shadow と oracle の lifecycle が workflow と docs で一致する。
- Evidence: `.github/workflows/ci.yml`, `docs/development/operations/legacy-isolation-spec.md`

<a id="ops-04-legacy-isolation"></a>
### OPS-04 Legacy isolation

- Goal: Rust 実装を `legacy-rust-bootstrap/` へ段階的に隔離する。
- Current state: Rust crate はまだ main tree の主経路に存在する。
- Rust source: `docs/development/operations/legacy-isolation-spec.md`
- L# target: `legacy-rust-bootstrap/`, `Cargo.toml`, mainline tree
- Implementation direction: 削除順は `docs -> lsp -> driver -> wasm -> ir -> types -> syntax` に固定し、各 crate は parity test / golden zero diff / shadow 1 week / ADR / tag を満たしてから isolated へ移動する。
- Dependencies: `CP-04`, `CP-05`
- Acceptance: isolated crate は mainline workspace から外れ、L# 正本 path だけで CI が回る。
- Evidence: `legacy-rust-bootstrap/`, `git tag -l 'legacy-rust-*'`

<a id="ops-05-default-path-migration"></a>
### OPS-05 Default path migration

- Goal: public command の default path を **host launcher + embedded guest の hybrid 構成**へ移し、embedded 化する subset と host-backed subset を明示する。
- Current state: `default-path-smoke.sh` でビルド済み `lsharp` バイナリ経路は blocking 化され、さらに `fresh-clone-smoke` で clean checkout 由来の同経路も継続検証できる。加えて `crates/lsharp-driver/src/main.rs` の `LSHARP_PATH` は external compiler executable / 配置ディレクトリに加えて preview1 `.wasm` selfhost artifact も受け付けるようになり、`crates/lsharp-driver/tests/default_path_delegation.rs` では executable path / directory path / preview1 `.wasm` artifact / invalid path error を固定した。embedded guest default path も `parse` / `check` / `compile` / `build` / `test` / `review`（text + `--json` / `--format json`） / `doc-ack <file>`（+ `--trailer`） / `doc-check <file>`（+ `--strict`） / `fmt` まで広がり、`scripts/ci/default-path-smoke.sh` は stage1 Wasm smoke artifact に加えて metadata summary、review diagnostics summary、review schema-object JSON、doc ack/check payload summary、doc trailer/strict option subset、strict missing-trailer failure と、runtime disable 時の `parse` / `review` / `doc-ack` / `doc-check` option-aware delegation hint 復帰も relative path 上で継続検証できる。`install` / `repl` / `lsp` / `doc` と richer doc argv shape は package/fs/network/stdio server surface を持つため **intentional host-backed subset** として host launcher 側に残し、`docs/development/operations/default-path-migration.md` / `compatibility-matrix.md` もこの hybrid ownership を正本としている。
- Rust source: `docs/development/planning/compatibility-matrix.md`, `docs/development/operations/default-path-migration.md`
- L# target: `docs/development/planning/compatibility-matrix.md`, CLI/LSP entrypoints
- Implementation direction: 切替順は `compile -> check -> parse -> test -> review -> doc-ack/doc-check -> build -> fmt -> lsp -> docs` に固定し、各切替前に parity/golden/smoke を通す。切替後の Rust path は shadow へ下げる。
- Dependencies: `CLI-02`, `LSP-02`, `FMT-01`, `DOC-02`, `OPS-03`
- Acceptance: compatibility matrix / default-path-migration / smoke が、embedded guest subset と intentional host-backed subset の ownership に矛盾なく一致する。
- Evidence: `docs/development/planning/compatibility-matrix.md`, default path switch PRs

<a id="ops-06-release-playbook"></a>
### OPS-06 Release playbook

- Goal: host launcher + embedded guest component 配布の build/sign/checksum/changelog を自動化する。
- Current state: `scripts/release-playbook.sh` は release binary を作り、`compile-phase11-inputs.sh` / `default-path-smoke.sh` を再利用して smoke まで回せる。`.github/workflows/release.yml` も `v*` tag push 起点の `verify` / `build` / `release-smoke` / `release` まで接続済みで、build job は supported product/release targets (`aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`) の host launcher archive と companion sidecar `lsharp-{version}-{target}.component.wasm` を生成し、`scripts/ci/release-smoke.sh` で生成済み archive の required payload (`README.md` / `LICENSE` / `checksums.txt` / `lsharp-lsp` / `lsharp.component.wasm`) 検証、checksum 検証、packaged binary smoke を実行する。`release` job は build artifact download 後に `bash scripts/checksum.sh dist > dist/checksums.txt` を実行し、archive 群と sidecar component、attached release-level checksum asset を GitHub Release に添付する。`release-smoke` job は Ubuntu 上で実行可能な Linux x86_64 archive を Rust toolchain 無しで再検証する current middle gate として運用する。さらに host-backed `doc` ownership として `release-smoke.sh` は packaged `lsharp` の `doc` / `doc --json` と `lsharp-lsp --version` を通し、`scripts/smoke_test_readme.sh` も inline Quick Start fixture に対する checksum / compile / test / doc smoke を README/fresh-clone gate に含めるようになった。Mac Apple Silicon notarization は secret-gated workflow hook まで接続し、credential 未設定時は skip する。Windows Authenticode は archived / out of support scope とする。**ただし** 実際の署名完了は credential 未投入のため blocked。
- Rust source: `docs/development/operations/release-playbook.md`, `docs/development/operations/release-distribution-signing.md`
- L# target: release workflow, `scripts/release-playbook.sh`, `scripts/release.sh`, `scripts/checksum.sh`, `scripts/ci/release-smoke.sh`
- Implementation direction: stable/nightly の 2 チャネルを固定し、release は `version bump -> CI -> host-launcher artifact -> checksum -> signing -> smoke -> tag -> GitHub Release` の順に自動化する。
- Dependencies: `PKG-01`, `OPS-01`
- Acceptance: tag push だけで tier1 host launcher release artifact, companion sidecar `lsharp-{version}-{target}.component.wasm`, `dist/checksums.txt`, notes が生成される。
- Evidence: release workflow, GitHub Releases, `dist/checksums.txt`, `lsharp-{version}-{target}.component.wasm`

<a id="ops-07-fresh-clone-without-rust"></a>
### OPS-07 Fresh clone without Rust

- Goal: Rust 未導入の利用者環境から host launcher single binary release smoke を再現する。
- Current state: `scripts/ci/test-fresh-clone.sh` + `fresh-clone-smoke` により clean checkout 相当コピーからの再ビルド / smoke は blocking 化された。さらに mainline CI では `fresh-clone-artifact` → `test-fresh-clone` を追加し、同一 workflow 内で作った release-style archive を download して、Rust toolchain 無しで `release-smoke.sh` / `default-path-smoke.sh` / `smoke_test_readme.sh` を通す **現行の closest viable binary-only gate** まで接続済み。加えて `scripts/fetch-stage0.sh` / `scripts/bootstrap.sh` / `scripts/release-bundle.sh` により、手元では release asset 取得 → stage1/stage2 component compare → release-style rebundle まで試せる scaffold を追加した。`bootstrap.sh` は current release contract に合わせて `selfhost/src/App/EmbeddedCli.ls` を stage1/stage2 `lsharp.component.wasm` へ再生成し、launcher は隣接 `lsharp.component.wasm` sidecar を優先する host runner をそのまま持ち回る。**ただし** GitHub Releases 上の stage0 package を fresh clone 直後に取得する true no-Rust end-state を required gate として mainline に接続したわけではない。
- Rust source: `docs/development/operations/fresh-clone-spec.md`
- L# target: `scripts/ci/test-fresh-clone.sh`, `scripts/ci/release-smoke.sh`, `scripts/smoke_test_readme.sh`
- Implementation direction: 現行の `test-fresh-clone` は workflow-local artifact を使う binary-only gate とし、手元検証は `fetch-stage0.sh` / `bootstrap.sh` / `release-bundle.sh` で先行して詰める。GitHub Releases / stage0 fetch を required gate へ昇格するのは、release credential / branch protection / retention を含む repo 外運用が揃ってから行う。
- Dependencies: `OPS-06`, `PKG-01`
- Acceptance: fresh clone / release smoke CI が main merge ごとに green になる。
- Evidence: `fresh-clone-artifact` job, `test-fresh-clone` job, `release-smoke` job, `docs/development/operations/fresh-clone-spec.md`

<a id="ops-08-final-removal-and-rollback"></a>
### OPS-08 Host launcher cutover and rollback

- Goal: host launcher + guest component 構成の最終切替と rollback 手順を decision-complete にする。
- Current state: rollback docs / release docs / playbook / script は host launcher + guest component 基準へ同期済み。GitHub Release notes の `Rollback anchor` を last-known-good release tag / host launcher asset / guest component sidecar asset (`lsharp-{version}-{target}.component.wasm`) / checksum の正本として運用する契約まで固定した。
- Rust source: `docs/development/operations/adr-rust-removal.md`, `docs/development/planning/completion-criteria.md`
- L# target: root tree, `docs/adr/`, release docs
- Implementation direction: Rust workspace は host launcher として残存させ、rollback は GitHub Release 上の `Rollback anchor` から同一 tag の host launcher / guest component sidecar asset set を復元する運用に統一する。
- Dependencies: `OPS-04`, `OPS-05`, `OPS-07`
- Acceptance: Rust 物理撤去を前提にせず、host launcher / guest component の rollback 手順が release docs と ADR で追える。
- Evidence: `docs/development/operations/adr-rust-removal.md`, `docs/development/operations/rollback-procedure.md`, `docs/development/operations/release-distribution-signing.md`, `docs/development/operations/release-playbook.md`, `scripts/rollback.sh`, `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops08_final_removal_rollback`, `test_e2e_ops08_rollback_lkg_contract`)

## Gate 外 / v2

<a id="v2-01-lsp-incremental-sync"></a>
### V2-01 LSP incremental sync

- Goal: full sync 固定の後段最適化として incremental sync を追加する。
- Current state: v1 は full sync を採用。
- Rust source: LSP 3.17 incremental sync semantics
- L# target: `selfhost/src/Tools/Lsp/LspServer.ls`
- Implementation direction: v1 完了後に text edit diff apply layer を追加し、diagnostic ordering contract は維持する。
- Dependencies: `LSP-01`, `LSP-02`
- Acceptance: incremental sync を有効にしても JSON snapshot と diagnostics order が壊れない。
- Evidence: `test_snapshot_lsp_incremental_*`

<a id="v2-02-formatterlinter-custom-rule-api"></a>
### V2-02 Formatter/linter custom rule API

- Goal: builtin rule 完成後に custom rule API を公開する。
- Current state: v1 では custom rule は scope 外。
- Rust source: `docs/development/planning/toolchain-parity-spec.md`
- L# target: formatter/linter plugin API
- Implementation direction: AST walker と lint context を public API 化し、config loader から external rule を注入できるようにする。
- Dependencies: `FMT-01`, `LINT-01`
- Acceptance: custom rule を有効化しても builtin rule ordering が変わらない。
- Evidence: RFC doc, `test_unit_lint_plugin_*`

<a id="v2-03-package-manager-distribution"></a>
### V2-03 Package manager distribution

- Goal: Homebrew/apt/scoop 等の package manager 配布を整備する。
- Current state: v1 は公式アーカイブのみ。
- Rust source: `docs/development/planning/toolchain-parity-spec.md`, `docs/development/operations/release-distribution-signing.md`
- L# target: formula/manifests
- Implementation direction: package manager manifest は release artifact を正本とし、checksum と署名検証を流用する。
- Dependencies: `PKG-01`, `OPS-06`
- Acceptance: 公式アーカイブと package manager の version/checksum が一致する。
- Evidence: formula/manifests, release notes

<a id="v2-04-linux-aarch64-tier2-distribution"></a>
### V2-04 Linux aarch64 archived design

- Goal: Linux aarch64 を将来再導入する場合の archived design を保持する。
- Current state: out of support scope。
- Rust source: `docs/development/operations/release-distribution-signing.md`
- L# target: release workflow
- Implementation direction: support scope を変更した上で cross build descriptor と smoke test を追加し、artifact 名と checksum 規則を supported target と同期する。
- Dependencies: `PKG-01`
- Acceptance: support scope を再変更した場合のみ、linux-aarch64 artifact が nightly/stable の両方で生成される。
- Evidence: release workflow, artifacts

<a id="v2-05-windows-authenticode-signing"></a>
### V2-05 Windows Authenticode signing

- Goal: Windows 配布物へ Authenticode 署名を導入する。
- Current state: v1 は Windows 署名なし。
- Rust source: `docs/development/operations/release-distribution-signing.md`
- L# target: Windows release workflow
- Implementation direction: signing secret と verify step を release pipeline へ追加し、zip 内 `.exe` に署名済み stamp を残す。
- Dependencies: `PKG-01`, `OPS-06`
- Acceptance: Windows artifact に verify step が追加され、unsigned build を配布しない。
- Evidence: Windows release job, signing logs

<a id="v2-06-region-optimization"></a>
### V2-06 Region optimization

- Goal: GC の補助最適化として region allocator を導入する。
- Current state: precise tracing GC 優先。
- Rust source: `docs/development/planning/memory-management-roadmap.md`
- L# target: runtime allocator
- Implementation direction: compiler 内短命 scratch object と builtins の一時 buffer だけを region 対象にし、user-visible heap は GC 正本を維持する。
- Dependencies: `GC-03`
- Acceptance: region 無効でも意味論が変わらず、region 有効時に benchmark 改善を確認できる。
- Evidence: `benchmark-results/*.json`, `test_runtime_region_*`

<a id="v2-07-wasmgc-optional-backend"></a>
### V2-07 WasmGC optional backend

- Goal: optional backend として WasmGC を試験導入する。
- Current state: linear memory collector が mainline。
- Rust source: `docs/development/planning/memory-management-roadmap.md`
- L# target: optional WasmGC backend
- Implementation direction: AST/type/IR は共有し、codegen/runtime ABI だけを backend ごとに分離する。records/ADT/strings から先に移植する。
- Dependencies: `GC-03`, `WASM-06`
- Acceptance: linear memory backend と同じ入力集合で differential benchmark を取れる。
- Evidence: `benchmark-results/wasmgc-*.json`, `test_golden_wasmgc_*`

<a id="v2-08-native-backend-self-regeneration"></a>
### V2-08 Native backend self-regeneration

- Goal: Deferred research track として native backend の true self-regeneration を成立させる。
- Current state: `test_e2e_stage23_actual_native_self_regeneration_harness_stage2_stage3_match` により representative `stage1-native -> stage2-native -> stage3-native` の actual artifact observation / transport payload 一致を固定済みで、Darwin arm64 の representative actual self-regeneration は完了。Linux x86_64 の `V2-13a-5` は完了し、`stage23-map-insert-staged-merge-full-compare-v1` の stage2/stage3 stdout byte-for-byte 一致、同一 SHA-256、stderr 0 を証跡とする。Linux x86_64 の `V2-15` は完了し、実 `App.Cli` native release input、rollback compatibility archive、Mac/Lima local official smoke を確認した。supported product/release targets は `aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` で、Intel Mac / Windows / Linux ARM は release blocker に含めない。
- Rust source: `docs/language/native-backend-spec.md`, `docs/development/planning/completion-criteria.md`
- L# target: `selfhost/src/Backend/Native/NativeTarget.ls`, `selfhost/src/Backend/Native/NativeCodegen.ls`, `selfhost/src/Backend/Native/NativeEmit.ls`, `scripts/ci/build-native.sh`
- Implementation direction: representative build entry を `selfhost/src/App/Main.ls` compile に固定し、`actual-stage23-gap.json` の先頭 families (`Call` / i32 dataflow / memory / control flow) を順に潰しながら tier1 target ごとに `stage1-native -> stage2-native -> stage3-native` を走らせて exit/stdout/stderr/artifact hash の観測値を比較する。
- Dependencies: `BOOT-04`, `NATIVE-04`
- Acceptance: representative input set に対して stage2-native と stage3-native の観測値が一致する。
- Evidence: `crates/lsharp-wasm/tests/e2e/selfhost_native_differential.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`, `docs/development/planning/v2-designs/v2-08-native-backend-self-regeneration.md`

<a id="v2-09-wasm-native-differential-zero"></a>
### V2-09 Wasm/native differential zero

- Goal: Deferred research track として Wasm と native の観測差分を tier1 でゼロにする。
- Current state: `tests/differential-allowlist.yaml` は空を維持し、`test_e2e_wasm_native_differential_five_observation_points`、`test_e2e_differential_allowlist_empty`、`test_e2e_wasm_native_differential_structural_parity`、`test_e2e_zero_diff_const_{0,1,42,100}`、`test_e2e_zero_diff_sample_summary` により differential harness と narrow zero-diff sample は保持されている。**ただし** tier1 native artifact 全体で exit/stdout/stderr/generated bytes/diagnostics JSON のゼロ差分は未証明。
- Rust source: `docs/development/planning/completion-criteria.md`, `docs/development/validation/verification-spec.md`
- L# target: `tests/differential-allowlist.yaml`, `scripts/ci/compare-differential.sh`, `.github/workflows/ci.yml`
- Implementation direction: normal / parse-error / type-error / module-import / file-io / macro / formatter の代表入力集合で Wasm/native の 5 観測点を比較し、allowlist 0 件を維持したまま tier1 artifact 比較へ拡張する。
- Dependencies: `V2-08`, `WASM-06`, `META-05`
- Acceptance: differential job が tier1 で green となり、allowlist 件数 0 のまま representative input set を通過する。
- Evidence: `tests/differential-allowlist.yaml`, `crates/lsharp-wasm/tests/e2e/selfhost_native_differential.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`, `docs/development/planning/v2-designs/v2-09-wasm-native-differential-zero.md`

<a id="v2-10-native-only-rc-distribution"></a>
### V2-10 Native-only RC distribution

- Goal: official host launcher + embedded guest component 配布とは分離した experimental native-only RC channel を設計する。
- Current state: Component Model pivot により native-only RC は Phase 13 移行前 gate から外れ、公式配布は host launcher + embedded guest component を維持する。native-only RC の artifact 契約、workflow、署名・checksum 手順は未着手。
- Rust source: `docs/development/planning/completion-criteria.md`, `docs/development/operations/release-distribution-signing.md`
- L# target: release workflow, package manifests, release notes
- Implementation direction: V2-08 / V2-09 完了後に tier1 限定の experimental workflow を追加し、公式配布と混同しない命名・release note・署名/checksum 導線を separate channel として整える。
- Dependencies: `V2-08`, `V2-09`, `OPS-06`
- Acceptance: native-only RC artifact が experimental channel でのみ生成され、公式 component distribution を置き換えない。
- Evidence: experimental release workflow, release notes, checksum / signing logs, `docs/development/planning/v2-designs/v2-10-native-only-rc-distribution.md`

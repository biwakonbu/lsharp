# Phase 11 実装バックログ詳細設計

## 位置づけ

- `TODO.md` は実行順の master checklist。
- この文書は task ID ごとの implementation bridge。仕様変更の正本ではなく、既存 spec を実コードへ落とすための実装方針・依存・受入条件を固定する。
- 正本 spec は `docs/rust-parity-spec.md`, `docs/toolchain-parity-spec.md`, `docs/verification-spec.md`, `docs/completion-criteria.md`, `docs/runtime-stability-spec.md`, `docs/ci-migration-spec.md` を優先する。

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
| `CP-03` | IR/backend/native/bootstrap parity を成立させる | `IR-01`〜`IR-06`, `WASM-01`〜`WASM-06`, `NATIVE-01`〜`NATIVE-06` 完了 |
| `CP-04` | 公開 toolchain を L# 実装へ移す | `CLI-01`〜`PKG-01` 完了 |
| `CP-05` | 長寿命 runtime を gate 化する | `GC-01`〜`GC-06` 完了 |
| `CP-06` | CI/release/default path を L# 正本へ切り替え、Rust を撤去する | `OPS-01`〜`OPS-08` 完了 |

## Workstream Details

## WS-META Evidence / backlog hygiene

<a id="meta-01-compatibility-matrix-evidence-enrichment"></a>
### META-01 Compatibility matrix evidence enrichment

- Goal: `docs/compatibility-matrix.md` を削除可否判断に使える evidence table に引き上げる。
- Current state: status と parity test の有無はあるが、Rust/L# 実装箇所と具体的 evidence が足りない。
- Rust source: `crates/lsharp-syntax/src/*`, `crates/lsharp-types/src/*`, `crates/lsharp-ir/src/*`, `crates/lsharp-wasm/src/*`
- L# target: `docs/compatibility-matrix.md`
- Implementation direction: CLI/LSP/selfhost 各表を 8 列へ拡張し、`Rust source` と `L# source` を追加する。パス表記は `crate::module` と `selfhost/File.ls` に固定し、`Parity test` には件数ではなく concrete test 名または snapshot 名を列挙する。
- Dependencies: なし。
- Acceptance: active row すべてに `Rust source`, `L# source`, `Parity test`, `Default path`, `Deletion gate` が埋まり、`-` が残るのは未着手機能だけ。
- Evidence: `docs/compatibility-matrix.md`, `scripts/audit_docs.sh`

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
- Rust source: `docs/gap-classification.md`
- L# target: `TODO.md`, `docs/phase11-implementation-plan.md`
- Implementation direction: 各 task に `Gap class` を暗黙的に持たせ、`仕様差分` は spec 更新、`実装欠落` はコード追加、`出力差分` は golden 修正、`性能差分` は GC/benchmark、`運用差分` は CI/release へ振り分ける。
- Dependencies: `META-01`
- Acceptance: Phase 11 task のいずれも `仕様差分` と `実装欠落` を混同しない。
- Evidence: この文書の各 task 記述, `docs/gap-classification.md`

<a id="meta-05-differential-allowlist-registry"></a>
### META-05 Differential allowlist registry

- Goal: Wasm/native 既知差分の一時退避先を 1 つに固定する。
- Current state: allowlist ファイルが未作成で、差分が TODO や会話ログに散っている。
- Rust source: `docs/verification-spec.md`
- L# target: `tests/differential-allowlist.yaml`
- Implementation direction: allowlist は `id`, `category`, `observation`, `reason`, `resolve_condition`, `tracking_issue` の YAML 配列に固定し、追加時は TODO の該当 task へ逆リンクを張る。
- Dependencies: `NATIVE-06`, `OPS-01`
- Acceptance: 差分例外はすべて `tests/differential-allowlist.yaml` に集約され、件数 0 を完了条件にできる。
- Evidence: `tests/differential-allowlist.yaml`, `docs/verification-spec.md`

## WS-BOOTSTRAP Frontend unblock

<a id="boot-01-mainls-import-path-consolidation"></a>
### BOOT-01 Main.ls import path consolidation

- Goal: `selfhost/Main.ls` を統合パイプラインだけに縮退させ、暫定インライン定義を除去する。
- Current state: [Main.ls](/Users/biwakonbu/github/lsharp/selfhost/Main.ls) は `import` を宣言しつつ、Token/AST/IR/Compiler/WasmEmit を再定義している。
- Rust source: `crates/lsharp-syntax/src/*`, `crates/lsharp-ir/src/*`, `crates/lsharp-wasm/src/*`
- L# target: `selfhost/Main.ls`, `selfhost/Token.ls`, `selfhost/AST.ls`, `selfhost/IR.ls`, `selfhost/Compiler.ls`, `selfhost/WasmEmit.ls`
- Implementation direction: `Main.ls` の責務を `pipeline orchestration + CLI entry` に限定する。各モジュールは最低 1 つの public entrypoint を持つ。`Lexer.tokenize`, `Parser.parse-program`, `MacroExpand.expand-program`, `TypeInfer.infer-program`, `Compiler.lower-program`, `WasmEmit.emit-module` を固定 API にする。
- Dependencies: `SYNTAX-01`, `TYPE-01`, `IR-02`, `WASM-01`
- Acceptance: `Main.ls` に Token/AST/IR/Compiler/WasmEmit の再定義が残らず、selfhost core modules import だけでパイプラインが組み上がる。
- Evidence: `selfhost/Main.ls`, `test_e2e_selfhost_pipeline_complete_stages`

<a id="boot-02-macroexpand-parser-compat-cleanup"></a>
### BOOT-02 MacroExpand parser-compat cleanup

- Goal: `selfhost/MacroExpand.ls` を現行 parser が受理できる構文へ揃える。
- Current state: [MacroExpand.ls](/Users/biwakonbu/github/lsharp/selfhost/MacroExpand.ls) は direct compile blocker になっている。
- Rust source: `crates/lsharp-syntax/src/macro_expand.rs`, `crates/lsharp-syntax/src/parser.rs`
- L# target: `selfhost/MacroExpand.ls`
- Implementation direction: 第 1 段階では parser widening ではなく `MacroExpand.ls` を parser-compatible subset に書き換える。reader shorthand や未対応 syntactic sugar は使わず、quote/unquote は explicit AST constructor 経由で表す。
- Dependencies: `BOOT-01`, `SYNTAX-02`
- Acceptance: `cargo run -- compile selfhost/MacroExpand.ls` と `cargo run -- compile selfhost/Main.ls` が成功する。
- Evidence: `cargo run -- compile selfhost/MacroExpand.ls`, `cargo run -- compile selfhost/Main.ls`

<a id="boot-03-stdlib-direct-compile-blockers"></a>
### BOOT-03 stdlib direct compile blockers

- Goal: `stdlib/*.ls` を selfhost compiler で個別に direct compile できるようにする。
- Current state: stdlib compile は監視扱いで、`stdlib/Map.ls` などの blocker が残っている。
- Rust source: `stdlib/*.ls`, `crates/lsharp-wasm/tests/e2e.rs`
- L# target: `stdlib/*.ls`, `scripts/ci/compile-phase11-inputs.sh`
- Implementation direction: 固定入力集合を shell script で列挙し、`selfhost/*.ls`, `stdlib/*.ls`, `examples/*.ls` を 1 ファイルずつ compile する。失敗は module/type/macro/backend のどこで落ちたかに分類して TODO へ戻す。
- Dependencies: `BOOT-02`, `TYPE-02`, `IR-01`
- Acceptance: fixed input set 全件が individual compile で pass し、CI の stdlib compile を `continue-on-error` から blocking に上げられる。
- Evidence: `scripts/ci/compile-phase11-inputs.sh`, `test_e2e_bootstrap_ci_stdlib_compile`, `test_e2e_bootstrap_stage1_compile_selfhost_sources`

<a id="boot-04-true-stage1-stage2-stage3-bootstrap"></a>
### BOOT-04 True stage1-stage2-stage3 bootstrap

- Goal: proxy bootstrap を本物の 3 段固定点検証へ置換する。
- Current state: `test_e2e_bootstrap_stage1_stage2_match` と `test_e2e_bootstrap_fixed_point_stage2_stage3` は proxy 判定。
- Rust source: `docs/verification-spec.md`, `crates/lsharp-wasm/tests/e2e.rs`
- L# target: `crates/lsharp-wasm/tests/e2e.rs`, `.github/workflows/ci.yml`, `ci-artifacts/bootstrap-diff/`
- Implementation direction: stage0(Rust) -> stage1.wasm -> stage2.wasm -> stage3.wasm を必ず実体生成し、比較層は `raw wasm bytes`, `exported symbol list`, `data section bytes`, `compiler diagnostics` の 4 つに固定する。失敗時 diff は artifact へ保存する。
- Dependencies: `BOOT-01`, `BOOT-03`, `WASM-03`
- Acceptance: `test_e2e_bootstrap_stage1_stage2_match`, `test_e2e_bootstrap_fixed_point_stage2_stage3`, `test_e2e_bootstrap_stage1_section_stability`, `test_e2e_bootstrap_stage1_symbol_stability` が実体比較で pass する。
- Evidence: `crates/lsharp-wasm/tests/e2e.rs`, `ci-artifacts/bootstrap-diff/`

## WS-SYNTAX Frontend syntax parity

<a id="syntax-01-span-model"></a>
### SYNTAX-01 Span model

- Goal: Rust `Span` と等価な selfhost span/token モデルを導入する。
- Current state: parser/lexer は 3 つ組 span 配列を扱うが、独立した `Span.ls` がなく AST の正本にもなっていない。
- Rust source: `crates/lsharp-syntax/src/span.rs`, `crates/lsharp-syntax/src/token.rs`
- L# target: `selfhost/Span.ls`, `selfhost/Token.ls`, `selfhost/Lexer.ls`, `selfhost/Parser.ls`
- Implementation direction: `Span` は Rust と同じ `[start end]` に固定し、line/column は diagnostics 層で導出する。`Token` は `[kind payload span]` に正規化し、literal payload は token 側で保持する。
- Dependencies: `BOOT-01`
- Acceptance: lexer/parser/type error が全て span を返し、span merge と dummy が `Span.ls` に集約される。
- Evidence: `selfhost/Span.ls`, `test_e2e_selfhost_lexer_basic`, `test_e2e_selfhost_parser_basic`

<a id="syntax-02-full-ast-coverage"></a>
### SYNTAX-02 Full AST coverage

- Goal: Rust AST の Decl/Expr/Pattern/Literal/Metadata を selfhost AST に 1:1 対応させる。
- Current state: `Parser.ls` と `AST.ls` は最小 subset だけを扱い、quote/record/annotation/computation/trait metadata 等が欠けている。
- Rust source: `crates/lsharp-syntax/src/ast.rs`
- L# target: `selfhost/AST.ls`, `selfhost/Parser.ls`
- Implementation direction: 全 AST ノードを `[tag span ...payload]` 形式に統一し、constructor/accessor を `AST.ls` へ集約する。`Pattern`, `Metadata`, `TypeExpr`, `WhereClause`, `MatchArm`, `Variant`, `Param` も tagged vector で正規化する。
- Dependencies: `SYNTAX-01`
- Acceptance: parser が Rust AST に存在する全ノード型を lossless に表現できる。
- Evidence: `selfhost/AST.ls`, `test_e2e_selfhost_parser_full_sexp`, `test_e2e_selfhost_module_declarations`

<a id="syntax-03-parser-recovery-and-diagnostics"></a>
### SYNTAX-03 Parser recovery and diagnostics

- Goal: parser recovery と複数診断収集を parity 条件に入れる。
- Current state: parser は fail-fast に近く、診断の severity/code/order が固定されていない。
- Rust source: `crates/lsharp-syntax/src/parser.rs`
- L# target: `selfhost/Parser.ls`
- Implementation direction: parser の戻り値を `program + diagnostics` に拡張し、diagnostic は `[severity code span message-hash]` へ固定する。recovery point は `)`・`]`・`}`・次トップレベル宣言キーワードの 4 種に固定する。
- Dependencies: `SYNTAX-01`, `SYNTAX-02`
- Acceptance: 1 回の parse で複数エラーを収集し、diagnostics が source order で安定する。
- Evidence: `test_golden_syntax_recovery_*`, `test_e2e_selfhost_parser_basic`

<a id="syntax-04-hygiene-gensym-and-expansion-trace"></a>
### SYNTAX-04 Hygiene gensym and expansion trace

- Goal: 衛生マクロの最小完全モデルを selfhost へ導入する。
- Current state: `MacroExpand.ls` は引数置換中心で、gensym と expansion trace が未整備。
- Rust source: `crates/lsharp-syntax/src/hygiene.rs`, `crates/lsharp-syntax/src/macro_expand.rs`
- L# target: `selfhost/Hygiene.ls`, `selfhost/MacroExpand.ls`, `selfhost/AST.ls`
- Implementation direction: `Hygiene.ls` に `scope-id`, `gensym-counter`, `expansion-trace` を集約する。binder/var は raw hash ではなく hygiene symbol を保持し、マクロ展開時に generated symbol へ fresh gensym を割り当てる。
- Dependencies: `SYNTAX-02`, `BOOT-02`
- Acceptance: nested macro 展開で衝突しない生成名を得られ、エラー時に expansion trace を表示できる。
- Evidence: `test_e2e_selfhost_macro_nested_expansion`, `test_e2e_selfhost_macro_defmacro_with_args`

<a id="syntax-05-derive-expansion"></a>
### SYNTAX-05 Derive expansion

- Goal: derive expansion を MacroExpand から分離して selfhost へ実装する。
- Current state: `Derive.ls` が存在せず、derive は未移植。
- Rust source: `crates/lsharp-syntax/src/derive.rs`
- L# target: `selfhost/Derive.ls`, `selfhost/Main.ls`
- Implementation direction: pipeline 順を `Parser -> Derive -> MacroExpand -> TypeInfer` に固定する。derive は top-level `Decl` を受け取り helper decl 群へ展開し、一般 macro 展開前に実行する。
- Dependencies: `SYNTAX-02`
- Acceptance: derive 由来の helper decl が parser と type infer の入力として安定生成される。
- Evidence: `selfhost/Derive.ls`, `test_golden_syntax_derive_*`

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
- Current state: [TypeInfer.ls](/Users/biwakonbu/github/lsharp/selfhost/TypeInfer.ls) が type constructor, substitution, instantiate を再定義している。
- Rust source: `crates/lsharp-types/src/types.rs`
- L# target: `selfhost/Type.ls`, `selfhost/TypeScheme.ls`, `selfhost/TypeInfer.ls`
- Implementation direction: `Type.ls` は type representation + substitution + occurs-check のみ、`TypeScheme.ls` は `mono/poly/free-type-vars/generalize/instantiate` のみ、`TypeInfer.ls` は inference state と algorithm のみに限定する。
- Dependencies: `BOOT-01`
- Acceptance: `TypeInfer.ls` に type representation 再定義が残らない。
- Evidence: `selfhost/Type.ls`, `selfhost/TypeScheme.ls`, `test_e2e_selfhost_type_system`

<a id="type-02-unify-generalize-instantiate"></a>
### TYPE-02 Unify generalize instantiate

- Goal: HM 推論の核を Rust 実装と同じ公開挙動へ揃える。
- Current state: unify と instantiate は最小版で、program-level inference と let-polymorphism が限定的。
- Rust source: `crates/lsharp-types/src/infer.rs`, `crates/lsharp-types/src/types.rs`
- L# target: `selfhost/TypeInfer.ls`, `selfhost/TypeScheme.ls`
- Implementation direction: `InferState = [subst counter diagnostics]` を導入し、`infer-expr`, `generalize`, `instantiate`, `apply-subst-env` を state threading で実装する。let-bound value は必ず generalize し、application は arity と function type を明示的に unify する。
- Dependencies: `TYPE-01`
- Acceptance: literal/variable/function/let-polymorphism/unification の既存 E2E が stateful 実装へ置換されても通る。
- Evidence: `test_e2e_selfhost_typeinfer_literal`, `test_e2e_selfhost_typeinfer_variable`, `test_e2e_selfhost_typeinfer_function`, `test_e2e_selfhost_typeinfer_let_poly`, `test_e2e_selfhost_typeinfer_unification`

<a id="type-03-match-inference"></a>
### TYPE-03 Match inference

- Goal: `match` の型推論とパターン束縛を selfhost 側で完結させる。
- Current state: pattern match は E2E 1 本あるが、arm ごとの binder/type refinement が限定的。
- Rust source: `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/TypeInfer.ls`, `selfhost/AST.ls`
- Implementation direction: `infer-pattern` を追加し、pattern が生成する binder env と scrutinee expected type を返す。各 arm body は共通 result type へ unify し、guard は Bool に固定する。
- Dependencies: `SYNTAX-02`, `TYPE-02`
- Acceptance: wildcard/var/literal/constructor/record pattern の binder と arm return type が正しく推論される。
- Evidence: `test_e2e_selfhost_typeinfer_pattern_match`, `test_golden_types_match_*`

<a id="type-04-constraints-trait-where"></a>
### TYPE-04 Constraints trait where

- Goal: trait/where/constraint solving を `Constraints.ls` へ分離する。
- Current state: `Constraints.ls` が存在せず、trait 解決ロジックを置く場所がない。
- Rust source: `crates/lsharp-types/src/constraints.rs`
- L# target: `selfhost/Constraints.ls`, `selfhost/TypeInfer.ls`
- Implementation direction: `Constraints.ls` は trait registry, impl registry, pending constraint queue, solver を持つ。`TypeInfer.ls` は constraint を生成するだけにし、解決は `Constraints.solve-all` へ委譲する。
- Dependencies: `TYPE-01`, `TYPE-02`
- Acceptance: where clause と trait constraint が pending queue で解決され、未解決時は deterministic な error code を返す。
- Evidence: `selfhost/Constraints.ls`, `test_golden_types_constraints_*`

<a id="type-05-metadata-check"></a>
### TYPE-05 Metadata check

- Goal: metadata validation を type inference とは別モジュールへ隔離する。
- Current state: `MetadataCheck.ls` が存在せず、metadata の構造検証も未移植。
- Rust source: `crates/lsharp-types/src/metadata_check.rs`
- L# target: `selfhost/MetadataCheck.ls`
- Implementation direction: `MetadataCheck.validate-program` を導入し、doc/params/returns/invariant/example/transitions の構造制約を検証する。type checker は metadata expression の type だけを返し、メタデータ schema 検証はここへ移す。
- Dependencies: `SYNTAX-02`, `TYPE-02`
- Acceptance: metadata 不正が type error と混ざらず、独立した validation error として返る。
- Evidence: `selfhost/MetadataCheck.ls`, `test_golden_types_metadata_*`

<a id="type-06-hkt-gadt-alias-record-update"></a>
### TYPE-06 HKT GADT alias record update

- Goal: 高度型機能の最小完了集合を selfhost 側へ揃える。
- Current state: HKT/GADT/alias/record update はほぼ未移植。
- Rust source: `crates/lsharp-types/src/types.rs`, `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/Type.ls`, `selfhost/TypeInfer.ls`, `selfhost/Constraints.ls`
- Implementation direction: kind environment を `Type.ls` に追加し、type constructor application で kind check を行う。GADT は constructor return type の specialization を match inference に渡し、record update は original record type と field patch を unify する。
- Dependencies: `TYPE-03`, `TYPE-04`
- Acceptance: type alias 展開、record update、GADT constructor match、HKT kind error が自動検証で閉じる。
- Evidence: `test_golden_types_hkt_*`, `test_golden_types_gadt_*`, `test_e2e_type_alias`

<a id="type-07-type-error-parity"></a>
### TYPE-07 Type error parity

- Goal: error code / span / primary message の parity を固定する。
- Current state: selfhost 側はエラーマーカー中心で、Rust と同じエラー構造を返していない。
- Rust source: `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/TypeInfer.ls`, `selfhost/Constraints.ls`
- Implementation direction: type error は `[code span primary secondary* help*]` に固定し、primary message の意味一致を必須にする。secondary/help は byte-perfect ではなく meaning-preserving でよいが、`code` と `span` は一致させる。
- Dependencies: `SYNTAX-01`, `TYPE-02`, `TYPE-04`
- Acceptance: representative type mismatch/undefined var/arity/trait/kind errors が golden fixture と一致する。
- Evidence: `tests/golden/types/errors/*`, `test_e2e_type_error_rejected`

<a id="type-08-deterministic-ordering"></a>
### TYPE-08 Deterministic ordering

- Goal: type variable naming と diagnostics/display 順序を安定化する。
- Current state: hash map 由来の順序揺れ余地が残る。
- Rust source: `crates/lsharp-types/src/infer.rs`
- L# target: `selfhost/TypeInfer.ls`, `selfhost/TypeScheme.ls`
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
- L# target: `selfhost/ModuleGraph.ls`
- Implementation direction: import decl から adjacency list を生成し、topological sort と cycle diagnostic を提供する。入力は file path list、出力は ordered module plan。
- Dependencies: `SYNTAX-02`
- Acceptance: module graph の順序と循環依存エラーが deterministic である。
- Evidence: `selfhost/ModuleGraph.ls`, `test_e2e_selfhost_module_graph_topological_sort`

<a id="ir-02-lower-split"></a>
### IR-02 Lower split

- Goal: lowering を `Lower.ls`, `LowerExpr.ls`, `LowerDecl.ls`, `LowerPattern.ls` に分割する。
- Current state: lowering は `Main.ls` と `Compiler.ls` に混在している。
- Rust source: `crates/lsharp-ir/src/lower/mod.rs`, `crates/lsharp-ir/src/lower/expr.rs`, `crates/lsharp-ir/src/lower/decl.rs`, `crates/lsharp-ir/src/lower/pattern.rs`
- L# target: `selfhost/Lower.ls`, `selfhost/LowerExpr.ls`, `selfhost/LowerDecl.ls`, `selfhost/LowerPattern.ls`, `selfhost/Compiler.ls`
- Implementation direction: `Compiler.ls` は orchestration 層へ縮退し、decl lowering と expr lowering を分離する。`lower-program` は `FrontendResult -> LoweredModule` の唯一の入口にする。
- Dependencies: `BOOT-01`, `TYPE-02`
- Acceptance: `Main.ls` から lowering 実装が剥がれ、`Compiler.ls` は `Lower.ls` を呼ぶだけになる。
- Evidence: `selfhost/Lower.ls`, `test_e2e_selfhost_ir`, `test_e2e_selfhost_compiler`

<a id="ir-03-closure-conversion"></a>
### IR-03 Closure conversion

- Goal: 自由変数解析と環境キャプチャを独立段階にする。
- Current state: closure conversion 専用モジュールがない。
- Rust source: `crates/lsharp-ir/src/closure.rs`
- L# target: `selfhost/Closure.ls`, `selfhost/LowerExpr.ls`
- Implementation direction: lambda lowering 前に free var set を計算し、environment record を明示生成する。 direct call と closure call を別 IR 命令にし、table slot 生成は codegen へ渡す。
- Dependencies: `IR-02`
- Acceptance: nested lambda と higher-order function が closure conversion 後も deterministic IR を出す。
- Evidence: `test_golden_ir_closure_*`, `crates/lsharp-ir/src/lower/snapshots/*lower_closure.snap`

<a id="ir-04-pattern-lowering"></a>
### IR-04 Pattern lowering

- Goal: pattern match を IR の branch sequence へ落とす。
- Current state: pattern lowering は限定的または未分離。
- Rust source: `crates/lsharp-ir/src/lower/pattern.rs`
- L# target: `selfhost/LowerPattern.ls`
- Implementation direction: literal/constructor/record/wildcard pattern をタグ比較・field extraction・guard branch へ展開する。 match arm 順は source order を維持し、failure path を次 arm へ接続する。
- Dependencies: `TYPE-03`, `IR-02`
- Acceptance: ADT match と nested pattern が stable IR snapshot を生成する。
- Evidence: `test_golden_ir_pattern_*`, `crates/lsharp-ir/src/lower/snapshots/*lower_adt_match.snap`

<a id="ir-05-trait-dispatch-lowering"></a>
### IR-05 Trait dispatch lowering

- Goal: trait method call を辞書引数付き call へ lowering する。
- Current state: trait dispatch lowering の専用段階がない。
- Rust source: `crates/lsharp-ir/src/lower/mod.rs`
- L# target: `selfhost/LowerDecl.ls`, `selfhost/LowerExpr.ls`
- Implementation direction: solved trait constraints から dictionary value を materialize し、method call site に hidden first arg として挿入する。 impl selection は type phase の結果だけを使い、lowering では分岐しない。
- Dependencies: `TYPE-04`, `IR-02`
- Acceptance: trait dispatch IR が impl order ではなく solved constraint order で安定する。
- Evidence: `crates/lsharp-ir/src/lower/snapshots/*lower_trait_dispatch.snap`, `test_golden_ir_trait_dispatch_*`

<a id="ir-06-ir-snapshot-serializer"></a>
### IR-06 IR snapshot serializer

- Goal: Rust/L# IR を line-based snapshot で比較できるようにする。
- Current state: IR snapshot の L# 側 serializer がない。
- Rust source: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/lower/tests.rs`
- L# target: `selfhost/IR.ls`, `tests/golden/ir/`
- Implementation direction: snapshot format は `; module`, `; function`, `instruction` の 3 階層に固定し、type var と temp id は正規化する。 map 由来順序は serializer 側で stable sort する。
- Dependencies: `IR-02`〜`IR-05`
- Acceptance: examples/stdlib/selfhost の representative set で Rust/L# snapshot diff が空になる。
- Evidence: `tests/golden/ir/*`, `test_golden_ir_*`

<a id="wasm-01-backend-boundary"></a>
### WASM-01 Backend boundary

- Goal: backend 境界を `FrontendResult -> LoweredModule -> CodegenArtifact` の 3 層に固定する。
- Current state: `Main.ls` が pipeline 全段の暫定データを vector で持っている。
- Rust source: `docs/backend-boundary.md`, `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-wasm/src/lib.rs`
- L# target: `selfhost/Main.ls`, `selfhost/IR.ls`, `selfhost/WasmEmit.ls`
- Implementation direction: `FrontendResult`, `LoweredModule`, `CodegenArtifact` の tagged record を `IR.ls` と `WasmEmit.ls` で定義し、Main はこの 3 層を受け渡すだけにする。
- Dependencies: `IR-02`, `BOOT-01`
- Acceptance: Main の中間値は raw vector 配列ではなく named stage artifact に置き換わる。
- Evidence: `docs/backend-boundary.md`, `test_e2e_selfhost_full_pipeline`

<a id="wasm-02-section-builders"></a>
### WASM-02 Section builders

- Goal: Wasm backend を section builder 単位に分割する。
- Current state: [WasmEmit.ls](/Users/biwakonbu/github/lsharp/selfhost/WasmEmit.ls) は 1 ファイルで LEB, header, section 生成を抱えている。
- Rust source: `crates/lsharp-wasm/src/codegen.rs`, `crates/lsharp-wasm/src/emit.rs`, `crates/lsharp-wasm/src/wasi.rs`
- L# target: `selfhost/WasmEmit.ls`, `selfhost/Codegen.ls`, `selfhost/Emit.ls`, `selfhost/WasiBackend.ls`
- Implementation direction: `Codegen.ls` は IR -> Wasm op sequence, `Emit.ls` は section bytes builder, `WasiBackend.ls` は imports/exports/memory/runtime wiring を担当する。`WasmEmit.ls` は façade に縮退させる。
- Dependencies: `WASM-01`
- Acceptance: type/import/function/memory/export/code/data section が独立 builder になり、unit test 可能になる。
- Evidence: `selfhost/WasmEmit.ls`, `test_e2e_selfhost_wasm_emit`, `test_e2e_selfhost_wasmemit`

<a id="wasm-03-deterministic-leb-emit"></a>
### WASM-03 Deterministic LEB emit

- Goal: Wasm バイナリ出力の非決定性をなくす。
- Current state: LEB encode と section 生成が ad-hoc 実装で、ordering と padding の保証が弱い。
- Rust source: `crates/lsharp-wasm/src/emit.rs`
- L# target: `selfhost/Emit.ls`, `selfhost/WasmEmit.ls`
- Implementation direction: signed/unsigned LEB, section order, symbol order, data order を spec 固定順で出力する。ハッシュやホストパスや時刻を埋め込まない。
- Dependencies: `WASM-02`, `BOOT-04`
- Acceptance: 同一入力 2 回 compile で bytes/hash/export/data section が一致する。
- Evidence: `test_e2e_bootstrap_stage1_deterministic`, `test_e2e_bootstrap_deterministic_output`, `test_e2e_bootstrap_selfhost_modules_deterministic`

<a id="wasm-04-wasi-helpers"></a>
### WASM-04 WASI helpers

- Goal: runtime boundary を selfhost backend 側に移植する。
- Current state: helper 群の責務分離が不完全。
- Rust source: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi_runner.rs`
- L# target: `selfhost/WasiBackend.ls`, `selfhost/WasiRunner.ls`
- Implementation direction: `print`, `read_file`, `write_file`, `clock_now` などの helper import を固定し、WASI runner 側で path/time normalization を行う。
- Dependencies: `WASM-02`
- Acceptance: file I/O と time を含む program が deterministic な観測値を返す。
- Evidence: `test_golden_wasm_wasi_*`, `docs/verification-spec.md`

<a id="wasm-05-test-runner"></a>
### WASM-05 Test runner

- Goal: `:example` / `:invariant` から自動テストを生成する。
- Current state: `TestRunner.ls` が存在せず、docs metadata と test harness が接続されていない。
- Rust source: `crates/lsharp-wasm/src/test_runner.rs`
- L# target: `selfhost/TestRunner.ls`
- Implementation direction: metadata を走査し、example は compile-and-run assertion、invariant は compile-time property check へ変換する。生成 test 名は `test_generated_example_<symbol>` 形式に固定する。
- Dependencies: `TYPE-05`, `WASM-04`
- Acceptance: example/invariant 付き宣言から deterministic な generated test suite が得られる。
- Evidence: `selfhost/TestRunner.ls`, `test_generated_example_*`

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

- Goal: tier1 native target ごとの差分を descriptor に閉じ込める。
- Current state: native backend 実装自体が未着手。
- Rust source: `docs/native-backend-spec.md`
- L# target: `selfhost/NativeTarget.ls`
- Implementation direction: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu` の target descriptor を `abi`, `section names`, `relocation types`, `linker flavor` の 4 項目で定義する。
- Dependencies: `IR-06`
- Acceptance: target 固有条件分岐が codegen 本体ではなく descriptor 参照だけになる。
- Evidence: `selfhost/NativeTarget.ls`

<a id="native-02-object-emitter"></a>
### NATIVE-02 Object emitter

- Goal: LoweredModule から relocation 付き object を生成する。
- Current state: object emitter が存在しない。
- Rust source: `docs/native-backend-spec.md`
- L# target: `selfhost/NativeCodegen.ls`, `selfhost/NativeEmit.ls`
- Implementation direction: codegen は machine-instr list を生成し、emit は Mach-O/ELF object と runtime object を別出力する。標準 artifact 名は `program.o`, `runtime.o`, `linker-response.txt`, `program.native` に固定する。
- Dependencies: `NATIVE-01`, `IR-02`
- Acceptance: tier1 target で object file が生成され、linker 手前で停止できる。
- Evidence: `build/native/*`, `test_unit_native_emit_*`

<a id="native-03-linker-response"></a>
### NATIVE-03 Linker response

- Goal: linker invocation を deterministic response file 化する。
- Current state: linker path と引数の置き場が未定。
- Rust source: `docs/native-backend-spec.md`, `docs/ci-migration-spec.md`
- L# target: `selfhost/Linker.ls`
- Implementation direction: linker 呼び出しは command line 直打ちではなく response file 経由に固定する。response file には object 順、runtime object、output path、system libs を source order で記載する。
- Dependencies: `NATIVE-02`
- Acceptance: 2 回の native build で response file と linked binary hash が一致する。
- Evidence: `build/native/linker-response.txt`, `test_unit_native_linker_*`

<a id="native-04-deterministic-codegen"></a>
### NATIVE-04 Deterministic codegen

- Goal: native backend を fixed-point を壊さない deterministic backend にする。
- Current state: native backend 未実装のため determinism も未確認。
- Rust source: `docs/native-backend-spec.md`, `docs/completion-criteria.md`
- L# target: `selfhost/NativeCodegen.ls`, `selfhost/NativeEmit.ls`
- Implementation direction: function order, static data order, relocation order, symbol numbering は source order + stable sort に固定する。debug info は v1 では出さない。
- Dependencies: `NATIVE-01`〜`NATIVE-03`
- Acceptance: 同一 commit 同一 target の 2 回 native build で binary hash が一致する。
- Evidence: `test_bootstrap_native_deterministic_*`, `docs/native-backend-spec.md`

<a id="native-05-stage1-native-self-regeneration"></a>
### NATIVE-05 Stage1-native self-regeneration

- Goal: stage1-native が selfhost compiler を自分で再生成できるようにする。
- Current state: native path は存在せず、完了条件の stage1-native gate 未着手。
- Rust source: `docs/completion-criteria.md`
- L# target: `scripts/ci/build-native.sh`, `.github/workflows/ci.yml`
- Implementation direction: `stage1-native` の build entry は `selfhost/Main.ls` compile で固定し、`stage1-native -> stage2-native -> stage3-native` の functional equivalence を gate にする。
- Dependencies: `BOOT-04`, `NATIVE-04`
- Acceptance: representative input set に対して stage2-native と stage3-native の観測値が一致する。
- Evidence: `test_bootstrap_native_stage2_stage3_*`, `.github/workflows/ci.yml`

<a id="native-06-wasmnative-differential"></a>
### NATIVE-06 Wasm/native differential

- Goal: Wasm と native の観測差分をゼロにする。
- Current state: differential harness と allowlist が未整備。
- Rust source: `docs/verification-spec.md`
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
- Rust source: Rust driver, `docs/toolchain-parity-spec.md`
- L# target: `selfhost/Cli.ls`, `docs/toolchain-parity-spec.md`
- Implementation direction: command contract は `args`, `stdin`, `stdout`, `stderr`, `exit code`, `artifacts` の 6 項目で表にし、`--help`, `--version`, `-o/--output` もこの表に含める。
- Dependencies: `META-01`
- Acceptance: 13 command すべてに contract table があり、help/version snapshot の正本になる。
- Evidence: `docs/toolchain-parity-spec.md`, `test_snapshot_cli_help_*`

<a id="cli-02-13-command-implementations"></a>
### CLI-02 13 command implementations

- Goal: `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` を L# 実装で提供する。
- Current state: `compile`, `lsp`, `fmt` が PoC、他は未実装。
- Rust source: Rust CLI 実装群
- L# target: `selfhost/Cli.ls`, `selfhost/Main.ls`, `selfhost/JsonRpc.ls`, `selfhost/Formatter.ls`, `selfhost/Linter.ls`
- Implementation direction: `selfhost/Cli.ls` を新設し、arg parse と subcommand dispatch を一元化する。`Main.ls` は compiler core only にし、CLI entry はここへ移す。
- Dependencies: `CLI-01`, `BOOT-01`, `LSP-01`, `FMT-01`, `DOC-01`
- Acceptance: 13 command すべてが help/version/exit code contract を満たし、default path を Rust から切り替えられる。
- Evidence: `test_snapshot_cli_help_*`, `scripts/smoke_test_readme.sh`

<a id="lsp-01-full-sync-skeleton"></a>
### LSP-01 Full-sync skeleton

- Goal: JSON-RPC PoC を full-sync LSP server の骨格へ引き上げる。
- Current state: [JsonRpc.ls](/Users/biwakonbu/github/lsharp/selfhost/JsonRpc.ls) は整数タグベースの PoC で、JSON schema 互換ではない。
- Rust source: Rust LSP server, `docs/toolchain-parity-spec.md`
- L# target: `selfhost/JsonRpc.ls`, `selfhost/LspServer.ls`
- Implementation direction: `JsonRpc.ls` は codec のみに限定し、`LspServer.ls` を新設して session state, full document sync, request dispatch を担当させる。v1 は `TextDocumentSyncKind.Full` に固定する。
- Dependencies: `CLI-01`
- Acceptance: initialize/didOpen/didChange/shutdown の skeleton が JSON-RPC 2.0 / LSP 3.17 互換で通る。
- Evidence: `test_snapshot_lsp_initialize`, `test_snapshot_lsp_full_sync`

<a id="lsp-02-10-method-parity"></a>
### LSP-02 10 method parity

- Goal: 10 LSP method の公開挙動を Rust 版互換にする。
- Current state: initialize/shutdown/didOpen/didChange は設計のみ、hover/definition/references/rename/completion は未実装または PoC。
- Rust source: Rust LSP 実装, `docs/toolchain-parity-spec.md`
- L# target: `selfhost/LspServer.ls`, `selfhost/Formatter.ls`, `selfhost/Linter.ls`
- Implementation direction: method 実装順は `initialize`, `shutdown`, `didOpen`, `didChange`, `hover`, `goto_definition`, `references`, `rename`, `formatting`, `completion` に固定する。レスポンス shape は JSON snapshot を正本にする。
- Dependencies: `LSP-01`, `FMT-01`, `LINT-01`, `TYPE-07`
- Acceptance: 10 method が JSON schema 互換レスポンスを返し、VSCode extension から spawn できる。
- Evidence: `test_snapshot_lsp_hover`, `test_snapshot_lsp_definition`, `test_snapshot_lsp_references`, `test_snapshot_lsp_rename`, `test_snapshot_lsp_completion`

<a id="lsp-03-diagnostic-ordering-and-json-snapshots"></a>
### LSP-03 Diagnostic ordering and JSON snapshots

- Goal: diagnostics の順序と schema を安定化する。
- Current state: JSON snapshot の正本がなく、diagnostic dedup ルールも未固定。
- Rust source: Rust LSP diagnostics path
- L# target: `selfhost/LspServer.ls`, `tests/snapshots/lsp/`
- Implementation direction: diagnostics は `source(parse/type/lint)`, `severity`, `line`, `column` の順で sort し、同一 span は最も severity の高いものだけを残す。
- Dependencies: `SYNTAX-03`, `TYPE-07`, `LINT-01`
- Acceptance: 同一ファイルを再パースしても diagnostics JSON diff が空になる。
- Evidence: `tests/snapshots/lsp/diagnostics/*.json`, `test_snapshot_lsp_diagnostics_*`

<a id="fmt-01-formatter-roundtrip"></a>
### FMT-01 Formatter roundtrip

- Goal: formatter を AST 全体対応 + roundtrip/idempotency gate へ引き上げる。
- Current state: [Formatter.ls](/Users/biwakonbu/github/lsharp/selfhost/Formatter.ls) は integer/var/apply 중심の PoC。
- Rust source: Rust formatter path, `docs/toolchain-parity-spec.md`
- L# target: `selfhost/Formatter.ls`
- Implementation direction: formatter の public API は `format-program ast -> text` に固定し、parser と対で `parse(format(parse(src))) == parse(src)` を gate にする。短形式/長形式/let 整列ルールは spec に従う。
- Dependencies: `SYNTAX-02`, `SYNTAX-06`
- Acceptance: AST 全 node coverage, roundtrip, idempotency が CI gate に入る。
- Evidence: `test_snapshot_formatter_*`, `test_roundtrip_formatter_*`

<a id="lint-01-rule-ids-and-clilsp-parity"></a>
### LINT-01 Rule IDs and CLI/LSP parity

- Goal: linter を stable rule ID と CLI/LSP 共通 diagnostics に揃える。
- Current state: [Linter.ls](/Users/biwakonbu/github/lsharp/selfhost/Linter.ls) は簡易 rule 実装で AST coverage が狭い。
- Rust source: Rust lint path, `docs/toolchain-parity-spec.md`
- L# target: `selfhost/Linter.ls`
- Implementation direction: builtin rule は `L0001` 形式の ID へ固定し、出力を `[rule-id severity span message]` に正規化する。CLI と LSP は同じ core API を使う。
- Dependencies: `SYNTAX-02`, `LSP-03`
- Acceptance: same source に対する CLI lint と LSP diagnostics の rule/severity/span が一致する。
- Evidence: `test_snapshot_lint_*`, `test_e2e_selfhost_linter`, `test_e2e_selfhost_linter_lsp_integration`

<a id="doc-01-schemas-and-snapshots"></a>
### DOC-01 Schemas and snapshots

- Goal: knowledge/review/doc output の schema を固定する。
- Current state: `docs/schemas/` が存在しない。
- Rust source: Rust docs/review path, `docs/toolchain-parity-spec.md`
- L# target: `docs/schemas/knowledge.schema.json`, `docs/schemas/review.schema.json`, `docs/schemas/doc-output.schema.json`
- Implementation direction: docs 系は structured JSON schema を先に固定し、CLI/LSP/extension はその schema だけを見る。snapshot test は JSON canonicalization 後に比較する。
- Dependencies: `META-01`
- Acceptance: 3 schema が配布物に同梱され、knowledge/review/doc 出力が snapshot gate を持つ。
- Evidence: `docs/schemas/*.json`, `test_snapshot_docs_*`

<a id="doc-02-trailer-and-deterministic-html"></a>
### DOC-02 Trailer and deterministic HTML

- Goal: doc-ack/doc-check trailer と HTML doc 生成の deterministic 性を揃える。
- Current state: trailer 実装は未移植、HTML output の deterministic policy も未固定。
- Rust source: Rust doc tooling, `docs/toolchain-parity-spec.md`
- L# target: `selfhost/DocTools.ls`, `selfhost/HtmlDoc.ls`
- Implementation direction: trailer syntax は Rust 版互換の comment form に固定し、HTML は timestamp/hostname/absolute path を埋め込まない。environment-dependent metadata は opt-in flag がある時だけ出す。
- Dependencies: `DOC-01`
- Acceptance: `doc-ack`, `doc-check`, `doc` が deterministic output を返し、2 回実行 diff が空になる。
- Evidence: `test_snapshot_doc_ack_*`, `test_snapshot_doc_check_*`, `test_snapshot_doc_html_*`

<a id="pkg-01-archives-checksums-and-quick-start"></a>
### PKG-01 Archives checksums and Quick Start

- Goal: native 配布物だけで Quick Start が完走する配布形を固定する。
- Current state: package shape と smoke path が未完成。
- Rust source: `docs/release-operations-spec.md`, `docs/toolchain-parity-spec.md`
- L# target: `scripts/ci/release-smoke.sh`, `scripts/smoke_test_readme.sh`, release workflow
- Implementation direction: 配布アーカイブ内容は `lsharp`, `lsharp-lsp`, `README.md`, `LICENSE`, `checksums.txt`, `CHANGELOG.md` に固定し、Quick Start smoke は展開後 `--version -> check -> build/test/run` を自動化する。
- Dependencies: `CLI-02`, `LSP-02`, `OPS-06`
- Acceptance: release artifact 展開だけで README Quick Start が通り、checksum 検証も自動化される。
- Evidence: `scripts/smoke_test_readme.sh`, `scripts/ci/release-smoke.sh`

## WS-RUNTIME Long-lived runtime stability

<a id="gc-01-m1-object-model"></a>
### GC-01 M1 object model

- Goal: collector 前提の object header / trace map / root API を導入する。
- Current state: precise tracing GC の前提モデルが未実装。
- Rust source: `docs/runtime-spec.md`, `docs/memory-management-roadmap.md`, `docs/runtime-stability-spec.md`
- L# target: runtime layer, builtins, `selfhost/*`
- Implementation direction: object header は `[tag size-or-words mark-state aux]` で固定し、`root_push`, `root_pop`, `root_set` を no-op 互換 API として先行導入する。trace map は string/adt/record/vector/hashmap/closure/ref-cell ごとに定義する。
- Dependencies: なし。
- Acceptance: all heap object kinds に trace 規約があり、GC 未導入でも root API を呼べる。
- Evidence: `test_unit_runtime_object_header_*`, `test_unit_runtime_root_api_*`

<a id="gc-02-m2-mark-sweep-mvp"></a>
### GC-02 M2 mark-sweep MVP

- Goal: mark-sweep collector の最小版を導入する。
- Current state: free list, mark bit, sweep loop が未実装。
- Rust source: `docs/memory-management-roadmap.md`, `docs/runtime-stability-spec.md`
- L# target: runtime allocator / collector
- Implementation direction: allocator は fast path と slow path に分け、slow path で `mark -> sweep -> free list reuse` を実行する。collector reentry は global state で禁止する。
- Dependencies: `GC-01`
- Acceptance: collector 有効でも既存 E2E 群が落ちず、free list reuse が確認できる。
- Evidence: `test_unit_gc_mark_sweep_*`, `test_e2e_runtime_gc_mark_sweep_*`

<a id="gc-03-m3-generational-pass"></a>
### GC-03 M3 generational pass

- Goal: nursery + write barrier + promotion policy を導入する。
- Current state: performance pass が未着手。
- Rust source: `docs/memory-management-roadmap.md`, `docs/runtime-stability-spec.md`
- L# target: runtime allocator / write barrier
- Implementation direction: young generation を bump allocator、old generation を non-moving mark-sweep に固定し、minor GC と full GC を分離する。promotion は survival count or size threshold の 2 条件で決める。
- Dependencies: `GC-02`
- Acceptance: generational mode が mark-sweep mode と同じ意味論を保ち、minor GC metrics を収集できる。
- Evidence: `test_unit_gc_generational_*`, `benchmark-results/*.json`

<a id="gc-04-longevity-benchmarks"></a>
### GC-04 Longevity benchmarks

- Goal: 1000x format/hover と 100x self-compile を標準 longevity benchmark に固定する。
- Current state: 長寿命 workload の標準セットがコード化されていない。
- Rust source: `docs/runtime-stability-spec.md`
- L# target: `scripts/bench/longevity.sh`, CI benchmark harness
- Implementation direction: format 1000 回, hover 1000 回, self-compile 100 回を同一プロセスで回し、CI では 1/10 スケール版を使う。対象プロジェクトは selfhost source に固定する。
- Dependencies: `GC-02`
- Acceptance: benchmark harness が CI 用簡易モードと手元用詳細モードを両方持つ。
- Evidence: `benchmark-results/*.json`, `scripts/bench/longevity.sh`

<a id="gc-05-lsp-soak-and-repl-gc"></a>
### GC-05 LSP soak and REPL GC

- Goal: LSP と REPL の長寿命挙動を GC 前提で検証する。
- Current state: LSP soak test と stateful REPL GC テストがない。
- Rust source: `docs/runtime-stability-spec.md`
- L# target: LSP server, REPL implementation, test harness
- Implementation direction: LSP は `open -> edit -> diagnostics -> hover -> completion` を 1000 サイクル、REPL は single-session 500 eval を固定 workload にする。両方とも同じ root API を使用する。
- Dependencies: `LSP-02`, `GC-02`
- Acceptance: GC 有効で soak test を完走し、crash/trap/unreachable が出ない。
- Evidence: `test_runtime_lsp_soak_*`, `test_runtime_repl_gc_*`

<a id="gc-06-leak-detection-and-metrics"></a>
### GC-06 Leak detection and metrics

- Goal: leak suspect 検知と metrics 出力を CI gate にする。
- Current state: `peak RSS`, `heap bytes`, `live object count`, `GC pause`, `full GC count` の収集系が未整備。
- Rust source: `docs/runtime-stability-spec.md`
- L# target: runtime metrics collector, CI jobs
- Implementation direction: CI では `peak RSS` と `full GC count` だけを fail threshold に使い、手元実行では live object count と pause histogram まで出す。 leak suspect は tag ごと単調増加を検出して stderr 出力する。
- Dependencies: `GC-03`, `GC-04`, `GC-05`
- Acceptance: runtime metrics が CI artifact 化され、S14-S16 を機械判定できる。
- Evidence: `benchmark-results/*.json`, `test_runtime_metrics_*`

## WS-OPS CI / release / removal

<a id="ops-01-ci-gate-v2-job-graph"></a>
### OPS-01 CI gate-v2 job graph

- Goal: CI の主経路を L# ベース job graph へ切り替える。
- Current state: bootstrap 強化は一部済みだが、job graph 全体は未再編。
- Rust source: `docs/ci-migration-spec.md`, `.github/workflows/ci.yml`
- L# target: `.github/workflows/ci.yml`
- Implementation direction: job graph は `bootstrap-wasm -> bootstrap-native -> golden-parity -> release-smoke -> packaging`, `docs` 独立、`ci-gate-v2` 集約に固定する。required checks もこの名前に合わせる。
- Dependencies: `BOOT-04`, `NATIVE-06`, `PKG-01`
- Acceptance: branch protection の required check が `ci-gate-v2` に移行できる。
- Evidence: `.github/workflows/ci.yml`, GitHub branch protection

<a id="ops-02-artifact-policy"></a>
### OPS-02 Artifact policy

- Goal: bootstrap/native/differential/release の artifact 保存規則を固定する。
- Current state: artifact 種別と retention policy が未統一。
- Rust source: `docs/ci-migration-spec.md`
- L# target: `.github/workflows/ci.yml`
- Implementation direction: artifact は `bootstrap-stages`, `bootstrap-diff`, `native-binaries`, `differential-report`, `release-artifacts`, `benchmark-results` に固定し、PR/main/tag ごとの retention day を spec どおりに設定する。
- Dependencies: `OPS-01`
- Acceptance: CI が artifact 名と retention rule を一貫して使う。
- Evidence: `.github/workflows/ci.yml`, Actions artifact 一覧

<a id="ops-03-shadoworacle-lifecycle"></a>
### OPS-03 Shadow/oracle lifecycle

- Goal: legacy Rust を shadow/oracle として計画的に縮退させる。
- Current state: Rust path はまだ主経路と shadow の境界が曖昧。
- Rust source: `docs/ci-migration-spec.md`, `docs/legacy-isolation-spec.md`
- L# target: `.github/workflows/ci.yml`, `legacy-rust-bootstrap/`
- Implementation direction: `shadow-legacy` は PR 2 週間 -> main push 2 週間 -> 削除、`oracle-parity` は exit code mismatch 0 を条件に warning から fail へ上げた後に撤去する。
- Dependencies: `OPS-01`, `META-05`
- Acceptance: shadow と oracle の lifecycle が workflow と docs で一致する。
- Evidence: `.github/workflows/ci.yml`, `docs/legacy-isolation-spec.md`

<a id="ops-04-legacy-isolation"></a>
### OPS-04 Legacy isolation

- Goal: Rust 実装を `legacy-rust-bootstrap/` へ段階的に隔離する。
- Current state: Rust crate はまだ main tree の主経路に存在する。
- Rust source: `docs/legacy-isolation-spec.md`
- L# target: `legacy-rust-bootstrap/`, `Cargo.toml`, mainline tree
- Implementation direction: 削除順は `docs -> lsp -> driver -> wasm -> ir -> types -> syntax` に固定し、各 crate は parity test / golden zero diff / shadow 1 week / ADR / tag を満たしてから isolated へ移動する。
- Dependencies: `CP-04`, `CP-05`
- Acceptance: isolated crate は mainline workspace から外れ、L# 正本 path だけで CI が回る。
- Evidence: `legacy-rust-bootstrap/`, `git tag -l 'legacy-rust-*'`

<a id="ops-05-default-path-migration"></a>
### OPS-05 Default path migration

- Goal: public command の default path を Rust から L# へ切り替える。
- Current state: compatibility matrix の `Default path` はほぼ Rust のまま。
- Rust source: `docs/compatibility-matrix.md`, `docs/legacy-isolation-spec.md`
- L# target: `docs/compatibility-matrix.md`, CLI/LSP entrypoints
- Implementation direction: 切替順は `compile -> check -> parse -> test -> build -> fmt -> lsp -> docs` に固定し、各切替前に parity/golden/smoke を通す。切替後の Rust path は shadow へ下げる。
- Dependencies: `CLI-02`, `LSP-02`, `FMT-01`, `DOC-02`, `OPS-03`
- Acceptance: compatibility matrix の `Default path` が切替順に従って L# へ更新される。
- Evidence: `docs/compatibility-matrix.md`, default path switch PRs

<a id="ops-06-release-playbook"></a>
### OPS-06 Release playbook

- Goal: native release の build/sign/checksum/changelog を自動化する。
- Current state: release playbook は仕様だけで、実 workflow と scripts が未接続。
- Rust source: `docs/release-operations-spec.md`
- L# target: release workflow, `scripts/generate-changelog.sh`, `scripts/ci/verify-signature.sh`
- Implementation direction: stable/nightly の 2 チャネルを固定し、release は `version bump -> CI -> artifact -> checksum -> signing -> smoke -> tag -> GitHub Release` の順に自動化する。
- Dependencies: `PKG-01`, `OPS-01`
- Acceptance: tag push だけで tier1 release artifact, checksums, notes が生成される。
- Evidence: release workflow, GitHub Releases, `checksums-*.txt`

<a id="ops-07-fresh-clone-without-rust"></a>
### OPS-07 Fresh clone without Rust

- Goal: Rust 未導入環境から bootstrap と native release を再現する。
- Current state: fresh clone without Rust の自動検証がない。
- Rust source: `docs/final-removal-spec.md`
- L# target: `make fetch-stage0`, `make bootstrap`, `make native-release`, `make test`, `make release-bundle`
- Implementation direction: `test-fresh-clone` job を追加し、Rust toolchain を含まない container/runner で `fetch-stage0 -> bootstrap -> native-release -> test -> release-bundle` を通す。
- Dependencies: `OPS-06`, `NATIVE-05`
- Acceptance: fresh clone CI が main merge ごとに green になる。
- Evidence: `test-fresh-clone` job, `docs/final-removal-spec.md`

<a id="ops-08-final-removal-and-rollback"></a>
### OPS-08 Final removal and rollback

- Goal: Rust 最終撤去と rollback 手順を decision-complete にする。
- Current state: final removal checklist と rollback plan は仕様だけで、repo state へ未反映。
- Rust source: `docs/final-removal-spec.md`, `docs/completion-criteria.md`
- L# target: root tree, `docs/adr/`, release docs
- Implementation direction: removal は `Cargo.toml`, `Cargo.lock`, `crates/`, `rust-toolchain.toml`, `.cargo/`, workflow 内 `cargo` を段階削除し、同時に `v0.x.y-rust-final` tag, rollback ADR, CI 復旧手順を固定する。
- Dependencies: `OPS-04`, `OPS-05`, `OPS-07`
- Acceptance: Rust 依存ファイルが repo から消え、rollback 手順が release docs と ADR で追える。
- Evidence: `git ls-files '*rs'`, `git tag -l 'v*-rust-final'`, rollback ADR

## Gate 外 / v2

<a id="v2-01-lsp-incremental-sync"></a>
### V2-01 LSP incremental sync

- Goal: full sync 固定の後段最適化として incremental sync を追加する。
- Current state: v1 は full sync を採用。
- Rust source: LSP 3.17 incremental sync semantics
- L# target: `selfhost/LspServer.ls`
- Implementation direction: v1 完了後に text edit diff apply layer を追加し、diagnostic ordering contract は維持する。
- Dependencies: `LSP-01`, `LSP-02`
- Acceptance: incremental sync を有効にしても JSON snapshot と diagnostics order が壊れない。
- Evidence: `test_snapshot_lsp_incremental_*`

<a id="v2-02-formatterlinter-custom-rule-api"></a>
### V2-02 Formatter/linter custom rule API

- Goal: builtin rule 完成後に custom rule API を公開する。
- Current state: v1 では custom rule は scope 外。
- Rust source: `docs/toolchain-parity-spec.md`
- L# target: formatter/linter plugin API
- Implementation direction: AST walker と lint context を public API 化し、config loader から external rule を注入できるようにする。
- Dependencies: `FMT-01`, `LINT-01`
- Acceptance: custom rule を有効化しても builtin rule ordering が変わらない。
- Evidence: RFC doc, `test_unit_lint_plugin_*`

<a id="v2-03-package-manager-distribution"></a>
### V2-03 Package manager distribution

- Goal: Homebrew/apt/scoop 等の package manager 配布を整備する。
- Current state: v1 は公式アーカイブのみ。
- Rust source: `docs/toolchain-parity-spec.md`, `docs/release-operations-spec.md`
- L# target: formula/manifests
- Implementation direction: package manager manifest は release artifact を正本とし、checksum と署名検証を流用する。
- Dependencies: `PKG-01`, `OPS-06`
- Acceptance: 公式アーカイブと package manager の version/checksum が一致する。
- Evidence: formula/manifests, release notes

<a id="v2-04-linux-aarch64-tier2-distribution"></a>
### V2-04 Linux aarch64 tier2 distribution

- Goal: Linux aarch64 を tier2 常設へ上げる。
- Current state: v1 tier1 対象外。
- Rust source: `docs/release-operations-spec.md`
- L# target: release workflow
- Implementation direction: cross build descriptor と smoke test を追加し、artifact 名と checksum 規則は tier1 と同一にする。
- Dependencies: `PKG-01`
- Acceptance: linux-aarch64 artifact が nightly/stable の両方で生成される。
- Evidence: release workflow, artifacts

<a id="v2-05-windows-authenticode-signing"></a>
### V2-05 Windows Authenticode signing

- Goal: Windows 配布物へ Authenticode 署名を導入する。
- Current state: v1 は Windows 署名なし。
- Rust source: `docs/release-operations-spec.md`
- L# target: Windows release workflow
- Implementation direction: signing secret と verify step を release pipeline へ追加し、zip 内 `.exe` に署名済み stamp を残す。
- Dependencies: `PKG-01`, `OPS-06`
- Acceptance: Windows artifact に verify step が追加され、unsigned build を配布しない。
- Evidence: Windows release job, signing logs

<a id="v2-06-region-optimization"></a>
### V2-06 Region optimization

- Goal: GC の補助最適化として region allocator を導入する。
- Current state: precise tracing GC 優先。
- Rust source: `docs/memory-management-roadmap.md`
- L# target: runtime allocator
- Implementation direction: compiler 内短命 scratch object と builtins の一時 buffer だけを region 対象にし、user-visible heap は GC 正本を維持する。
- Dependencies: `GC-03`
- Acceptance: region 無効でも意味論が変わらず、region 有効時に benchmark 改善を確認できる。
- Evidence: `benchmark-results/*.json`, `test_runtime_region_*`

<a id="v2-07-wasmgc-optional-backend"></a>
### V2-07 WasmGC optional backend

- Goal: optional backend として WasmGC を試験導入する。
- Current state: linear memory collector が mainline。
- Rust source: `docs/memory-management-roadmap.md`
- L# target: optional WasmGC backend
- Implementation direction: AST/type/IR は共有し、codegen/runtime ABI だけを backend ごとに分離する。records/ADT/strings から先に移植する。
- Dependencies: `GC-03`, `WASM-06`
- Acceptance: linear memory backend と同じ入力集合で differential benchmark を取れる。
- Evidence: `benchmark-results/wasmgc-*.json`, `test_golden_wasmgc_*`

# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P8, P9-1/2/3/4/6, P10。
> **Phase 11**: ADR-152〜ADR-157 で仕様固定済みだが、実装完了ではない。完了判定は `docs/development/planning/completion-criteria.md`, `docs/development/validation/verification-spec.md`, `docs/development/planning/compatibility-matrix.md` を優先する。
>
> P8-9 (T4-4/T4-5) → ADR-148, P9-6b → ADR-149, P9-6c → ADR-150, P9-6d → ADR-151

---

## Phase 11: Rust 完全撤去

> **直近反映 (2026-03-26)** — 実測・コードベース同期:
> - E2E: `crates/lsharp-wasm/tests/e2e.rs` に `#[test]` **517 件**（通常実行は **516 passed / 1 ignored**）。ブートストラップ検証の主経路は `try_compile_and_run_file` / `compile_and_run_file`（マルチファイル・import）。インラインソース用の `try_compile_and_run` は将来の最小再現テスト用に **残置**（現状 `#[allow(dead_code)]`）。
> - `selfhost/Main.ls` は import-only パイプライン (BOOT-01)。マルチファイル Wasm は `ModuleGraph::topological_sort` でモジュール名・import 名をソートし出力の再現性を担保。複数 `main` 定義がある場合は **最後**の `main` をエントリにする（`crates/lsharp-wasm/src/wasi.rs`）。
> - `Lower.ls` / `LowerPattern.ls` の stage0 stack overflow は `lsharp-types` の `Type::apply_subst` ループ化・Var サイクル打ち切りで解消。`scripts/ci/compile-phase11-inputs.sh` に含め、`KNOWN_BLOCKERS` なし。
> - `test_e2e_bootstrap_stage1_stage2_match` 等は proxy のまま。加え `test_e2e_bootstrap_stage0_oracle_chain_four_way_identity` で Rust oracle 4 連一致を固定。
> - `scripts/ci/compile-phase11-inputs.sh` は known blocker なしで通過。
> - OPS-05 第1段: `scripts/ci/default-path-smoke.sh`、`docs/development/operations/default-path-migration.md`、`crates/lsharp-driver/src/main.rs` の path 予約コメント、CI ジョブ `default-path-smoke`（`ci-gate` / `ci-gate-v2` の必須）、E2E `test_e2e_ops05_default_path_migration` で **`lsharp` バイナリ経路**を blocking 化。
> - `CP-02` type slice は standalone `selfhost/TypeInfer.ls` check を回復しつつ、field access の最小 parity と quote 系 dispatch parity を更新。covered known-record slice では `Int` を返し、shape 不明時は fresh var fallback、direct record literal field access は AST 直読みで具体型を返し、quote / unquote / unquote-splice は inner expr 推論へ委譲する。`test_e2e_selfhost_typeinfer_field_access`, `test_e2e_selfhost_typeinfer_field_access_fallback_var`, `test_e2e_selfhost_typeinfer_field_access_on_record_literal`, `test_e2e_selfhost_typeinfer_quote_expr`, `test_e2e_selfhost_typeinfer_unquote_expr`, `test_e2e_selfhost_typeinfer_unquote_splice_expr`, `test_e2e_selfhost_main_import_only_pipeline`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く小さい slice として direct record literal field access を 2-field case まで広げ、後続 field も AST 直読みで具体型を返せるようにした。`test_e2e_selfhost_typeinfer_field_access_on_record_literal_second_field`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く小さい slice として single-step `computation` expression は最終式の型へ委譲する special case を追加した。`test_e2e_selfhost_typeinfer_computation_single_step_bool`、`test_e2e_selfhost_typeinfer_computation_expr`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く小さい slice として `infer-lambda` を AST 実レイアウト `[8, param-count, ...]` に合わせ、0/1/2 引数 lambda を compile-safe に扱えるようにした。`test_e2e_selfhost_typeinfer_lambda_two_params_curried`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - さらに隣接 slice として `infer-defn` も parser 実レイアウト `[20, name-hash, param-count, ...]` に追従させ、0/1/2 引数 top-level `defn` を compile-safe に扱えるようにした。`test_e2e_selfhost_typeinfer_defn_two_params_curried`、`test_e2e_selfhost_typeinfer_lambda_two_params_curried`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く隣接 slice として `infer-apply` も 3 引数までの covered slice を追加し、curried function type を 3 段目まで辿れるようにした。`test_e2e_selfhost_typeinfer_apply_three_args_curried`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く隣接 slice として `infer-lambda` / `infer-defn` は 3 引数までの covered slice へ拡張し、3 段のカリー化関数型を返せるようにした。`test_e2e_selfhost_typeinfer_lambda_three_params_curried`、`test_e2e_selfhost_typeinfer_defn_three_params_curried`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - さらに `infer-computation` も 2-step covered slice を追加し、`let!` binder を最後の式へ渡し、`do!` は subst を進めた上で最後の式型へ委譲できるようにした。`test_e2e_selfhost_typeinfer_computation_let_bang_bool_binder`、`test_e2e_selfhost_typeinfer_computation_do_bang_bool_return`、`test_e2e_selfhost_typeinfer_computation_single_step_bool`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く隣接 slice として `infer-apply` を 4 引数までの covered slice へ拡張し、4 段の curried function type も末尾の戻り型まで辿れるようにした。`test_e2e_selfhost_typeinfer_apply_four_args_curried`、既存 `test_e2e_selfhost_typeinfer_apply_three_args_curried`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く隣接 slice として `infer-lambda` / `infer-defn` を 4 引数までの covered slice へ拡張し、4 段のカリー化関数型も返せるようにした。`test_e2e_selfhost_typeinfer_lambda_four_params_curried`、`test_e2e_selfhost_typeinfer_defn_four_params_curried`、既存 3 引数系 regression、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く隣接 slice として `infer-computation` を 3-step covered slice へ広げ、`let! -> do! -> return` と `do! -> let! -> return` の両方で subst / binder env を最後の式へ渡せるようにした。`test_e2e_selfhost_typeinfer_computation_let_bang_do_bang_return_bool`、`test_e2e_selfhost_typeinfer_computation_do_bang_let_bang_return_bool`、既存 computation regression、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く隣接 slice として `infer-do` に 6 式 covered slice を追加し、6 個目の式まで subst を通しつつ最後の式型を返せるようにした。`test_e2e_selfhost_typeinfer_do_six_exprs_last_bool`、`cargo run --quiet -- check selfhost/TypeInfer.ls`、`test_e2e_selfhost_main_import_only_pipeline`、`test_e2e_selfhost_pipeline_complete_stages`、`cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く error parity slice として direct apply の `E0005` に加え、lambda / top-level `defn` / `let` init / `do` subexpression / `computation` step / `match` body・scrutinee failure でも `propagate-error-result` で nested error code を保持するようにした。`test_e2e_selfhost_typeinfer_error_infinite_type_code`, `test_e2e_selfhost_typeinfer_error_lambda_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_defn_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_let_propagates_infinite_init_code`, `test_e2e_selfhost_typeinfer_error_do_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_computation_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_match_propagates_infinite_body_code`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_main_import_only_pipeline`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く slice として record 系 wrapper (`infer-record-fields` / `infer-recordlit` / `infer-fieldaccess` / `infer-recordupdate-node`) でも nested `E0005` を保持し、`match` pattern では undefined constructor と constructor/record child pattern failure が `E0001` のまま観測できるようにした。`infer-pattern-children` は child error code を subst metadata に退避する safer な shape に整理し、standalone `cargo run --quiet -- check selfhost/TypeInfer.ls` を壊さずに `test_e2e_selfhost_typeinfer_error_record_literal_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_field_access_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_record_update_propagates_infinite_code`, `test_e2e_selfhost_typeinfer_error_match_undefined_constructor_pattern_code`, `test_e2e_selfhost_typeinfer_error_match_constructor_child_pattern_code`, `test_e2e_selfhost_typeinfer_error_match_record_child_pattern_code`, `test_e2e_selfhost_main_import_only_pipeline`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` を再 green に戻した。
> - 続く slice では constructor pattern が実際にコンストラクタ引数型を消費しながら subpattern を unify するようになり、arity mismatch で `E0006`、unary constructor binder では body 側へ具体型を渡せるようにした。あわせて `crates/lsharp-types/src/infer.rs` の `infer_decl_functions` で generalize 時に未確定 top-level placeholder をまとめて除外し、import-based standalone check の under-generalize を抑止した。`test_e2e_selfhost_typeinfer_error_match_constructor_arity_mismatch_code`, `test_e2e_selfhost_typeinfer_match_constructor_pattern_binder`, `test_e2e_multi_file_import_open_polymorphic_helper_stays_generalized`, `test_check_selfhost_typeinfer_standalone_import_path`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_main_import_only_pipeline`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く evidence slice として `match` の arm 同士の結果型不一致、および scrutinee/pattern 型不一致がどちらも `E0006` を返すことを固定した。`test_e2e_selfhost_typeinfer_error_match_arm_result_mismatch_code`, `test_e2e_selfhost_typeinfer_error_match_pattern_scrutinee_mismatch_code` を追加し、その時点のテスト数は **493** / **492 passed + 1 ignored** になった。
> - 続く deterministic ordering slice として `selfhost/TypeScheme.ls` の `free-vars` / `generalize` / `instantiate` を source-order の再帰 helper へ置き換え、4 変数関数型でも束縛順と fresh 化順が崩れないようにした。`test_e2e_selfhost_typescheme_generalize_preserves_four_var_order`, `test_e2e_selfhost_typescheme_instantiate_rewrites_all_bound_vars`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_check_selfhost_typeinfer_standalone_import_path`, `test_e2e_selfhost_main_import_only_pipeline`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` が再 green。
> - 続く record deterministic slice として record field 型も `TypeScheme` の `free-vars` / `instantiate` が左から辿るようにし、record field の自由変数順と fresh 化順が安定するようにした。`test_e2e_selfhost_typescheme_generalize_record_field_vars`, `test_e2e_selfhost_typescheme_instantiate_record_field_vars` を追加し、その時点のテスト数は **497** / **496 passed + 1 ignored** になった。
> - 続く syntax slice として selfhost `Parser.ls` の `match` arm pattern に最小 `parse-pattern-v3` を導入し、`_` を `ast-pat-wildcard` として返すようにした。`test_e2e_selfhost_parser_match_wildcard_pattern`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` を再通過し、その時点のテスト数は **498** / **497 passed + 1 ignored** になった。
> - 続く syntax/type slice として同じ helper を symbol pattern にも広げ、selfhost parser が `match` arm の通常 symbol を `ast-pat-var` で返し、selfhost `TypeInfer` でも canonical `ast-pat-var` binder を body 側へ渡せるようにした。`test_e2e_selfhost_parser_match_var_pattern_tag`, `test_e2e_selfhost_typeinfer_match_pat_var_tag_binder`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` を再通過し、その時点のテスト数は **500** / **499 passed + 1 ignored** になった。
> - 続く syntax/type slice として selfhost parser の `match` pattern で parenthesized constructor / brace record も canonical `ast-pat-constructor` / `ast-pat-recordpat` へ寄せ、selfhost `TypeInfer` でも canonical `ast-pat-constructor` / `ast-pat-recordpat` binder と error path を legacy shape と同様に扱えるようにした。`test_e2e_selfhost_parser_match_constructor_pattern_tag`, `test_e2e_selfhost_parser_match_record_pattern_tag`, `test_e2e_selfhost_typeinfer_match_pat_record_tag_binder`, `test_e2e_selfhost_typeinfer_match_pat_constructor_tag_binder`, `test_e2e_selfhost_typeinfer_error_match_pat_constructor_tag_undefined_code`, `test_e2e_selfhost_typeinfer_error_match_pat_constructor_tag_arity_code`, `test_e2e_selfhost_typeinfer_error_match_pat_record_tag_child_code`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` を再通過し、最新テスト数は **507** / **506 passed + 1 ignored** になった。
> - 続く syntax/type slice として selfhost parser の int/bool literal pattern も canonical `ast-pat-lit` へ寄せ、selfhost `TypeInfer` でも `[42, lit-node]` を int/bool 型へ戻せるようにした。`selfhost/LowerPattern.ls` も nested literal payload 前提へ同期し、`test_e2e_selfhost_parser_match_literal_pattern_tag`, `test_e2e_selfhost_typeinfer_match_pat_lit_tag`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_pipeline_complete_stages`, `cargo build && cargo test && cargo clippy --quiet && bash scripts/audit_docs.sh` を再通過し、最新テスト数は **509** / **508 passed + 1 ignored** になった。
> - 続く evidence slice として canonical `ast-pat-lit` が constructor / record child pattern の再帰経路でも保持されることを固定した。`test_e2e_selfhost_parser_match_nested_literal_pattern_tag`, `test_e2e_selfhost_typeinfer_match_constructor_child_pat_lit`, `test_e2e_selfhost_typeinfer_match_record_child_pat_lit` を追加し、最新テスト数は **512** / **511 passed + 1 ignored** になった。
> - 続く syntax/type slice として selfhost parser の `()` unit pattern も canonical `ast-pat-lit` へ寄せ、selfhost `TypeInfer` と `LowerPattern` でも unit payload を扱えるようにした。`test_e2e_selfhost_parser_match_unit_literal_pattern_tag`, `test_e2e_selfhost_typeinfer_match_pat_lit_unit_tag`, `test_e2e_selfhost_typeinfer_match_constructor_child_pat_unit_lit`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_pipeline_complete_stages` を再通過し、最新テスト数は **515** / **514 passed + 1 ignored** になった。
> - 続く evidence slice として nested constructor / record child の unit literal pattern も canonical `ast-pat-lit` として保持されることを固定した。`test_e2e_selfhost_parser_match_nested_unit_literal_pattern_tag`, `test_e2e_selfhost_typeinfer_match_record_child_pat_unit_lit`, `cargo run --quiet -- check selfhost/TypeInfer.ls`, `test_e2e_selfhost_pipeline_complete_stages` を再通過し、最新テスト数は **517** / **516 passed + 1 ignored** になった。
> - 残る大物: true bootstrap（stage1.wasm → stage2 → stage3）、native 自己再生成、Wasm/native 観測差分ゼロ、CLI/LSP/fmt/doc の公開契約 parity、stateful LSP/REPL + GC fixed-point CI、**Cargo.toml 不在までの撤去**（P11-2e-3）。

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
> `docs/development/planning/runtime-stability-spec.md`, `docs/development/operations/ci-gate-v2-job-graph.md`, `docs/development/operations/artifact-policy.md`,
> `docs/development/operations/default-path-migration.md`, `docs/development/operations/release-playbook.md`, `docs/development/operations/fresh-clone-spec.md`,
> `docs/development/operations/adr-rust-removal.md`, `docs/development/operations/rollback-procedure.md`,
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

- [~] [`BOOT-04 True stage1-stage2-stage3 bootstrap`](docs/development/planning/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap) -- 4 層比較テスト (`test_e2e_bootstrap_four_layer_comparison`) + ステージチェーン検証 (`test_e2e_bootstrap_stage_chain_verification`) を追加し、stage0(Rust) の決定性と stage1.wasm の実行可能性までは確認済み。**ただし** 依然として proxy / structural check であり、stage1.wasm が自分で stage2/stage3 を生成する true self-bootstrap fixed point は未接続。既存 proxy + oracle テストは維持。
- [x] [`WASM-03 Deterministic LEB emit`](docs/development/planning/phase11-implementation-plan.md#wasm-03-deterministic-leb-emit) -- マルチファイル決定性 (`ModuleGraph` ソート) + E2E: `test_e2e_wasm03_token_module_compile_deterministic`, 既存 `test_e2e_bootstrap_*deterministic*`。
- [~] Step 2 exit gate -- 4 層比較 + ステージチェーン検証で stage0 決定性と stage1 実行可能性、`docs/development/operations/bootstrap-diff-artifacts.md` の CI diff 仕様追加までは完了。**ただし** stage1→stage2→stage3 の実体生成と self-bootstrap fixed-point gate は未了。

#### Step 3. `CP-03` Native parity を閉じる

- [~] [`NATIVE-05 Stage1-native self-regeneration`](docs/development/planning/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration) -- 機能的等価性検証 + stage chain 構造テストを追加。`test_e2e_native_self_regeneration_functional_equivalence` と `test_e2e_native_stage_chain_structure` で Wasm 基準の structural parity / deterministic compile を確認。**ただし** stage1-native→stage2-native→stage3-native の実バイナリ再生成・実行比較は未達。
- [~] [`NATIVE-06 Wasm/native differential`](docs/development/planning/phase11-implementation-plan.md#native-06-wasmnative-differential) -- 5 観測点用 harness、空 allowlist、structural parity テストは追加済み。`test_e2e_wasm_native_differential_five_observation_points`, `test_e2e_differential_allowlist_empty`, `test_e2e_wasm_native_differential_structural_parity`。**ただし** 現状は file structure / diagnostics / Wasm 由来の structural check が中心で、実 native 生成物の differential 0 は未証明。
- [x] [`META-05 Differential allowlist registry`](docs/development/planning/phase11-implementation-plan.md#meta-05-differential-allowlist-registry) の完了 -- `tests/differential-allowlist.yaml` が `allowlist: []` であることを `test_e2e_meta05_differential_allowlist` で固定。
- [~] Step 3 exit gate -- native structural parity / allowlist registry / proxy differential evidence までは揃った。`test_e2e_native_self_regeneration_functional_equivalence`, `test_e2e_native_stage_chain_structure`, `test_e2e_wasm_native_differential_five_observation_points`, `test_e2e_differential_allowlist_empty`, `test_e2e_wasm_native_differential_structural_parity`。**ただし** true native self-regeneration と Wasm/native zero-diff gate は未通過。

#### Step 4. `CP-04` Public toolchain parity を閉じる

- [~] [`CLI-02 13 command implementations`](docs/development/planning/phase11-implementation-plan.md#cli-02-13-command-implementations) -- `selfhost/Cli.ls` に 13 サブコマンド名、終了コード API (`exit-code-success`/`exit-code-compile-error`/`exit-code-runtime-error`/`exit-code-unknown-command`)、stdout/stderr 分離 (`cli-stdout`/`cli-stderr`)、`run-command`、`format-subcommand-help` と E2E 6 件は追加済み。**ただし** 多くの handler は PoC のままで、公開コマンド契約 / default path 切替を満たす実動作には未達。
- [~] [`LSP-02 10 method parity`](docs/development/planning/phase11-implementation-plan.md#lsp-02-10-method-parity) / [~] [`LSP-03 Diagnostic ordering`](docs/development/planning/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots) -- `selfhost/LspServer.ls` に 10 メソッド名、sort/dedup helper、JSON-RPC helper 相当の関数、E2E 18 件は追加済み。**ただし** hover/definition/references/completion はハッシュ・固定 location・固定 keyword vector など mock 値が中心で、実 JSON-RPC/LSP parity と diagnostic snapshot gate は未達。
- [~] [`FMT-01 Formatter roundtrip`](docs/development/planning/phase11-implementation-plan.md#fmt-01-formatter-roundtrip) / [~] [`DOC-01 Schemas and snapshots`](docs/development/planning/phase11-implementation-plan.md#doc-01-schemas-and-snapshots) -- AST node coverage、決定的 roundtrip/idempotency テスト、`docs/schemas/`、`generate-knowledge` / `generate-review` / `generate-doc-output` / `generate-html` と関連 E2E は追加済み。**ただし** Formatter は formatted text ではなく fingerprint を返し、DocTools は vector/count 構造と placeholder HTML が中心で、公開出力の schema/html parity は未達。
- [~] Step 4 exit gate -- CLI/LSP/FMT/DOC の selfhost source、proxy tests、schema/documentation 追加までは完了。`docs/development/planning/compatibility-matrix.md` でも selfhost source と関連 test は列挙済み。**ただし** public toolchain parity gate（実コマンド挙動、JSON/LSP parity、formatted text、full schema/html）は reopen。

#### Step 5. `CP-05` Runtime stability gate を閉じる

- [~] [`GC-05 LSP soak and REPL GC`](docs/development/planning/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc) -- `test_e2e_gc_light_compile_run_loop`, `test_e2e_gc_compile_run_loop_1000`, `test_e2e_gc_repl_soak_50_eval`, `test_e2e_gc_repl_soak_500_eval` で compile+run / eval loop と alloc 呼び出し付き proxy soak は追加済み。**ただし** 実 LSP server の stateful `open -> edit -> diagnostics -> hover -> completion` や単一 REPL セッション長寿命 GC gate には未達。
- [~] [`GC-06 Leak detection and metrics`](docs/development/planning/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics) -- `test_e2e_alloc_metrics_peak_usage`, `test_e2e_alloc_metrics_monotonic_check`, `test_e2e_alloc_metrics_five_metric_collection`, `test_e2e_alloc_metrics_leak_suspect_detection` と CI gate 仕様 `docs/development/planning/gc-ci-gate-spec.md` は追加済み。**ただし** 現状は structural proxy metric が中心で、S14-S16 を機械判定する blocking CI artifact / GC fixed-point proof は未完成。
- [~] Step 5 exit gate -- compile+run / REPL loop、metrics API、CI gate 文書までは追加済み。**ただし** stateful LSP/REPL soak と GC fixed-point proof が不足しており、`docs/development/planning/runtime-stability-spec.md` S14-S16 を gate complete と呼べる段階ではない。

#### Step 6. `CP-06` の運用文書と第1段 gate を整える

- [~] [`OPS-01 CI gate-v2 job graph`](docs/development/planning/phase11-implementation-plan.md#ops-01-ci-gate-v2-job-graph) / [`OPS-02 Artifact policy`](docs/development/planning/phase11-implementation-plan.md#ops-02-artifact-policy) -- `ci-gate` / `ci-gate-v2` に `default-path-smoke` required job と関連文書は追加済み。**ただし** E2E `test_e2e_ops01_ci_gate_v2` / `test_e2e_ops02_artifact_policy` は主に存在確認で、branch protection の required check 移行と artifact 名 / retention rule の完全一致は未実証。
- [~] [`OPS-05 Default path migration`](docs/development/planning/phase11-implementation-plan.md#ops-05-default-path-migration) -- **第1段のみ完了**: `scripts/ci/default-path-smoke.sh`, `docs/development/operations/default-path-migration.md`（全 13 コマンドの移行マトリクス）, `crates/lsharp-driver/src/main.rs` の path 予約ドキュメント、E2E `test_e2e_ops05_default_path_migration`。`compatibility-matrix.md` の `Default path` は依然として Rust 列が主で、完全な Rust 非依存 default は未了。
- [~] [`OPS-06 Release playbook`](docs/development/planning/phase11-implementation-plan.md#ops-06-release-playbook) / [`OPS-07 Fresh clone without Rust`](docs/development/planning/phase11-implementation-plan.md#ops-07-fresh-clone-without-rust) -- `scripts/release-playbook.sh` と `docs/development/operations/release-playbook.md`、`docs/development/operations/fresh-clone-spec.md` は追加済み。**ただし** tag push だけでの release 自動化、署名、`test-fresh-clone` job、Rust 不要 bootstrap scripts は未実装で、現状は文書化 / 手順草案が中心。
- [~] [`OPS-08 Final removal and rollback`](docs/development/planning/phase11-implementation-plan.md#ops-08-final-removal-and-rollback) -- `scripts/rollback.sh`, `docs/development/operations/rollback-procedure.md`, `docs/development/operations/adr-rust-removal.md` は存在する。**ただし** `adr-rust-removal.md` は提案状態で、`v*-rust-final` tag、Rust workspace 物理撤去、repo からの Rust 依存除去は未了。
- [~] Step 6 exit gate -- 文書整備と `default-path-smoke` の第1段 blocking 化までは確認できる。`docs/development/planning/completion-criteria.md` では Rust 無効化 2 週間安定期間・native-only RC・rollback gate の一部がまだ pending / in-progress で、**actual Rust removal gate は未通過**。

### Phase 11 クリティカルパス現況

- [~] `CP-01 Frontend/bootstrap` -- Step 1 は完了し、WASM-03 / oracle 4 連一致 / 4 層比較 / ステージチェーン検証で stage0 proxy evidence は揃った。**ただし** `test_e2e_bootstrap_four_layer_comparison` と `test_e2e_bootstrap_stage_chain_verification` は true stage1→stage2→stage3 self-bootstrap ではなく、fixed point gate は reopen。Evidence: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer.rs`, `docs/development/operations/bootstrap-diff-artifacts.md`
- [x] `CP-02 Syntax/types parity` -- syntax/type 系テストは 100 件超まで拡大。selfhost parser では全宣言タグ・全式ノード・全パターンタグの parse coverage を達成し、TypeInfer は 7 引数 curried lambda/defn/apply、14 式 do、3-step computation、全 match pattern 種別（wildcard/var/lit/constructor/record）、error code parity（E0001-E0006）、TypeScheme の deterministic ordering まで固定。`infer_decl_functions` の generalize 改修で standalone check / open import polymorphism を維持。E2E **516 passed / 1 ignored**、docs audit **error 0 / warning 1**。Evidence: `docs/development/planning/compatibility-matrix.md`, `tests/golden/syntax/ast_node_map.json`, `selfhost/TypeInfer.ls`, `selfhost/TypeScheme.ls`, `selfhost/Parser.ls`, `crates/lsharp-types/src/infer.rs`
- [~] `CP-03 IR/backend/native` -- Lower/LowerPattern の stage0 compile、NATIVE-05 の structural parity、NATIVE-06 の proxy differential harness と空 allowlist までは揃った。**ただし** stage1-native→stage2-native→stage3-native の実再生成と Wasm/native zero diff は未証明で、native parity gate は reopen。Evidence: `selfhost/NativeCodegen.ls`, `selfhost/NativeEmit.ls`, `selfhost/NativeTarget.ls`, `tests/differential-allowlist.yaml`
- [~] `CP-04 Public toolchain` -- CLI/LSP/FMT/DOC の selfhost source、help/version・sort/dedup・schema/documentation・fingerprint tests は揃った。**ただし** `selfhost/Cli.ls` の PoC handler、`selfhost/LspServer.ls` の mock vector 応答、`selfhost/Formatter.ls` の fingerprint 出力、`selfhost/DocTools.ls` の vector/count 出力により、公開 toolchain parity gate は未通過。Evidence: `selfhost/Cli.ls`, `selfhost/LspServer.ls`, `selfhost/Formatter.ls`, `selfhost/DocTools.ls`, `selfhost/HtmlDoc.ls`, `selfhost/TestRunner.ls`, `crates/lsharp-wasm/tests/e2e.rs`
- [~] `CP-05 Runtime stability` -- GC-05/06 の proxy soak、metrics API、CI gate 文書は追加済み。**ただし** stateful LSP/REPL soak と GC fixed-point proof / blocking CI artifact が不足しており、runtime stability gate は reopen。Evidence: `crates/lsharp-wasm/tests/e2e.rs`, `docs/development/planning/gc-ci-gate-spec.md`
- [~] `CP-06 CI cutover` -- `ci-gate` / `ci-gate-v2` に compile gate + audit-docs + default-path-smoke は構成済み。**ただし** `compatibility-matrix.md` は全 13 CLI コマンドの `Default path` を Rust のまま示し、OPS-06/07/08 acceptance（release 自動化、fresh clone CI、Rust 物理撤去）は文書化止まり。Evidence: `.github/workflows/ci.yml`, `scripts/ci/default-path-smoke.sh`, `docs/development/operations/ci-gate-v2-job-graph.md`, `docs/development/operations/artifact-policy.md`, `docs/development/operations/adr-rust-removal.md`, `docs/development/operations/rollback-procedure.md`, `test_e2e_ops05_default_path_migration`, `test_e2e_ops08_final_removal_rollback`

### Phase 11 実装状態

- [x] [META-02 Completion marker sync](docs/development/planning/phase11-implementation-plan.md#meta-02-completion-marker-sync) -- `TODO.md`, `docs/development/planning/compatibility-matrix.md`, `docs/development/planning/completion-criteria.md` を実装実態へ同期。
- [x] [META-03 Audit-docs gate](docs/development/planning/phase11-implementation-plan.md#meta-03-audit-docs-gate) -- `scripts/audit_docs.sh`, `.github/workflows/ci.yml` で Phase 11 完了矛盾とエビデンス欠落を fail-fast 化。
- [x] [BOOT-03 stdlib direct compile blockers](docs/development/planning/phase11-implementation-plan.md#boot-03-stdlib-direct-compile-blockers) -- `scripts/ci/compile-phase11-inputs.sh` を追加し、bootstrap job で selfhost/stdlib/examples の fixed input set を blocking 化。
- [x] [BOOT-01 Main.ls import path consolidation](docs/development/planning/phase11-implementation-plan.md#boot-01-mainls-import-path-consolidation) -- Evidence: `selfhost/Main.ls` import-only コメント・パイプライン、`crates/lsharp-wasm/tests/e2e.rs`（`compile_and_run_file` / `selfhost_main_path`）。
- [~] [BOOT-04 True stage1-stage2-stage3 bootstrap](docs/development/planning/phase11-implementation-plan.md#boot-04-true-stage1-stage2-stage3-bootstrap) -- 4 層比較 + ステージチェーン検証 + proxy / oracle evidence は追加済み。**ただし** true stage1→stage2→stage3 self-bootstrap fixed point は未接続。Evidence: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer.rs`
- [~] [NATIVE-05 Stage1-native self-regeneration](docs/development/planning/phase11-implementation-plan.md#native-05-stage1-native-self-regeneration) -- structural parity / stage chain テストは追加済み。**ただし** stage1-native→stage2-native→stage3-native の実再生成・実行比較は未達。`test_e2e_native_self_regeneration_functional_equivalence`, `test_e2e_native_stage_chain_structure`。
- [~] [NATIVE-06 Wasm/native differential](docs/development/planning/phase11-implementation-plan.md#native-06-wasmnative-differential) -- 5観測点 harness、空 allowlist、structural parity テストは追加済み。**ただし** 実 native 生成物の zero diff 証明には未達。`test_e2e_wasm_native_differential_five_observation_points`, `test_e2e_differential_allowlist_empty`, `test_e2e_wasm_native_differential_structural_parity`。
- [~] [CLI-02 13 command implementations](docs/development/planning/phase11-implementation-plan.md#cli-02-13-command-implementations) -- `selfhost/Cli.ls` に 13 サブコマンド名、終了コード API、stdout/stderr 分離、`run-command`、`format-subcommand-help` と E2E 6 件は追加済み。**ただし** handler の多くは PoC で、公開コマンド契約 parity は未達。
- [~] [LSP-02 10 method parity](docs/development/planning/phase11-implementation-plan.md#lsp-02-10-method-parity) / [~] [LSP-03 Diagnostic ordering](docs/development/planning/phase11-implementation-plan.md#lsp-03-diagnostic-ordering-and-json-snapshots) -- 10 メソッド名、sort/dedup helper、JSON-RPC helper 相当の関数と E2E 18 件は追加済み。**ただし** 応答は mock vector が中心で、実 JSON/LSP parity と diagnostic snapshot gate は未達。
- [~] [FMT-01 Formatter roundtrip](docs/development/planning/phase11-implementation-plan.md#fmt-01-formatter-roundtrip) -- 全 AST node coverage と fingerprint ベース roundtrip/idempotency テストは追加済み。**ただし** formatted text formatter parity は未達。
- [~] [DOC-01 Schemas and snapshots](docs/development/planning/phase11-implementation-plan.md#doc-01-schemas-and-snapshots) -- `docs/schemas/`、`generate-html` / `generate-knowledge` / `generate-review` / `generate-doc-output`、E2E 7 件は追加済み。**ただし** 出力は vector/count / placeholder HTML が中心で、full schema/html parity は未達。
- [x] DOC-02 HTML template engine library -- L# 製 HTML テンプレートエンジンライブラリ。`selfhost/HtmlTemplate.ls` (エスケープ・DSL・レンダリング 143行)、`selfhost/HtmlLayout.ls` (共通レイアウト 57行) 新規作成。`selfhost/HtmlDoc.ls` を DSL ベースに移行 (106→107行)。`selfhost/DocTools.ls` の render 系関数を削除して責務分離 (324→279行)。E2E テスト **20 件** (HtmlTemplate 12 + HtmlLayout 5 + 統合 3)。既存 DocTools テスト 13 件回帰なし。AC-408〜415 (T4d-3/T4d-4) に貢献。

  **DOC-02a テンプレートエンジンコア** (`selfhost/HtmlTemplate.ls` 新規作成):
  - [x] `html-escape`: `<>&"'` の 5 文字を HTML エンティティに変換 (`&lt;` `&gt;` `&amp;` `&quot;` `&#39;`)
  - [x] `attr-escape`: 属性値用エスケープ (html-escape に加え属性コンテキスト安全)
  - [x] `elem`: `(elem "div" attrs children)` → element ノード `[1, tag-name, attrs-vec, children-vec]` 生成
  - [x] `text`: `(text value)` → エスケープ済みテキストノード `[2, escaped-string]` 生成
  - [x] `raw`: `(raw html-string)` → エスケープなし raw HTML ノード `[3, html-string]` 生成
  - [x] `render-attr`: `[key, value]` → ` key="escaped-value"` 文字列
  - [x] `render-attrs`: attrs vector をループして属性文字列を連結
  - [x] `render-node`: テンプレートノード → HTML 文字列 (tag-id で分岐、children を再帰)
  - [x] `render-children`: children vector を idx ループで render-node を連結
  - [x] `void-element?`: `br/hr/img/input/meta/link` を判定し閉じタグを省略
  - [x] `each`: `(each items render-fn)` → items の各要素に render-fn を適用しノードリストを展開
  - [x] `when`: `(when cond node)` → cond が真の場合のみ node をレンダリング
  - [x] `render-template`: ルートノードを受け取り完全な HTML 文字列を返すエントリポイント
  - [x] `doctype`: `"<!doctype html>"` 定数関数

  **DOC-02b レイアウトテンプレート** (`selfhost/HtmlLayout.ls` 新規作成):
  - [x] `css-inline`: モジュールドキュメント用の最小 CSS 文字列 (外部ファイル依存なし)
  - [x] `base-layout`: `[title, content-node]` → `<!doctype html><html><head>...<body>content</body></html>` の完全 HTML ドキュメントノード
  - [x] `doc-page-layout`: モジュールドキュメント用レイアウト (`<main><h1>title</h1><section id="functions">...<section id="types">...`)
  - [x] `index-page-layout`: モジュール一覧インデックスページ用レイアウト (`<main><h1>modules</h1><ul>...`)

  **DOC-02c HtmlDoc.ls 移行** (既存 `selfhost/HtmlDoc.ls` を HtmlTemplate/HtmlLayout ベースに書き換え):
  - [x] `render-function-signature` / `render-type-definition`: string-concat → `elem`+`text` DSL に置換
  - [x] `render-function-items-loop` / `render-type-items-loop`: 手動再帰ループ → `each` に置換
  - [x] `render-module-page`: string-concat ベタ組み → `doc-page-layout` + `each` で構築
  - [x] `html-header` / `html-footer`: 削除し `base-layout` に統合
  - [x] `render-html`: `base-layout` + `render-module-page` + `render-template` のパイプラインに変更
  - [x] `render-index` / `render-index-items-loop`: `index-page-layout` + `each` に置換
  - [x] 公開 API 互換: `render-html`, `render-module-page`, `render-index` のシグネチャは維持し既存テスト回帰なし

  **DOC-02d DocTools.ls 責務分離** (既存 `selfhost/DocTools.ls` の HTML 生成を HtmlDoc へ委譲):
  - [x] `render-function-entry` / `render-functions-section` / `render-types-section` / `render-doc-body` (L192-241) を HtmlDoc の対応関数呼び出しに委譲
  - [x] DocTools.ls は AST 解析・エントリ抽出・スキーマ出力のみに専念
  - [x] `generate-html` 内の `render-doc-body` 呼び出しが HtmlDoc 経由の DSL レンダリングを使用

  **DOC-02e 統合検証 — DocTools が実 HTML を生成できることの証明**:
  - [x] `cargo run -- check selfhost/HtmlTemplate.ls` が通過 (standalone compile)
  - [x] `cargo run -- check selfhost/HtmlLayout.ls` が通過 (standalone compile)
  - [x] `cargo run -- check selfhost/HtmlDoc.ls` が通過 (import HtmlTemplate + HtmlLayout)
  - [x] `cargo run -- check selfhost/DocTools.ls` が通過 (import HtmlDoc)
  - [x] `generate-html` が `<section id="functions"><ul><li>fn-...</li></ul></section>` 形式の実 HTML body を返す (placeholder ではない)
  - [x] `render-html` が `<!doctype html><html><head>...<body><main>...</main></body></html>` 形式の完全 HTML を返す
  - [x] `render-index` が全モジュール名を `<li>` で列挙した完全な HTML インデックスページを返す
  - [x] HTML 出力に `<>&"'` が混入するソースを与えた場合にエスケープされる (XSS 安全)
  - [x] 同一入力で 2 回実行して diff が空 (AC-408 deterministic)
  - [x] 生成 HTML にタイムスタンプ・ホスト名・絶対パスが含まれない (AC-409)

  **DOC-02f E2E テスト** (`crates/lsharp-wasm/tests/e2e/` に追加):
  - [x] HtmlTemplate 系 12 件: escape (lt/gt/amp, quotes, passthrough), elem (basic, attrs, void, nested), each, when-true, when-false, raw, deterministic
  - [x] HtmlLayout 系 5 件: base-doctype, base-charset, title-escaped, doc-page, index-page
  - [x] 既存 DocTools 回帰: `test_e2e_selfhost_doctools_*` 13 件 + `test_e2e_selfhost_doctools_html_doc_*` が全 green 維持
  - [x] 統合テスト: DocTools.generate-html → HtmlDoc.render-html パイプラインが実 HTML 文字列を返しその string-length > 0
- [~] [GC-05 LSP soak and REPL GC](docs/development/planning/phase11-implementation-plan.md#gc-05-lsp-soak-and-repl-gc) -- proxy soak テスト群は追加済み。**ただし** stateful LSP/REPL 長寿命 GC gate は未達。
- [~] [GC-06 Leak detection and metrics](docs/development/planning/phase11-implementation-plan.md#gc-06-leak-detection-and-metrics) -- metrics API / leak suspect テストと CI gate 仕様は追加済み。**ただし** GC fixed-point を機械判定する blocking artifact / CI は未完成。
- [~] [OPS-05 Default path migration](docs/development/planning/phase11-implementation-plan.md#ops-05-default-path-migration) -- CI + `default-path-smoke.sh` + `default-path-migration.md`（全 13 コマンド移行マトリクス） + `test_e2e_ops05_default_path_migration` で `lsharp` バイナリ経路の第1段 smoke を固定。`compatibility-matrix.md` の `Default path` 切替は未完了で、完全移行（Cargo 不在）は Phase 11 完了後。

### Deferred / v2

> Gate 外タスク。Phase 11 完了判定には含めない。各項目の受入・Evidence は `phase11-implementation-plan.md` の V2-01〜V2-07 節を正とし、着手時に個別ブランチ／PR で切る。

- [x] [V2-01 LSP incremental sync](docs/development/planning/v2-designs/v2-01-lsp-incremental-sync.md) — デザインドキュメント作成済み
- [x] [V2-02 Formatter/linter custom rule API](docs/development/planning/v2-designs/v2-02-formatter-linter-custom-rule-api.md) — デザインドキュメント作成済み
- [x] [V2-03 Package manager distribution](docs/development/planning/v2-designs/v2-03-package-manager-distribution.md) — デザインドキュメント作成済み
- [x] [V2-04 Linux aarch64 tier2 distribution](docs/development/planning/v2-designs/v2-04-linux-aarch64-tier2.md) — デザインドキュメント作成済み
- [x] [V2-05 Windows Authenticode signing](docs/development/planning/v2-designs/v2-05-windows-authenticode-signing.md) — デザインドキュメント作成済み
- [x] [V2-06 Region optimization](docs/development/planning/v2-designs/v2-06-region-optimization.md) — デザインドキュメント作成済み
- [x] [V2-07 WasmGC optional backend](docs/development/planning/v2-designs/v2-07-wasmgc-optional-backend.md) — デザインドキュメント作成済み

---

## 既知の制限事項

### リニアメモリランタイム
> 全項目仕様固定済み → ADR-158
> 詳細: `docs/development/planning/memory-management-roadmap.md` (Phase 0-6) + `docs/development/planning/runtime-stability-spec.md` (P11-5接続)

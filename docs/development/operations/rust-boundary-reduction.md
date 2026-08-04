# Rust 依存境界の縮小

## 目的と対象

L# の通常開発を Rust toolchain や `cargo` の実行待ちから切り離す。対象の product/release target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) のみである。

ここでいう「Rust 不要」は、あらかじめ取得した native stage0 package を使う日常の編集・検査・テスト・Wasm 出力の経路に `cargo`、`rustc`、host の `lsharp` を置かないという意味である。公開 Rust driver の embedded guest 成功時も host `compile_file` を重ねず、失敗時だけ明示的な fallback を使う。Rust workspace の物理削除や、MCP/LSP を含む全 host integration の native 化は含まない。

### 日常開発を Rust なしで開始する判定 (2026-07-17)

Mac Apple Silicon (`aarch64-apple-darwin`) または Linux x86_64 (`x86_64-unknown-linux-gnu`) で、source fingerprint が一致する verified stage0 package が手元にある場合は、`scripts/native-selfhost-dev.sh` の `parse` / `check` / `fmt` / `test` / `compile -o` / `build -o` を Rust toolchain と host `lsharp` なしで日常開発に使ってよい。したがって、L# の対応済み core slice は L# で実際に開発しながら自己ホスト側へ置き換えを進める運用へ移行する。2026-07-17 の Linux x86_64 gate と Mac の historical evidence は履歴として保持するが、current checkout に一致する stage0 package と二 target の current-source gate は未完了である。

再監査（2026-07-18）では、手元の Linux stage0 manifest に必須の `source_commit` がなく、release manifest も `2cf731c458f3902399afa44cb908da932dc32449` の古い source を指していた。したがって、上記の Linux/Mac pass は履歴 evidence として保持するが、現在の `main` に対する current-source stage0 または二 target native gate の証拠には数えない。`scripts/native-selfhost-dev.sh` の provenance 検査を通る stage0 を current checkout ごとに用意するまで、Rust-free daily boundary は仕組みとして利用可能でも、現行 checkout の実行済み release evidence とは扱わない。

これは「Rust を完全に削除してよい」という判定ではない。stage0 の生成・取得・更新、Rust oracle / differential、emergency rollback、component packaging などの host integration、未完の record pattern / ordinary ADT / GADT semantics、standalone I/O の残件は Rust 境界として保持する。未対応機能を変更するときは Rust implementation を oracle とし、native runner の verified command boundary を越える場合は明示的な fallback または外部 tool boundary を使う。

この経路の成立は、自己ホスト実装が L# の全ての型・宣言意味論と parity を持つことを意味しない。現在自己ホストで検証済みの型注釈は `Int` / `Bool` / `String` / `Float` / `Unit` の named primitive、closed named head の再帰的な `TypeApp`、複数引数の関数型、lower-case `TypeExpr::Var` の raw representation である。`Ref (Vector Int)` と `(-> Int String Bool)` は parser から annotation unification まで確認済みであり、`Ref` / `Vector` の source 名は internal type constructor へ解決される。closed non-parametric `type-alias Name Target` は raw target を保存し、source order の prepass で `defn` の param / return signature と式内 `(: expr Alias)` に透過展開する。`Text -> String`、`RefText -> (Ref Text)`、`TextFn -> (-> Text Text)`、`(: "world" Str)` を parser-to-inference bundle で確認した。parametric `type-alias (Name a ...) Target` は parameter と raw target を保存し、source order の prepass で parameter ごとに fresh 型変数を割り当てる。arity が一致する `(Name Arg ...)` は target へ置換展開され、`Id Int -> Int`、`Callback Int String -> (-> Int String)`、`Box String -> (Ref String)` と式内 `(: "text" (Id String))` を確認した。forward closed alias chain も `Later -> LaterTarget -> String` の source-order 非依存な再評価により signature の受理と不一致診断まで確認済みであり、recursive alias は Rust と同じく `E0006` で拒否する parity を確認済みである（`test_e2e_selfhost_parser_forward_type_alias_unifies_signature`、`test_e2e_selfhost_parser_recursive_type_alias_is_rejected`、closed / parametric alias regression、`TypeInfer.ls` parse/check）。通常の `defn` 注釈では、同じ lower-case `TypeExpr::Var` 名を signature 内で共有し、異なる scoped 変数名も独立した fresh 型変数として扱う polymorphic slice を提供する。`id` の Int / Bool 別 call site と、`choose-first` の `a` / `b` を別々に具体化する call site を確認した。GADT variant の raw return type と match arm-local refinement は parser/type inference slice として検証済みだが、GADT exhaustiveness と full runtime parity は未完了である。record pattern runtime は source / ftable の direct field、binder、literal、fallback、nominal mismatch、patch/base chain marker propagation に加え、source / ftable compiler-mode の nonparametric nested record binder、nested literal child、record field 内の nested constructor child まで検証済みだが、検証済み contains/remove string-key slice を超える一般 Map API parity は未完了である。`map-contains?` / `map-remove` / `map-size` は integer key と string literal key の source / ftable actual Wasm slice まで確認済みである。immutable record update は `CompilerMode` と ftable runtime slice で patch Map の recursive fallback と元 record の不変性を検証済みだが、内部表現の `map-size` / 反復まで含む完全な record API parity は未主張である。未完了の意味論を変更・検証する開発では、現時点では Rust implementation を source of truth / oracle として必要とする。

### EC-M1-01 invariant parameter scope parity (2026-07-17)

`succ(x)` の legacy `:invariant (= result (+ x 1))` について、Rust oracle と selfhost TestRunner を同一 fixture で実行する cross-boundary E2E を追加した。Rust oracle は invariant 1 件を生成して 5 deterministic samples を pass し、selfhost Wasm も `1 invariant / passed=1 / actual=5 / diagnostic=0` を返す。これにより、対応済み legacy invariant slice では元関数引数 `x` と暗黙の `result` が同じ scope で扱われることを実行結果で確認した。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_scope`、`test_e2e_selfhost_test_runner_binds_invariant_parameters`、`cargo test -p lsharp-wasm --test e2e selfhost_cli_core::test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_scope -- --nocapture`。

これは EC-M1-01 の parameter-scope verified slice であり、strict Bool、full diagnostic/span parity、structured report、current-source Mac/Linux native artifact/runtime gate は残件である。したがって、この slice の日常開発は Rust なしで進められるが、未完の contract semantics を変更するときは Rust oracle を保持する。

### EC-M1-01 invariant local-let scope (2026-07-18)

Rust `metadata_check` は legacy `:invariant` 内の lexical `let` binding を scope-aware に収集し、`(let [delta 1] (= result (+ x delta)))` の `delta` を未定義変数として誤診断しないようになった。値式を binding 前、body を binding 後に検査し、lambda / match pattern / computation binding も同じ自由変数規則で扱う。さらに selfhost `TestRunner` も local-let の binding scope、lambda body、computation の各 step、match の scrutinee と arm body を同じ規則で走査し、lambda 内の `missing`、`let! delta missing`、arm body の `missing` を `LS1001` として拒否するようにした。computation/match の valid evaluation parity は別の未完了境界として扱う。

Evidence: `test_run_metadata_tests_allows_local_let_binding_in_invariant`、`test_run_metadata_tests_reports_unknown_invariant_variable`、`test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_local_let_scope`、`test_e2e_selfhost_test_runner_reports_unknown_invariant_lambda_variable`、`test_e2e_selfhost_test_runner_reports_unknown_invariant_computation_variable`、`test_e2e_selfhost_test_runner_reports_unknown_invariant_match_variable`、`cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls`（`diagnostics:0`）、lambda/computation/match の各 focused test。これは legacy invariant の lexical scope と `LS1001` diagnostic の verified slice であり、selfhost/native runner の全 contract semantics、diagnostic/span parity、computation/match の valid evaluation、Mac/Linux current-source artifact/runtime gate、EC-M1-01 aggregate は未完了である。

### EC-M1-01 invariant computation direct evaluation slice (2026-07-18)

Rust `Expr::Computation` の Display が parser で再読込できる `(computation ...)` 形式を生成するよう修正した。selfhost `Tools.Test.TestRunner` には、identity 相当の computation builder に対して各 step を順に評価し、`let!` の値を後続 step の環境へ束縛する限定 evaluator を追加した。同一 fixture の Rust oracle と selfhost Wasm は `1 invariant / passed=1 / actual=5 / diagnostic=0` で一致する。

Evidence: `test_computation_display_roundtrips_to_parser_syntax`、`test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_computation`、computation scope / unknown-variable focused tests、`cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls`（`diagnostics:0`）。これは direct identity-builder の `let!` / final value に限定した verified sliceであり、一般の builder bind / return semantics、`do!` の effect semantics、computation runtime 全体、match valid evaluation、diagnostic/span parity、Mac/Linux current-source native artifact/runtime gate、EC-M1-01 aggregate は残件である。未完了の computation semantics を変更するときは Rust oracle を保持する。

### EC-M1-01 invariant match direct evaluation slice (2026-07-18)

Selfhost `Tools.Test.TestRunner` は legacy invariant の `match` について、literal / wildcard / variable pattern を順に照合し、variable pattern の値を arm body の環境へ束縛する限定 evaluator を追加した。`(match x [value (= result (+ value 1))])` は Rust oracle と selfhost Wasm の双方で `1 invariant / passed=1 / actual=5 / diagnostic=0` となり、unknown variable の `LS1001` 診断も維持される。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_match`、`test_e2e_selfhost_test_runner_matches_rust_oracle_for_literal_and_wildcard_match`、`test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_scope`、`test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_local_let_scope`、`test_e2e_selfhost_test_runner_reports_unknown_invariant_match_variable`、`cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls`（`diagnostics:0`）。これは primitive pattern の direct evaluation に限定した verified sliceであり、constructor / record / GADT pattern、exhaustiveness、full match runtime、diagnostic/span parity、Mac/Linux current-source native artifact/runtime gate、EC-M1-01 aggregate は残件である。未完了の match semantics を変更するときは Rust oracle を保持する。

### EC-M1-01 selfhost invariant Bool diagnostic (2026-07-17)

Selfhost `Tools.Test.TestRunner` は invariant を各 deterministic sample で評価し、全ての実値が `Bool` であることを確認するようになった。`(defn succ [x] :invariant (+ x 1) (+ x 1))` は Int を truthy として通さず、`diagnostics:1,LS1002` と failure code `2` を返す。未定義変数の `LS1001` と、Bool invariant の `diagnostics:0` は既存経路で維持する。

Evidence: `test_e2e_selfhost_test_runner_rejects_non_bool_invariant`、`cargo run --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` (`diagnostics:0`)。

これは selfhost runner の strict Bool verified slice であり、Rust `run_metadata_tests` との diagnostic/span parity、structured report、current-source Mac/Linux native artifact/runtime gate は残件である。したがって、日常開発でこの slice を利用できるが、Rust oracle / bootstrap 境界は引き続き保持する。

### EC-M1-01 Rust legacy invariant Bool preflight (2026-07-19)

Rust `metadata_check` は legacy `:invariant` も、元関数を同じ引数で呼ぶ synthetic probe の戻り値を `result` に束縛して型検査するようになった。`(+ x 1)` のような non-Bool invariant は、生成テストの後段 `E0002` に漏れず、invariant 式の source span と owner を持つ `LS1002` 相当の metadata diagnostic で拒否する。既存の unknown variable は `LS1001` のまま維持し、`(>= result 0)` など Bool invariant は受理する。

Evidence: `legacy_invariant_requires_bool_at_invariant_span`、`test_run_metadata_tests_rejects_non_bool_invariant`、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（23 passed）、`cargo test -p lsharp-types --lib metadata_check -- --nocapture`（29 passed）、`cargo test -p lsharp-tooling metadata_test::tests::test_run_metadata_tests_ -- --nocapture`（32 passed）。

これは Rust tooling の legacy invariant strict Bool preflight に限定した verified slice であり、selfhost runner との detailed diagnostic/span parity、structured report、current-source Mac/Linux native artifact/runtime gate、EC-M1-01 aggregate は残件である。対応済み slice は L# で日常開発できるが、未完了 contract semantics の oracle / bootstrap 境界として Rust は保持する。

### EC-M1-02 canonical assert inventory bridge (2026-07-17)

Rust syntax は `:assert [predicate ...]` を lossless な ordered metadata form として parse し、metadata inventory は各 predicate を source 順の `ExecutableContract::Assertion` へ投影するようになった。directive span と predicate span を保持し、canonical form は legacy `pending_migration` へ混ぜない。既存の `:example` / `:invariant` aggregate projection と migration queue は変更していない。

Evidence: RED の `metadata_contract_assert` compile failure、GREEN の `canonical_assert_forms_preserve_order_and_predicate_spans`、`cargo test -p lsharp-syntax -p lsharp-types -p lsharp-tooling -- --nocapture`、`bash scripts/audit_docs.sh`。

これは Rust parser/types の canonical inventory bridge に限定した verified slice であり、selfhost 側の全 parser/formatter/runner parity、assert の型検査・diagnostic report、migration diagnostic、Mac/Linux current-source native artifact/runtime parity は残件である。selfhost の parser/formatter と限定 runner projection は後続の EC-M1-03 slice で個別に検証する。

### EC-M1-02 canonical `:case` parser/inventory bridge (2026-07-17)

Rust syntax が canonical `:case [(expect actual expected) ...]` を lossless form として parse し、各 expectation の actual / expected 式、entry span、directive span を保持するようになった。metadata inventory は source 順に `ExecutableContract::Case` へ投影し、legacy `:example` / `:invariant` projection と混同しない。

Evidence: RED の `canonical_case_metadata_preserves_expectations_and_spans` compile failure、GREEN の同 test、`canonical_case_forms_project_to_ordered_inventory_entries`、`cargo test -p lsharp-syntax --test metadata_inventory -- --nocapture`、`cargo test -p lsharp-types --test metadata_contract_case -- --nocapture`。

これは parser と canonical IR の verified sliceであり、`:case` の実行 runner、selfhost parser/formatter/runner、empty/malformed case diagnostics、Mac/Linux current-source native artifact/runtime parity は残件である。次の実装は、型検査済みの `:case` を runner の実行結果へ materialize し、未対応 case を空の成功として扱わない境界を追加する。

### EC-M1-02 canonical `:case` type preflight (2026-07-17)

Rust metadata checker は canonical `:case` の actual / expected を引数なしの内部 probe として HM 型推論へ渡し、owner の defn parameter を暗黙 capture しない。actual / expected の型不一致、型推論中の未定義変数、Int / Bool 以外の比較対象を metadata diagnostic として拒否する。tooling の `test` 経路では未定義変数を `LS1001`、その他の `:case` 型不整合を `LS1002` として返す。

Evidence: `canonical_case_requires_matching_actual_and_expected_types`、`canonical_case_accepts_int_and_bool_comparisons`、`canonical_case_rejects_unsupported_string_comparison`、`canonical_case_does_not_capture_defn_parameters`、`test_run_metadata_tests_rejects_mismatched_canonical_case_types`、`test_run_metadata_tests_rejects_canonical_case_parameter_capture`。

これは `:case` の type preflight の verified sliceであり、実行 runner/materialization は Rust tooling/Wasmtime 経路で検証済み、selfhost parser/formatter/runner、malformed case diagnostics、Mac/Linux current-source native artifact/runtime parity は残件である。空の `:case []` は `LS2006` として明示拒否し、テスト 0 件の成功を隠さない。

### EC-M1-02 canonical `:case` runner materialization (2026-07-17)

Rust tooling は type preflight 済みの top-level / private `:case` を `GeneratedTest` の ordered `Case` entries へ materialize し、expected value と actual expression を保持する。Wasm test runner は各 entry を `(= actual expected)` として一行ずつ実行し、結果を `succ_case_0` のような安定名で返す。成功 1 件と期待値不一致 1 件を同じ source から確認し、空 case は `LS2006` で拒否する。

Evidence: `test_generate_ordered_canonical_cases`、`test_run_metadata_tests_executes_canonical_cases`、`canonical_case_requires_at_least_one_expectation`、`test_run_metadata_tests_rejects_empty_canonical_case`。

これは Rust runner の verified slice であり、selfhost runner/materialization、module-qualified/native stage0 の current-source artifact/runtime parity、property/assertion との統合は残件である。

### EC-M1-02 selfhost canonical `:case` parser bridge (2026-07-17)

Selfhost `Syntax.AST` / `Syntax.Parser` は canonical `:case [(expect actual expected) ...]` を ordered metadata slot の kind `4` と expectation pair vector として保持する。actual / expected の AST を source 順に保持し、既存の `:example` / `:invariant` / `:assert` form kind と混同しない。embedded selfhost parser check でも current source を受理する。

Evidence: RED の `test_e2e_selfhost_parser_preserves_ordered_case_forms`、GREEN の同 test（`1, 4, 2, 5, 1, 5, 1`）、`cargo run --quiet --bin lsharp -- check selfhost/src/Syntax/Parser.ls`。

これは selfhost parser の verified slice に限定され、empty/malformed case diagnostics、Rust/native inventory parity、Mac/Linux current-source artifact/runtime parity は残件である。runner projection と CLI summary は次の selfhost case runner slice で検証する。

### EC-M1-02 selfhost canonical `:case` runner and CLI slice (2026-07-17)

Selfhost `Tools.Test.TestRunner` は parser-owned kind `4` の expectation pair を `[name, actual-ast, expected-ast, diagnostic]` の case test case へ投影し、`generate-tests` の suite slot `3` で actual / expected を Int / Bool value として比較する。期待値不一致は `passed=0` で返し、空の `:case []` は synthetic failure と `LS2006` にする。`App.Cli` / `App.EmbeddedCli` は `cases:N`、case failure、case diagnostic を既存の examples/invariants/assertions 集計へ追加する。

Evidence: `test_e2e_selfhost_test_runner_materializes_canonical_cases`、`test_e2e_selfhost_test_runner_rejects_empty_canonical_case`（`diagnostics:1,LS2006`）、`test_e2e_selfhost_cli_reports_canonical_cases`、`cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` / `Cli.ls` / `EmbeddedCli.ls`、full CLI GREEN (`362.93s`)。

これは selfhost runner/evaluator と text CLI summary の verified sliceであり、selfhost case の Rust type-check/detailed span parity、malformed case diagnostics、property/assertion との共通 ContractSuite、module-qualified/native stage0 の current-source artifact/runtime parity、Mac/Linux native gate は残件である。Int / Bool 以外を selfhost runner が成功扱いしないための static preflight は Rust/selfhost checker の後続境界として保持する。

### EC-M1-03 selfhost canonical assert parser/formatter bridge (2026-07-17)

Selfhost `Syntax.AST` / `Syntax.Parser` は canonical `:assert [predicate ...]` を ordered metadata slot の kind `3` と predicate vector として保持し、`Tools.Text.FormatterDecl` は source-aware / canonical formatter の両方で `:assert` の grouping と predicate 順を再構成する。legacy `:example` / `:invariant` の metadata slot と runner projection は変更せず、既存 typed metadata の 6-slot contract も回帰で固定した。

Evidence: RED の `test_e2e_selfhost_formatter_roundtrips_canonical_assert_form`、GREEN の同 test、`selfhost_formatter_source_roundtrip` 6 tests、legacy parser metadata regression、`cargo run --bin lsharp -- check selfhost/src/Syntax/Parser.ls` / `FormatterDecl.ls` / `TestRunner.ls`。

これは selfhost parser/formatter の round-trip verified slice に限定され、predicate span、assert の全 type check / diagnostic report、legacy migration diagnostic、Rust/native contract inventory parity、Mac/Linux current-source artifact/runtime gate は残件である。限定 runner projection と実行は次の EC-M1-03 sliceで扱う。

### EC-M1-03 selfhost canonical assert runner projection (2026-07-17)

Selfhost `Tools.Test.TestRunner` は parser-owned ordered form kind `3` の predicate vector を predicate 単位の test case へ投影し、既存の result tuple `[name, passed, actual, diagnostic]` を再利用して strict Bool の deterministic assertion 実行を行う。`generate-tests` は既存の examples/invariants slot を保持したまま assertion slot `2` を追加し、`App.Cli` / `App.EmbeddedCli` は assertion 件数を表示し、failure と diagnostic の集計にも含める。assertion がない既存 source の text output は従来どおりである。

Evidence: RED の `test_e2e_selfhost_test_runner_projects_and_runs_ordered_assertion_forms`、GREEN の同 test、`cargo run --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` / `Cli.ls` / `EmbeddedCli.ls` に加え、`test_e2e_selfhost_cli_reports_canonical_assertions` が full selfhost CLI bundle で `1 passed`（`429.35s`）となった。CLI は `assertions:2`、`failures:0`、exit `0` を返すため、以前の ignored manual gateを通常の focused laneへ昇格した。

これは parser-owned predicate projection と full selfhost CLI summary の verified slice であり、predicate source span、Rust checker/oracle との assertion diagnostic parity、undefined-variable の専用診断、全 AST/runtime の assertion evaluation、legacy migration、Mac/Linux current-source artifact/runtime gate は残件である。したがって `:assert` は selfhost runner と CLI の supported subset で実行可能になったが、EC-M1-03/04 または全機能 Rust-free 完了とは扱わない。

### EC-M1-03 Rust canonical `:assert` runner materialization (2026-07-20)

Rust `metadata_check::generate_tests` は canonical `:assert` の predicate を `TestKind::Assertion` の ordered test entry へ materialize するようになった。Wasm test runner は predicate ごとに Bool を `1/0` へ変換して一行ずつ実行し、tooling の `test` 経路は assertion の pass/fail、安定名、失敗理由を `MetadataTestRun` へ返す。これにより `:assert` 単独の source が `tests.is_empty()` 経由で空の成功になる Rust runner 境界を閉じた。

Evidence: RED の `test_assertion_execution_reports_each_predicate`（生成件数が `0`）、GREEN の同 test（`1 passed`）、`test_run_metadata_tests_executes_canonical_assertions`（`1 passed`、`total=2 / passed=1 / failed=1`）、`test_runner::tests`（8 tests pass）。

これは Rust metadata runner の predicate materialization / execution verified slice であり、checker の全 assertion diagnostic/span parity、selfhost と Rust の differential report、全 AST/runtime evaluator、legacy migration、Mac/Linux current-source artifact/runtime gate は残件である。今回の変更とは独立した既存 property test 2件は `LS2005` で失敗しており、tooling crate 全体の green には含めていない。

### EC-M1-03 Rust/selfhost canonical `:assert` differential slice (2026-07-20)

同一 source の `(truth)` / `(falsehood)` predicate 2件を Rust metadata runner と selfhost `generate-tests-from-source` へ渡し、両者の件数、pass/fail、Bool predicate の diagnostic code `0` を比較する E2E contract test を追加した。Rust oracle の `1 pass / 1 fail` と selfhost runner の同じ結果を current checkout の Wasm harness で確認した。

Evidence: `e2e::selfhost_assertion_spans::selfhost_assertion_results_match_rust_oracle`（`1 passed`、`26.18s`）。既存の selfhost predicate source span / non-Bool diagnostic span test と組み合わせ、assertion の typed preflight、predicate span、runtime result の境界を分離して記録している。

これは同一 fixture の result parity を閉じる verified slice であり、failure message/schema の Rust/selfhost parity、compound/dynamic predicate の全 evaluator、all-form aggregation、legacy migration、Mac/Linux current-source artifact/runtime gate は残件である。

### EC-M1-04 Rust canonical assert type-check bridge (2026-07-17)

Rust `metadata_check::check_metadata` は canonical contract inventory の `ExecutableContract::Assertion` を検査対象へ追加し、各 predicate を元 program の clone に引数なしの HM probe として付加するようになった。assertion は関数引数や `result` を暗黙に束縛せず、推論結果の戻り値が正確に `Bool` であることを要求する。非 `Bool` は assertion の predicate span、推論エラーは predicate 内の error span に `MetadataDiagnostic` を返す。元の AST と既存の legacy metadata 診断は変更しない。

Evidence: RED の `metadata_contract_check` 2ケース、GREEN の同 2ケース（非 `Bool` の型と predicate span、defn parameter を捕捉しない未定義変数診断）、`cargo test -p lsharp-types -- --nocapture`（202 unit tests + integration tests）、対象ファイルの `rustfmt --check`、`git diff --check`。

これは Rust の parser/types oracle に接続した canonical assertion 型検査の verified slice であり、selfhost `check` からの詳細診断 parity、安定した診断 code/schema、全 evaluator/runtime の assertion parity、到達不能 precondition・constant-true property の non-vacuity、Mac Apple Silicon / Linux x86_64 の current-source native artifact/runtime gate は残件である。また inventory 失敗時にこの narrow checker は既存の metadata 診断を優先して変更しないため、inventory の fail-closed 契約を広げる作業も別に必要である。EC-M1-04 や全機能 Rust-free 完了の判定には使わない。

### EC-M1-04 selfhost `check` canonical assertion Bool preflight (2026-07-17)

Selfhost `Types.TypeInferAssertions` は parser-owned canonical `:assert` predicate を既存 `TypeInfer` の AST/環境へ渡す `check-canonical-assertions` を公開し、`App.Cli` と `App.EmbeddedCli` の `run-check-source` から preflight として呼び出すようになった。predicate を実行せずに静的推論し、正確な Bool なら診断 0、Int など Bool 以外なら診断 1 / code `1002`、未定義変数・関数引数の捕捉・predicate 内の推論失敗なら code `1001` を `check` の既存診断集計へ加える。これにより、対応済みの canonical assertion Bool strictness は selfhost の `check` 経路にも入った。

Evidence: RED で未定義の `check-canonical-assertions` を固定した後、`test_e2e_selfhost_metadata_check_rejects_non_bool_canonical_assertion` が selfhost parser/TypeInfer bundle 上で non-Bool `1,1002`、valid Bool `0,0`、undefined `1,1001`、parameter capture `1,1001` を確認、`cargo run --quiet --bin lsharp -- check selfhost/src/Types/TypeInferAssertions.ls` / `Cli.ls` / `EmbeddedCli.ls`、`git diff --check`、`bash scripts/audit_docs.sh`。

同じ E2E で flattened module body、`private` 宣言、nested module を走査し、module/private/Inner module 内の non-Bool predicate と local helper の `Int` 結果をそれぞれ `1,1002` として検出することも確認した。module body は selfhost の AST 表現から専用 program vector へ戻し、module ごとの TypeInfer environment を使う。loop 再帰中の module decl と state は root して、inference allocation による AST 消失を防ぐ。

これは Rust の synthetic HM probe と同じ全体 parity を閉じたものではなく、selfhost `TypeInfer` の解析済み environment を使う narrow static preflight である。predicate span、Rust checker と同じ型エラーの詳細・code/span parity、module name qualification/collision と imported module scope、full CLI bundle の runtime/manual gate、property/assertion の他の non-vacuity、Mac Apple Silicon / Linux x86_64 の native artifact/runtime parity は残件である。この slice だけで EC-M1-04 や全機能 Rust-free 完了とは扱わない。

### EC-M1-04 empty canonical assertion non-vacuity (2026-07-17)

空の canonical `:assert []` と literal `true` predicate を、検査 0 件または実装非依存の成功として扱わない境界を Rust metadata checker / tooling と selfhost checker で揃えた。Rust 側は directive/predicate span を保持した metadata error を返し、tooling の `test` path はそれぞれ `LS2004` / `LS2005` として拒否する。selfhost `check-canonical-assertions` はそれぞれ code `2004` / `2005` を返し、empty case は top-level、module、`private`、nested module で同じ判定にする。

Evidence: `canonical_assertion_requires_at_least_one_predicate`、`canonical_assertion_non_vacuity_qualifies_module_owner`、`canonical_assertion_rejects_literal_true_as_vacuous`、`test_run_metadata_tests_rejects_empty_canonical_assertion`、`test_run_metadata_tests_rejects_literal_true_canonical_assertion`、`test_errors_tool_returns_empty_executable_contract_code`、`test_errors_tool_returns_vacuous_contract_code`、`test_e2e_selfhost_metadata_check_rejects_non_bool_canonical_assertion` 内の empty scope 4ケースと literal true、`cargo run --quiet --bin lsharp -- check selfhost/src/Types/TypeInferAssertions.ls` / `Cli.ls` / `EmbeddedCli.ls`。到達不能 precondition、constant-true property、full CLI bundle、両対応 target の current-source artifact/runtime parity は残件である。

### EC-M1-04 static integer comparison non-vacuity (2026-07-17)

Rust canonical checker と selfhost `TypeInferAssertions` は、整数 literal に対する `=` / `==` / `!=` / `<` / `>` / `<=` / `>=` の静的に常真な比較を `LS2005` 相当の vacuous diagnostic として拒否する。tooling の legacy message 判定も literal `true` 固有文字列に依存せず、vacuous diagnostic を同じ `LS2005` へ forwarding する。`MetadataMigration` は `TypeInferAssertions` を明示 import し、bundle だけでなく EmbeddedCli の module build でも成立する。

Evidence: `canonical_assertion_rejects_statically_true_integer_comparisons_as_vacuous`、`test_run_metadata_tests_rejects_statically_true_integer_comparison`、`test_e2e_selfhost_metadata_check_rejects_non_bool_canonical_assertion` の `= 1 1` fixture、`test_error_reference_doc_mentions_all_mcp_error_codes`、`cargo test -p lsharp-driver mcp_server::tests::test_error_reference_doc_mentions_all_mcp_error_codes -- --nocapture`。

これは integer literal comparison の verified slice に限られる。到達不能 precondition、constant-true property、property sampling、full assertion diagnostic/span parity、Mac Apple Silicon / Linux x86_64 の current-source native artifact/runtime parity は未完了であり、Rust oracle / bootstrap 境界は維持する。

### EC-M1-03 canonical `:property` parser/inventory boundary (2026-07-18)

Rust `Syntax.Parser` は `:property` の typed binder、precondition/postcondition、`cases` / `seed` / `shrink` を `PropertyForm` として保持し、Rust canonical contract inventory は既定 sampling を補った `ExecutableContract::Property` へ射影する。canonical checker は空 binder、`cases=0`、literal `true` postcondition を成功扱いしない。tooling の metadata test runner は valid property を test 0 件へ丸めず、未接続 evaluator の明示境界 `LS3002` を返す。

Selfhost `Syntax.Parser` は ordered form kind `5` と bracket-aware raw payload を保持し、`Tools.Test.TestRunner` は declaration tree の property を検出する。typed projection / sampling / evaluator が未実装の間は、`App.Cli` と `App.EmbeddedCli` の `run-test-source` が既存 preflight 形式で `diagnostics:1,LS3002` と runtime error を返す。

Evidence: `metadata_property` 2件、`metadata_contract_property` 2件、`metadata_contract_check` の property 7件を含む focused tests、`test_run_metadata_tests_rejects_unimplemented_property_runner`、`test_e2e_selfhost_parser_preserves_ordered_property_forms`、`test_e2e_selfhost_runner_reports_unimplemented_property_boundary`、`test_selfhost_cli_sources_route_property_runner_boundary`、`./target/debug/lsharp check selfhost/src/Tools/Test/TestRunner.ls` / `Cli.ls` / `EmbeddedCli.ls`。full CLI bundle の runtime/manual gate は型推論待ち時間が大きいためこの slice の default gate へ追加していない。

これは Rust typed metadata と selfhost raw payload/明示拒否の verified slice であり、selfhost typed binder projection、property non-vacuity の全条件、type-directed sampling/shrink、property evaluator、Rust/selfhost diagnostic/span parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate、formatter の typed projection と docs/public command parity は残件である。したがって EC-M1-03、EC-M1-04、または全機能 Rust-free 完了には使わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-02 selfhost typed deterministic property projection (2026-07-18)

Selfhost `Tools.Test.PropertyRunner` と `Tools.Test.TestRunner` は、移行期の deterministic property projection（`for-all` の 1 個以上の source-order `Int` binder、positive `cases`、任意の単一 bracketed `precondition`、`postcondition`）を `extract-parser-contract-suites` の executable property form へ接続し、`[kind, [binders, preconditions, postcondition, sampling, profile-code]]` へ投影する。lossless な `ordered-forms` は raw payload を維持するため formatter/source-order の境界を壊さない。binder は名前/type hash と type-directed generator marker を持ち、sampling は Rust canonical `SamplingPlan` と同じ順序で cases、seed `0`、generator version、shrink `true`、coverage 件数 `0` を保持する。`seed` など未対応 option は typed default に丸めず、profile code `3002` と空の typed payload で明示拒否する。

Evidence: RED の `test_e2e_selfhost_parser_contract_suite_projects_typed_property_payload`、`test_e2e_selfhost_parser_contract_suite_projects_property_precondition`、`test_e2e_selfhost_parser_contract_suite_projects_multiple_property_binders`、GREEN の同 3 tests、`test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms`、`test_e2e_selfhost_parser_projects_typed_property_sampling_contract`、`test_e2e_selfhost_parser_keeps_typed_property_profile_boundary`、parser regression 4件、runner regression 3件、`./target/debug/lsharp check selfhost/src/Tools/Test/PropertyRunner.ls` / `TestRunner.ls`（各 `Fn` / `diagnostics:0`）。

これは Rust `Property` / `SamplingPlan` の deterministic Int projection に限定した selfhost slice である。複数 precondition、一般の `TypeExpr`、既定 cases `256`、seed/shrink の source option、coverage bucket、binder/predicate 個別 source span、type-directed generator 実装、property evaluator、Rust/selfhost diagnostic parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件であり、EC-M1-02 / EC-M1-03 または全機能 Rust-free 完了には使わない。Rust oracle / bootstrap 境界は維持する。

### EC-M1-03 selfhost canonical `:property` formatter bridge (2026-07-18)

Selfhost `Tools.Text.FormatterDecl` は ordered form kind `5` の raw payload を source-aware / canonical formatter の両方で `:property [...]` として再構成する。typed projection や evaluator が未実装でも、canonical property の binder、sampling options、precondition、postcondition、body を formatter が削除・並べ替えしないことを固定した。

Evidence: RED の `test_e2e_selfhost_formatter_roundtrips_canonical_property_form`、GREEN の同 test、`./target/debug/lsharp check selfhost/src/Tools/Text/FormatterDecl.ls`。これは raw payload round-trip の verified slice であり、selfhost typed projection、property evaluation、diagnostic/span parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。

### EC-M1-03 Rust MCP migration enum/string report (2026-07-18)

Rust driver の MCP `lsharp_check` は、parse/check 結果に加えて legacy `:example` / `:invariant` の migration rows を `migrationDiagnostics` として返す。rows は source order を保ち、`LS2001` / `LS2002` / `LS2003`、owner、`legacy-example-truthiness` / `legacy-invariant-deterministic-smoke`、`docs-only-example` / `assertion` / `property-postcondition` / `manual-review`、LSP の line/character range、message を structured JSON へ射影する。`tools/list` の `lsharp_check.outputSchema` も同じ enum を宣言するため、MCP client が unknown string を成功扱いしない。

Evidence: RED の `test_check_tool_reports_legacy_migration_enum_strings` と `test_check_tool_declares_legacy_migration_output_schema`、GREEN の同 2 tests。fixture `(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))` で `LS2001` 2件と `LS2002` 1件、`(25,33)` / `(61,79)` の source range を確認した。selfhost の既存 `migration` JSON/row projection とは別に、Rust MCP の structured output boundary を閉じた verified slice である。

これは MCP の migration report schema/射影だけを対象とする。Rust-free の selfhost MCP server、全 form の evaluator、全 diagnostic/span parity、EmbeddedCli/MCP の両対応 target artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate、stage0 provenance は残件であり、EC-M1-03 全体または全機能 Rust-free 完了とは扱わない。Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-04 Rust canonical `:property` predicate type-check bridge (2026-07-18)

Rust `metadata_check::check_metadata` は canonical `ExecutableContract::Property` の precondition / postcondition を property binder と synthetic `result` の lexical scopeで HM 型推論へ渡し、戻り値が正確に `Bool` であることを要求する。非 `Bool` は predicate source span と owner を持つ `MetadataDiagnostic` として拒否し、valid Bool predicates は受理する。元の AST と既存の assertion / case checker は変更しない。

Evidence: `canonical_property_requires_bool_postcondition`、`canonical_property_requires_bool_preconditions`、`canonical_property_accepts_bool_predicates_in_binder_scope`、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（14 tests）、`cargo test -p lsharp-syntax --test metadata_property -- --nocapture`（2 tests）。この slice は property の戻り値型を owner の関数型へ結び付ける検査、type-directed sampling/shrink、evaluator、selfhost detailed diagnostic parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の native gateをまだ閉じていないため、EC-M1-04 や全機能 Rust-free 完了には使わない。

### EC-M1-04 selfhost `check` canonical `:property` Bool preflight (2026-07-18)

Selfhost `Types.TypeInferAssertions` は canonical `:property` の raw postcondition を bracket-aware に取り出し、synthetic `result` probe を `TypeInfer` へ渡して戻り値が `Bool` かを `check-canonical-properties-with-analysis` で検査する。non-Bool predicate は `1002` として返し、`App.Cli` / `App.EmbeddedCli` の `check` は assertion / case と同じ diagnostics 集計へ property の件数と専用本文 `property predicate must be Bool` を接続する。`test` の property evaluator は未実装のため、既存の `LS3002` 明示境界は変更していない。

Evidence: RED の `test_selfhost_cli_sources_route_property_runner_boundary` 拡張、GREEN の `test_e2e_selfhost_cli_check_rejects_non_bool_canonical_property`（parser + type inference + selfhost checker の Wasm E2E）、`cargo test -p lsharp-wasm --test e2e selfhost_cli_sources_route_property_runner_boundary -- --nocapture`、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls` / `App/Cli.ls` / `App/EmbeddedCli.ls`。full CLI bundle runtime/manual gate は重いためこの slice では再実行していない。

これは selfhost の postcondition Bool preflight に限定した verified slice であり、typed binder / `precondition` projection、empty/non-vacuous property、type-directed sampling/shrink、property evaluator、Rust/selfhost diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。したがって property を Rust なしで変更・検査できる範囲は増えたが、property 全体または全機能 Rust-free 完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-04 selfhost property first binder/precondition scope (2026-07-18)

Selfhost `Types.TypeInferAssertions` は raw `:property` payload の最初の typed binder を synthetic probe の parameter scope へ投影し、最初の `:precondition` が存在する場合だけ同じ scope で Bool preflight するようになった。precondition が未指定なら postcondition の検査へ進み、non-Bool の precondition / postcondition は `1002`、valid な typed binder + Bool precondition/postcondition は `0` とする。probe と解析結果は native GC の safe point を跨いで root する。

Evidence: `test_e2e_selfhost_cli_check_rejects_non_bool_canonical_property`、`test_e2e_selfhost_cli_check_accepts_typed_property_binder`、`test_e2e_selfhost_cli_check_rejects_non_bool_property_precondition`、`cargo test -p lsharp-wasm --test e2e selfhost_cli_check_rejects_non_bool_canonical_property -- --nocapture`、同 typed binder / precondition の各 focused test、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。実装ファイルは 800 行、括弧深度 0、`git diff --check` を満たす。

これは最初の binder / 最初の precondition に限定した verified slice であり、複数 typed binder / 複数 precondition、binder 名 `result` の衝突、完全な TypeExpr/parser projection、empty/non-vacuous property、type-directed sampling/shrink、evaluator、Rust/selfhost diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。したがってこの範囲は Rust なしの日常開発で利用できるが、property 全体または全機能 Rust-free 完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-04 selfhost property binder/precondition full-list scope (2026-07-18)

Selfhost `Types.TypeInferAssertions` は binder bracket 内の typed binder を source 順に全て synthetic probe の parameter scopeへ投影し、precondition bracket 内の predicate を全て順に Bool preflight するようになった。いずれかの predicate が non-Bool / type error なら最初の error code を返し、全て valid なら postcondition まで検査する。複数 predicate の各 call boundary では `payload` / `expression` を root し、native GC による probe 入力の消失と root stack leak を防ぐ。

Evidence: RED の `test_e2e_selfhost_cli_check_accepts_multiple_typed_property_binders` と `test_e2e_selfhost_cli_check_rejects_non_bool_second_property_precondition`、GREEN の同 2 tests、既存 postcondition / first precondition / typed binder の focused E2E 4件、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。実装ファイルは 800 行、括弧深度 0、`git diff --check` を満たす。

これは raw payload の full-list scope projection に限定した verified slice であり、重複 binder と binder 名 `result` の衝突、nested vector を含む完全な bracket-aware expression parser、TypeExpr / diagnostic span の Rust parity、empty/non-vacuous property、type-directed sampling/shrink、evaluator、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。property 全体または全機能 Rust-free 完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-04 selfhost property empty/non-vacuous preflight (2026-07-18)

Selfhost `Types.TypeInferAssertions` は empty canonical `:property []` を `2007`（property requires a postcondition）として拒否し、postcondition が literal `true` の property を `2005`（vacuous）として拒否するようになった。literal `true` の判定は postcondition に限定し、precondition の optional/full-list traversal と valid typed binder scope は維持する。

Evidence: RED の `test_e2e_selfhost_cli_check_rejects_empty_canonical_property` と `test_e2e_selfhost_cli_check_rejects_vacuous_property_postcondition`、GREEN の同 2 tests、`test_e2e_selfhost_cli_check_accepts_typed_property_binder`、`test_e2e_selfhost_cli_check_rejects_non_bool_canonical_property`、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。Rust oracle 側の `canonical_property_requires_at_least_one_for_all` / `canonical_property_rejects_literal_true_postcondition_as_vacuous` と同じ non-vacuity boundary を確認した。

これは empty property と literal `true` postcondition の verified slice に限られる。typed binder なし、`cases=0`、到達不能 precondition、sampling/shrink/evaluator、diagnostic span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件であり、Rust oracle / bootstrap 境界を維持する。

### EC-M1-04 selfhost property structural non-vacuity (2026-07-18)

Selfhost `Types.TypeInferAssertions` は `for-all` の typed binder が 0 件、または `:cases 0` の property を structural code `2007` で拒否するようになった。`App.Cli` / `App.EmbeddedCli` の diagnostic body も、postcondition だけでなく typed binder と positive case count を要求する共通境界へ更新した。これにより empty property、binder-less property、zero-case property が検査 0 件の成功として隠れない。

Evidence: `test_e2e_selfhost_cli_check_rejects_empty_canonical_property`、`test_e2e_selfhost_cli_check_rejects_property_without_typed_binder`、`test_e2e_selfhost_cli_check_rejects_zero_case_property`、`test_selfhost_cli_sources_route_property_runner_boundary`、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls` / `App/Cli.ls` / `App/EmbeddedCli.ls`、`bash scripts/audit_docs.sh`。

これは structural code `2007` の selfhost verified slice であり、Rust metadata checker の個別 message/span、malformed option、到達不能 precondition、sampling/shrink/evaluator、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。property 全体または全機能 Rust-free 完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-04 property static integer non-vacuity parity (2026-07-18)

Rust `canonical_contract_check` と selfhost `Types.TypeInferAssertions` は、property postcondition の整数 literal に対する `=` / `==` / `!=` / `<` / `>` / `<=` / `>=` の静的に常真な比較を `2005` 相当の vacuous diagnostic として拒否する。Rust は property の postcondition AST を既存の assertion 判定器へ渡し、selfhost は synthetic probe の parsed program を root したまま同じ比較規約を適用する。precondition の static-false reachability は別の境界として後続 slice で扱い、入力に依存し得る predicate の Bool 型検査を維持する。

Evidence: RED の `canonical_property_rejects_statically_true_integer_comparisons_as_vacuous` と `test_e2e_selfhost_cli_check_rejects_statically_true_property_postcondition`、GREEN の同 2 tests、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（18 tests）、empty / literal true / typed binder / binderless / zero-case の selfhost focused E2E、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`、`bash scripts/audit_docs.sh`。残るのは動的または compound な precondition reachability、malformed option、type-directed sampling/shrink、property evaluator、diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate であり、この slice 単独では EC-M1-04 全体または全機能 Rust-free 完了とは扱わない。

### EC-M1-04 literal-false precondition reachability (2026-07-18)

Rust canonical checker と selfhost `check` は、property precondition が正確な literal `false` の場合に `2005` 相当の vacuous diagnostic を返すようになった。Rust は property form の AST span、selfhost は bracket-aware に抽出した expression を対象にする。precondition は `true` の Bool 型検査や入力依存 predicate の検査を継続し、compound logic の恒真/恒偽判定は別の残件としている。

Evidence: RED の `canonical_property_rejects_unreachable_literal_false_precondition` と `test_e2e_selfhost_cli_check_rejects_unreachable_literal_false_precondition`、GREEN の同 2 tests、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。残るのは compound expression の reachability、malformed option、sampling/shrink/evaluator、diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gateである。

### EC-M1-04 static-false integer precondition reachability (2026-07-18)

Rust canonical checker と selfhost `check` は、整数 literal に対する `=` / `==` / `!=` / `<` / `>` / `<=` / `>=` が静的に false となる property precondition を `2005` 相当の vacuous diagnostic として拒否する。比較結果を true / false / unknown に分け、postcondition の static-true 判定と precondition の static-false 判定を混同しない。Rust は AST の比較結果を `Option<bool>` として評価し、selfhost は root 済み synthetic probe AST へ同じ演算子集合を適用する。

Evidence: RED の `canonical_property_rejects_statically_false_integer_preconditions` と `test_e2e_selfhost_cli_check_rejects_statically_false_property_precondition`、GREEN の同 2 tests、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（18 tests）、static-true postcondition / literal-false precondition / valid typed binder の selfhost focused E2E、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。残るのは compound expression の reachability、malformed option、sampling/shrink/evaluator、diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate であり、EC-M1-04 全体または全機能 Rust-free 完了とは扱わない。

### EC-M1-04 annotated-false precondition reachability (2026-07-18)

Rust canonical checker と selfhost `check` は、`(: false Bool)` のように `Ann` で包まれた literal false precondition も annotation を unwrap して `2005` 相当の vacuous diagnostic として拒否する。postcondition の annotated false は拒否せず、precondition 専用の reachability policyを保つ。

Evidence: RED の `canonical_property_rejects_annotated_false_precondition` と `test_e2e_selfhost_cli_check_rejects_annotated_false_property_precondition`、GREEN の同 2 tests、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。残るのは compound expression の reachability、malformed option、sampling/shrink/evaluator、diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate である。

### EC-M1-04 compound boolean precondition reachability (2026-07-18)

Rust canonical checker と selfhost `check` は、Bool literal、integer literal comparison、`and` / `or` の compound predicate を `true` / `false` / `unknown` の三値として評価する。`(and false true)` のように入力へ依存しない false precondition は、compound expression 全体の span を保持した `2005` 相当の vacuous diagnostic として拒否する。一方、unknown operand を含む predicate は到達不能と決め打ちせず、通常の Bool 型検査へ進む。既存の literal / annotated literal / static integer comparison と property postcondition の常真判定も同じ評価器へ統合した。

Evidence: RED の `canonical_property_rejects_compound_false_precondition` と `test_e2e_selfhost_cli_check_rejects_compound_false_property_precondition`、GREEN の同 2 tests、既存 static-false precondition / static-true postcondition の selfhost E2E、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（19 tests）、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。残るのは compound predicate の網羅的 operator/span parity、dynamic predicate の evaluator、malformed option、sampling/shrink、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate であり、EC-M1-04 全体または全機能 Rust-free 完了とは扱わない。Rust oracle / bootstrap 境界は維持する。

### EC-M1-04 unary `not` reachability (2026-07-18)

Rust canonical checker と selfhost `check` は、unary `not` に対しても `true` / `false` / `unknown` の三値評価を共有する。`(not true)` の property precondition は入力へ依存しない false として `2005` 相当の vacuous diagnostic を返し、compound expression 全体の span を保持する。未知の operand は unknown のまま通常の Bool 型検査へ進み、既存の literal、integer comparison、`and` / `or` の判定を変更しない。

Evidence: RED の `canonical_property_rejects_unary_not_true_precondition` と `test_e2e_selfhost_cli_check_rejects_unary_not_true_property_precondition`、GREEN の同 2 tests、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（20 tests）、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。残るのは compound predicate の他 operator/span parity、dynamic predicate の evaluator、malformed option、sampling/shrink、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate であり、EC-M1-04 全体または全機能 Rust-free 完了とは扱わない。Rust oracle / bootstrap 境界は維持する。

### EC-M1-04 invalid `:cases` option boundary (2026-07-18)

Rust `Syntax.Parser` は `:cases -1` と `:cases false` を `non-negative case count` の parse error として拒否する。selfhost `Types.TypeInferAssertions` は raw property payload の `:cases` token が digit でも負号でもない場合を structural code `2007` で拒否し、parse error を検査 0 件の成功へ丸めない。selfhost の raw payload boundary と Rust parser の exact diagnostic layer は異なるため、両方の拒否を別 evidence として保持する。

Evidence: `property_form_rejects_negative_cases`、`property_form_rejects_non_numeric_cases`、`test_e2e_selfhost_cli_check_rejects_negative_property_cases`、`test_e2e_selfhost_cli_check_rejects_non_numeric_property_cases` の RED/GREEN、`./target/debug/lsharp check selfhost/src/Types/TypeInferAssertions.ls`。残るのは unknown option、missing option value、malformed bracket の explicit diagnostic、sampling/shrink/evaluator、diagnostic/span parity、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate である。

### EC-M1-04 unknown property option boundary (2026-07-18)

Rust `Syntax.Parser` は `:cases`、`:precondition`、`:postcondition`、`:seed`、`:shrink` 以外の property option を parse error として拒否する。selfhost `Types.TypeInferAssertions` も、binder後から top-level option の値を bracket/parenthesis 単位で skip しながら同じ option 集合を検査し、option の任意順序、postcondition 後、既知 option 名の prefix を含む未知の `:` token に structural code `2007` を返す。これにより、selfhost `check` が未知 option を無視して `diagnostics:0` を返す silent success を防ぐ。既存の deterministic property runner の profile 外拒否は変更していない。

Evidence: RED の `test_e2e_selfhost_cli_check_rejects_unknown_property_option`（`BEGIN / 0 / 0`）と境界追加時の `seed/postcondition` 未検出、GREEN の同 test と `test_e2e_selfhost_cli_check_rejects_unknown_property_option_at_each_boundary`、`property_form_rejects_unknown_option` / `property_form_rejects_prefixed_option_name` を含む `cargo test -p lsharp-syntax --test metadata_property -- --nocapture`（6 tests）、`cargo test -p lsharp-types --test metadata_contract_check -- --nocapture`（20 tests）、selfhost source `check`、既存 unary-not selfhost regression。残るのは missing option value / malformed bracket の explicit diagnostic、compound option/span parity、dynamic precondition evaluator、sampling/shrink、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate である。

### EC-M1-04 missing property option value (2026-07-18)

Rust `Syntax.Parser` は `:cases` / `:seed` / `:shrink` の scalar 値に加え、`:precondition` / `:postcondition` の list/expression 値が次の option または property 終端で欠落している場合も parse error として拒否する。selfhost `Types.TypeInferAssertions` も既知 option の値開始を検査し、end・閉じ括弧・次の `:` を値欠落として structural code `2007` にする。これにより、欠落した option を後続の valid option だけで成功扱いしない。

Evidence: `test_e2e_selfhost_cli_check_rejects_missing_property_option_value`（`BEGIN` に続く 4 件の `1 / 2007`）、`property_form_rejects_missing_scalar_option_value` / `property_form_rejects_missing_list_and_expression_option_value` を含む `cargo test -p lsharp-syntax --test metadata_property -- --nocapture`（9 tests）。残るのは option-value の詳細 diagnostic/span parity、mismatched delimiter の詳細 span、dynamic precondition evaluator、sampling/shrink、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate である。

### EC-M1-04 unclosed delimiter diagnostic boundary (2026-07-18)

Rust `Syntax.Parser` は property payload の外側 bracket が閉じていない入力を parse error として拒否する。selfhost `Syntax.Parser` は bracket skip の EOF を明示的に停止し、`parse-diagnostics` の前段に `()` / `[]` の delimiter balance scan を追加した。未閉鎖の括弧は `1001`、未閉鎖の角括弧は `1002` の diagnostic record として返し、parse recovery の深い再帰が EOF で無限に進まないようにする。CLI の `App.Cli`、`App.EmbeddedCli`、`App.SmokeCli` は同じ scan を parse diagnostics の入口へ接続している。

Evidence: RED で未閉鎖 property expression の selfhost parse diagnostics が長時間完走しなかった。GREEN の `property_form_rejects_unclosed_outer_bracket`、`test_e2e_selfhost_parser_delimiter_diagnostics_rejects_unclosed_property_expression`（`1 / 1001`）、`cargo test -p lsharp-syntax --test metadata_property -- --nocapture`（9 tests）、`Parser.ls` と各 `App.*` の source check、`bash scripts/audit_docs.sh` を確認した。この slice は未閉鎖 delimiter の停止と明示診断を閉じるが、mismatched delimiter の詳細 span、option-value の詳細 diagnostic/span parity、dynamic precondition evaluator、sampling/shrink、full CLI artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。

### scoped polymorphic `defn` signature (2026-07-15)

`TypeInferFunctions.ls` は `defn` の parameter / return annotation に現れる scoped 名ごとに共有 fresh 型変数を割り当て、通常の型環境で関数を一般化する。これにより `id` を Int と Bool の別 call site で使え、`choose-first [(: x a) (: y b)] : a x` では `a` と `b` を独立に具体化できる。GADT refinement と exhaustiveness は別タスクである。Evidence: `test_e2e_selfhost_scoped_type_var_defn_signature_is_polymorphic`、`test_e2e_selfhost_scoped_multiple_type_vars_defn_signature_is_polymorphic`、`TypeInfer.ls` check。

### `App.Cli` Preview1 output boundary 更新 (2026-07-17)

`App.Cli` の `compile` / `build` Preview1 経路を、旧 `env` import の `build-wasm-bytes-wasi` から、`EmbeddedCli` と同じ guarded `build-wasm-bytes-wasi-standalone` へ切り替えた。file/source compile は standalone 用 function/data base `12` を使い、入力長、data layout、unsupported opcode、空 artifact を compile error として扱う。component target は従来どおり外部 packaging boundary として拒否し、size-only の `compile` / `build` も standalone artifact の実バイト長を使う。Evidence: `test_e2e_selfhost_cli_source_compile_uses_full_program_builder`、`test_e2e_selfhost_standalone_read_file_returns_empty_on_path_open_errno`、`cargo run --bin lsharp -- check selfhost/src/App/Cli.ls`。

この変更は source contract と standalone runtime slice までの証拠であり、Linux x86_64 の current-source stage0 source-file smoke は下記の範囲で pass した。ただし `App.Cli` が生成した artifact の `wasm-tools validate` / standalone runtime、長大入力・未対応言語機能の negative gate はまだ完了扱いにしない。したがって `LEGACY-IO-01` と `LEGACY-BOOT-01` の Rust oracle / bootstrap 境界は維持する。

### Linux current-source App.Cli source-file smoke (2026-07-17)

current checkout `b0e6c73` の selfhost source（App.Cli standalone Preview1 output と EC-M1-02 parser/formatter/runner invariant AST slice を含む）を、provenance を付けた Linux x86_64 stage0 package から Lima VM `lsharp-linux-x86` 内で再生成した。`LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=256` を指定し、`function_start_len=2918` を 12 chunk に分けて transport した。VM は 4 CPU、16 GiB RAM、12 GiB diskで、compiler の最大 RSS は約 12.3 GiB、OOM なしだった。`cargo`、`rustc`、host `lsharp` は blocklist した。

生成後の native program で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を実行し、stdout/stderr、core Wasm header、positive `wasm-size` を確認して pass した。VM workdir は終了時に削除され、disk 使用量は 11 GiB 中 3.1 GiB（30%）に戻った。この evidence は Rust-free daily core development boundary を Linux x86_64 にも広げるが、生成 artifact の `wasm-tools validate` / standalone runtime、4096 bytes 超 read、dynamic root/data/heap layout、component sidecar、public stage0 acquisition は残件である。

### Linux current-source replay memory boundary (2026-07-18)

`5b8cd24a` の packaging/runtime script 変更を含む source commit `b62badf5` の provenance 付き Linux x86_64 stage0 package を Lima VM `lsharp-linux-x86` で再利用し、`LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=256` の source-file smoke を実行した。chunk `0-1280` までは完了したが、`1280-1536` の compiler RSS が約 13.5 GiB（VM 使用可能メモリ約 15 GiB）に達したため、安全境界で停止した。最終 artifact、native `parse/check/fmt/test/compile/build` の完走証拠は生成されていない。

停止後は VM process、lock、workdir を回収し、disk は 11 GiB 中 3.3 GiB 使用（31%）に戻った。この結果は chunk `256` の current-source replay が未完であることだけを示し、既定 chunk `64` または責務分割で再検証すべきことを示す。`LEGACY-IO-01`、`LEGACY-BOOT-01`、`EC-M1-07` は完了扱いにしない。

### LEGACY-RUNTIME-01 object table growth slice (2026-07-18)

WASI codegen の object table base / capacity を mutable runtime globals へ移し、live object metadata が初期容量 `4096` に達したとき Wasm memory の末尾へ容量を倍増して `memory.copy` するようにした。既存の object payload address は移動せず、collector の mark / sweep は更新後の table base を参照する。memory growth failure は現段階では明示 trap の境界として残している。

Evidence: RED の `test_e2e_runtime_object_table_grows_past_initial_capacity`（`alloc_count=4097` に対して `gc_live_alloc_count=4096`）、GREEN の同 test（`alloc_count=4097`、`root_stack_top=4097`、`gc_live_alloc_count=4097`）、collector focused 15 tests、allocator focused 11 tests、`cargo check -p lsharp-wasm`。これは object table の verified growth slice であり、root stack / free-list capacity growth、上限診断、size class、precise sentinel、component/native runtime parity、current-source stage0 gate は残件である。

### LEGACY-RUNTIME-01 free-list growth slice (2026-07-18)

WASI / HTTP codegen の free-list base / capacity を mutable runtime globals へ移し、free-list が初期容量 `4096` に達したとき Wasm memory の末尾へ容量を倍増して既存 entries を `memory.copy` するようにした。allocator の first-fit search と collector の追加先は更新後の base を参照し、payload address は移動しない。memory growth failure は object table と同じく現段階では明示 trap の境界として残している。

Evidence: RED の `test_e2e_runtime_free_list_grows_past_initial_capacity`（`alloc_count=4097` に対して `gc_freed_count=4096`、`gc_free_list_count=4096`）、GREEN の同 test（`gc_freed_count=4097`、`gc_free_list_count=4097`、`gc_live_alloc_count=0`）、`test_e2e_runtime_free_list_growth_reuses_moved_entries`（2 回目の `alloc_count=8194`）、collector focused 15 tests、allocator focused 11 tests、object table growth test、`cargo check -p lsharp-wasm`。これは WASI actual runtime の free-list growth verified slice であり、HTTP/component actual runtime parity、root stack growth、上限診断、size class、precise sentinel、current-source stage0 gate は残件である。

### LEGACY-RUNTIME-01 root stack growth slice (2026-07-18)

WASI / HTTP codegen の root stack base / capacity を mutable runtime globals へ移し、root stack が初期容量 `32768` に達したとき Wasm memory の末尾へ容量を倍増して既存 roots を `memory.copy` するようにした。collector、`root_set`、`root_pop` は更新後の base を参照し、root metadata の移動で heap payload address は移動しない。memory growth failure は明示 trap の境界として残している。

Evidence: RED の `test_e2e_runtime_root_stack_grows_past_initial_capacity`（32769 番目の push で旧 `unreachable`）、GREEN の同 test（`root_stack_top=32769`）、`test_e2e_runtime_root_stack_growth_preserves_root_api`（移動後の `root_set` / `root_pop` が `42` を返す）、4097 rooted object table growth test、root/free-list/collector/allocator focused tests、`cargo check -p lsharp-wasm`。これは WASI actual runtime の root stack growth verified slice であり、HTTP/component actual runtime parity、allocation failure diagnostic、size class、precise sentinel、current-source stage0 gate は残件である。

### LEGACY-RUNTIME-01 bump allocation failure boundary (2026-07-18)

通常の bump allocation が必要ページ数を `memory.grow` で追加できない場合、戻り値を捨てず `-1` を検査して Wasm `unreachable` へ遷移するようにした。これにより、上限超過後に heap pointer を更新して payload 範囲外の allocation address を成功値として返す経路を閉じた。WASI と HTTP は共有 `emit_alloc_func` を使うため、同じ codegen boundary が適用される。

Evidence: RED の `test_e2e_alloc_memory_grow_failure_does_not_return_out_of_bounds_address`（1 MiB `StoreLimits` 下で旧実装が `360960` を返す）、GREEN の同 test（同条件で明示 runtime trap）、通常の `test_e2e_alloc_memory_grow`、object/free-list/root growth と collector focused 22 tests。これは allocation failure の fail-closed trap slice であり、ユーザー向け `LS4002` 診断、free-list size class、sentinel precise discrimination、HTTP/component actual runtime parity、current-source stage0 gate は残件である。

### EC-M1-02 legacy metadata invariant slice (2026-07-17)

Selfhost `Syntax.Parser` は legacy `:invariant` predicate を既存 metadata vector の slot 4 に AST として保持し、`Tools.Text.FormatterDecl` の source-aware 経路は `:doc` と invariant を canonical 順で出力する。既存の `:doc` / `:example` / `:params` / `:returns` の slot と、`DocTools` の docs payload 抽出は維持した。RED で invariant と後続 body が skip payload に吸収される failure を固定し、GREEN で parser AST shape、既存 metadata regression、source-aware formatter の string / float / metadata / invariant 4ケースを pass した。

Evidence: `test_e2e_selfhost_parser_defn_preserves_invariant_metadata`、`test_e2e_selfhost_formatter_format_program_with_source_invariant_metadata`、`cargo test -p lsharp-wasm --test e2e selfhost_parser_metadata_forms::test_e2e_selfhost_parser_defn_preserves`、`cargo test -p lsharp-wasm --test e2e selfhost_formatter_source_roundtrip::test_e2e_selfhost_formatter_format_program_with_source`。

これは selfhost parser/AST/formatter の verified slice であり、EC-M1-02 全体の完了ではない。Rust `MetadataForm` との canonical conversion、selfhost docs payload と `:example` の raw source scan 除去、new `:case` / `:assert` / `:property` / `:postcondition` forms、migration diagnostic、Mac/Linux artifact/runtime parity は残件である。

### EC-M1-02 selfhost parser-owned ordered legacy forms (2026-07-17)

Selfhost `Syntax.Parser` は defn metadata の既存 slot `0..4` を互換形のまま保持しつつ、slot `5` に parser-owned ordered forms を追加した。各 form は `[kind, payload]` で、`:example` は kind `1` と raw payload string、`:invariant` は kind `2` と predicate AST を保持する。同じ directive の繰り返しも source order のまま蓄積するため、後段 consumer が metadata の集約文字列だけに依存せず、directive 順を復元できる。

RED で `:example` / `:invariant` / `:example` が metadata length `5`、forms `0` へ欠落する failure を固定し、GREEN では forms length `3`、kind `1,2,1`、両 example payload、invariant AST、既存 doc/params/returns/invariant regression 4件を確認した。Evidence: `test_e2e_selfhost_parser_defn_preserves_ordered_metadata_forms`、`cargo test -p lsharp-wasm --test e2e selfhost_parser_metadata_forms::test_e2e_selfhost_parser_defn_preserves_`、`cargo run --bin lsharp -- check selfhost/src/Syntax/Parser.ls`。

これは parser-owned legacy bridge の verified slice であり、forms に source span はまだなく、Rust `MetadataForm` / `ContractSuite` への canonical conversion、module/private qualification、new `:case` / `:assert` / `:property` / `:postcondition` forms、Mac/Linux current-source artifact/runtime parity は残件である。

### EC-M1-02 selfhost raw inventory canonical form bridge (2026-07-18)

Selfhost `Tools.Test.TestRunner` の raw `extract-contract-forms` inventory は legacy `:example` / `:invariant` に加えて canonical `:case` / `:assert` / `:property` を同じ ordered form vectorへ収集するようになった。form は `[kind, owner-hash, payload, start, end]` を保持し、legacy payload は AST vector、canonical payload は raw text の一要素 vectorとして型を揃えた。全 5 種類を混在させた fixture で kind 順 `1,4,3,5,2`、canonical payload、全 form の start/end span を selfhost Wasm で確認した。metadata stripping の directive 判定も同じ 5 種類へ揃えた。

Evidence: `test_e2e_selfhost_contract_inventory_includes_canonical_forms`、`test_e2e_selfhost_test_runner_preserves_contract_form_order_and_spans`、`cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls`（`diagnostics:0`）。これは raw source inventory の verified sliceであり、Rust `MetadataForm` と同型の `ContractSuite` / `Example` / `Case` / `Assertion` / `Property` IR、parser-owned canonical metadataとの統合、module/private の qualified owner、predicate/expectation span、migration diagnostic、Mac/Linux current-source artifact/runtime parity は残件である。

### EC-M1-02 selfhost parser-owned contract suite projection (2026-07-18)

Selfhost `TestRunner` は parser が保持した ordered metadata form を `[owner-hash, ordered-forms, executable-forms, pending-migration-forms]` の suite projection へ変換するようになった。canonical `:case` / `:assert` / `:property`（kind `4,3,5`）は executable 側へ、legacy `:example` / `:invariant`（kind `1,2`）は pending migration 側へ分離し、混在 fixture の source order と parser-owned payload shape を保持することを selfhost Wasm E2E で確認した。これは既存の個別 runner bucket を置き換えずに canonical `ContractSuite` の入力境界を固定する移行 sliceである。

Evidence: `test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms`、`cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls`（`diagnostics:0`）。この projection は Rust `ContractSuite` と同一の typed IR ではなく、module-qualified/private owner、docs `Example`、predicate/expectation span、canonical checker/formatter/docs の共通変換、runner の suite 一本化、migration diagnostic、Mac/Linux current-source artifact/runtime parity は残件である。directive 単位の source span は後続の typed form span slice で検証する。

### EC-M1-02 selfhost contract directive span projection (2026-07-18)

Selfhost `Syntax.Parser` の ordered metadata form を `[kind, payload, directive-start, directive-end]` に拡張し、既存 consumer が使う `kind` / `payload` の index を維持した。`TestRunner` の parser-owned `ContractSuite` executable form も同じ directive span を保持するため、raw inventory `[kind, owner, payload, start, end]` と canonical suite form の start/end が一致する。property の typed payload、legacy/canonical form の source order、pending/executable 分離は従来どおりである。

Evidence: `test_e2e_selfhost_parser_contract_forms_keep_directive_spans`、`test_e2e_selfhost_parser_contract_suite_preserves_property_directive_span`、`test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms`、`./target/debug/lsharp check selfhost/src/Syntax/Parser.ls` / `Tools.Test.TestRunner.ls`（各 `diagnostics:0`）、parser 10件・parser-owned suite 5件・runner 5件の focused E2E。RED では parser form が `[kind,payload]` で span を失って `2,0,0` となったが、GREEN では form length `4` と raw inventoryとの start/end 一致を確認した。

これは directive-level span の verified sliceであり、binder/predicate/expectation 個別 span、module-qualified/private owner、Rust canonical `ContractSuite` 全 variant、formatter/docs の span forwarding、diagnostic parity、Wasm artifact/runtime、Mac/Linux current-source native gate は残件である。

### EC-M1-03 selfhost legacy migration row directive span (2026-07-18)

Selfhost `Types.MetadataMigration` の legacy migration row の先頭 7 fields を `[diagnostic-code, disposition, directive-start, directive-end, owner-hash, message, selected-semantics-code]` として確定した。既存の summary が参照する `code` / `disposition` と directive span の index は維持し、parser-owned ordered form の owner hash、directive span、disposition-specific message、selected semantics code（`1=legacy-example-truthiness`、`2=legacy-invariant-deterministic-smoke`）を `:example` の各 expression row と `:invariant` row へコピーする。`legacy-migration-row-detail-text` は row を `LS...|owner=...|selected=...|disposition=...|span=...|message=...` の deterministic detail schema へ変換する。`:example` が複数 expression を含む場合も、同じ legacy directive に属する各 row が同じ directive span・owner・message・semantics code を保持する。あわせて `Syntax.Parser` の `:example` span capture timing を `:invariant` と同じ directive token 位置へ揃えた。

Evidence: RED では rows が `[code, disposition]` の 2 要素で、`:example` row と raw `extract-contract-forms` の span 比較が `0` になった。owner RED では row が 4 要素のままで、owner 比較 3 件が `0` になった。message RED では row が 5 要素のままで、3 種類の message 比較が `0` になった。selected semantics RED では row が 6 要素のままで、`1/2` の比較 3 件が `0` になった。detail schema RED では helper が未定義だった。migration detail aggregation RED では `legacy-migration-detail-summary` が未定義だった。CLI detail wiring RED では旧 `run-check-source` 出力が 4 行のままで、detail 行を含む 5 行期待に対して失敗した。typed message RED では docs-only row の `non-Bool (Int)` 期待に対して旧 message が返った。typed message 実装の初回 bundle は括弧不足の parse RED になり、修正後の GREEN で `test_e2e_selfhost_migration_rows_preserve_legacy_owner_and_directive_spans` は、2 expression の `:example` と `:invariant` の計 3 rows が 7 要素になり、raw inventory の owner/start/end の 9 値、docs-only/assertion/property-postcondition の message 3 値（Int detail 含む）、semantics code 3 値、row detail schema 1 値、detail aggregation 1 値で一致することを、migration 専用 21-module selfhost bundle（実行 54.76 秒）で確認した。JSON projection RED では `legacy-migration-row-detail-json` が未定義だった。JSON projection の span literal / expected brace RED を修正した GREEN では、同テストが row JSON の string equality と Rust `serde_json` parse、code/selected/disposition/message/span numeric field type を確認した（実行 54.12 秒）。JSON array projection RED では `legacy-migration-detail-json-summary` が未定義だった。GREEN では 3 rows の source-order JSON array、Rust `serde_json` array parse、array length 3 を確認した（実行 54.32 秒）。`test_e2e_selfhost_cli_check_reports_legacy_migration_summary` は旧出力の RED（463.72 秒）後、`App.Cli` の `run-check-source` が summary を維持しながら detail 行を返す GREEN（728.17 秒）を確認した。`./target/debug/lsharp check selfhost/src/App/Cli.ls`、`selfhost/src/App/EmbeddedCli.ls`、`selfhost/src/Types/MetadataMigration.ls` は各 `diagnostics:0`。

これは migration row の owner・directive-level span・disposition-specific message・selected semantics code・typed detail text・source-order row JSON object/array projection・`run-check-source` の text output 接続に限定した verified sliceであり、enum/string schema、CLI `check --json` と structured diagnostic/exit code、module/private owner parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。row shape/helper と text/row JSON projection の拡張だけで legacy metadata の migration 完了や全機能 Rust-free 完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-03 selfhost migration enum/string schema fail-closed (2026-07-25)

`Types.MetadataMigration` に migration row の canonical schema validator を追加した。row は少なくとも
`[diagnostic-code, disposition, directive-start, directive-end, owner-hash, message,
selected-semantics-code]` を持ち、diagnostic code は `2001/2002/2003`、disposition は `1..4`、
selected semantics は `1/2` の wire enum だけを受理する。`LS2002` は selected semantics `2`、
それ以外の code は `1` でなければ invalid とする。未知の値を `manual-review`、
`legacy-example-truthiness`、または `LS<number>` へ丸めず、row detail text/JSON/summary text の
projection は空文字で fail-closed に停止する。

Evidence: RED の `test_e2e_selfhost_migration_row_schema_rejects_unknown_enum_values` は未定義の
`legacy-migration-row-schema-valid?` で失敗した。GREEN は同じ migration-only selfhost Wasm bundle
で、valid row `1`、unknown code/disposition/selected semantics 各 `0`、valid detail text の存在、
invalid text/JSON/summary projection の空文字を確認した。Rust `metadata_migration` の typed enum
oracle は変更せず、selfhost Wasm E2E は Rust host compile/run の oracle laneとして記録する。

これは selfhost の row enum/string boundary と fail-closed projection に限定した verified sliceであり、
CLI `check --json` の structured diagnostic/exit code、全 legacy form evaluator、module/private owner
parity、Mac Apple Silicon / Linux x86_64 の current-source native stage0、EC-M1-03 aggregate は残件で
ある。TODO の migration 全体は `[~]` のまま扱い、この sliceだけで legacy metadata migration 完了や
全機能 Rust-free 完了とは宣言しない。

### EC-M1-03 selfhost migration expression span projection (2026-07-18)

`Syntax.Parser` の ordered legacy form に optional field `4` として `:example` の各 top-level expression span、および `:invariant` predicate span を保持するようにした。`Types.MetadataMigration` は row の先頭 7 fields を維持したまま index `7/8` に expression の absolute source `start/end` を追加し、JSON object には `expressionSpan` を追加する。既存の directive-level `span`、owner、message、selected semantics code、text detail は変更しない。複数の bracketed example expression は source order の row と span order を対応させ、scanner は開き括弧を消費した既存 depth 契約と flat span vector の token 数境界を共有する。

Evidence: RED の `test_e2e_selfhost_migration_rows_preserve_expression_spans` は 3 rows が 7 fields で expression span `-1` となった。GREEN は同 test で `(succ 0)` `25..33`、`(= (succ 1) 2)` `34..48`、invariant predicate `(= result (+ x 1))` `61..79` を row index `7/8` から取得した（53.23 秒）。既存 `test_e2e_selfhost_migration_rows_preserve_legacy_owner_and_directive_spans` でも全 row length `9`、directive `span` の互換、JSON `expressionSpan` の numeric fields、`serde_json` parse を確認した（44.70 秒）。`./target/debug/lsharp check selfhost/src/Syntax/Parser.ls`、`selfhost/src/Types/MetadataMigration.ls` は各 `diagnostics:0`。

これは expression-level span projection の verified sliceであり、enum/string schema、全 form evaluator、module/private owner parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。selfhost Wasm E2E は Rust host が compile/run する oracle lane のため、native stage0 の証拠には数えない。legacy migration 完了や全機能 Rust-free 完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

### EC-M1-03 selfhost migration polymorphic manual-review boundary (2026-07-18)

Rust `metadata_migration` と selfhost `Types.MetadataMigration` は、legacy `:example` の expression 型を再帰的に確認し、`Fun` / `App` / `Record` の内部に未確定型変数が残る場合も `LS2003`、`manual-review`、`legacy-example-truthiness` として分類するようになった。selfhost は型変数を `legacy-type-text` へ投影して、silent conversion を拒否した理由を migration message に含める。Rust と selfhost の型変数 allocator は別実装なので、reason 内の variable id は target lane ごとに異なり得るが、diagnostic code、selected semantics、disposition、manual-review boundary は同じ fixture で確認する。

Evidence: `polymorphic_legacy_example_requires_manual_review`、`test_e2e_selfhost_metadata_migration_marks_polymorphic_example_manual_review`。Rust oracle は `LS2003` / `ManualReview` と reason prefix を確認し、selfhost Wasm E2E は `LS2003` / disposition `4` / selected semantics `1` と `型 (t1000) -> t1000 を concrete に確定できません` を確認した。これは Rust host compile/run を使う oracle lane の focused evidence であり、全 enum/string schema、全 form evaluator、module/private owner parity、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate、EC-M1-03 全体または全機能 Rust-free 完了の証拠には数えない。Rust oracle / bootstrap 境界を維持する。

### EC-M1-03 selfhost migration JSON string escaping (2026-07-18)

`Types.MetadataMigration` の `legacy-json-quote` が migration row の JSON object/array projection に使う文字列値を、JSON の quote、backslash、newline、carriage return、tab、backspace、form feed、およびその他の ASCII control escape へ変換するようになった。Types 層から LSP 層へ逆依存させず、migration 層に bounded な escape helper を持たせた。

Evidence: RED の `test_e2e_selfhost_migration_json_quote_escapes_delimiters_and_controls` は quote/backslash/newline/tab が未 escape の text と一致せず失敗した。GREEN は同じ migration-only 21-module bundle を selfhost Wasm へ compile/run し、`\"`、`\\`、`\n`、`\t` を含む JSON quoted value を確認した（61.08 秒）。`./target/debug/lsharp check selfhost/src/Types/MetadataMigration.ls` は `diagnostics:0`、既存の row JSON object/array の `serde_json` parse regression も保持する。

これは generic JSON string escaping の verified sliceであり、JSON schema の enum/string contract、CLI `check --json` の option routing・structured diagnostic・exit code、全 form evaluator、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。selfhost Wasm E2E は Rust host が compile/run する oracle lane であり、native stage0 の証拠には数えない。

### EC-M1-03 selfhost check JSON report source contract (2026-07-18)

`App.Cli.run-check-source` に JSON option の source contract を追加した。option 値 `1` は、text summary の代わりに `command`、推論結果の `type`、`diagnostics` object（`count`、`firstErrorCode`、`message`）、source-order の `migration` JSON array を 1 行で返す。既存の migration row JSON builder を再利用し、診断がない場合も numeric zero と空 message を明示する。

Evidence: RED の `test_e2e_selfhost_cli_check_source_json_returns_structured_migration_report` は option `1` が旧 text 5 行を返し、JSON report 2 行契約に対して失敗した（456.19 秒）。GREEN は同じ selfhost CLI bundle を compile/run し、Rust `serde_json` で report を parse、`command=check`、`type=Fn`、diagnostics zero、migration 3 rows、終了コード `0` を確認した（420.13 秒）。`./target/debug/lsharp check selfhost/src/App/Cli.ls` は `diagnostics:0`。

これは `run-check-source` の verified source sliceであり、実 argv の `check --json` / `--format json` option routing、non-zero diagnostic の exit code、enum/string schema の固定、全 form evaluator、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。selfhost Wasm E2E は Rust host が compile/run する oracle lane であり、native stage0 の証拠には数えない。

### EC-M1-03 selfhost check JSON argv routing (2026-07-18)

`App.Cli` の実 argv dispatch に `check --json` と `check --format json` を接続し、valid option は `run-check-source` の structured report へ渡し、未知 option は compile error exit へ明示的に分岐するようにした。`App.EmbeddedCli` にも同じ `check-json-report` schema、option parser、main dispatch を反映した。actual main は `proc-exit` を使うため、JSON report は stdout 1 行、終了値は stdout に印字せず WASI exit code として観測する。

Evidence: actual argv RED の `test_e2e_selfhost_cli_main_with_args_check_json_file` は、最初に harness の stack overflow を検出したため expanded-stack wrapper を追加し、その後 `--json` 未配線の text stdout を検出した。GREEN は current selfhost CLI bundle を実行し、Rust `serde_json` の `command=check`、`type=Int`、diagnostics zero、空 migration array、stdout 1 行、WASI exit code `0` を確認した（430.13 秒）。`test_e2e_selfhost_cli_main_check_json_aliases` は同じ compiled Wasm から `--json` と `--format json` を実行し、両 report の deep-equal と各 exit code `0` を確認した（388.08 秒）。EmbeddedCli の `test_e2e_selfhost_embedded_cli_check_json_contract_is_present` は builder/parser/option branch/main dispatch の source contract を確認し、`./target/debug/lsharp check selfhost/src/App/Cli.ls` と `EmbeddedCli.ls` は各 `diagnostics:0`。

これは実 argv `check --json` / `--format json` と EmbeddedCli source contract の verified sliceであり、non-zero diagnostic の exit code、enum/string schema、全 form evaluator、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。selfhost Wasm E2E は Rust host が compile/run する oracle lane であり、native stage0 の証拠には数えない。

### EC-M1-03 selfhost check JSON diagnostic exit (2026-07-18)

`run-check-source` と EmbeddedCli の check path に診断件数ベースの exit boundary を追加した。diagnostics が 0 件なら `0`、1 件以上なら compile error `1` を返し、JSON report は従来どおり stdout に残す。actual main では `proc-exit` がこの値を WASI exit code へ伝えるため、structured report の解析と process failure を別々に観測できる。

Evidence: RED の `test_e2e_selfhost_cli_check_source_json_returns_diagnostic_exit` は `(if 42 1 0)` の structured report が valid でも戻り値 `0` のままで失敗した（461.46 秒）。GREEN は同じ source harness で diagnostics count / firstErrorCode / message を保持しつつ戻り値 `1` を確認した（577.41 秒）。actual argv の `test_e2e_selfhost_cli_main_with_args_check_json_diagnostic_exit` は Rust `serde_json` で stdout report を parse し、WASI exit code `1` と stdout 1 行を確認した（415.95 秒）。既存の ignored text diagnostic tests も compile error `1` の期待へ更新した。

これは check JSON の non-zero exit verified sliceであり、enum/string schema、全 form evaluator、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。selfhost Wasm E2E は Rust host が compile/run する oracle lane であり、native stage0 の証拠には数えない。

### EC-M1-02 selfhost typed property runner bridge (2026-07-18)

`extract-property-test-cases` は parser-owned `ContractSuite` の typed property projection を移行期 evaluator の入力へ変換するようになった。Rust `property_smoke_test_spec` と同じく、実行対象は単一の `Int` binder、precondition なし、`cases 1..5`、seed/shrink なしの deterministic profile に限定する。profile 外の複数 binder / precondition / 未対応 option は binder を先頭だけへ縮退させず `LS3002` として明示拒否し、parser projection に保持された canonical payload は失わない。

Evidence: `test_e2e_selfhost_runner_executes_deterministic_property_smoke`、`test_e2e_selfhost_runner_rejects_property_precondition_before_execution`、`test_e2e_selfhost_runner_rejects_multiple_property_binders_before_execution`、`test_e2e_selfhost_runner_rejects_property_seed_option`、`./target/debug/lsharp check selfhost/src/Tools/Test/PropertyRunner.ls`（`diagnostics:0`）、Rust `metadata_contract_check` 18 tests。RED では precondition 付き property が `1,1,1,0` と成功していたが、GREEN では typed contract bridge の execution boundary が `1,0,0,3002` を返すことを確認した。

これは canonical property の parser projection と deterministic smoke runner の接続を閉じた verified sliceであり、複数 binder / 複数 precondition の evaluator、一般 `TypeExpr`、type-directed generator、seed/shrink/coverage、binder/predicate 個別 span、structured report、Wasm artifact/runtime、Mac/Linux current-source native gate は残件である。したがって profile 外の property は Rust fallback で成功させず、Rust oracle/未移行 evaluator 境界を維持する。

### EC-M1-02 selfhost runner invariant AST projection (2026-07-17)

Selfhost `Tools.Test.TestRunner` は declaration tree 内の `defn` metadata vector slot 5 にある ordered kind `2` form を優先し、parser が保持した legacy `:invariant` AST から test case を直接生成する。旧 metadata では slot 4 に fallback する。`generate-tests` も同じ parser AST projection を使用し、`succ(x)` の predicate shape、invariant 1 件の抽出、5 sample の実行結果 `passed=1` を一つの selfhost Wasm E2E で確認した。RED は未定義の `extract-invariants-from-program` により固定し、GREEN は `test_e2e_selfhost_test_runner_extracts_invariant_from_parser_ast` と Rust driver の `check selfhost/src/Tools/Test/TestRunner.ls` で確認した。

これは runner の verified slice であり、EC-M1-02 全体の完了ではない。legacy `:example` / `:invariant` の互換 raw source scanner API は order/span projection 用に残し、module/private の module-qualified owner、Rust `MetadataForm` との ordered canonical conversion、docs payload、new contract forms、migration diagnostic、Mac/Linux artifact/runtime parity は残件である。

### EC-M1-02 selfhost runner example metadata projection (2026-07-17)

Selfhost `Tools.Test.TestRunner` は parser が defn metadata vector slot 5 に保持した ordered legacy forms を優先し、kind `1` の `:example` raw payload だけを `parse-program` で AST 化して top-level `defn` の test case へ投影するようになった。ordered forms がない旧 metadata では slot 1 の集約 payload へ fallback する。`generate-tests` の実行経路もこの projection を使い、`:example` / `:invariant` / `:example` の順序から kind `1,2,1` を確認し、2 example を抽出して両方 `passed=1` になることを selfhost Wasm E2E で確認した。metadata slot 1 は formatter/docs 互換の文字列のままとし、既存の `extract-examples` / `extract-contract-forms` source scanner API は legacy order/span projection 用に残している。

Evidence: `test_e2e_selfhost_test_runner_projects_ordered_example_forms`、`test_e2e_selfhost_test_runner_projects_examples_from_parser_metadata`、`test_e2e_selfhost_test_runner_preserves_example_metadata_across_defn_shapes`、`test_e2e_selfhost_parser_defn_preserves_invariant_metadata`、`cargo run --bin lsharp -- check selfhost/src/Syntax/Parser.ls`、`cargo run --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls`。

これは raw payload を parser-owned ordered metadata から再パースする移行 bridge であり、canonical `ContractSuite` IR ではない。module/private の module-qualified owner、Rust `MetadataForm` との ordered canonical conversion、source span、docs payload、new contract forms、migration diagnostic、Mac/Linux artifact/runtime parity は残件である。

### EC-M1-02 selfhost runner ordered invariant projection (2026-07-17)

Selfhost `Tools.Test.TestRunner` は parser-owned ordered forms の kind `2` を directive 順に走査し、同じ `defn` に繰り返し現れる legacy `:invariant` をそれぞれ test case へ投影する。ordered forms がない旧 metadata では slot 4 の集約 invariant へ fallback する。これにより、slot 4 の「最後の invariant だけ」という互換 accessor による欠落を、parser-owned metadata が存在する経路では防ぐ。

RED では二つの invariant を持つ `succ(x)` が forms 2 件に対して invariant/result 1 件しか生成しない failure `2,2,2,1,1,1,0` を固定した。GREEN では forms 2 件、kind `2,2`、invariant/result 2 件、両方 `passed=1` を最小 selfhost TestRunner bundle の Wasm E2E `test_e2e_selfhost_test_runner_projects_ordered_invariant_forms` で確認し、`cargo run --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` も `diagnostics:0` で通過した。

これは ordered legacy projection の verified slice であり、canonical `ContractSuite` IR、source span、module/private の module-qualified owner、new contract forms、migration diagnostic、Mac/Linux current-source artifact/runtime parity は残件である。

### EC-M1-02 selfhost runner nested declaration projection (2026-07-17)

Selfhost `Tools.Test.TestRunner` は parser AST の module body `[tag, name, body-count, declarations...]` と private wrapper `[tag, inner]` を再帰的に走査し、top-level と同じ ordered kind `1` / `2` metadata projection を適用する。invariant materialization の function lookup も declaration tree を再帰的に探索するため、module/private 内の関数を `generate-tests` から実行できる。

RED では module 内 `succ` と private `pred` の invariant が top-level 走査から欠落し、抽出数/result 数/結果が `0,0,0,0` になる failure を固定した。GREEN では同じ fixture の抽出数 `2`、result 数 `2`、両方 `passed=1` を最小 selfhost TestRunner bundle の Wasm E2E `test_e2e_selfhost_test_runner_projects_nested_invariant_forms` で確認し、`cargo run --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` は `diagnostics:0` で通過した。

これは declaration-tree projection の verified slice であり、module-qualified `SymbolId`、private/export policy、canonical `ContractSuite` IR、source span、new contract forms、migration diagnostic、Mac/Linux current-source artifact/runtime parity は残件である。

### EC-M1-02 selfhost formatter typed metadata projection (2026-07-17)

Selfhost `Tools.Text.FormatterDecl` は typed `defn` の body 直後にある optional signature tag `65` を skip してから metadata vector を読むようになった。これにより、signature を doc payload と誤認せず、typed `defn` の `:doc` / `:example` を source-aware formatter から取得できる。RED で metadata が `0` になる failure を固定し、GREEN で typed metadata 3項目を確認したうえで、string / float / untyped metadata / invariant を含む formatter 5ケースを pass した。Rust driver の selfhost source `check` も `diagnostics:0` で通過している。

Evidence: `test_e2e_selfhost_formatter_extracts_typed_defn_metadata`、`cargo test -p lsharp-wasm --test e2e selfhost_formatter_source_roundtrip::test_e2e_selfhost_formatter -- --nocapture`、`cargo run --bin lsharp -- check selfhost/src/Tools/Text/FormatterDecl.ls`。

これは formatter consumer の typed accessor parity に限定した verified slice である。`DocTools` の同型 accessor、parser-owned ordered forms、Rust `MetadataForm` との canonical `ContractSuite` IR conversion、module/private declaration、new contract forms、migration diagnostic、Mac/Linux current-source artifact/runtime parity は残件である。

### EC-M1-02 selfhost DocTools typed metadata accessor (2026-07-17)

Selfhost `Tools.Doc.DocTools` は typed `defn` の body 後にある optional signature tag `65` を skip して metadata vector を読むようになった。さらに docs/knowledge の declaration range が state から取り出した entries/env を root 保持して再帰へ渡すようになり、GC による Wasm `unreachable` trap を防いだ。巨大 bundle の test thread stack overflowを避けるため、`doctools_parity` の selfhost bundle runner も expanded stack を使う。

RED では typed metadata の fixed-offset 誤認、range recursion の `unreachable`、typed docs payload の trap を固定し、GREEN では source contract、Rust driver `check`、typed accessor、typed inference、range、typed/untyped docs payload、knowledge payload を確認した。Evidence: `test_doctools_typed_metadata_accessor_source_contract`、`test_e2e_doctools_extracts_typed_defn_metadata`、`test_e2e_doctools_infers_typed_defn_for_docs`、`test_e2e_doctools_extracts_doc_function_range`、`test_e2e_doctools_generate_doc_output_typed_function_metadata`、`test_e2e_doctools_generate_doc_output_function_metadata`、`test_e2e_doctools_generate_knowledge_structure`、`cargo run --bin lsharp -- check selfhost/src/Tools/Doc/DocTools.ls`。

これは selfhost docs consumer の typed accessor と range lifetime に限定した verified slice であり、canonical `ContractSuite` IR、parser-owned ordered forms、new contract forms、migration diagnostic、module-qualified docs、Mac/Linux current-source artifact/runtime parity は残件である。

### EC-M1-02 canonical inventory の module body projection (2026-07-17)

Rust `metadata_contract` inventory は `ModuleDecl.body` を再帰走査し、module 内の legacy `:example` / `:invariant` を top-level と同じ `ContractSuite.pending_migration` へ lossless に投影するようになった。RED で module body の suite が 0 件になる欠落を固定し、GREEN では suite 1 件、owner `succ`、pending form 1 件、元 metadata span、空の docs/executable を確認した。module-qualified `SymbolId` や module/private の統合規則はこの slice では変更していない。

Evidence: `module_nested_legacy_forms_are_inventoried_without_loss`、`cargo test -p lsharp-types --test metadata_contract -- --nocapture`。

これは Rust canonical inventory の module traversal に限定した verified slice である。selfhost parser/consumer との canonical conversion、`DocTools` accessor、new contract forms、migration diagnostic、Mac/Linux current-source artifact/runtime parity は残件である。

### 型・宣言意味論の更新 (2026-07-14)

直前の概要にある record 宣言未実装という記述は更新済みである。自己ホスト parser は field 名、`Type.field` accessor 名、raw TypeExpr を保持し、推論 prepass は record schema、constructor、accessor scheme を値環境へ登録して既知 record literal の field 型不一致を診断する。parametric record は `TypeInferRecordDecl.ls` が parameter ごとの bound variable を持つ structural record scheme を登録し、constructor、literal、accessor の使用ごとに scheme を instantiate する。Int field を持つ `Box` と Bool field を持つ `Box` の別使用箇所は独立であり、同じ `Pair a` literal 内の field は同じ具体化を共有する。`(. record field)` は let 束縛後も具体化済み schema の field 型を返し、field 型不一致と未定義 field を診断する。`{record | field value}` update も同じ schema 型へ単一化し、型不一致と未定義 field を診断する。`Type.field` は structural record 型との単一化を経て field 型を返し、不一致を診断する。record pattern はこの型推論 slice に含まない。static accessor の実行時 lowering は下記で実証済みである。

### private record の module visibility (2026-07-25)

Selfhost の record schema prepass / value-env 登録は `private` wrapper 内の record も宣言元 module では利用できるようにし、module 境界で raw name が残らないよう record literal / pattern の schema fallback に current value env の公開確認を加えた。宣言元 module の `{Secret x 1}` は diagnostics `0`、`{Secret x true}` は `1`、後続 module の raw `Secret` literal / pattern は `1` となる。import traversal は private wrapper を unwrap しないため、`L.Secret` の外部公開は従来どおり拒否する。

Evidence: `test_e2e_selfhost_typeinfer_analysis_accepts_private_record_in_same_module`、`test_e2e_selfhost_typeinfer_analysis_filters_imported_private_record`、qualified record type/literal/pattern/update regression 6件、`cargo run --quiet --bin lsharp -- parse selfhost/src/Types/TypeInferRecordDecl.ls` / `TypeInferRecord.ls` / `TypeInferPattern.ls` (`diagnostics:0`)。Rust oracle は private wrapper 内 record schema を現状登録しないため、local public-record equivalent で field/pattern 型契約を照合する。

追加で、同一スコープの `compile-program-functions-with-base` から Wasmを生成し、private record literal を同じ record patternへ渡す runtime sliceを `test_e2e_selfhost_compiler_mode_private_record_literal_pattern_runs` で実行して `41\n` を確認した。さらに `(Secret 41)` と `(Secret.x value)` の constructor/accessorを `test_e2e_selfhost_compiler_mode_private_record_constructor_runs` で実行し、同じく `41\n` を確認した。REDでは private wrapperが compiler preludeの直接 `RecordDef` 判定から外れ、constructor callが本体へ到達せず `65577\n` となった。GREENは `record-prelude-step` の宣言走査で private wrapperだけを unwrap する narrow fixであり、TypeInferの import/export visibilityや ftable lookup順は変更していない。

これは private record の同一 source scopeにおける TypeInfer、constructor/literal/pattern runtimeを閉じる verified sliceであり、private declarationの module import alias/`:only` runtime、standalone native stage0、Wasm artifact/runtime の Mac Apple Silicon / Linux x86_64 parity、EC-M1-01 aggregateは未完了である。

### record runtime 更新 (2026-07-14)

自己ホスト Wasm compiler は `CompilerMode` の file-compile 経路と legacy `compile-program-functions` / `compile-program-functions-with-base` で、`RecordLit`、`RecordUpdate`、direct `FieldAccess`、nonparametric record の `Point ...` constructor、`Point.field` static accessor を既存の `Map` runtime に lower する。record 本体を field 式の allocation 中も root に保持し、field hash を key に `map-insert` / `map-get` を使う。record update は更新 field だけを持つ patch Map に base Map を sentinel key `-1` で保持し、field lookup が patch chain を再帰的に辿るため、元の record は変更されない。record constructor と static accessor は user `defn` より前に prelude として function table / Wasm body へ登録し、Wasm entrypoint が最後の user function のままになる順序を保つ。actual compiler-mode E2E は `{Point label "record" x 42}` から `(. point label)` を `string-length` へ渡して `6`、`(. point x)` から `42` を出力し、`Point (inc 40) 2` の `Point.x` / `Point.y` が `41` / `2` を出力することを確認した。import された別 module の `Point` でも同じ `41` / `2` を generated Wasm で確認し、parametric `Box Int` / `Box Bool` の別具体化も `41` / `1` を出力する専用 E2E で確認した。さらに `p -> q -> r` の nested update を static / dynamic access で読み、`p` の値が保持されることを `test_e2e_selfhost_compiler_mode_record_update_runs` と ftable 経路の `test_e2e_selfhost_ftable_compiler_record_update_and_static_accessor_run` で確認した。normal compiler-mode の 11-import ABI に function table base `11` を揃え、direct source compile、imported file compile、`Cli` / `EmbeddedCli` の source-to-Wasm 入口で constructor/accessor call が runtime import と衝突しないことを確認した。record pattern は未完である。`App.Cli`、`EmbeddedCli`、`SmokeCli`、`PipelineSmoke` の source/full helper は full functions/data payload を `build-wasm-bytes-wasi` へ渡し、`PipelineSmoke` は Rust host compile と Wasm validate まで確認した。一方 `run-main-smoke` の単一 AST `lower` は診断用に残り、no-arg pipeline entrypoint の full-program runtime/native E2E と component sidecar の生成は未完了である。generated Wasm の opcode 87 (`print-string`) は通常の `CompilerMode` 出力で 11 番目の `env` runtime import への `call 10` として出力するところまで修正済みだが、外部 runtime の文字列 ABI 接続と standalone WASI Preview1 実行は別の output parity gap として残る。

source / ftable compiler-mode では record literal / static constructor が nominal marker `-3` を Map に保存し、canonical record pattern の type hash と照合する。同じ field layout を持つ別 record type の arm fallback、ftable nominal pattern の独立 E2E、`p -> q -> r` patch/base Map chain への marker 伝播を確認済みである。さらに source / ftable compiler-mode の nonparametric nested record pattern で親・子 field binder、nested literal child、record field 内の nested constructor child を actual Wasm 実行まで確認した。`map-contains?` / `map-remove` / `map-size` は integer key と string literal key の source / ftable actual Wasm slice まで確認済みだが、その先の一般 Map API parity は残課題である。

### nested parametric record pattern materialization (2026-08-01)

Nested record patterns whose expected field type is a parametric record application are now
materialized consistently at the Rust and selfhost type-inference boundaries. Rust resolves only
registered record applications, substitutes their parameters, and leaves unknown applications and
non-record ADTs unchanged. Selfhost applies the equivalent materialization lazily at each record
pattern field boundary, while the existing constructor-scheme visibility guard remains in force.

Evidence: `test_nested_parametric_record_pattern_propagates_field_type`,
`test_e2e_selfhost_typeinfer_nested_parametric_record_pattern_binds_field_type`, and the complete
`selfhost_typeinfer_quote_patterns` group (15 passed). The source compiler-mode Wasm runtime test
`test_e2e_selfhost_compiler_mode_nested_parametric_record_pattern_runs` also passed and produced
`41\n1\n7\n` for binder, literal, and fallback arms. The Mac Apple Silicon native gate passed for
source commit `fa97fa948489f635dc8888b5a269755a75776670` (1 native test, 4660 KiB artifact). The
Linux x86_64 hostgen/VM gate also passed for that source commit; stage2/stage3 code lengths were
both `11332908` and their stdout SHA-256 values matched at
`aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad`.

The imported source compiler-mode and ftable alias-qualified fixtures,
`test_e2e_selfhost_compiler_mode_imported_nested_parametric_record_pattern_runs` and
`test_e2e_selfhost_ftable_compiler_nested_parametric_record_pattern_runs`, passed in the same
focused batch and produced `41\n1\n7\n` as well.

The bounded deep regression `test_e2e_selfhost_compiler_mode_deep_nested_parametric_record_pattern_runs`
also passed for a four-record chain (`Root a -> Outer a -> Middle a -> Box a`) with the same
output. It is evidence for recursive materialization at that depth, not an arbitrary-depth claim.
The matching `test_e2e_selfhost_ftable_compiler_deep_nested_parametric_record_pattern_runs` passed
in the same focused batch, so source and ftable lowering agree for this four-record shape.

This is a verified nested parametric field-binding slice plus source/import/ftable runtime
fixtures, not completion of record-pattern semantic parity. Greater depths, full
ftable/linear-memory ABI, and the `LEGACY-LANG-01` aggregate remain Rust-oracle or otherwise
incomplete boundaries.

### WasmGC nested parametric record reference slice (2026-08-01)

The Rust WasmGC lowering boundary now resolves the named head of a registered `TypeExpr::App`
field to the corresponding GC `Ref` type. Before this fix, `Outer.inner : (Box a)` was lowered as
`I64`, so a nested `StructGet` could not preserve the reference-typed `Box` value. The change is
limited to registered record/ADT heads; unknown applications retain the previous fallback and
remain outside this evidence.

`test_wasmgc_nested_parametric_record_pattern_preserves_reference_field_types` verifies the IR
field type, and `wasm_gc_emitter_executes_lowered_nested_parametric_record_pattern` validates,
instantiates, and executes the WasmGC module in Wasmtime with result `41`. This is a Rust IR and
emitter backend slice. It does not update the native stage0 source producer or prove Rust-free
selfhost parity, so the record-pattern aggregate and the full WasmGC type-argument/runtime
surface remain incomplete.

### WasmGC ordinary ADT pattern runtime slice (2026-08-01)

The Rust WasmGC backend now has direct source-level runtime evidence for an ordinary
non-parametric ADT: `Option` with `Some Int` and `None`. The lowered module checks the variant tag,
extracts the typed payload for the `Some` binder, and falls through to the `None` arm. The IR
regression `test_wasmgc_adt_pattern_lowers_typed_payload_and_tag_checks` and the Wasmtime runtime
regression `wasm_gc_emitter_executes_lowered_adt_pattern_with_typed_payload` both pass; the fixture
executes both branches and returns `42`.

This is Rust IR/emitter backend evidence only. Parametric or recursive ADT representation,
nominal/exhaustiveness parity, import/ftable and linear-memory ABI parity, native stage0 producer
parity, and the `LEGACY-LANG-02` aggregate remain incomplete.

### selfhost imported ADT constructor pattern runtime slice (2026-08-01)

The selfhost compiler-mode file path now has an actual runtime regression for an ADT declared in a
separate module. `App.Shapes` declares parameterized `Maybe a`, and `App.Main` imports only `Just`
and `Nothing` with `:open :only`, matches both constructors, and emits Wasm through
`compile-file-mode`. `test_e2e_selfhost_compiler_mode_imported_adt_constructor_pattern_runs` runs the
generated artifact with the compiler-mode runtime imports and produces `41\n0\n`.

This verifies the source-file resolver, constructor export filtering, pattern binder/fallback, and
generated artifact/runtime as one selfhost slice. Ftable import parity, qualified or alias-qualified
constructor patterns, broad parametric/recursive ADT representation, nominal/exhaustiveness,
WasmGC/linear-memory parity, and current-source native stage0 evidence on both supported targets
remain incomplete at this initial open-import boundary; the alias-qualified source/compiler-mode
slice is recorded below. The change does not broaden prior native producer evidence.

### selfhost imported alias-qualified ADT constructor pattern runtime slice (2026-08-01)

The same two-file compiler-mode fixture now covers `(import App.Shapes :as S :only [Just Nothing])`,
`S.Just` / `S.Nothing` construction, and alias-qualified constructor patterns. The parser preserves
the full constructor hash for type-environment lookup and stores the raw suffix after constructor
children. The selfhost Wasm compiler uses that raw suffix only for the runtime ADT tag check, while
legacy unqualified patterns continue to use their existing full hash fallback.

Evidence: `test_e2e_selfhost_compiler_mode_imported_adt_constructor_pattern_runs` and
`test_e2e_selfhost_compiler_mode_imported_alias_qualified_adt_constructor_pattern_runs` passed in
one focused invocation (2 passed, 101.19s). Both open and alias-qualified forms executed generated
Wasm and produced `41\n0\n`. This also exercises the alias registration in the CompilerMode prelude ftable.

This closes the source-file/CompilerMode prelude-ftable alias-qualified ADT construction and pattern
runtime slice. Flat ftable compiler import parity, multi-segment module-qualified names, broad
parametric/recursive ADT representation, nominal/exhaustiveness, WasmGC/linear-memory parity,
current-source native stage0 evidence on both supported targets, and `LEGACY-LANG-02` remain
incomplete. The existing `[~]` TODO and Rust oracle/bootstrap boundary remain in force.

### selfhost alias-qualified ADT pattern type-inference visibility slice (2026-08-01)

The selfhost TypeInfer bundle now applies the same alias and `:only` visibility contract to ADT
constructor patterns that it already applies to qualified constructor expressions. The parser stores
the canonical `ast-qualified-name-hash(alias, constructor)` in the constructor pattern's type-inference
slot while retaining the raw constructor suffix metadata for the runtime tag boundary. Rust's
`Pattern::Constructor` oracle lookup also resolves an alias through the module import table before
looking up the injected module-qualified scheme.

Evidence: `test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_adt_constructor_pattern`
passed with selected `L.Some` at `0` diagnostics and excluded `L.Other` at `1` diagnostic. The Rust
oracle accepted the selected pattern and rejected the excluded `:only` pattern. The two source-file
compiler-mode runtime regressions remain green in the same validation batch. This is a parser/type
inference visibility slice; flat ftable import parity, multi-segment names, broad ADT semantics,
standalone native stage0 producer parity on both supported targets, and `LEGACY-LANG-02` remain
incomplete.

### selfhost flat ftable ADT alias target and recursive pattern scratch slice (2026-08-01)

The flat ftable compiler now registers imported ADT variants through the target module's qualified
constructor key before writing the alias-qualified key. A single fixture defines `Thing Int` in
`Left` and `Thing Int Int` in `Right`, imports them as `L` and `R`, and constructs/matches both
variants without allowing the later raw `Thing` registration to overwrite the earlier call target.
The generated Wasm produces `41\n5\n`.

Evidence: `test_e2e_selfhost_ftable_compiler_imported_alias_qualified_same_name_adt_constructors_run`,
`test_e2e_selfhost_ftable_compiler_adt_constructor_pattern_runs`, and
`test_e2e_selfhost_compiler_mode_adt_nested_constructor_pattern_runs` passed after the change.
The source-aware and ftable deep scratch regressions in
`selfhost_pattern_scratch_contract` also passed and produced `42\n`. `pattern-temp-base` now
advances by the six locals used by `map-get` / `map-contains`, so recursive child checks and
binders do not overlap the parent scratch or arm binder locals. The existing
`test_e2e_selfhost_ftable_compiler_alias_qualified_record_pattern_runs` remained green.

This closes only the flat ftable same-name alias target and recursive pattern scratch slices.
File-import ftable parity, multi-segment qualified names, broad parametric/recursive ADT
representation, nominal/exhaustiveness, WasmGC/linear-memory parity, current-source native
stage0 evidence on both supported targets, and the `LEGACY-LANG-02` / `LEGACY-ROOT-01`
aggregates remain incomplete. The existing `[~]` TODO and Rust oracle/bootstrap boundary remain.

The companion source-file boundary now has an actual runtime regression:
`test_e2e_selfhost_compiler_mode_imported_same_name_adt_aliases_run` writes `App.Left` and
`App.Right` files with same-name `Thing` constructors, imports them as `L` and `R`, and runs
`App.Main` through `compile-file-mode`. The generated Wasm produces `41\n5\n`. This verifies
CompilerMode's import prelude ftable alias target selection for separate files; it does not close
the flat `program-functions-base` file-import transport or the broader ADT/native target boundaries.

### legacy source compile boundary 更新 (2026-07-14)

`App.Cli`、`EmbeddedCli`、`SmokeCli`、`PipelineSmoke` の source/full helper は、`parse-program` の結果を `compile-program-functions-with-source` に渡し、先頭 IR だけを返す `lower` ではなく全 functions/data を `build-wasm-bytes-wasi` へ渡す。これにより helper 自体は複数 top-level function を落とさない。`PipelineSmoke` は Rust host compile と Wasm validate まで確認した。`EmbeddedCli` の component target は summary text を出力せず、外部 component packaging が必要な境界を明示的に返す。一方 `run-main-smoke` の単一 AST `lower` は診断用として残り、App.Cli / EmbeddedCli の component sidecar、no-arg pipeline entrypoint の full-program runtime/native E2E は別の未完了 surface である。

### 型・宣言意味論の更新: ordinary ADT (2026-07-14)

ordinary ADT は parser が variant 名と raw field TypeExpr を保持し、`TypeInferAdt.ls` の prepass が type parameter を束縛した constructor scheme を値環境へ登録する。`(type (Maybe a) (Just a) Nothing)` の constructor application と match pattern は同じ polymorphic scheme を使い、`Int` と `Bool` の別使用箇所で独立に instantiate される。さらに selfhost Wasm compiler の source / ftable 経路で Map-based constructor、variant tag、direct field binder、nested constructor pattern、constructor mismatch fallback を actual Wasm 実行で確認した（`test_e2e_selfhost_compiler_mode_adt_constructor_pattern_binds_and_falls_back`、`test_e2e_selfhost_compiler_mode_adt_nested_constructor_pattern_runs`、`test_e2e_selfhost_ftable_compiler_adt_constructor_pattern_runs`）。複数フィールドの内部 key は Map runtime の空スロット sentinel `0` を避ける `idx+1` 契約を constructor insertion / pattern check / binder で共有する。これは ordinary ADT の parser / 型検査と source / ftable runtime の Rust-free slice を示すが、full ftable/import target parity、Rust linear-memory ABI parity、nominal/exhaustiveness closure は未完了である。GADT の variant return type、pattern refinement、exhaustiveness も含まれない。

## 現在の事実

- `lsharp-native-selfhost-stage0` package は `compiler`、`transport_driver`、`materializer`、40 桁 lowercase hex の `source_commit` を持つ manifest で native bootstrap を開始する。`scripts/native-selfhost-dev.sh` は provenance のない manifest を拒否する。release 用の `App.Cli` archive は stage0 package ではない。
- Mac Apple Silicon では、current fixed-point stage3 compiler を stage0 package 化し、`scripts/native-selfhost-dev.sh` を通す source-file smoke が成功している。smoke は `cargo`、`rustc`、host `lsharp` を PATH 上で失敗させた状態で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を実行する。
- 2026-07-25 の Linux x86_64 current-source gate では、fixed-point artifact `5e64f3e4-map-remove` の stage2/stage3 code、data、entrypoint が一致し、transport stdout SHA-256 `9328c55d918d1a4d22acfbbc2033706223280a30f151e43029aa14346e615a64`、code length 各 `11057713` を確認した。`stage2-debug/program.native` は raw compiler ABI を実際の stage3生成に使った native compilerであり、`App.Cli` release programとは別物である。docs commit 前の検証時 commit `9fdd3b31606d4e8a0b85d4c62dbe0b5a5f3f6e5d` に provenance を合わせた stage0 package `/tmp/lsharp-stage0-9fdd3b31-linux-x86` を Lima `lsharp-linux-x86` で実行し、その後の `cda801dd` と main merge `f7112ad0` は `selfhost/src` に変更がない（main 側は Rust validation/docs の変更）。transport chunk `256`、timeout `900` 秒、`cargo` / `rustc` / host `lsharp` blocklist の source-file smoke が `parse`、`check`、`fmt`、通常/metadata `test`、`compile -o`、`build -o` まで passした。これは Linux core source-file boundary の verified sliceであり、full public surface、Mac/Linux aggregate parity、未移行 semanticsの完了ではない。Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/5e64f3e4-map-remove/actual-selfregen-summary.json`、`scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh`。
- Linux x86_64 は、commit `4bd9ee9` から生成した fresh actual-stage1 を stage0 package 化し、Lima `lsharp-linux-x86` VM 内で source-file smoke を成功させた。続く current-source stage0 `/tmp/lsharp-native-linux-x86-stage0-7807089` の再確認でも、`LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=64`、timeout 1200 秒で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を完走した。実行中は `cargo`、`rustc`、host `lsharp` を blocklist にし、VM は 11 GiB disk 中 3.2 GiB 使用（30%）で終了、temporary workdir/lock は残していない。`7807089` の actual stage1 -> stage2 -> stage3 selfregen も別 gateで pass している。2026-07-14 の historical `8dd37ef-static-string-fixedpoint` replay における `parse stdout is missing decls:1` は、fresh stage0 で解消された過去の failure evidence として残す。
- 2026-07-19 の current HEAD `c5c9751d53a6d8845a24c61593a0364aecad09b1` では、Linux x86_64 actual stage1 の `source_commit` を検証してから、現行の data/heap frontier materializer を含む stage0 package を Lima `lsharp-linux-x86` で再作成した。この package から current `selfhost/` source を再生成し、`LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=64`、timeout 900 秒、`cargo` / `rustc` / host `lsharp` blocklist の source-file smoke を実行して `parse`、`check`、`fmt`、通常/metadata/property `test`、`compile -o`、`build -o` を完走した。actual stage1 -> stage2 -> stage3 selfregen も同じ source commit で pass し、stage2/stage3 の code length は各 10,744,009 bytes、stdout SHA-256 は `50111731985fe62d4107aaafa2a2afecfff035a1796caa6f74748e65404b5163b` で一致した。これは Linux x86_64 の current-source daily core boundary を閉じる evidence であり、EC-M1-04/05/06 の各 Linux evidence を補うが、各 milestone 全体、Mac/Linux の aggregate parity、公開 surface、未移行 semantics の完了を意味しない。
- native bootstrap の初回だけは source tree を再生成する。fingerprint が不変なら `scripts/native-selfhost-dev.sh` は生成済み `program.native` を再利用する。
- repo 内の旧 stage0 artifact に `source_commit` がない場合は、native runner の成功経路へ再利用せず、source commit と fixed-point evidence を付けた package を再生成する。
- `LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY` は macOS host policy 上、生成済み Mach-O の実行に署名が必要な環境でだけ指定する。成功時の codesign 出力は command stderr に漏らさず、失敗時だけ診断として返す。
- GitHub Actions の自動 build は使わない。検証と release は Mac と Lima VM の手動 local gate で行う。
- 2026-07-31 に TypeInferAssertions batch（検証時 source commit `3b5dbef50e478dad0c71c12e6108d9fb2ce2c6fe`）を
  Mac host から生成し、Lima `lsharp-linux-x86` の actual stage1 -> stage2 -> stage3 self-regenerationを
  完走した。`actual-selfregen-summary.json` は `status: pass`、stage2/stage3 code length 各
  `11491724`、stdout SHA-256 は両方
  `bfff156740a634e25a4fc968ca2a83c9ce62227ed3846d70a3d59658fd6d1d76`、stderr は空だった。
  TypeInferAssertions の 64 要素 bounded rooted scanner/aggregation と 65 要素 cross-chunk E2E の
  Linux x86_64 verified sliceであり、artifact/runtime の全 target aggregate、未移行 assertion/property
  semantics、Rust oracle との全 diagnostic/span parityを完了扱いにはしない。VM workdir、replay lock、
  約 1.8 GiB の task-owned Cargo target は gate 後に削除し、CI は起動していない。
- 2026-07-31 に TypeInferAdt batch（検証時 source commit `81103bfd3cd6b2dcb771b297d0cc10a547dc6ee1`）を
  Mac host から生成し、Lima `lsharp-linux-x86` の actual stage1 -> stage2 -> stage3 self-regenerationを
  完走した。`actual-selfregen-summary.json` は `status: pass`、stage2/stage3 code length 各
  `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。TypeInferAdt の
  64 要素 bounded rooted scanner と 65 要素 cross-chunk focused E2E の Linux x86_64 verified
  sliceであり、nominal/exhaustiveness、full ftable/import、linear-memory/WasmGC runtime、
  Mac/Linux aggregate parity の完了を意味しない。VM workdir と replay lock は gate 後に削除し、
  task-owned build target も再利用不要分を回収した。
- 2026-07-31 に TypeInferApply batch（検証時 source commit `b1b9431dbfee940513e7b0939eb8a659435ce489`）を
  Mac host から生成し、Lima `lsharp-linux-x86` の actual stage1 -> stage2 -> stage3 self-regenerationを
  完走した。`actual-selfregen-summary.json` は `status: pass`、stage2/stage3 code length 各
  `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。TypeInferApply の
  3-7 引数 bounded rooted scan、high-arity failure propagation、curried apply 11 tests の Linux
  x86_64 verified sliceであり、64超の診断契約、全 diagnostic/span、Mac/Linux aggregate parity は
  未完了である。VM workdir と replay lock は gate 後に削除し、task-owned build target も回収した。
- 2026-07-31 に TypeInferApply の arity boundary batch（検証時 source commit
  `754bbcd7072f7d880e055b45d8ca9cf469419b6b`）を Mac host から生成し、Lima `lsharp-linux-x86` の
  actual stage1 -> stage2 -> stage3 self-regenerationを完走した。`actual-selfregen-summary.json` は
  `status: pass`、stage2/stage3 code length 各 `11332908`、stdout SHA-256 は両方
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad` で一致した。新規の
  8/64 引数成功、65 引数の明示的 error result、既存の curried apply 11 tests と合わせた 13 tests、
  Rust `lsharp-types` infer focused 99 testsを確認した。`TypeInferApply` の 0-64 bounded arity と
  current-source Linux x86_64 native self-regenerationの verified sliceであり、65以上の診断/span、
  全 diagnostic/span、Mac/Linux aggregate parity は未完了である。Evidence は
  `ci-artifacts/native-linux-x86-hostgen-vm/754bbcd7-typeinfer-apply-64/actual-selfregen-summary.json`。
  VM workdir と replay lock は gate 後に削除し、task-owned build artifact は summary のみに縮小した。
- 2026-07-31 に TypeInferPattern batch（検証時 source commit `84becbb30bf93a15ffbe34155235c6321e67b245`）を
  Mac host から生成し、Lima `lsharp-linux-x86` の actual stage1 -> stage2 -> stage3 self-regenerationを
  完走した。`actual-selfregen-summary.json` は `status: pass`、stage2/stage3 code length 各
  `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。TypeInferPattern の
  constructor/record child bounded rooted scan、cross-chunk 2 tests、match error 10 tests、quote/
  pattern 12 testsの Linux x86_64 verified sliceである。既存の record schema inference test は
  origin/main でも同じ failure を再現したため別残件とし、全 diagnostic/span、Mac/Linux aggregate
  parity は未完了である。VM workdir と replay lock は gate 後に削除し、task-owned build target も回収した。

### EC-M2-01 selfhost source metadata storage (2026-07-25)

Selfhost `Syntax.Parser` は `defn` metadata の `:intent` / `:claim` / `:assumption` /
`:open-question` と `:motivates` / `:constrained-by` / `:tested-by` / `:supports` /
`:contradicts` を既存 ordered form の `[kind, payload, directive-start, directive-end]` として保持する。
payload は2つの string を `[wire-id, text-or-endpoint]` の vector にし、parser では ID の kind推測や
typed graph validationを行わない。`intent`、`claim`、`motivates` の directive順・wire ID・本文/endpointを
`test_e2e_selfhost_parser_preserves_source_intent_metadata_forms` の selfhost parser runtimeで確認した。

これは Rust host が compile/run する parser bundle の verified sliceであり、native stage0 の証拠ではない。
追加で、TypeDef / RecordDef 後の source metadata を parser が保持し、`IntentSource` が node と typed edge へ
投影する境界を `test_e2e_selfhost_source_adapter_projects_type_definition_metadata` と
`test_e2e_selfhost_source_adapter_projects_record_definition_metadata` で確認した。これは
`docs/adr/decisions-v0.2-selfhost-source-type-record-metadata.md` に記録した partial parity slice である。
TypeDef / RecordDef の native stage0 parity、evidence record、全 nested declaration と typed graph
projection、`validate` / `--emit-manifest`、EmbeddedCli/MCP、Mac Apple Silicon / Linux x86_64
artifact/runtime parityは残件である。

### EC-M2-02 selfhost evidence registry initial source-form slice (2026-07-25)

`selfhost/src/Syntax/Parser.ls` が `:evidence` の named required fieldsと `:shrinks` / `:coverage` を
17-field payloadへ変換し、`selfhost/src/Tools/Validation/Evidence.ls` の registry consumerへ渡す。
top-level source textからの parser → registry、required field/typed subject、sampling、duplicate IDの
fail-closed boundaryを `test_e2e_selfhost_evidence_registry_consumes_parser_form` と registry focused 13件で
確認した。sampling では負の seed / shrink 値と malformed coverage entry を `invalid-sampling` code `11` と
それぞれの field で拒否し、
17-field 以外の evidence payload は `malformed` code `1` / field `form` として拒否する。
`source-evidence-graph-from-program` は registry 登録後に `supports` / `contradicts` を
graphへ投影し、登録済み edgeと未登録 edgeの拒否を確認する。既存 `selfhost_intent_source_adapter` 8件も
再実行して passした。これは Rust-host actual Wasm の selfhost runtime evidenceであり、既存 validate
graph/CLIへの接続、manifest、native stage0、Mac Apple Silicon / Linux x86_64 current-source parityは
未完了である。

### EC-M2-02 source review/invalidation edge adapter (2026-07-27)

Rust source metadata parser が `:evaluates "review:namespace/key" "intent|claim|evidence:namespace/key"` と
`:invalidates "change:namespace/key" "review|evidence:namespace/key"` を ordered typed form として保持する。
`validation_source` は ReviewId/ChangeId と subject kind を fail-closed に復元し、Intent/Claim subject は
node registry、Evidence subject は evidence registry へ解決してから `Edge::Evaluates` /
`Edge::Invalidates` を追加する。外部 ReviewId は invalidation endpoint として保持するが、Review/Evidence
の provenance 認証は後続境界である。

`crates/lsharp-syntax/tests/intent_edges.rs` の parser contract 10件と
`crates/lsharp-types/tests/validation_source/edges.rs` の source adapter contract 12件で、ordered
wire ID、review subject、change invalidation、orphan/mismatch/registry-required failure を確認した。
これは Rust-host source→graph の verified sliceであり、selfhost parser/IntentSource、manifest/CLI、
native stage0、Mac Apple Silicon / Linux x86_64 parity、review provenance/privacy policy は未完了である。

### EC-M2-03 Rust CLI input I/O diagnostic boundary (2026-07-31)

公開 `lsharp validate <manifest>` と `lsharp validate --source <source>` の入力ファイル読み込み失敗を
driver 共通の `[LS5001]` I/O 診断へ接続した。`--format json` でも読み込み failure は report として
投影せず、stdout を空にして exit `1` を返す。`--emit-manifest` を同時指定しても、入力を読めない
段階で manifest を作らない。parser/source-adapter の stable code と report の `pass` / `fail` /
`unknown` semantics は変更していない。

Evidence: `validate_manifest_read_failure_preserves_driver_io_error_boundary` と
`validate_source_read_failure_preserves_driver_io_error_boundary` の RED は generic miette error に
`[LS5001]` がないことを確認し、GREEN で両実 binary が exit `1`、空 stdout、`[LS5001]` を含む stderr
を返した。既存 `validate_cli` 全34件も pass し、manifest/source report、emit-manifest、parser
diagnostic、project-config path の回帰を確認した。

これは Rust-host 公開 CLI の入力 I/O boundary に限定した verified partial sliceであり、selfhost/native
stage0、MCP、current-source Mac Apple Silicon / Linux x86_64 artifact/runtime、EC-M2-03 aggregate は
残件である。ADR: `docs/adr/decisions-v0.2-validation-cli-io-diagnostics.md`。

### EC-M2-03 Rust CLI source review/invalidation projection (2026-07-27)

Rust の公開 `lsharp validate --source --format json --emit-manifest` を同一 source fixture で実行し、
`:evaluates` / `:invalidates` の typed edge が JSON report と version 1 manifest の両方へ source order
を保って射影されることを確認した。review subject kind mismatch は non-zero exit と `subject kind`
診断を返し、manifest を生成しない fail-closed 境界も固定した。

Evidence: `cargo test -p lsharp-driver --test validate_source_review_edges -- --nocapture`（2 passed）。
これは Rust-host source adapter と公開 CLI の report/manifest projection の verified slice であり、
selfhost/native stage0、durable atomic write の全条件、review provenance/privacy policy、MCP、
Mac Apple Silicon / Linux x86_64 current-source artifact/runtime parity、EC-M2-02/03 aggregate は
未完了である。

### EC-M2-02 review provenance registry and redaction boundary (2026-07-27)

version 1 manifest に optional な `reviews` registry を追加し、review ID、opaque
`provenance_digest`、`public` / `redacted` の visibility だけを保持するようにした。registry が
明示された manifest では `evaluates` / `invalidates` の review endpoint を登録済み ID に限定し、
未登録 review は `MissingReview` で fail-closed に拒否する。author、email、本文、URL、token は
schema に存在せず、既存の registry を持たない source graph の external `ReviewId` 互換性は維持する。

Evidence: `cargo test -p lsharp-types --test review_provenance -- --nocapture`（4 passed）、
`validation_input` 16件、`validation_output` 5件、`validation_schema` 2件、
`intent_validation` 6件。これは Rust canonical manifest の privacy/registry verified slice であり、
provider/署名による provenance authentication、digest format、source `:review` producer、
selfhost/native parity、review lifecycle/stale propagation、selfhost/native MCP、Mac Apple Silicon / Linux x86_64
current-source artifact/runtime parity、EC-M2-02/03 aggregate は未完了である。

### EC-M2-03 Rust CLI manifest review registry projection (2026-07-27)

`lsharp validate <manifest> --format json --emit-manifest <output>` の公開 Rust CLI 入出力で、
optional `reviews` registry の `namespace` / `key` / `provenance_digest` / `visibility` を
登録順のまま roundtrip し、author/email/body のような private field を出力しない契約を確認した。
`invalidates.subject(kind=review)` が registry にない場合は non-zero を返し、manifest output を
生成しない fail-closed boundary も同じ CLI fixture で固定した。

Evidence: `cargo test -p lsharp-driver --test validate_review_registry -- --nocapture`（2 passed）。
これは Rust host の manifest input/output boundary の verified slice であり、selfhost/native stage0、
provider/署名 authentication、review lifecycle、selfhost/native MCP、Mac Apple Silicon / Linux x86_64
current-source artifact/runtime parity、EC-M2-02/03 aggregate は未完了である。

### EC-M2-03 Rust MCP review registry schema and inline artifact (2026-07-27)

Rust MCP `lsharp_validate` の `tools/list` schema に、manifest input と output の optional
`reviews` registry を同じ wire shape で公開した。各 record は `namespace`、`key`、
`provenance_digest`、`visibility` (`public` / `redacted`) だけを許し、additional property と
author/email/body を受け付けない。`include_manifest` の inline artifact は、manifest input の
redacted registry を登録順のまま同じ privacy boundary で返し、未登録 review edge は既存の
fail-closed error へ到達する。

Evidence: `cargo test -p lsharp-driver mcp_server::tests -- --nocapture`（40 passed）。
これは Rust MCP の schema／inline artifact verified slice であり、selfhost/native MCP server、
provider/署名 authentication、review lifecycle、Mac Apple Silicon / Linux x86_64 current-source
artifact/runtime parity、EC-M2-02/03 aggregate は未完了である。

### EC-M2-03 Rust MCP validation input envelope closure (2026-07-31)

`lsharp_validate` の `tools/list` input schema の top-level object に
`additionalProperties: false` を追加し、runtime parser が拒否する未知 field を MCP consumer の
静的 schema でも拒否するようにした。manifest object 内の strict schema は既存のまま維持し、
`source` / `file` / `manifest` / `manifest_file` の `oneOf` と review context option は変更していない。

RED は `test_validate_tool_declares_source_input_and_report_output_schema` で input schema に
`additionalProperties` がなく `null` になっていることを確認した。GREEN は同じ assertion と
`test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures` の未知 top-level
field reject で固定し、`mcp_server::tests` 全69件を通過した。

これは Rust-host MCP の静的 input envelope／Draft 2020-12 validator boundary に限定した verified
partial sliceであり、selfhost/native MCP producer、current-source stage0 artifact/runtime、対応2 target、
EC-M2-03 aggregate は残件である。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

### EC-M2-03 Rust MCP validation route string closure (2026-07-31)

`lsharp_validate` の runtime は空の `manifest` JSON string、`file` / `manifest_file` path、
`trust_store` / `review_lifecycle` pathをそれぞれ parse/I/O/path errorとして拒否するが、MCP input
schemaは空文字を `string` として受理していた。`manifest` の string variantと5つの route/context
文字列へ `minLength: 1` を追加し、schema consumerがruntimeより弱い入力を送らないようにした。空の
`source`は空programの既存 semanticsを保つため対象外とした。

REDは `test_validate_tool_input_schema_rejects_empty_manifest_and_path_strings` で、各 propertyの
`minLength`欠落とDraft 2020-12 validatorの空入力受理を検出した。GREENでは同じ validatorが
`manifest` / `file` / `manifest_file` / `trust_store` / `review_lifecycle` の空入力をすべて拒否し、
`mcp_server::tests` 70件が通過した。

これは Rust-host MCP schema/runtime boundary の verified partial sliceであり、selfhost/native MCP、
current-source stage0 artifact/runtime、Mac Apple Silicon / Linux x86_64、EC-M2-03 aggregate の完了証拠
ではない。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

### EC-M2-02/03 canonical and MCP coverage bucket name schema closure (2026-07-31)

runtime の `SamplingPlan` は `coverage` bucket 名を `trim().is_empty()` で拒否するが、canonical manifest
schema と MCP input/output schema は任意の property nameを受理していた。canonical
`intent-graph.schema.json` と MCPの共有 `sampling_schema()` に `propertyNames.pattern: "\\S"` を追加し、
空文字・ASCII空白・NBSP-only の bucket 名を schema consumerの段階で fail-closed にした。countの
non-negative/maximum contractと、coverage省略時の互換性は維持する。

REDは `test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures` に3種類の空 bucket
を追加し、canonical/input/output Draft 2020-12 validatorが受理することを確認した。GREENでは同じ3ケース
を全validatorで拒否し、既存の valid fixtureと schema meta validationを維持した。focused
`mcp_server::tests` は70件 passした。

これは Rust-host canonical/MCP schema parityの verified partial sliceであり、selfhost/native manifest parser、
current-source stage0 artifact/runtime、Mac Apple Silicon / Linux x86_64、EC-M2-02/03 aggregateの完了証拠では
ない。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

### EC-M2-03 selfhost `validate --source` initial CLI slice (2026-07-25)

`selfhost/src/App/Cli.ls` に `validate` command、`--source <file> --format json` option、top-level
`defn` metadata の intent/claim/motivates/tested-by 集計、claim trace gap JSON projection、unknown exit
code `2` を追加した。さらに `Tools.Validation.Evidence` の registry/edge consumerを bundleへ接続し、
contradictory evidenceだけは `fail` / exit `1`、独立 review数、contradicting observation数を reportへ
投影する。`selfhost_cli_validation_contract` は command/help/option/report-code の source contractを
RED から GREEN へ確認している。

同じ fixtureを actual Wasm で実行する `test_e2e_selfhost_cli_validate_source_json_reports_trace_gap` は、
lowering前に検出されていた既存 `typeinfer-builtin-root-value` の関数間 root leaseを修正した後、
`1 passed`（291.84s）で完走した。専用 helperの acquire/release shape checkを追加し、
`run-check-program` の3 slotと `run-test-source-json/text` の各4 slotを全経路で解放する focused
RED/GREENを通し、argv/filesystem、unknown exit `2`、JSON status/trace gapを同じ fixtureで確認した。
typed signature、nested traversal、全 evidence/contract report parity、EmbeddedCli/MCP、native stage0 と
対応2 targetの current-source parityは未完了である。source graphの version 1 manifest serializer と
`--emit-manifest` file output は、`test_e2e_selfhost_cli_validate_source_emits_manifest` の actual Wasm
で report stdout分離、nodes/evidence/edges、sampling/provenance、unknown exit `2` を確認した。軽量
serializer focused test `test_e2e_selfhost_evidence_manifest_serializer_matches_version_one_shape` も
passしている。これは Rust-host actual Wasm の verified sliceであり、native stage0、durable atomic
write、release provenanceの証拠ではない。registry/contradictory fixtureの
`test_e2e_selfhost_cli_validate_source_json_reports_contradicting_evidence` を含む validation 5件が
同一 bundle compileで passした（manifest graph-error/write-failure negativeを含む、write-failure単体は324.56s）。

## Native 開発経路

`fetch-stage0.sh` が配置した `./stage0` package があれば、通常のコア開発は次の runner を使う。

```bash
./scripts/native-selfhost-dev.sh check examples/fib.ls

./scripts/native-selfhost-dev.sh --bootstrap compile examples/fib.ls -o fib.wasm
```

`NATIVE_STAGE0_DIR=/path/to/lsharp-native-selfhost-stage0` または `--stage0-dir` は、別の stage0 package を比較・検証する場合だけ指定する。`--bootstrap` は stage0 compiler で current `selfhost/` を native program に再生成する。通常コマンドだけであれば、同じ source fingerprint で bootstrap を繰り返さない。

Linux x86_64 の final gate は macOS host から Lima へ package と必要最小限の source/scripts をコピーして実行する。

```bash
LSHARP_NATIVE_LINUX_X86_STAGE0_DIR=/path/to/linux-stage0 \
  ./scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
```

この wrapper は VM の `/tmp` 空き容量を 4 GiB 以上で確認し、VM 内で `scripts/ci/native-selfhost-dev-source-file-smoke.sh` を実行する。source-file smoke は `cargo`、`rustc`、host `lsharp` を blocklist に入れるため、Rust host fallback は成功条件にならない。
既定の transport は 64 functions/chunk、chunk timeout は 900 秒である。checkpoint を再利用する診断時だけ `LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE` と `LSHARP_NATIVE_LINUX_X86_TRANSPORT_TIMEOUT_SECONDS` を指定して VM 側へ引き渡せる。VM の disk size や空き容量 gate は変更しない。

## Command Boundary

| Command surface | Native の責務 | Rust の要否 | 外部条件・制約 |
| --- | --- | --- | --- |
| `parse` / `check` / `fmt` / `test` | native `program.native` が直接実行する core CLI | 検証済み core slice では不要 | Bash、Python 3、hash tool。stage materialize は Mac で `clang`、Linux で `cc` を使う。Mac は必要な host でのみ codesign identity を指定する。型・宣言の未実装 P0 は Rust oracle が必要。 |
| embedded driver の guest-success compile/build | guest の artifact summary / output をそのまま返す | 不要 | guest exit code 0 では Rust `compile_file` を呼ばず、失敗時だけ host artifact fallback。runtime disable 下の `test` は delegation hint。 |
| `compile -o` / `build -o` (WASI Preview1) | native CLI が actual core Wasm bytes を出力する | 通常開発では不要 | 上と同じ。Mac と Linux x86_64 の current source-file smoke で検証済み。 |
| component `compile` / `build` | native core Wasm を component 化する | 不要 | Python helper と外部 `wasm-tools` が必要。これは Rust host fallback ではない。 |
| `install` | package install / module index helper | 不要 | Python 3。git dependency は `git` が必要。 |
| `repl` | expression ごとの native compile + run | 不要 | Python helper と外部 `wasmtime`。stateful evaluator ではない。 |
| `doc` | native `doc --json` を document helper が整形する | 不要 | Python helper。 |
| `lsp --stdio` | native program に stdio replay shim を接続する | 不要 | Python shim。bare `lsp` は native runner が明示的に拒否する。 |
| `mcp-server` | native runner は提供しない | 必要 | Rust host integration の責務として明示的に失敗する。 |
| `compile --emit-ir` | native runner は提供しない | 必要 | Rust host integration の責務として明示的に失敗する。 |
| `--target web-wasm` / `--target native` | native runner は提供しない | 必要 | native selfhost の supported output target 外として明示的に失敗する。 |

## Record pattern の現在地

2026-07-16 時点で、selfhost parser は record pattern の field 配置を維持したまま record type name hash を AST 末尾へ保存し、`TypeInferPattern.ls` は登録済み record schema を instantiate して各 child pattern を field 型と unify する。未登録 record、未定義 field、field child の型不一致は selfhost 側で診断できる。既存の type name なし手組み AST は shallow fallback を維持する。

一方、selfhost Wasm compiler の match lowering は direct record Map の field presence/value lookup、field binder local、literal child check、arm fallback、nominal type mismatch fallback を source / ftable の actual Wasm 実行で確認済みであり、source / ftable compiler-mode の nonparametric nested record pattern では親・子 field binder、nested literal child、record field 内の nested constructor child も実行結果まで確認済みである。`p -> q -> r` の patch/base Map chain でも nominal marker を保持し、ftable 経路の独立 record pattern E2E を通過した。ただし record pattern 全体 parity と、contains/remove の string-key slice を超える一般 Map API parity は未完了である。したがって、この進捗は record pattern の一部を Rust oracle の必須範囲から外したものであり、L# の全 record pattern 機能が Rust なしで使えることを意味しない。

上記の nominal type hash は全経路の完了を意味しない。source / ftable compiler-mode の direct record literal / canonical pattern mismatch と patch/base Map chain の marker 伝播、Map contains/remove の integer/string-key slice、source / ftable compiler-mode の nonparametric nested record binder/literal/constructor slice は検証済みだが、record pattern 全体 parity とその先の一般 Map API parity は未検証である。

## Rust に残る責務

Rust が完全に不要になったわけではない。次の作業は native base development loop の外側に残る。

1. stage0 の生成・配布・取得。fresh clone が自動で stage0 を取得する public release contract は別途閉じる必要がある。通常開発は供給済み stage0 package を前提にする。
2. native selfhost と Rust implementation の oracle/differential 比較、障害解析、emergency rollback。
3. `mcp-server`、bare LSP、`--emit-ir`、native target など、上表で明示した Rust host integration surface。

したがって、source-file smoke と current selfregen の両方が確認できた範囲では「検証済み core CLI の日常ループは Rust なしで開発可能」と言える。一方で、今回の `7807089` selfregen pass だけでは source-file command coverage を更新したことにはならない。自己ホストの型・宣言意味論 P0 が未完了の間は「base language development 全体」や「L# の全機能」が Rust なしとは言えない。closed / parametric alias の signature・式内 annotation、forward closed alias の signature、recursive alias の E0006 rejection、scoped polymorphic `defn` signature、ordinary ADT の parser / 型検査と direct runtime slice、parametric record の constructor/literal/field access/update/runtime、`Type.field` accessor の型検査、immutable record update の nested runtime slice、record pattern の source / ftable direct runtime slice と source / ftable compiler-mode の nonparametric nested record binder/literal/constructor はその境界を少し狭めた。ordinary ADT runtime の残り、record runtime の full public closure、legacy `lower` / embedded compiler surface、上表の Rust-only surface、external tool dependency、record pattern の残り、GADT exhaustiveness / full runtime parity などの未実装 P0 は残る。

### 残る Base Language Gap

Rust を base implementation から外すため、legacy `lower` / embedded compiler の full-program 化、ordinary ADT runtime の残り、record pattern 全体 parity、GADT exhaustiveness / full runtime parity を自己ホスト側で実装・差分検証する必要がある。recursive alias は Rust implementation と同じく拒否するため、未対応の recursive language feature としては数えない。`CompilerMode` と ftable 経路における nonparametric / parametric record の constructor/literal/direct/static accessor/update runtime、record pattern の direct field/binder/literal/fallback/nominal/patch-chain slice と source / ftable compiler-mode の nonparametric nested record binder/literal/constructor、ordinary ADT の parser / 型検査と source / ftable 経路の direct / nested constructor/tag/binder/fallback slice、Map contains/remove の integer/string-key slice はこの一覧から除外する。ただし ordinary ADT の full ftable/import target parity、Rust linear-memory ABI parity、nominal/exhaustiveness closure、検証済み slice を超えて record を一般 Map として扱う全 API の parity は別途確認する。出力側では標準 WASI Preview1 の `fd_write` / `args_sizes_get` / `args_get` / `path_open` / `fd_close` / `fd_read` import、print、record/accessor、root stack の standalone slice と未対応 opcode の明示拒否まで確認済みであり、bounded individual argv string、bounded `file-exists?`、一回の `fd_read` による 1024-byte bounded `read-file`、一回の `fd_write` による bounded string `write-file` と bounded raw vector `write-file-bytes` runtime も追加した。残る capability（full fd error semantics、1024 bytes を超える read、dynamic root/data/heap layout、component sidecar、Linux `App.Cli` output）は個別に閉じる。

ここでいう record pattern の残件は、検証済みの direct / nominal / patch-chain slice と source / ftable compiler-mode の nonparametric nested record binder/literal/constructor 以外の全体 parity、特に import/parametric/deeper cases と一般 Map API を指す。

2026-07-16 追加進捗: standalone string `write-file` の partial `fd_write` を RED→GREEN で閉じた。最初の write を 2 bytes に制限する WASI shim で single-call 実装の truncation を再現し、修正後は `nwritten` を累積し、offset / remaining を更新して再試行する loop を生成する。errno、0 bytes、要求長超過は `-1` とし、selfhost compiler 生成 artifact（2402 bytes）は `wasm-tools validate`、通常 WASI、partial shim 下の exact E2E（`fd_write` 2 回、`payload` 全量）を pass した。raw `write-file-bytes`、read の partial/error、full fd error semantics、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。Evidence: `test_e2e_selfhost_standalone_write_file_retries_partial_fd_write`、`test_wasi_fd_write_shim_is_used_for_standalone_import`、`selfhost/src/Backend/Wasm/WasmEmit.ls`。

2026-07-16 追加進捗: standalone raw vector `write-file-bytes` の partial `fd_write` を RED→GREEN で閉じた。5 byte payload に対して最初の write を 2 bytes に制限する WASI shim で single-call 実装の `[0, 97]` truncation を再現し、修正後は `nwritten` を検査し、offset / remaining / total を更新して再試行する loop を生成する。errno、0 bytes、要求長超過は signed `-1` とし、selfhost compiler 生成 artifact（3586 bytes）は `wasm-tools validate` と partial shim 下の exact E2E（`fd_write` 2 回、raw payload 全量）を pass した。path-open / full fd error、read の partial/error、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。Evidence: `test_e2e_selfhost_standalone_write_file_bytes_retries_partial_fd_write`、`test_wasi_fd_write_shim_is_used_for_standalone_import`、`selfhost/src/Backend/Wasm/WasmEmit.ls`。

2026-07-16 追加進捗: standalone bounded `read-file` の partial `fd_read` を RED→GREEN で閉じた。2 bytes、残り 5 bytes、EOF の shim で旧 single-read body の `pa` truncation を再現し、修正後は buffer offset / remaining を更新する loop、0-byte 終了、errno 時の close 境界を生成する。selfhost compiler 生成 artifact（2531 bytes）は `wasm-tools validate` と partial shim 下の exact E2E（`fd_read` 3 回、`payload` 全量）を pass した。1024 bytes 超の read、path-open / full fd error、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。Evidence: `test_e2e_selfhost_standalone_read_file_retries_partial_fd_read`、`selfhost/src/Backend/Wasm/WasmEmit.ls`。

2026-07-16 追加進捗: standalone `read-file` の `fd_close` 戻り値を drop せず local 8 へ保存し、close errno が非ゼロなら累積 length を 0 に戻す fail-closed body を RED→GREEN で固定した。body scan の focused gateに加えて、selfhost compiler が生成した artifact（2570 bytes）の `wasm-tools validate` と close errno shim 下の actual runtime E2E（read payload 全量、`fd_read` 3 回、close 1 回、stdout 空）を pass した（`test_e2e_selfhost_wasmemit_read_file_preserves_fd_close_errno`、`test_e2e_selfhost_standalone_read_file_returns_fd_close_errno`）。path-open / full fd error、1024 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。

2026-07-16 追加進捗: standalone string/raw `write-file` の `fd_close` errno を Rust oracle と selfhost codegen の両方で保持するよう RED→GREEN で閉じた。`fd_close=1` shim 下で selfhost compiler 生成 artifact（3729 bytes）は `wasm-tools validate` と actual E2E を passし、string/raw のファイル payload は全量 (`payload\0asm!`)、close 呼び出しは 2 回、両戻り値は `-1` の i64 bit pattern になった。Evidence: `test_wasi_write_helpers_preserve_fd_close_errno`、`test_e2e_selfhost_wasmemit_write_file_preserves_fd_close_errno`、`test_e2e_selfhost_wasmemit_write_file_bytes_preserves_fd_close_errno`、`test_e2e_selfhost_standalone_write_helpers_return_fd_close_errno`、`selfhost/src/Backend/Wasm/WasmEmit.ls`、`crates/lsharp-wasm/src/wasi.rs`。残る I/O は path-open / full fd error、1024 bytes 超の read と dynamic layout であり、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary も未完了である。
2026-07-16 追加進捗: standalone `print` の負数表示を Rust oracle と揃えた。signed `i64` の符号判定、絶対値の数字化、負数時の `-` prefix を selfhost Wasm emitter に追加し、直前の `fd_close=1` artifact で `-1\n-1\n`、`payload\0asm!` 全量、close 2 回を actual E2E で確認した。生成 artifact（3776 bytes）は `wasm-tools validate` と保存 artifact の fast rerun も pass。Evidence: `test_e2e_selfhost_standalone_write_helpers_return_fd_close_errno`、`selfhost/src/Backend/Wasm/WasmEmit.ls`、`/tmp/lsharp-selfhost-close-errno-write-signed.wasm`。残る I/O は path-open / full fd error、1024 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary である。
2026-07-16 追加進捗: standalone bounded `read-file` の partial `fd_read` errno を fail-closed に固定した。最初の `fd_read` が 2 bytes を書いて errno `1` を返す shim で、selfhost artifact は `read_payload=pa` を内部に残しつつ String length を 0 として stdout を空にし、close 1 回で終了した。Rust oracle も `fd_read` errno を local 保存して nread を 0 にする focused test を passした。Evidence: `test_e2e_selfhost_standalone_read_file_returns_fd_read_errno_after_partial_read`、`test_wasi_read_file_preserves_fd_read_errno`、`/tmp/lsharp-selfhost-close-errno-read-valid.wasm`。残る I/O は path-open / full fd error、1024 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary である。

2026-07-16 追加進捗: Rust WASI oracle の `read-file` / `write-file` / `write-file-bytes` で `path_open` errno を保存し、open failure を read は空 String、write は signed `-1` として fail-closed にした。未初期化 fd に対する後続 `fd_read` / `fd_write` / `fd_close` を行わないことを Wasm body scan と no-preopen runtime test で確認した。Evidence: `test_wasi_file_helpers_preserve_path_open_errno`、`test_wasi_file_helpers_fail_closed_on_path_open_errno`、`crates/lsharp-wasm/src/wasi.rs`。selfhost standalone の custom `path_open` errno shim parity、`fd_close` を含む full fd error semantics、4096 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。

2026-07-16 追加進捗: Rust oracle の `read-file` で `fd_filestat_get` errno を local へ保存し、stat 失敗時は開いた fd を close して空 String を返す fail-closed 分岐を追加した。Wasm body scan の RED→GREEN (`call fd_filestat_get` 後の `LocalSet`) を passした。Evidence: `test_wasi_file_helpers_preserve_path_open_errno`、`crates/lsharp-wasm/src/wasi.rs`。custom stat-error runtime shim、read close errno の Rust oracle parity、full fd error differential、4096 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。

2026-07-16 追加進捗: standalone bounded `read-file` の上限を 1024 bytes から 4096 bytes へ拡張した。4096 bytes (`a` * 4095 + `b`) の fixture で旧 body が全量を返せない RED を再現し、修正後は selfhost compiler 生成 artifact を標準 WASI preopened directory で実行して全量を byte-for-byte で確認した。Evidence: `test_e2e_selfhost_standalone_read_file_returns_all_bytes_at_4096`、`selfhost/src/Backend/Wasm/WasmEmit.ls`。これは bounded 4096-byte slice の拡張であり、4096 bytes 超の read、custom `path_open` errno shim parity、`fd_filestat_get` を含む full fd error semantics、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は未完了である。

2026-07-17 追加進捗: standalone `file-exists?` の `fd_close` errno を fail-closed に揃えた。`fd_close=1` shim 下で旧 selfhost artifact が `true` を返す RED を再現し、Rust oracle は `fd_close` の戻り値を errno local へ保存、selfhost Wasm emitter は close結果を既存 errno local へ保存する GREEN を実装した。修正後の selfhost compiler 生成 artifact は同じ shim で `false` を返し、Rust body scan と actual E2E（close 1 回）を passした。Evidence: `test_wasi_file_exists_preserves_fd_close_errno`、`test_e2e_selfhost_standalone_file_exists_returns_false_on_fd_close_errno`、`crates/lsharp-wasm/src/wasi.rs`、`selfhost/src/Backend/Wasm/WasmEmit.ls`。残る full fd error は selfhost custom `path_open` / stat parity、zero-byte / over-reportingの全 helper差分、4096 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary である。
2026-07-17 追加進捗: standalone bounded `read-file` の custom `path_open` errno parity を actual E2E で固定した。`path_open=1` shim 下で selfhost artifact は空 String を返し、`fd_read=0`、`fd_close=0` で open failure 後の未初期化 fd 使用を避けることを確認した。なお bounded standalone `read-file` は `fd_filestat_get` を import しないため、stat errno はこの slice の境界外であり、full pipeline 側の fd error differential として残す。Evidence: `test_e2e_selfhost_standalone_read_file_returns_empty_on_path_open_errno`、`run_with_partial_fd_read_with_path_open_errno`、`selfhost/src/Backend/Wasm/WasmEmit.ls`。残る full fd error は stat/full pipeline parity、zero-byte / over-reporting の全 helper差分、4096 bytes 超の read、root capacity / `memory.grow`、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary である。

## 検証と残タスク

- `bash scripts/ci/test-native-selfhost-dev.sh` は runner の source refresh、native direct command routing、external helper routing、Rust-only command の明示拒否を検証する。
- 2026-07-16 の local helper contract gate は `bash scripts/ci/test-native-selfhost-dev.sh`、`test-native-selfhost-install.py`（7 tests）、`test-native-selfhost-doc.py`（8 tests）、`test-native-selfhost-lsp-stdio.py`（5 tests）、`test-native-selfhost-repl.py`（8 tests）、`test-native-selfhost-component.py`（9 tests）がすべて pass した。Cargo / host `lsharp` を poison した fixtureで、install / doc / lsp / repl / component の routing と失敗時の stderr・exit 境界を確認したが、fake stage0/native program の contract gateであり、実 stage0 `program.native` と外部 toolを結んだ release evidence は未完了である。
- `test_native_selfhost_dev_source_file_smoke_script_contract` は smoke が host fallback を発見・利用しないことを固定する。
- `test_e2e_native_macos_aarch64_materializer_executes_tiny_stage_code` は macOS materializer の再署名成功時に stderr が空であることを固定する。
- `test_guest_compile_success_does_not_request_host_fallback` と `test_test_command_is_selfhost_shadow_command` は driver の guest-success / Rust fallback boundary を固定する。
- Mac Apple Silicon の actual stage0 source-file smoke は 2026-07-13 の historical evidence として残るが、current checkout `8e39a82` に一致する stage0 package は `/tmp`、`ci-artifacts`、`dist` 内で確認できなかった。既存の Mac release archive は `lsharp-native-selfhost-stage0` ではないため、current-source Mac gate の証拠には再利用しない。
- Linux x86_64 の 2026-07-17 source-file smoke は historical verified slice として保持する。stage0 producer `/tmp/lsharp-native-linux-x86-stage0-7807089` の一時 package は `parse` / `check` / `fmt` / 通常+metadata `test` / `compile -o` / `build -o`、stdout/stderr、core Wasm header / positive size gate を pass したが、current checkout に一致する provenance package としては再利用しない。VM は 16 GiB RAM / 12 GiB disk、最大 compiler RSS 約 12.3 GiB、終了後は 11 GiB 中 3.1 GiB 使用に戻った。`7807089` の stage1 -> stage2 -> stage3 selfregen fixed-point pass は履歴 evidence として保持する。生成 artifact の `wasm-tools validate` / standalone runtime は別の残ゲートである。
- 2026-07-15 の追試では、source commit を持たない repo 内 `v2-16d-twoarg-stage0-v1` を再利用した場合に `parse` が tagged pointer を `decls` として出力して停止した。これは成功済み `7807089` package の結果とは別の artifact provenance failure であり、fresh clone / release input には source commit と fixed-point evidence を伴う stage0 だけを使う必要がある。
- selfhost Wasm の standalone Preview1 first slice は、標準 `wasi_snapshot_preview1.fd_write` / `args_sizes_get` / `args_get` / `path_open` / `fd_close` / `fd_read` import、record/accessor output、root_push/root_set/root_pop output、static string literal の `print-string` output、length-header と standalone allocator を使う `string-concat` output、`substring "hello world" 6 11` の `world` output を `run_wasm_wasi` で検証済みである。`command-line-args` は `alpha` / `beta` を渡して `2\n` を返し、bounded individual argv runtime は `command-line-arg 0` で `alpha` を返す。bounded `file-exists?` runtime は preopened dir の `exists.txt` / `missing.txt` に対して `1\n0\n` を返し、bounded `read-file` runtime は `read.txt` の `payload`、300-byte fixture、4096-byte fixture の全量を返す。一回の read は最大 4096 bytes で、missing path は空文字列、4096 bytes 超の file は先頭 chunk のみという narrow contract である。bounded string `write-file` runtime は preopened dir の `write.txt` に `payload` を作成し、open failure は `-1`、成功時は `nwritten` を返す。bounded raw vector `write-file-bytes` runtime は Vector の下位8 bitを packed buffer へ変換し、preopened dir の `raw.wasm` に `00 61 73 6d` を作成する。substring は source length の header、範囲検証、結果の length-header、byte copy loop を持ち、範囲外は runtime trap になる。これは全 capability parity ではなく、runtime stub の誤成功を防ぐための最小 boundary であり、partial-write / full fd error semantics、4096 bytes を超える read、dynamic root stack capacity、動的 data/heap layout、component sidecar、Linux `App.Cli` native source-file output は残件である。
- `selfhost/src/Backend/Wasm/WasmEmit.ls` の standalone root stack は top cell `64`、stack base `128`、heap start `8192` の固定 layout を使う。data section 自体は `1024` から始まるが、standalone compiled data vector の 3072 byte prefix により実リテラル領域は `4096` から始まる。240 slot の root_push/root_set bounds trap と、`EmbeddedCli` の source/data conservative gate を追加したため、範囲外入力は artifact を成功生成しない。root stack、I/O scratch、data、heap の動的拡張を実装したわけではなく、first slice の安全な上限である。
- source length `1024` 以上は拒否し、standalone compiled data vector は 3072 byte prefix を含むため内部 data length `<4096` でゲートする。実リテラル領域は `4096` から始まるが、raw source の data payload は従来どおり 1024 未満という conservative policy である。コメントや literal 以外の source 量でも拒否し得るため、dynamic data/heap layout を実装した時点で置き換える。
- 2026-07-16 の `retain-roots 120` fixture で、root stack が static literal を上書きする RED を確認し、source/file compile 両経路の data prefix と heap start relocation を実装した。`test_embedded_cli_component_compile_preview1_writes_runnable_wasm_without_driver` は既存 Preview1 capability と `"safe"` output を含めて pass した。残りの I/O scratch relocation、root capacity / memory.grow、partial-write / full error semantics、1024 bytes を超える read、component sidecar、Linux `App.Cli` native source-file output は未完了である。
- 2026-07-16 の standalone raw `write-file-bytes` fixture で、iovec `352` / nwritten `360` が root slot を上書きする RED を確認し、raw write の scratch を root stack 後方の `2048` / `2056` へ移した。exact actual Wasm test は既存 Preview1 capability、`"42"` root output、raw file output を含めて pass した。read/string write/argv の scratch、root capacity / memory.grow、partial-write / full error semantics、長大 read、component sidecar、Linux `App.Cli` native source-file output は未完了である。
- 2026-07-16 追加進捗: standalone Preview1 の file runtime 全体で root stack 内 scratch を再配置した。iovec は `2176`、nread/nwritten は `2184`、fd pointer は `2240`（root stack 外、raw opcode scanner と衝突しない signed LEB）へ移し、`write-file-bytes` の slot 28 RED (`17179877424`)、string `write-file` の slot 28 RED (`30064772519`)、fd pointer の slot 19 RED (`4`) をそれぞれ fixture で再現してから修正した。raw/string/read の deep-root actual Wasm fixture は exact test `1 passed`（126.99s）。さらに `command-line-arg` の args_sizes_get scratch `256`/`260` が slot 16 を壊す RED (`47244640258`) を確認し、`2256`/`2260` へ移した。`command-line-args` も `160`/`164` が slot 4 を壊す同値 RED (`47244640258`) を確認し、`2272`/`2276` へ移した後、argv table/buffer はこの時点では `16384`/`32768` 固定のまま argv 全 fixtureを含む exact test `1 passed`（128.43s）となった。残りは root capacity / memory.grow、partial-write / full error semantics、256 bytes を超える read、dynamic data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary である。
- 2026-07-16 追加進捗: `command-line-arg` の argv table / buffer を固定 `16384` / `32768` から allocator-backed の連続領域へ移した。`vector-new 4096` の既存 heap を sentinel にした RED では table / buffer overwrite による `140763258191872` / `7305401963912391777` を確認し、GREEN では `alpha0\n0\n` と `wasm-tools validate`、embedded Preview1 回帰が pass（exact test 1 passed、174.24s）した。argv scratch overlap は verified slice から除外し、残りは root capacity / memory.grow、partial-write / full error semantics、1024 bytes を超える read、dynamic data/heap layout の全面化、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary とする。
- 2026-07-16 追加進捗: standalone Preview1 `read-file` の allocator size / `fd_read` iovec length を 256 bytes から 1024 bytes へ揃えた。300-byte fixture は RED で先頭 256 bytesに切れ、GREEN では全量を返す exact actual Wasm test `1 passed`（145.22s）となり、`read-large.wasm` の `wasm-tools validate` も pass した。これは single-read bounded slice の拡張であり、partial-read / full error semantics、1024 bytes を超える read、dynamic root/data/heap layout、component sidecar、Linux `App.Cli` native source-file output、bootstrap/oracle/host boundary は残る。
- V2-16d の Linux/Mac core smoke は historical evidence として保持するが、2026-07-18 の current-source Linux replay が exit 137 で停止し、current-source native gate、stage0 provenance、artifact/runtime の完了条件が未充足であるため、V2-16d は完了扱いに戻さない。V2-16e と bootstrap/oracle、public stage0 acquisition、Rust oracle/differential、emergency rollback、host integration、未完の言語意味論は継続する。

### EC-M1-05 deterministic property smoke profile (2026-07-18)

移行期の `:property` に、Rust oracle と selfhost `TestRunner` の両方で実行できる狭い verified slice を追加した。対象は、単一の typed binder `[x Int]`、precondition/seed/shrink なし、`:cases 1..5 :postcondition expr` の option 順序、外側の `for-all` 括弧を含む raw payload である。入力列は既存の deterministic invariant prefix `[0, 1, 5, -1, 42]` を共有し、postcondition の Bool 結果を各入力で評価する。selfhost CLI は property を `properties:N` として集計し、偽の結果は failure、non-Bool predicate は `LS1002`、profile 外は `LS3002` として成功扱いにしない。

Evidence: Rust `test_run_metadata_tests_executes_deterministic_property_smoke` / `test_run_metadata_tests_reports_failing_deterministic_property` / `test_run_metadata_tests_rejects_property_outside_deterministic_smoke_profile`、selfhost `test_e2e_selfhost_runner_executes_deterministic_property_smoke`、`test_e2e_selfhost_cli_reports_deterministic_property_smoke`、`test_e2e_selfhost_cli_rejects_non_bool_deterministic_property`、`test_e2e_selfhost_runner_rejects_property_seed_option`、Rust `metadata_check` 29 tests、Wasm `test_runner` unit 6 tests。RED で `:cases` と `:postcondition` の間の `:seed` が profile code `0` になる抜けを確認し、GREEN では option の直結を要求して `:seed` / `:shrink` / `:precondition` を `LS3002` に揃えた。Rust 側は `PropertySmokeTestSpec` と固定 sample projection、selfhost 側は `Tools.Test.PropertyRunner` の raw projection と既存 evaluator を使う。

これは EC-M1-05 全体または Rust 完全撤去ではない。type-directed generator、constraint conjunction、複数 binder / `Bool` 以外の型、precondition、seed、shrink、coverage bucket、diagnostic/span parity、Mac Apple Silicon / Linux x86_64 の current-source native artifact/runtime gate は残る。したがって、現時点ではこの deterministic smoke profile を使う日常開発は Rust なしで進められるが、profile 外の property と全機能の oracle/bootstrap 境界には Rust を残す。

### EC-M1-02 single precondition property evaluator (2026-07-18)

deterministic `Int` property の verified slice を、precondition なしから単一 precondition 付きへ拡張した。Rust `PropertySmokeTestSpec` と selfhost `PropertyRunner` は、単一の Bool precondition を sample filter として保持し、precondition が false の sample では対象関数と postcondition を評価せず skip する。selfhost の result `actual` は実行対象 sample 数を返し、全 sample が false の場合は vacuous success にせず `LS2005` を返す。precondition または postcondition が Bool でない場合は `LS1002`、複数 precondition、複数 binder、seed/shrink 付き profile は従来どおり `LS3002` で明示拒否する。

Evidence: `test_run_metadata_tests_executes_property_precondition_and_skips_false_samples`、`test_e2e_selfhost_runner_executes_property_precondition_and_skips_false_samples`（5 samples 中 4 executed）、`test_e2e_selfhost_runner_rejects_vacuous_property_precondition`（`LS2005`）、selfhost runner prefix 6 tests、Wasm `test_runner` unit 6 tests、Rust tooling metadata 15 tests、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは単一 `Int` binder の deterministic evaluator に限定した verified sliceであり、複数 precondition の conjunction、一般 `TypeExpr`、type-directed generator、seed/shrink/coverage、predicate 個別 span、structured assurance report、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。したがって property evaluator 全体や Rust 完全撤去の完了とは扱わず、Rust oracle / bootstrap 境界を維持する。

2026-07-18 の current-source Linux x86_64 replay では、host-side native artifact probe 12 件と stage1 bundle生成（code 4,149,774 bytes、data 1,511 bytes）までは pass した。一方、Lima VM の actual stage1 は chunk `0-64` から `0-1` まで自動分割しても exit 137 となった。actual heap を 4 GiB から 2 GiB に下げた再利用 replayでも RSS は約 15.7 GiB まで増え、同じ `0-1` が再現したため、chunk 数や VM disk 容量ではなく native runtime の heap/root/data layout が failure boundary である。VM は 11 GiB disk 中約 7.8 GiB free のまま終了し、重い replay の再試行は行わない。Linux current-source native gate と、これを閉じる `LEGACY-RUNTIME-01` / `LEGACY-ROOT-01` 相当の runtime 容量調整は未完了である。

### EC-M1-02 multiple precondition conjunction evaluator (2026-07-18)

deterministic `Int` property の precondition evaluator を、単一条件から source order の複数条件へ拡張した。Rust `PropertySmokeTestSpec` は `Vec<Expr>` を保持し、生成した Wasm test は各 precondition を短絡 conjunction として外側から評価する。selfhost `PropertyRunner` / `TestRunner` も同じ vector を保持し、unknown-variable 検査、Bool 検査、sample filter を全条件へ適用する。どれか一つでも false の sample は target function と postcondition を評価せず、全条件が true の sample だけを `actual` に数える。全 sample が skip された場合は従来どおり `LS2005` とする。

Evidence: `test_run_metadata_tests_executes_all_property_preconditions_as_conjunction`、`test_e2e_selfhost_runner_executes_all_property_preconditions_as_conjunction`（`[0, 1, 5, -1, 42]` のうち 3 件を実行）、Rust tooling metadata 16 tests、selfhost runner property 7 tests、Wasm `test_runner` unit 6 tests、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは単一 `Int` binder と deterministic cases に限定した conjunction slice であり、一般 `TypeExpr`、複数 binder、type-directed generator、seed/shrink/coverage、predicate 個別 span、structured assurance report、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。したがって、対応済み profile の日常開発は Rust なしで進められるが、profile 外の property semantics、stage0 provenance、Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-02 two-Int-binder property evaluator (2026-07-18)

deterministic property profile を、単一 binder から最大 2 個の `Int` binder へ拡張した。Rust `PropertySmokeTestSpec` と selfhost typed contract bridge は binder name/hash を source order の vector として保持する。scalar prefix `[0, 1, 5]` の lexicographic pair prefix `(0,0)`, `(0,1)`, `(0,5)`, `(1,0)`, `(1,1)` を cases の先頭から共有し、precondition conjunction は両 binder を同じ環境へ束縛して評価する。target function の引数数は binder 数と一致することを要求し、selfhost は不一致を未実装 property として成功扱いにしない。

Evidence: `test_run_metadata_tests_executes_two_int_property_binders`、`test_e2e_selfhost_runner_executes_two_int_property_binders`（5 pair 中 1 件を precondition skip して `actual=4`）、既存の single-binder / conjunction / seed / unsupported profile regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは 1..2 個の `Int` binder と deterministic pair prefix に限定した verified slice であり、後続の generic typed prefix により 3〜8 binder の cases 1..2 は別途拡張された。一般 `TypeExpr`、owner/binder の独立 arity、type-directed generator、seed/shrink/coverage、predicate 個別 span、structured assurance report、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 の current-source native gate は残件である。対応済み profile の変更は Rust なしで進められるが、profile 外 semantics、stage0 provenance、Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-02 public selfhost `test` CLI property slice (2026-07-18)

公開 `run-test-source` まで同じ verified profile を接続した。2 個の `Int` binder、precondition conjunction、owner arity 一致の fixture が `properties:1`、`failures:0`、exit `0` で集計されることを `test_e2e_selfhost_cli_reports_two_int_property_binders` で確認した。これは public CLI summary の実行契約を閉じる evidence である。

この E2E は Rust host の `compile_and_run` で巨大な selfhost CLI bundle を生成・実行する oracle lane で、今回の focused run は 417.34 秒だった。通常開発の Rust-free native gate ではなく、Rust oracle / differential 用の重い検証として扱う。current-source stage0 provenance、Mac Apple Silicon / Linux x86_64 の native artifact/runtime gate、profile 外 property semantics は未完了であり、Rust の bootstrap / oracle / host integration 境界を維持する。

### EC-M1-04 property binder scope collision (2026-07-18)

canonical property binder の lexical scope を fail-closed にした。同じ binder 名の重複と予約名 `result` は、Rust canonical checker では binder span 付き metadata error、selfhost `check` では structural code `2007`、selfhost deterministic runner では未対応 profile code `3002` として拒否する。これにより property binder が暗黙の postcondition `result` を上書きしたり、同じ hash の環境束縛へ縮退したりしない。

Evidence: `canonical_property_rejects_duplicate_binder_names`、`canonical_property_rejects_result_binder_name`、`test_e2e_selfhost_cli_check_rejects_property_binder_name_collisions`、`test_e2e_selfhost_runner_rejects_property_binder_name_collisions`、Rust metadata contract 22 tests、`PropertyRunner.ls` / `TypeInferAssertions.ls` source check（各 `diagnostics:0`）。これは binder scope の verified sliceであり、一般 `TypeExpr`、type-directed sampling/shrink、全 evaluator、diagnostic/span parity、current-source Mac/Linux native artifact/runtime gate、stage0 provenance は残件である。Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-05 single-Bool property sampling (2026-07-18)

deterministic property profile に単一 `Bool` binder の verified slice を追加した。Rust の `PropertySmokeTestSpec` は binder type を `Int` / `Bool` として保持し、単一 Bool の入力列を `[false, true]` に固定する。selfhost `PropertyRunner` は typed binder の type-name hash を保持して test-case に伝播し、`TestRunner` は同じ列を生成して postcondition を実値評価する。`:cases` は Bool slice では `1..2` に限定し、`cases 3` 以上、Bool の複数 binder、Int/Bool 混合、一般 `TypeExpr` は暗黙に Int へ変換せず `LS3002` / `3002` で拒否する。

Evidence: Rust `test_run_metadata_tests_executes_bool_property_binder`、`test_run_metadata_tests_rejects_bool_property_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_bool_property_binder`、`test_e2e_selfhost_runner_rejects_bool_property_above_two_cases`、既存の 2-Int property regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。Rust oracle は Wasm test program の `false` / `true` を実行し、selfhost native bundle は同じ 2 cases を評価して `actual=2` を返す。

これは Bool 全体または EC-M1-05 全体の完了ではない。Bool と Int の混合 generator、複数 Bool binder、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。したがって、単一 Bool slice の日常開発は Rust なしで進められるが、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 mixed Int/Bool property sampling (2026-07-18)

deterministic property profile を、単一 Bool から 2 binder の `Int`/`Bool` mixed slice へ拡張した。対象は source order を保った `[value Int flag Bool]` または `[flag Bool value Int]`、`:cases 1..2` に限定し、型別の入力列を `[0, false]`, `[1, true]` とする。Rust `PropertySmokeTestSpec` は binder type vector を検査してこの profile を選択し、Wasm test runner は各 binder type の順序に従って literal を生成する。selfhost typed contract も同じ type-name hash vector を test-case へ渡し、`TestRunner` が Int/Bool の値を source order のまま評価する。

Evidence: Rust `test_run_metadata_tests_executes_mixed_int_bool_property_binders` / `test_run_metadata_tests_rejects_mixed_int_bool_property_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_mixed_int_bool_property_binders` / `test_e2e_selfhost_runner_rejects_mixed_int_bool_property_above_two_cases`、single Bool / two-Int regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは mixed type 全体または EC-M1-05 全体の完了ではない。mixed cases 3 以上、複数 Bool binder、3 binder、一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。verified slice の日常開発は Rust なしで進められるが、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 two-Bool property sampling (2026-07-18)

Bool binder の typed profile を、単一 Bool から最大 2 個の Bool binder へ拡張した。対象は `[a Bool b Bool]`、`:cases 1..2` に限定し、source order の deterministic prefix `[false, false]`, `[true, true]` を Rust oracle と selfhost runner で共有する。Rust の binder type vector、selfhost の type-name hash vector、既存の 2 binder sample projection をそのまま利用し、Bool 値を Int に丸めず実 Bool AST として postcondition/owner function へ渡す。

Evidence: Rust `test_run_metadata_tests_executes_two_bool_property_binders` / `test_run_metadata_tests_rejects_two_bool_property_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_two_bool_property_binders` / `test_e2e_selfhost_runner_rejects_two_bool_property_above_two_cases`、single Bool / Int-Bool mixed / two-Int regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは Bool generator 全体または EC-M1-05 全体の完了ではない。Bool 3 binder、Bool/Int/Bool の一般 mixed arity、一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。verified slice の日常開発は Rust なしで進められるが、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 three-Bool property sampling (2026-07-18)

Bool binder の typed profile を 3 個まで拡張した。対象は source order を保つ `[a Bool b Bool c Bool]`、`:cases 1..2` に限定し、Rust oracle と selfhost runner が `[false, false, false]`, `[true, true, true]` を共有する。Rust の `PropertySmokeTestSpec` は 3-要素の Bool vector を許可し、Wasm test runner は各 binder type を順番に Bool literal へ投影する。selfhost `PropertyRunner` は 3-要素 Bool profile のみ arity 3 を許可し、`TestRunner` は triple vector として owner/precondition/postcondition に渡す。3-`Int` や 3-要素 mixed profile、`:cases 3` は従来どおり未実装 profile として `LS3002` / `3002` で拒否する。

Evidence: Rust `test_run_metadata_tests_executes_three_bool_property_binders` / `test_run_metadata_tests_rejects_three_bool_property_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_three_bool_property_binders` / `test_e2e_selfhost_runner_rejects_three_bool_property_above_two_cases`、既存の single Bool / Int-Bool mixed / two Bool / three Int regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは 3-`Bool` の deterministic prefix に限定した verified slice であり、一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。3 個以上の Int/Bool binder は次の generic typed prefix で扱い、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 three-Int property sampling (2026-07-18)

deterministic property profile に 3 個の `Int` binder の最小 slice を追加した。対象は source order を保つ `[a Int b Int c Int]`、`:cases 1` に限定し、Rust oracle と selfhost runner が `[0, 0, 0]` を 1 case だけ共有した。これは後続の generic typed prefix へ統合され、現在は 3 個以上の Int/Bool binder を cases 1..2 で source-order loop から生成する。最初の slice の cases boundary として、cases 3 以上は Rust `LS3002` / selfhost `3002` で明示拒否する。

Evidence: Rust `test_run_metadata_tests_executes_three_int_property_binders` / `test_run_metadata_tests_rejects_three_int_property_binders_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_three_int_property_binders` / `test_e2e_selfhost_runner_rejects_three_int_property_binders_above_two_cases`、既存 single/two-Int、Bool、mixed、3-Int cases boundary regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは 3-Int の cases 1 deterministic prefix に限定した verified sliceであり、3 個以上の Int/Bool binder の cases 2 prefixは次の generic typed profileで扱う。一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。対応済み profile の日常開発は Rust なしで進められるが、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 three-binder mixed Int/Bool property sampling (2026-07-18)

deterministic property profile に 3 個の `Int`/`Bool` mixed binder の最小 slice を追加した。対象は source order を保つ `[left Int flag Bool right Int]`、`:cases 1..2` に限定し、Rust oracle と selfhost runner が `[0, false, 0]`, `[1, true, 1]` を共有する。Rust `PropertySmokeTestSpec` は 3-要素で少なくとも 1 個の Bool を持つ既知の Int/Bool 組み合わせを許可し、Wasm test runner は各 binder type に従って literal を生成する。selfhost `PropertyRunner` は 3-要素 mixed profile を cases 2 以下で許可し、`TestRunner` は source-order triple vector として owner/precondition/postcondition へ渡す。cases 3 は Rust `LS3002` / selfhost `3002` で明示拒否する。

Evidence: Rust `test_run_metadata_tests_executes_three_mixed_int_bool_property_binders` / `test_run_metadata_tests_rejects_three_mixed_int_bool_property_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_three_mixed_int_bool_property_binders` / `test_e2e_selfhost_runner_rejects_three_mixed_int_bool_property_above_two_cases`、既存 3-Int / 3-Bool / 2-binder mixed regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは既知の 3-要素 Int/Bool 組み合わせと cases 1..2 に限定した verified slice であり、4 個以上の typed binder は次の generic profileへ移行した。一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、structured assurance report、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。対応済み profile の日常開発は Rust なしで進められるが、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 generic Int/Bool typed prefix (2026-07-18)

deterministic property sampling の arity 分岐を、個別の 3-binder case から source-order `binder_types` vector の loop へ統合した。1〜2 個の `Int` は既存の scalar/pair prefix と `:cases 1..5` を維持し、3〜8 個の `Int`/`Bool` は各 binder の型に応じた `[0,false,...]` / `[1,true,...]` の typed prefix を `:cases 1..2` で実行する。Rust `PropertySmokeTestSpec`、Wasm test runner、selfhost `PropertyRunner` の profile 判定と `TestRunner` の materialize が同じ境界を持つ。4 binder mixed `[first Int flag Bool second Int again Bool]` の positive/negative fixture で cases 2 実行と cases 3 の `LS3002` / `3002` 拒否を確認した。

Evidence: Rust `test_run_metadata_tests_executes_four_mixed_int_bool_property_binders` / `test_run_metadata_tests_rejects_four_mixed_int_bool_property_above_two_cases`、selfhost `test_e2e_selfhost_runner_executes_four_mixed_int_bool_property_binders` / `test_e2e_selfhost_runner_rejects_four_mixed_int_bool_property_above_two_cases`、既存 3-Int / 3-Bool / 3-binder mixed regression、`PropertyRunner.ls` / `TestRunner.ls` source check（各 `diagnostics:0`）。

これは Int/Bool の 1〜8 binder deterministic prefix に限定した verified slice であり、9 個以上の binder、String/record/ADT などの一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、predicate/span parity、structured assurance report、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。対応済み profile の日常開発は Rust なしで進められるが、profile 外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-04 selfhost property runner non-vacuity (2026-07-18)

selfhost `TestRunner` の直接 materialize 経路でも、静的に true な `:property` postcondition を成功扱いしないようにした。`true` と静的な Int 比較を AST の 3 値（true / false / unknown）で判定し、postcondition が true の場合は `LS2005` を返す。precondition の静的 false 判定も同じ判定器を通し、既存の全 sample skip による `LS2005` と整合させる。型推論モジュールを runner bundle に暗黙追加せず、既存 canonical checker と同じ比較・`and`/`or`/`not` の narrow 判定を selfhost 実行境界へ明示した。

Evidence: selfhost `test_e2e_selfhost_runner_rejects_vacuous_property_postcondition`、`test_e2e_selfhost_runner_rejects_statically_true_property_postcondition`、既存 `test_e2e_selfhost_runner_rejects_vacuous_property_precondition`、Rust tooling の canonical vacuity tests、`TestRunner.ls` source check（`diagnostics:0`）。RED では runner が `postcondition true` と `(= 1 1)` を `passed=1, actual=1, diagnostic=0` としていたが、GREEN ではいずれも `passed=0, actual=0, diagnostic=2005` になった。

これは literal/static comparison と deterministic property runner の non-vacuity slice に限定される。一般的な constant propagation、動的に常に true となる postconditionの検出、一般 `TypeExpr`、type-directed generator、seed/shrink/coverage bucket、structured assurance report、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenance は残件である。対応済み runner slice の日常開発は Rust なしで進められるが、property evaluator 全体と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-05 single-String property sampling (2026-07-19)

deterministic property smoke profile に単一 `String` binder の verified slice を追加した。対象は `[sample String]`、`:cases 1..5`、`(string-eq result sample)` の postcondition に限定する。Rust `PropertySmokeTestSpec` は `PropertyBinderType::String` を保持し、Wasm test runner は `""`, `"a"`, `"hello"`, `"lsharp"`, `"42"` を source literal として生成する。selfhost `PropertyRunner` は `String` type-name hash と単一 binder profileを認識し、`TestRunner` は実行時StringをAST value wrapperへ保持して `string-eq` を意味比較する。

Evidence: `property_smoke_spec_accepts_single_string_binder`、Rust `test_runner::tests::test_property_string_binder_execution`、selfhost `test_e2e_selfhost_runner_executes_string_property_binder`、`./target/debug/lsharp check selfhost/src/Tools/Test/PropertyRunner.ls`、`./target/debug/lsharp check selfhost/src/Tools/Test/TestRunner.ls`（各 `diagnostics:0`）。Rust oracle と selfhost runner は同じ5 casesを実行し、`passed=1` / `actual=5` / `diagnostic=0` を確認した。

これは単一Stringの deterministic prefix に限定した verified sliceであり、Stringの複数binder・Int/Boolとの混在、一般 `TypeExpr`、constraint/type-directed generator、seed/shrink/coverage bucket、structured assurance report、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、stage0 provenanceは残件である。対応済みprofileの日常開発はRustなしで進められるが、profile外 property semantics と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-06 selfhost `test --format json` assurance slice (2026-07-19)

`App.Cli` の `test --json` / `test --format json` に、`implementation_conformance` と `intent_validation` を分けた単一行 JSON report を追加した。前者は `status`、`method`、実行 `cases`、seed、generator、shrinks、実行/失敗 coverage、diagnostics、target、provenance を返し、後者は stakeholder intent を自動検証せず `unknown` として open questions / independent reviews / contradicting observations を返す。top-level `verified` は生成しない。既存の text `test` 出力と exit code は維持し、JSON の conformance failure は exit `2` とする。

Evidence: Rust-hosted actual argv `test_e2e_selfhost_cli_main_with_args_test_format_json_file` (1 passed)、current-source Mac Apple Silicon native `App.Cli` の JSON success/failure、String property success、text vacuous `LS2005` rejection (5 ignored contracts passed)、`EmbeddedCli` の source contract `test_e2e_selfhost_embedded_cli_test_json_contract_is_present`、同 current-source native `test_native_embedded_cli_test_format_json_source_file_contract` / `test_native_embedded_cli_test_format_json_reports_vacuous_failure_source_file_contract` (2 ignored contracts passed)、`./target/debug/lsharp parse selfhost/src/App/Cli.ls` / `EmbeddedCli.ls` と `./target/debug/lsharp check selfhost/src/App/Cli.ls` / `EmbeddedCli.ls` (`diagnostics:0`)。current native CLI は stage0 の source provenance を自己取得しないため、report の `target` / `source_commit` / `artifact_digest` は `unknown` と明示する。

これは App.Cli の単一 String property と単一 Int vacuous failure、および EmbeddedCli の legacy metadata success と vacuous property failure に限定される。全 form の EmbeddedCli parity、Rust/selfhost differential、provenance 注入、Linux x86_64 current-source artifact/runtime、一般 `TypeExpr` の coverage/seed/shrink、EC-M1-06 全体の完了条件は残件である。対応済み profile の日常開発は Rust なしで進められるが、report の unknown を成功扱いせず、Rust oracle / bootstrap / host integration の境界を維持する。

### EC-M1-06 canonical `:assert` failure coverage accounting (2026-07-20)

`TestRunner.run-assertions-loop` は Bool predicate が失敗しても実行済み件数を `actual=1` として result に保持するようになった。従来は `actual=passed` だったため、2件の assertion のうち1件が失敗すると `assurance-total-actual` が `cases=1` / `coverage.executed=1` と報告していた。修正後は `status=fail`、`method=assert`、`cases=2`、`coverage.executed=2`、`coverage.failed=1`、Bool failure の diagnostics `count=0` を返す。

Evidence: RED の `test_e2e_selfhost_cli_test_source_json_reports_assertion_failure_coverage` は `cases` の実値 `1` と期待値 `2` の差で失敗（`416.54s`）。GREEN は同 test `1 passed`（`406.84s`）、短い selfhost runner differential の `1 passed`（`24.71s`）で assertion result の両件 `actual=1` を確認した。変更後の temporary Cargo target は削除済みである。

これは canonical assertion の failure coverage accounting に限定した verified sliceであり、全 form の executed-count semantics、failure message/schema の Rust/selfhost parity、EmbeddedCli 実 argv、provenance 注入、Linux x86_64 current-source artifact/runtime、EC-M1-06 aggregate は残件である。

### Historical Mac native stage0 gate for the `10d0983b` snapshot (2026-07-19)

`selfhost/` source snapshot commit `10d0983b4f8e17d8b9ded439161f653c1bf91e4e` から Mac Apple Silicon の actual stage23 fixed-point を再生成し、stage2/stage3 の `program.native` を `source_commit` 付き stage0 package `/tmp/lsharp-stage0-10d0983b-macos-compiler` として materialize した。stage23 test は `765.58s` で pass、stage2/stage3 は exit `0`、stderr `0`、program/runtime/response/stdout/stderr の観測 hash が一致した。ただし `3afab678` と `81457b39` が後続で `selfhost/` を変更しているため、これは現行 `main` の gate ではなく snapshot の履歴 evidence である。

この package を使う `scripts/ci/native-selfhost-dev-source-file-smoke.sh` は `aarch64-apple-darwin native selfhost source-file smoke passed` となり、PATH 上で `cargo`、`rustc`、host `lsharp` を blockした状態の `parse` / `check` / `fmt` / 通常と metadata の `test` / `compile` / `build`、Wasm magic/positive-size gateを完走した。`scripts/native-selfhost-dev.sh` の同一 fingerprint stage reuse でも `fmt` / `compile` / `test` を確認した。現行 `main` でこの経路を使うには、最新 `selfhost/src` と一致する `source_commit` 付き stage0 を再生成する必要がある。

これは Mac の小規模 core sliceに対する snapshot native evidenceであり、Rust-free 全体完了ではない。String `:property` fixtureについては、snapshot native `App.Cli test` の ignored contract `test_native_app_cli_test_string_property_source_file_contract` が `examples:0` / `invariants:0` / `properties:1` / `failures:0` を出力し、public property evaluator への接続を確認した。同じ snapshot stage の `test_native_app_cli_test_rejects_vacuous_property_source_file_contract` は `examples:0` / `invariants:0` / `properties:1` / `failures:1` / `diagnostics:1,LS2005` と exit `2` を確認し、non-vacuity の失敗境界も public CLI へ接続した。ただし約1,000行以上の selfhost source（`Cli.ls`、`Parser.ls`、`Compiler.ls`、`TestRunner.ls` など）を native `check` すると stderr なしの exit `139` が残る。現行 `main` の Mac/Linux current-source stage0/artifact/runtime、large-source check、String の一般 profileを含む未検証 property semantics、stage0 public acquisition/release provenanceは残件である。

### EC-M1-01 selfhost invariant diagnostic span slice (2026-07-19)

legacy `:invariant` の未知変数 `LS1001` について、selfhost `TestRunner` の診断結果に Rust oracle と同じ invariant payload expression span を保持する slice を追加した。既存 result vector の index `0..3`（name / passed / actual / diagnostic code）は変更せず、failure result の後方 index `4..5` に `span-start` / `span-end` を追加し、`test-result-diagnostic-start` / `test-result-diagnostic-end` accessor から取得できる。AST layout は変更せず、`generate-tests` が受け取った source を tokenizer で走査して対象 `defn` の `:invariant` payload span を再取得する。

Evidence: RED の `test_e2e_selfhost_test_runner_preserves_unknown_invariant_diagnostic_span` は未実装 accessor の `UndefinedVar` で失敗（529.41s）、GREEN は同 test が 22.73s で passし、Rust `check_metadata` の `MetadataDiagnostic.span.start/end` と selfhost result の start/end が一致した。`./target/debug/lsharp check selfhost/src/Tools/Test/TestRunner.ls` は `diagnostics:0`。同じ focused run で valid invariant scope / local-let / computation / match、unknown lambda / computation / match、ordered invariant/assertion、canonical case の既存回帰も pass した。

これは top-level/module 内 `defn` の invariant payload expression span を source token から照合する legacy invariant failure の verified slice であり、未知識別子 token 単位の詳細 span、全 metadata form の diagnostic/span parity、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、EC-M1-01 aggregate は残件である。strict Bool の Rust oracle parity は次の sliceで記録する。対応済み slice の日常開発は Rust なしで進められるが、未対応 contract semantics と target gate の Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-01 invariant strict Bool diagnostic/span parity (2026-07-19)

Rust `metadata_check` の legacy `:invariant` strict Bool preflight と selfhost `TestRunner` の failure result を同一 fixture で比較した。`(+ x 1)` は両境界で diagnostic code `LS1002`（selfhost internal code `2`）となり、source span も invariant payload expression に一致する。併せて、unknown variable `LS1001` の span parity regression も維持した。selfhost は defn 全体ではなく `:invariant` payload を source tokenizer から再取得し、既存 result index `0..3` を変更せず `4..5` の span fields を使う。

Evidence: `test_e2e_selfhost_test_runner_preserves_non_bool_invariant_diagnostic_span`、`test_e2e_selfhost_test_runner_preserves_unknown_invariant_diagnostic_span`、`cargo test -p lsharp-types --lib metadata_check -- --nocapture`（29 passed）、`./target/debug/lsharp check selfhost/src/Tools/Test/TestRunner.ls`（`diagnostics:0`）。

これは legacy invariant の strict Bool と unknown-variable の code/span parity に限定した verified sliceであり、structured report、他の metadata form、computation/match の一般 semantics、current-source Mac Apple Silicon / Linux x86_64 native artifact/runtime gate、EC-M1-01 aggregate は残件である。対応済み slice は L# で日常開発できるが、未対応 semantics の Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-01/06 structured diagnostic span report slice (2026-07-19)

legacy `:invariant` の strict Bool failure を `App.Cli` / `App.EmbeddedCli` の structured assurance report へ接続した。`TestRunner` は examples / invariants / assertions / cases / properties の順に最初の diagnostic span を選び、JSON の `implementation_conformance.diagnostics.firstErrorSpan` として `{start, end}` を返す。preflight で suite がまだ生成されない場合は `0..0` を返し、既存の diagnostic code / exit boundary は変更しない。

Evidence: RED で `firstErrorSpan` が JSON に存在せず `Null` になったことを確認し、GREEN の `test_e2e_selfhost_cli_test_source_json_reports_non_bool_invariant` は selfhost source runner で status `fail`、internal diagnostic code `2`、span `26..33`、exit output `2` を確認した（1 passed, 435.44s）。`test_e2e_selfhost_cli_main_with_args_test_format_json_non_bool_invariant` は実 CLI argv 経路で stdout 1 行の valid JSON、exit `2`、同じ code/span、runner `selfhost` を確認した（1 passed, 549.02s）。`test_e2e_selfhost_embedded_cli_test_json_contract_is_present` は EmbeddedCli の span builder / field 同期を確認し、変更ソースの `lsharp check` は diagnostics `0` になった。

これは Rust-hosted Wasm E2E と EmbeddedCli source contract に限定した verified sliceであり、current-source Mac Apple Silicon / Linux x86_64 native stage0 artifact/runtime、EmbeddedCli の実 argv runtime、全 metadata form の structured report、Rust/selfhost differential、EC-M1-01/06 aggregate は残件である。対応済み slice は L# で日常開発できるが、未対応 target gate と公開 surface の Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-06 EmbeddedCli actual argv JSON failure slice (2026-07-19)

`App.EmbeddedCli` を `Cli` と同じ non-Bool `:invariant` fixture に接続し、`test input.ls --format json` の実 argv 経路から structured failure report を返す slice を追加した。実行時 bundle は `EmbeddedCli.ls` を最終 entrypoint とし、`implementation_conformance.status=fail`、internal diagnostic code `2`、`firstErrorSpan=26..33`、`exit=2`、`runner=selfhost` を `Cli` と同じ contract で検証する。

Evidence: `test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_non_bool_invariant` は RED で canonical `EmbeddedCli.ls` の test mapping 欠落を検出し、mapping を追加した GREEN で `1 passed`、`407.43s`。続く differential assertion の GREEN は `1 passed`、`399.01s` で、Rust `metadata_check` が返す `succ` の diagnostic span と selfhost JSON の `firstErrorSpan` を一致確認した。既存 `Cli` actual argv failure test と共通 assertion を使い、出力 JSON が 1 行であること、failure status、diagnostics、span、exit boundary を確認した。

これは Rust-hosted Wasm による `EmbeddedCli` の単一 legacy invariant failure fixtureと、その diagnostic span の Rust/selfhost differential に限定した verified sliceであり、current-source Mac Apple Silicon / Linux x86_64 native stage0 artifact/runtime、EmbeddedCli の全 form parity、全 report field の differential、provenance 注入、EC-M1-06 aggregate は残件である。対応済み経路は Rust なしで L# 開発に使えるが、未検証の公開 surface を Rust fallback で完了扱いにせず、bootstrap / oracle / host integration 境界は維持する。

### EC-M1-06 Rust driver `test --format json` boundary (2026-07-31)

Rust driver の公開 `test` command に `--format text|json` を追加し、JSON を選んだ場合は
`implementation_conformance` と `intent_validation` を分離した単一行 report を返すようにした。
canonical `:case` / `:assert` の runtime pass/fail は conformance の `method`、`cases`、
`coverage`、`status` へ投影し、metadata preflight error は diagnostics として同じ report shapeへ
投影する。pass は exit `0`、conformance/preflight failure は exit `2` とし、top-level
`verified` は生成しない。既存の text 出力と `LSHARP_PATH`/EmbeddedCli delegation は維持し、
`test --format json` だけを Rust driver の明示的な JSON boundaryへ送る。

Evidence: `crates/lsharp-driver/tests/metadata_test_cli.rs` の canonical case/assert pass、assert
runtime failure、non-Bool invariant preflight failure の 3 actual binary tests が passし、stdout
1行、二軸 schema、runner `rust`、exit `0/2`、diagnostic code `1002` を固定した。
`test_json_metadata_test_stays_on_rust_driver_boundary` は JSON test が embedded componentへ
誤 delegate されないことを確認し、既存 text metadata unit testも passした。

これは Rust driver の canonical case/assert と preflight JSON boundaryに限定した verified partial
sliceである。property の sample-level executed count、全 formの Rust/selfhost field differential、
EmbeddedCli native runtime、source/artifact provenance、Mac Apple Silicon / Linux x86_64 current-source
artifact/runtime、EC-M1-06 aggregate は残件であり、TODO の `[~]` を維持する。

### Current-source Mac Apple Silicon native boundary (2026-07-19)

現行 `main` (`abe1e5d7e8f01248f622c15756e26f343f215a9c`) の `selfhost/src` から Mac Apple Silicon native fixed-point を再生成した。`test_e2e_native_macos_aarch64_actual_app_cli_release_program` は `864.77s` で passし、`App.Cli` の `program.native --version` は `lsharp 0.1.0`、manifest は `selfhost_fixed_point=true`、`source_commit` は現行 HEAD、`program_sha256=d1b5db348d8b793dea869597e8859131824d4b0ec9df831091734754d371cca1` となった。

同じ current source の stage23 fixed-point は `test_e2e_stage23_actual_native_self_regeneration_harness_stage2_stage3_match` が `756.38s` で passし、`actual-stage2-native` と `actual-stage3-native` の program/runtime/response/binary/stdout/stderr observation が一致した。stage3 native compiler、Mac transport driver、Mac materializer を `package-native-stage0.sh` で `source_commit` 付き stage0 packageにし、`native-selfhost-dev-source-file-smoke.sh` を Rust toolchain と host `lsharp` を blockした状態で実行した結果、`parse` / `check` / `fmt` / text `test` / metadata `test` / `compile` / `build` と Wasm magic gate が passした。

さらに同じ native `program.native` で non-Bool legacy invariant の `test --format json` を実行し、exit `2`、stdout 1行、stderr空、`status=fail`、`firstErrorCode=2`、`firstErrorSpan=26..33`、`runner=selfhost` を確認した。これにより current-source Mac の core CLI / structured failure slice は Rust なしの実行証跡を持つ。

これは Mac Apple Silicon の `App.Cli` と限定 fixtureに対する current-source evidenceであり、Linux x86_64 current-source stage0/artifact/runtime、`EmbeddedCli` の native実行、全 metadata form、全公開 command、stage0 acquisition/release/rollback、EC-M1-07 aggregate は残件である。stage0 package は bootstrap boundaryとして保持し、Rust oracle / differential と未移行 host integration を成功経路へ混ぜない。

### Current-source Linux x86_64 stage1 boundary and replay blocker (2026-07-19)

現行 `main` (`d1585818e7f0085d10a3bef45771daf4f9d97ec2`) を入力に、Mac Apple Silicon 上の Lima `lsharp-linux-x86` (`x86_64`, 16 GiB RAM, 12 GiB disk) で `scripts/ci/native-linux-x86-selfregen.sh` を再実行した。host-side native artifact probe 12 件はすべて passし、current-source stage1 bundle生成も `1402.27s` で passした。生成された stage1 は target `x86_64-unknown-linux-gnu`、code `4,161,375` bytes、data `1,511` bytes、`function_start_len=3214`、`main_func_idx=3223`、source `seed.ls` `98,720` bytes である。VM の空き容量は `7,961,264,128` bytes、必要量 `4,294,967,296` bytes で、disk free-space gate は通過した。

一方、VM 内の現行 stage1 `program.native` による stage2 transport は、chunk `0-64`、`0-32`、`0-16`、`0-8` の順に自動分割しても exit `137`（memory pressure による kill）となった。各 retry では RSS が約 `15.2`〜`15.8` GiB まで増加し、chunk サイズだけでは failure boundaryを越えられなかった。`actual-stage2-stdout.txt`、stage2/stage3 materialized bundle、`actual-selfregen-summary.json` は未生成であり、stage2/stage3 fixed-point や Linux current-source stage0 source-file smoke の pass evidence には数えない。中断後は VM 内の孤児 replay process、lock、temporary workdir を清掃し、保存済み host artifact と repo の変更は保持した。

この結果は Rust が必要な言語機能の診断ではなく、current-source native compiler の runtime heap/root/data layout または working-set 容量の blocker である。今回の replay guard fix では、stage1 materializer の manifest に `source_commit` を記録し、exit `137` を既定で retry/split せず failure summary へ即時伝播する `FAIL_FAST_ON_OOM=1` を追加した。RED の `scripts/ci/test-native-linux-x86-replay-contract.sh`、GREEN の同 test、既存 Rust script contract 33 tests、Linux x86 transport driver test は passした。次の RED は保存済み stage1 artifact を再利用した stage2 chunk ごとの RSS/heap 増加量の観測と、runtime heap/root/data layout の修正に固定する。stage1 の `1402.27s` 再生成を繰り返さず、原因修正後に stage2 -> stage3 -> materialize -> compare を再開する。

2026-07-19 の provenance follow-up では、host 側の actual stage1 generator に `SOURCE_COMMIT` を渡し、生成 manifest へ JSON の `source_commit` を保存する処理を追加した。`validate_actual_stage1_artifact` は現在の `HEAD` と manifest の commit が一致しない reuse artifact、または commit が欠落した旧 artifact を拒否する。これは stale artifact の誤再利用を防ぐ境界であり、Linux stage2 の RSS/heap/root/data layout blockerを解決した証拠ではない。保存済み旧 stage1 はこの検証を満たさないため、stage1 を再生成せずに Linux fixed-point pass として扱わない。

したがって、現時点の運用判断は二層である。current-source Mac Apple Silicon の verified stage0 がある環境では、対応済み core CLI slice の編集・`parse`・`check`・`fmt`・`test`・`compile`・`build` を Rust なしで L# 自身を使って進めてよい。未対応の language semantics、全公開 command、EmbeddedCli の native parity、Linux current-source artifact/runtime、stage0 acquisition/release/rollback は Rust oracle / bootstrap / host integration の明示境界として残し、Rust fallback で全機能対応済みとは扱わない。

2026-07-19 の次の RED では、stage2 OOM の有力な作業集合増加要因だった x86 native `vector-new` / `ref-new` の per-allocation `mmap` を対象にした。GREEN では materializer が確保した heap の先頭を cursor、`+8` を limit とし、`r14` を共有する bounded bump allocatorへ置き換えた。cursor は data 領域と衝突しない `8192` から始まり、limit 超過は null を返す。`test_native_codegen_x86_vector_new_uses_bounded_heap_cursor`、`test_native_codegen_x86_ref_new_uses_bounded_heap_cursor`、vector/ref helper byte-vector regression、capacity preservation、helper entry offset、materializer cursor/limit source contract が pass した。

これは native helper の byte-level / source-contract evidence であり、Linux current-source stage2 の RSS が下がったことや stage2 -> stage3 fixed-point を証明しない。`map-new`、`vector-push` の grow、string/その他の allocation、root/data layout、current-source stage1 provenance package の再生成と Linux VM replay は残件である。原因修正後は fresh stage1 を一度だけ生成し、stage2 -> stage3 -> materialize -> compare を再開する。VM の RAM/disk を増減して完了扱いにはしない。

同日、`map-new` も既存の 65,296-byte table allocation を `r14` の bounded cursor に移した。`test_native_codegen_x86_map_new_uses_bounded_heap_cursor`、map-new source/slot contract、map/file helper byte regression（map-insert 104-byte size contractを含む）が passした。これで x86 selfhost の vector/ref/map の新規 object allocation は per-allocation `mmap` を使わない。ただし `vector-push` の grow、string/その他の allocation、実 object の runtime semantics、current-source Linux stage2 replay は未検証である。

続く `vector-push` grow では、旧 205-byte helper の mmap 区間だけを `16 + new_capacity*8` の bounded cursor allocationへ置き換えた。overflow は既存の failure cleanupへ流し、old vector payload copy、capacity/length保存、value append、tagging、後続 helper offsetは維持した。`test_native_codegen_x86_vector_push_growth_uses_bounded_heap_cursor`、capacity/header/compare/register-order の focused tests、vector/ref helper byte regression、vector-push source/slot contract が passした。これは helper byte/ABI evidenceであり、actual runtimeで grow・copy・overflow semanticsを実行した証拠ではない。string/other allocation、root/data layout、current-source Linux stage2 replay、stage3 fixed-pointは残件である。

### Current-source Linux x86 map-new object ABI boundary (2026-07-19)

Linux x86 object smoke の RED で、bounded `map-new` helper の cursor offset を native heap baseへ加算せず低位アドレスへ header を書いていたこと、既存 slot 内の filler `00 00` が実行されていたこと、headerを書いた `rdi` を return register `rax` へ戻していなかったことを切り分けた。さらに helper slot を 72 bytes から 75 bytesへ拡張した後、map-size 以降の static helper offset が旧値のままで、生成された map-insert call が map-size の failure pathへ着地する二次 failureも fresh object disassembly で確認した。

GREEN では `r14 + cursor` を map header baseへ加算し、`xchg rax,rdi` と `or rcx,rax` で tagged objectを返し、slot内 fillerを executable `NOP` から `xchg` へ置換した。map-new後続の map-size / map-insert / map-get / file/CLI helper offsetをすべて `+3` へ同期した。`test_native_codegen_x86_map_new_helper_adds_heap_base_before_header`、map-new byte/size contract、offset chain、shell replay contract、`git diff --check` が passし、current-source から再生成した `map-program.o` は Lima `lsharp-linux-x86` の current object runtimeで `exit 42` を返した。

これは map-new / map-insert / map-get の Linux x86 object ABI verified sliceであり、current-source stage1 の再生成から stage2 -> stage3 -> materialize -> compareまでの fixed-point、vector-push growを含む全 allocation runtime、large-source native check、Mac/Linux全公開 surface、stage0 acquisition/release/rollbackを証明しない。したがって Linux x86 の全機能 Rust-free 完了とは扱わず、次は fresh stage1を一度だけ生成して stage2 fixed-point gateを再実行する。対応済み Mac core sliceと同様、検証済み object/CLI sliceの日常開発はRustなしで進めるが、未検証 target gate、Rust oracle、bootstrap、host integrationの境界は維持する。

### EC-M1-04 dynamic Boolean complement non-vacuity (2026-07-20)

property binder を参照する直接形 `or p (not p)` / `and p (not p)` を、literal や静的比較だけでは検出できない動的な恒真・恒偽 predicate の narrow slice として追加した。Rust canonical checker、selfhost `Types.TypeInferAssertions`、selfhost `Tools.Test.TestRunner` の三つの境界で、postcondition の恒真を `LS2005` 相当、precondition の恒偽を同じ non-vacuity failure として拒否する。AST の span を無視した shape 比較は `Ann`、literal、variable、arity 1/2 の直接 application に限定し、一般 theorem proving や広い constant propagation には拡張していない。selfhost 側は non-short-circuit `and` による不正な AST dereferenceを避け、tag/calleeを段階的な `if` で検査する。

Evidence: Rust `metadata_contract_check` の dynamic complement postcondition / contradiction precondition 2 testsを含む 26 tests、selfhost `dynamic_complement` / `dynamic_contradiction` の public runner/check 4 tests、既存の selfhost property runner 全 28 tests（Bool、Int、String、mixed binder、複数 precondition、profile拒否を含む）が passした。現行 commit は `16c47e3de9d96d51a218ee89e718a7c41a2d445b` で `main` / `origin/main` に一致している。

これは EC-M1-04 全体の完了ではない。current-source Mac Apple Silicon / Linux x86_64 の native artifact/runtime、一般的な動的恒真判定、一般 `TypeExpr`、seed/shrink/coverage、diagnostic/span parity、EC-M1-04 aggregate は残件である。したがってこの slice は Rust-hosted differential / selfhost runner の検証済み範囲として扱い、現行 source と一致しない過去 stage0 を成功経路へ流用せず、Rust の bootstrap / oracle / host integration 境界を維持する。

### Linux x86 normal transport first-chunk boundary (2026-07-20)

Linux x86 の seed が通常 transport と診断用 main を同じ巨大な `main` で処理していたため、tiny source の first chunk (`0..64`) を要求すると `function_start_len=1` を超える function segment を繰り返し出力していた。RED では decoder が `declared=4388 actual=6234` の segmented length mismatch で停止した。GREEN では通常経路を小さい `linux-x86-normal-transport-main` に分離し、要求された `range-end` を `vector-length starts` で clamp した。診断 mode と production payload は既存経路を維持した。

Evidence: `test_linux_x86_representative_seed_dispatches_normal_transport_through_small_main`、`test_linux_x86_normal_transport_caps_first_chunk_to_function_count`、clamp を含む actual stage1 host gate（1 passed、676.16s）。生成 artifact の code/data は `4,176,978` / `1,511` bytes、`function_start_len=3232`、`main_func_idx=3241`、`entrypoint_offset=4174595` だった。Lima `lsharp-linux-x86` では同 artifact を materialize し、`(defn main [] 42)` を chunk size `64` の公式 transport と decoder に通した。decoder の manifest は code `4388` bytes、data `0` bytes、`function_start_len=1`、`main_func_idx=10`、`entrypoint_offset=0` となり、function segment 1件と trailer を合計 `4388` bytesへ復元した。復元した native program は `exit 42` で終了した。

この stage1 の `manifest.json` は、プロセス起動時に渡していた旧 `source_commit` (`1f9754b4...`) を記録しているが、生成処理自体は clamp を含む現行作業ツリーで実行された。このため今回の証拠は first-chunk transport、decoder、tiny native runtime の regression evidence に限定し、current-source release provenance の証拠には数えない。`LEGACY-BOOT-01` / `EC-M1-07` の source-commit 一致 stage0 package、large-source stage2 -> stage3 -> materialize -> compare、Linux current-source source-file smoke は残件である。VM の RAM/disk resize を完了条件にせず、保存済み artifact と VM 作業領域は次の stage2 識別実験で再利用する。

### Linux x86 stage2 -> stage3 fixed-point and materialize boundary (2026-07-20)

保存済み stage1 artifact を VM-side replay lock と chunk size `64` で再利用し、`native-linux-x86-hostgen-vm-exec.sh` の stage2 -> decode -> materialize -> stage3 -> decode -> compare 経路を完走した。stage2/stage3 の transport stdout は各 `11,578,356` bytes、SHA-256 は双方 `86a487e12831c3509272b87ad3c0e250fbdbad5e7c2adb6a0876de0b15e738b0` で一致した。decoded stage2/stage3 code は各 `10,795,902` bytes、data は `1,511` bytes、`entrypoint_offset=10791471`、`function_start_len=3232`、`main_func_idx=3241` で一致し、stderr は両方 0 bytesだった。

追加の stage3 materialize smoke では、復元した `program.native` が `src/App/Seed.ls 0..64` を exit `0` で処理し、stderr 0、header `9000000005`、`function_start_len=3232`、`main_func_idx=3241`、code len `10795902` を返した。これにより Linux x86 の seed transport、stage2/stage3 fixed-point bytes、materializer/entrypoint の verified slice が揃った。VM job の lock と process は終了後に解放され、VM は 11 GiB disk 中 5.0 GiB free（54% 使用）を維持した。

この gate は保存済み stage1 の内容を使った seed fixed-point の証拠であり、手動で provenance を補正した stage1 manifest を含むため、fresh stage1 generation の source-commit 証明、stage0 package acquisition/release/rollback、Linux App.Cli source-file smoke、全公開 surface の Rust-free 完了には拡張しない。`LEGACY-BOOT-01` / `LEGACY-IO-01` / `LEGACY-TOOL-01` / `EC-M1-07` の current-source release contract と public command parity は残件である。

### Linux x86 App.Cli current-source target and source-file smoke (2026-07-20)

dirty な main worktree のユーザー差分を変更せず、同じ `HEAD=4d494a1bbdd85abff7ab4422d904bfff428264cf` の clean detached worktree で、保存済み stage2 artifact と current selfhost source tree を再利用した。stage3 target-only export は `src/App/Cli.ls` を入力に Linux x86_64 native App.Cli bundle を生成し、manifest は `source_commit` と `source_tree_sha256=38b13dada646d723a14b1cd54e341642809d15fba312cc0e00ac3a0320cea1ea` を記録した。`selfhost_fixed_point=true`、`program_sha256=8d5167ec6287ba88132c8297dfca42ef6db80e47534d5adee3ffac230d3d3181`、code length `10,328,070` bytes、stderr 0 bytesで、VM の `--version` は `lsharp 0.1.0` を返した。

同じ materialized bundle を `native-linux-x86-app-cli-source-file-smoke.sh` へ渡し、`parse`、`check`、`fmt`、text `test`、metadata `test`、`compile`、`build` を Linux x86_64 native program から実行した。全 command は exit `0` / stderr 空で、`compile.wasm` と `build.wasm` は各 `2,503` bytes、stdout は各 `wasm-size:2503`、core Wasm magic は両方 `0061736d` だった。これは Linux x86 の App.Cli source-file / compile / build public contract の current-source verified sliceである。

この gate は stage2 artifact を再利用し、元の stage1 manifest は docs-only commit の差分を反映するため validation 用に provenance を補正している。したがって fresh stage1 generation の再現性、public stage0 acquisition/release/rollback、4,096 bytes 超 read、fd error、component sidecar、全公開 command の parity、`LEGACY-BOOT-01` / `LEGACY-IO-01` / `LEGACY-TOOL-01` / `EC-M1-07` aggregate 完了には拡張しない。Linux App.Cli の current source-file gate は閉じたが、release provenance は別の残件として維持する。

### Linux x86 fresh stage1 provenance (2026-07-20)

current `HEAD=20cf455ff80c54ef2b8984c2b8056dfc1d189f84` を `LSHARP_NATIVE_LINUX_X86_SOURCE_COMMIT` に渡した fresh actual stage1 generation は 1 passed、696.28 秒で完了した。manifest は target `x86_64-unknown-linux-gnu`、`source_commit` はこの commit、code/data `4,176,978` / `1,511` bytes、`entrypoint_offset=4174595`、`function_start_len=3232`、`main_func_idx=3241` を記録した。`stage1-code.bin` SHA-256 は `48b01dfa58d67ea9cc5a060320f87c24606a8bb5b24acceaeadf08c1a5d30c6d`、data は `8edec2b719be2dcfcf7f71012b74c5450f7f80c52cbf27db3fa01c6fe436215c`、seed は `394a299d042dba63314e756276281f0f531dc8fcc8450f090f3de21a5b48b2f1` だった。

この fresh artifact の manifest 以外の全 stage1 bytes、seed、entrypoint/function metadata は、先行する Linux x86 stage2 -> stage3 fixed-point gate に入力した stage1 と完全一致した。したがって fixed-point bytes の evidence は保持しつつ、30 分級の同一 replay を重複起動せず、fresh stage1 の source-commit provenance と fixed-point content identity を分離して扱える。stage0 package の acquisition/release/rollback、現行 checkout ごとの package fingerprint、Rust oracle 隔離の実運用証拠は `LEGACY-BOOT-01` の残件として維持する。

### Linux x86 current-source stage0 package acquisition (2026-07-20)

fresh stage1 の source commit を厳密に一致させるため、clean detached checkout `84c783526f0f9f71ce40273f5e4599b62c79797a` で stage1 を一度だけ再生成した。stage1 は target `x86_64-unknown-linux-gnu`、code/data `4,176,978` / `1,511` bytes、`entrypoint_offset=4174595`、`function_start_len=3232`、`main_func_idx=3241`、code SHA-256 `48b01dfa58d67ea9cc5a060320f87c24606a8bb5b24acceaeadf08c1a5d30c6d`、data SHA-256 `8edec2b719be2dcfcf7f71012b74c5450f7f80c52cbf27db3fa01c6fe436215c` で、生成 test は `865.94s` の `1 passed` だった。専用 cargo target は検証後に `cargo clean --target-dir` で `1.6GiB` 回収した。

この artifact を Lima `lsharp-linux-x86` 内で materialize し、manifest の `source_commit` が checkout と一致する `lsharp-native-selfhost-stage0` packageを作成した。package manifest は compiler / transport driver / materializer を相対 pathで指し、`native-linux-x86-native-stage0-source-file-smoke.sh` は `LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=64`、VM free `5,402,787,840` bytes、required `4,294,967,296` bytes の条件で、`cargo`、`rustc`、host `lsharp` を blockしたまま `parse`、`check`、`fmt`、text/metadata/property `test`、`compile`、`build` を passした。全 command は stderr 空、compile/build の Wasm は magic `0061736d` と positive sizeを満たした。

続けて `package-native-stage0-release.sh` で `lsharp-stage0-v0.2.0-current-x86_64-unknown-linux-gnu.tar.gz`（404,795 bytes、SHA-256 `d53441c57976a23867c2d03859be344580e8dea05c207dec9326d30eaeffa3cb`）を生成し、local HTTP assetを使う `fetch-stage0.sh` で release checksum、tar path safety、package checksums 5行、target、`source_commit` を検証して fetched stage0 を installした。package/release contract tests、actual-stage1 package contract、native stage0 source-file provenance test、docs auditも passした。これは Linux x86_64 の stage0 acquisition と daily core source-file boundaryを exact commitで確認した evidenceである。

この evidence は stage0 package の取得・materialize・source-file smokeまでであり、公開 releaseへの upload、Mac/Linux rollback archiveの実成果物、rollback実行、全公開 command、component sidecar、`LEGACY-BOOT-01` 全体の完了を意味しない。strict runner は checkoutごとの `source_commit` 一致を要求するため、後続の source commit では stage1/packageを再生成してから採用する。Rust oracle / bootstrap / emergency rollback 境界は保持する。

### Standalone read-file chunk accumulation over 4096 bytes (2026-07-21)

standalone `read-file` の 4097-byte fixture を RED として追加し、最初の generated body は Wasm translation error で停止した。`wasm-tools validate` と dump で `func 19` の末尾 `local.get` 後に function-level `end` が欠け、offset `0x666` で control frame が残ることを特定した。次の実行では翻訳を通ったが stdout が最後の 1 byte だけになり、body の累積箇所が `acc = chunk` で上書きされ、raw `string-concat` call が欠けていることを特定した。これらの失敗値を変更せずに byte bundle を修正した。

GREEN では `emit-standalone-read-file-body-chunked` を追加し、allocator で 4096-byte chunk と累積 String を確保し、`fd_read` が EOF になるまで `string-concat` で連結する。`path_open`、`fd_read`、`fd_close` の errno は fail-closed とし、既存の 2176/2184/2240 WASI scratch 境界を維持する。current-source selfhost CLI が生成した standalone Wasm を Wasmtime で実行し、4096 bytes の chunk と 1 byte の tail を含む 4097 bytes 全量を確認した。

Evidence: `test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4096`、既存 4096-byte regression、`test_e2e_selfhost_standalone_read_file_retries_partial_fd_read`、`test_e2e_selfhost_standalone_read_file_returns_fd_read_errno_after_partial_read`、`test_e2e_selfhost_standalone_read_file_returns_fd_close_errno`、`test_e2e_selfhost_standalone_read_file_returns_empty_on_path_open_errno`、`test_e2e_selfhost_standalone_file_exists_returns_false_on_fd_close_errno`、`test_e2e_selfhost_wasmemit_chunked_read_body_contract`。raw body contract は concat call sequence と最終 `end` を固定する。`LEGACY-IO-01` は 4096 bytes 超 read の Mac-side verified slice を得たが、dynamic root/data/heap layout、Linux x86_64 native source-file E2E、component sidecar、全 fd ABI parity、stage0 release provenance は未完了のため `[~]` を維持する。

### Linux x86 current-source native self-regeneration fixed point (2026-07-21)

`HEAD=26a338bab7ef668fa3dcd196a8f93e7e9e992290` の fresh actual stage1 artifact を入力に、Lima `lsharp-linux-x86`（Linux `x86_64`）で stage1-native → stage2-native → stage3-native を `LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE=64`、chunk retry `1`、native timeout `900` 秒で実行した。VM free-space gate は `5,402,738,688` bytes available / `4,294,967,296` bytes required で、同じ replay lock を保持したまま重複 replay は起動していない。

stage1 manifest は target `x86_64-unknown-linux-gnu`、`source_commit=26a338bab7ef668fa3dcd196a8f93e7e9e992290`、code/data `4,201,538` / `1,511` bytes、`entrypoint_offset=4199155`、`function_start_len=3233`、`main_func_idx=3242` を記録した。stage2 と stage3 の manifest も同じ source commit、target、function metadata を保持し、code は両方 `10,822,510` bytes、data は `1,511` bytes、entrypoint は `10818079` で一致した。

VM 側の `actual-selfregen-summary.json` は `status=pass`、stage2/stage3 stdout SHA-256 は両方 `a81e845a11e99f004b60bf616c93759a4b39ccfc3348b1e1332df45f690fdeb6`、code length は両方 `10,822,510` bytes を示した。host 側でも stdout の byte compare が `match`、stage2/stage3 stderr は各 0 bytesだった。成功後に VM workdir と VM replay lock は削除され、host artifact は `ci-artifacts/native-linux-x86-hostgen-vm/26a338ba-stage2-stage3-current` に保存した。

Evidence command: `env NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=26a338ba-stage2-stage3-current LSHARP_NATIVE_LINUX_X86_REUSE_ACTUAL_STAGE1_ARTIFACT_DIR=/tmp/lsharp-native-linux-x86-hostgen-26a338ba-stage1 LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE=64 LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_RETRIES=1 LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT=900 LSHARP_NATIVE_LINUX_X86_VM_MIN_FREE_BYTES=4294967296 LSHARP_NATIVE_LINUX_X86_SKIP_HOST_PROBES=1 bash scripts/ci/native-linux-x86-hostgen-vm-exec.sh`。これは current-source Linux x86_64 の実バイナリ stage1→stage2→stage3 fixed-point verified slice であり、公開 stage0 acquisition/release/rollback、component sidecar、全公開 command、Mac current-source gate、`LEGACY-BOOT-01` / `LEGACY-COMP-01` / `EC-M1-07` 全体の完了を意味しないため、TODO の `[~]` は維持する。

### EC-M1-01 ADT constructor pattern evaluation slice (2026-07-20)

legacy `:invariant` の match evaluator に、型宣言で定義された ADT constructor の expression、zero-argument constructor、constructor pattern の tag/hash/arity 比較、payload の再帰照合、payload variable binding を追加した。未定義 function を constructor として値化しないよう、program 内の type declaration だけを登録源にしている。Rust oracle と selfhost TestRunner を同じ `Maybe` fixtureで実行し、`(Just value)` と `Nothing` の arm を含む invariant が双方で 5 cases pass することを確認した。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_constructor_match`、既存 match/computation/string invariant regression を含む 9 tests、`CARGO_INCREMENTAL=0 cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_test_runner_matches_rust_oracle_for -- --nocapture`。これは constructor の 1段 payload と legacy invariant runner に限定した verified sliceであり、nested/general ADT、record/GADT/exhaustiveness、full type/runtime parity、supported 2 targets の current-source artifact/runtime gate、EC-M1-01 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### EC-M1-01 record pattern evaluation slice (2026-07-20)

legacy `:invariant` の match evaluator に、record literal を evaluator 内の tagged value として materializeする経路を追加した。record pattern は nominal type hash を比較し、要求された field hash を value から検索して child pattern を再帰評価し、field binder を arm-local environment へ追加する。既存の `[pattern, body]` arm layout と wildcard fallback は維持した。Rust oracle と selfhost TestRunner を同じ `Point` fixtureで比較し、record field binder の invariant が双方で pass することを確認した。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_record_match`、既存 match/computation/string/constructor/guard regression を含む 11 tests、`CARGO_INCREMENTAL=0 cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_test_runner_matches_rust_oracle_for -- --nocapture`。これは Int field を持つ nominal record の legacy invariant runner sliceに限定した verified evidenceであり、nested/general record、record update、String/Map field semantics、full compiler/type/runtime parity、exhaustiveness、supported 2 targets の current-source artifact/runtime gate、EC-M1-01 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### Linux x86 current-source fixed point after invariant match slices (2026-07-21)

`HEAD=5b861a97e714cfe639c153d687bc9bc222cec8e4` の fresh actual stage1 artifact を入力に、Lima `lsharp-linux-x86`（Linux `x86_64`）で stage1-native → stage2-native → stage3-native を `LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE=64`、chunk retry `1`、native timeout `900` 秒で実行した。VM free-space gate は `5,467,693,056` bytes available / `4,294,967,296` bytes required で、replay lock により同じ job の重複起動はなかった。

stage1 manifest は target `x86_64-unknown-linux-gnu`、`source_commit=5b861a97e714cfe639c153d687bc9bc222cec8e4`、code/data `4,203,487` / `1,523` bytes、`entrypoint_offset=4201104`、`function_start_len=3237`、`main_func_idx=3246` を記録した。stage2 と stage3 の manifest も同じ source commit、target、function metadata を保持し、code は両方 `10,832,651` bytes、data は `1,523` bytes、entrypoint は `10828220` で一致した。

VM 側の `actual-selfregen-summary.json` は `status=pass`、stage2/stage3 stdout SHA-256 は双方 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`、stdout は各 `11,646,271` bytes、code length は各 `10,832,651` bytesを示した。host 側の byte compare も `match`、stage2/stage3 stderr は各 0 bytesだった。成功後に VM workdir、VM replay lock、専用 cargo target は削除され、host artifact は `ci-artifacts/native-linux-x86-hostgen-vm/5b861a97-stage2-stage3-current` に保存した。VM は 12 GiB disk 中 5.1 GiB free を維持している。

Evidence command: `env NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=5b861a97-stage2-stage3-current LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE=64 LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_RETRIES=1 LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT=900 LSHARP_NATIVE_LINUX_X86_VM_MIN_FREE_BYTES=4294967296 LSHARP_NATIVE_LINUX_X86_SKIP_HOST_PROBES=1 bash scripts/ci/native-linux-x86-hostgen-vm-exec.sh`。これは current-source Linux x86_64 の実バイナリ stage1→stage2→stage3 fixed-point verified sliceであり、guard/record を含む全言語機能、公開 stage0 acquisition/release/rollback、component sidecar、全公開 command、Mac current-source gate、`LEGACY-BOOT-01` / `LEGACY-COMP-01` / `EC-M1-07` 全体の完了を意味しないため、TODO の `[~]` は維持する。

### EC-M1-01 match arm guard fall-through slice (2026-07-20)

legacy `:invariant` の match evaluator に、`when` guard を既存の `[pattern, body]` arm 配置を保つ内部 wrapper として保持する parser 経路を追加した。TestRunner は pattern binding 後に guard を評価し、false なら次の arm へ fall-through、true なら同じ arm body を評価する。unknown-variable preflight と source-aware evaluation も wrapper の guard/body を再帰的に走査する。同一 fixture の false guard と true guard を Rust oracle と比較し、selfhost は結果数 `2`、各結果の pass `1`、failure/diagnostic `0` を返した。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_match_guard`、既存 match/computation/string invariant regression を含む 10 tests、`CARGO_INCREMENTAL=0 cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_test_runner_matches_rust_oracle_for -- --nocapture`。これは legacy invariant TestRunner の source-aware evaluation と unknown-variable preflight に限定した verified sliceであり、selfhost compiler/type inference/formatter の guard AST parity、一般の match runtime、exhaustiveness、structured diagnostic/span parity、supported 2 targets の current-source artifact/runtime gate、EC-M1-01 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### EC-M1-01 record field access evaluation slice (2026-07-20)

Rust syntax の lexer が `.` token を発行し、parser が `(. expr field)` を既存の `Expr::FieldAccess` として生成する経路を追加した。selfhost `TestRunner` は record literal を evaluator 内の tagged value として materializeした後、field hash lookup を通常 evaluator と source-aware evaluator の両方で実行する。Rust oracle と selfhost TestRunner を同じ `Point` fixtureで比較し、`(. {Point x 41 y 2} x)` が `41` を返す legacy invariant を双方で pass することを確認した。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_record_field_access`、`cargo test -p lsharp-syntax -- --nocapture`（160 unit tests、metadata testsを含む）、record/guard/constructor/match/computation/string regression を含む 12 tests、`CARGO_INCREMENTAL=0 cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_test_runner_matches_rust_oracle_for -- --nocapture`。これは Int field の nominal record と legacy invariant runner、直接 field-access syntax の verified sliceに限定した evidenceであり、nested/general record、record update、String/Map field semantics、full compiler/type/runtime parity、exhaustiveness、supported 2 targets の current-source artifact/runtime gate、EC-M1-01 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### EC-M1-01 nested match evaluation regression slices (2026-07-20)

legacy `:invariant` の match evaluator について、既存の再帰実装が nested record と nested ADT constructor の child pattern を同じ source-aware 経路で処理することを追加検証した。nested record は `Box -> Point` の nominal field をたどって child field binder を arm-local environment に束縛し、nested ADT は `Just (Just value)` の constructor tag/hash/arity を再帰照合して payload を束縛する。wildcard fallback、既存の `result` binding、deterministic invariant sample は変更していない。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_nested_record_match`、`test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_nested_constructor_match`、`CARGO_INCREMENTAL=0 cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_test_runner_matches_rust_oracle_for -- --nocapture`（16 tests）。両 nested fixture は Rust oracle と selfhost の結果数、pass、actual case count、diagnostic code を一致させた。

これは legacy invariant TestRunner の nested record / nested ordinary ADT pattern に限定した differential evidenceであり、general ADT/record semantics、GADT refinement、exhaustiveness、String/Map field semantics、full compiler/type/runtime parity、supported 2 targets の current-source artifact/runtime gate、EC-M1-01 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### EC-M1-02 multiple property preconditions evaluation slice (2026-07-20)

selfhost `TestRunner` の canonical `:property` evaluation について、source-order の複数 typed binder と複数 precondition を同一 sample に適用する経路を Rust oracle と比較した。`Int` binder `a` / `b` に `a >= 0` と `b < 5` を conjunction として適用し、5 cases の deterministic prefix のうち 4 cases を postcondition まで評価する。precondition を一つだけ評価して false sample を誤って実行する挙動や、全件を vacuous success とする挙動は確認されなかった。

Evidence: `test_e2e_selfhost_runner_matches_rust_oracle_for_multiple_property_preconditions`、既存の property binder / conjunction / vacuity regression、`CARGO_INCREMENTAL=0 cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_runner_matches_rust_oracle_for -- --nocapture`。fixture は Rust oracle の pass と selfhost の `1 / 1 / 4 / 0`（property count / pass / actual cases / diagnostic）を一致させた。

これは deterministic Int property の複数 precondition evaluator に限定した verified sliceであり、一般 `TypeExpr`、constraint-directed generator、seed/shrink/coverage、全 ContractSuite variant、structured report、supported 2 targets の current-source artifact/runtime evidence、EC-M1-02 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### EC-M1-01 record update evaluation slice (2026-07-20)

Rust AST の `RecordUpdate` display が `{(base) | ...}` を出力していたため、metadata test program の再パース時に `(base)` がゼロ引数関数呼び出しへ変わり、Rust oracle の型推論が record ではなく function を検査する不整合があった。display を `{base | ...}` に修正し、record update の parse/display/parse round-trip を syntax test で固定した。

selfhost `TestRunner` には `ast-recordupdate` の通常 evaluator と source-aware evaluator を追加した。既存の tagged record value を field hash で検索し、型検査済みの更新 field の値だけを置換するため、record literal の type hash と未更新 field は保持される。Rust oracle と selfhost TestRunner を同じ `Point` fixtureで比較し、`(. {{Point x 41 y 2} | x 42} x)` が `42` を返す legacy invariant を双方で pass することを確認した。

Evidence: `test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_record_update`、`cargo test -p lsharp-syntax -- --nocapture`（160 unit tests、metadata tests 11 件）、record/guard/constructor/match/computation/string regression を含む 13 tests（全件 pass、101.75s）、`git diff --check`。これは Int field の nominal record update を legacy invariant TestRunner で評価する verified sliceに限定した evidenceであり、nested/general record、String/Map field semantics、selfhost compiler/type/runtime の全 parity、supported 2 targets の current-source artifact/runtime gate、EC-M1-01 aggregate は残件である。したがって TODO の `[~]` と Rust oracle / bootstrap 境界は維持する。

### EC-M1-02 typed property source-span projection slice (2026-07-21)

selfhost `PropertyRunner` の source-aware contract projection を typed property へ接続した。parser form の relative binder offset を元ソースの絶対 offsetへ変換し、contract row の span sidecar として postcondition pair、および precondition の flat `[start, end]` vectorを保持する。recursive program/module/private declaration traversalも同じ `src` を伝播するため、単一関数の局所テストではなく parser-owned typed contract inventoryから取得できる。

Evidence: `test_e2e_selfhost_parser_typed_property_contract_preserves_expression_spans` と `test_e2e_selfhost_property_test_case_span_sidecar_preserves_expression_spans` が同一 fixtureで binder `40..49`、precondition `75..87`、postcondition `104..120`を検証し、focused E2Eは各 `1 passed`（24.35s / 26.01s）だった。後者は property index ごとの `[binder-spans, precondition-spans, postcondition-span]` sidecar を介して同じ絶対 span を取得する。current source commit `9e8dab64fe54fb37a720695e0c2e8b38019df27d` の Linux x86_64 gateも `status=pass`、stage2/stage3 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` 一致、code length `10,832,651` bytes 一致、stage1 manifestの `source_commit` 一致を確認した。VMは検証後に停止し、12 GiB disk / 16 GiB RAM構成を維持した。

これは typed contract inventory と property index sidecar の source-span verified sliceに限定される。source-aware contractを既存の source-less property test-case materializerへ直接流し込むと、legacy heterogeneous vector inferenceが `expected Int, found String` で止まるため、公開 `generate-tests` の suite/report shape への接続は今回変更せず、Rust-free runtime parityの証拠へ拡張していない。public test result と diagnostic/report への span forwarding、predicate/source diagnostic parity、Mac Apple Silicon current-source gate、EC-M1-02 aggregate、Rust oracle / bootstrap / host integration境界は残件である。したがって TODOの `[~]` と明示的な Rust boundary は維持する。

### EC-M1-02 public property postcondition diagnostic span slice (2026-07-20)

selfhost `Tools.Test.TestRunner` は、既存 property test-case の heterogeneous vector shapeを変更せず、`run-properties-from-source` 内で一度だけ作った source-span sidecarを `materialize-property` の診断生成へ渡すようになった。non-Bool postcondition、static vacuityなどの postcondition 起因 failureでは、既存の unknown-variable span経路を壊さず、Rust oracleの postcondition expression spanを test result の後方 `4..5`へ保持する。source-less `run-properties` は従来どおり `0..0` の明示的な非 source boundaryを使う。

Evidence: RED の `test_e2e_selfhost_property_runner_preserves_non_bool_postcondition_span` は selfhost の `0..0` と Rust oracle の `71..86` の差分で失敗し、GREEN は同じ fixtureで `LS1002 / 71 / 86` を確認した（focused E2E 1 passed、24.32s）。既存の typed contract / sidecar span testsは再実行対象として維持する。precondition の全型診断と Rust canonical checker の parity、property failureの text/JSON report forwarding、Mac Apple Silicon / Linux x86_64 current-source artifact/runtime、EC-M1-02 aggregateは残件であり、TODOの `[~]` は維持する。

### LEGACY-ROOT-01 shadowed root slot guard slice (2026-07-20)

`root_set` の lexical shadowing と allocating value の組み合わせを、最小 `TestRunner` runtime bundleで回帰固定した。旧値 `42` を外側 root slotへ保持し、内側で同名 `slot` を pushした後、`(root_set slot (vector-push (vector-new 1) 7))` で新しい heap 値を更新する。最内側の slotだけを popしてから値を読むことで、外側 slotを誤って解放・参照する実装を検出できる。期待値は `7\n` である。

Evidence: `test_e2e_selfhost_root_set_preserves_shadowed_slot_during_allocating_value` は、最初の誤った `vector-new` 長さ前提を `vector-push` の意味論に合わせて修正した後、`1 passed`（25.64s）となった。既存の Mac/native stage-chain fixtureも同じ旧値42・新値7の契約へ同期した。rooting 規約は `docs/development/planning/memory-management-roadmap.md` に追加した。

これは Rust/Wasm の current runtime bundle guard と native fixture correction の verified sliceであり、Linux x86_64 current-source native stage0、Mac Apple Silicon current-source native artifact、全 selfhost sourceの lint/guard、GC stress mode、`LEGACY-ROOT-01` aggregate の完了証拠ではない。Linux current-source gateは既存の `9e8dab64` evidenceを超えて再実行していないため、TODOの `[~]` と残る Rust/bootstrap/host boundary は維持する。

### LEGACY-ROOT-01 Mac native shadowed-slot gate (2026-07-20)

既存の `42 -> 7` shadowed root slot fixtureを native representative bundle の通常実行 gateへ昇格した。`root_set` の右辺で `vector-push` が新しい heap valueを確保する間も、同名の内側 slotと外側 slotを混同せず、`root_pop` 後に更新値 `7` を取得する。native bundle は selfhost/Wasm 期待値と比較され、exit code `0` と空 stderrを要求する。

Evidence: 変更前の ignored baseline `test_e2e_selfhost_pipeline_smoke_root_set_keeps_shadowed_slot_during_allocating_value` は `1 passed`（`321.79s`）。`#[ignore]` 撤去後の通常 filterも同じ testで `1 passed`（`341.69s`）、`2662 filtered out`、失敗なし。これは Mac Apple Silicon の一つの native shadowing/allocating `root_set` contractに限定した verified sliceであり、一般 heap-value root 規律、全 selfhost source lint/guard、GC stress、Linux x86_64 native/VM gate、`LEGACY-ROOT-01` aggregateの完了を意味しない。したがって TODO は `[~]` のまま維持する。

### EC-M1-02 public property precondition diagnostic span slice (2026-07-20)

selfhost `TestRunner` の source-aware property diagnosticについて、non-Bool preconditionをpostconditionのfallback spanへ誤って寄せず、typed contract sidecarの評価中に最初に失敗した precondition spanへ投影する経路を追加した。unknown-variable spanは従来どおり最優先し、precondition spanは `bool-valid=0` かつ実行済み sample が `0` 件の non-Bool precondition boundaryでのみ選択する。

Evidence: RED の `test_e2e_selfhost_property_runner_preserves_non_bool_precondition_span` は selfhost `95..`（postcondition fallback）と Rust oracle `71..`（precondition）の差分で失敗し、`test_e2e_selfhost_property_runner_preserves_second_non_bool_precondition_span` は selfhost `71..`（最初の span）と Rust oracle `80..`（2番目の span）の差分で失敗した。GREENでは両 fixtureの Rust/selfhost span一致、postcondition span、sidecar span、既存 16-case Rust/selfhost differentialを再確認した。さらに `test_e2e_selfhost_cli_test_source_json_reports_property_precondition_span` で、同じ Rust oracle span が `test --format json` 相当の `implementation_conformance.diagnostics.firstErrorSpan` へ転送されることを確認した（focused E2E `1 passed`、413.72s）。

これは評価中に最初に non-Boolと判定した precondition predicateの source spanと、その JSON report forwardingに限定した verified sliceであり、text report、静的型診断と動的評価の全ケース parity、全 property failure の report forwarding、Mac Apple Silicon / Linux x86_64 current-source artifact/runtime、EC-M1-02 aggregateは残件である。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### V2-16b / LEGACY-COMP-01 actual selfhost CLI artifact boundary (2026-08-02)

`test_e2e_selfhost_cli_main_compile_and_build_output_actual_preview1_wasm` は actual `Cli` bundleを
一度 compileし、`compile` と `build` の `-o output.wasm` をそれぞれ生成した。各 outputについて
Wasm magic、`wasmparser` validation、standalone WASI runtime、exit `0`、空 stdoutを確認した。
同じ gateで `wasi-component` targetは external packaging未接続として exit `1`、component artifactなしを
確認した。`1 passed`、`340.52s`。

これは Rust-host actual source bundleの artifact/runtime boundaryに限定した verified sliceであり、
native stage0の output-path、full-program entrypoint、component sidecar、fd error semantics、
両 supported targetの release artifact/provenance は未検証である。`V2-16b` / `LEGACY-IO-01` /
`LEGACY-COMP-01` と Rust oracle/bootstrap boundaryは維持する。ADR:
`docs/adr/decisions-v0.3-native-cli-artifact-output-e2e.md`。

### V2-16e / LEGACY-BOOT-01 Mac current-source App.Cli native release (2026-08-02)

`test_e2e_native_macos_aarch64_actual_app_cli_release_program` を current checkoutで実行し、
stage2/stage3 fixed-pointから `aarch64-apple-darwin` の actual App.Cli `program.native` を生成した。
gateは `1 passed`、`945.94s`。manifestは `source_commit=0dc6d67348195ad23575913841459cdf2e6a36b2`、
`selfhost_fixed_point=true`、program SHA-256
`a1dac9ff7146fbfd012c6e299df786c3c6c00680e3849cfb98abdeb1efcd76de`、4,327,168 bytesを記録し、
Mach-O arm64と `--version` の `lsharp 0.1.0`、stderr 0 bytesを確認した。

同じ生成 artifactを単体 postflightし、引数なし helpと `parse` fileがともに exit `0` / stderr `0`となる
ことを確認した。parse outputは `decls:1`、`first-decl:defn`、`first-body:int`、`diagnostics:0`。
これは Mac current-source native releaseの verified sliceであり、Linux x86_64 native regeneration、
packaged stage0 acquisition/release/rollback、全公開 command、component sidecar、両 targetの
artifact/provenance parityは未検証である。専用 Cargo targetと `/tmp` artifactは検証後に削除する。
ADR: `docs/adr/decisions-v0.3-native-macos-app-cli-release-e2e.md`。

### V2-16e / LEGACY-BOOT-01 Linux current-source stage1 metadata boundary (2026-08-02)

現行 `02201e1172ebb6dad8624186658f171fc9a88a3d` の Linux x86_64 stage1 host payloadを
`test_e2e_native_linux_x86_host_generates_actual_selfregen_stage1_bundle_artifact` で生成し、
`1 passed`、`333.21s`。manifestは target `x86_64-unknown-linux-gnu`、code `4,393,425` bytes、
data `2,757` bytes、entrypoint `4,390,965`、function-start `3,409`、main function `3,418`を
記録した。code SHA-256は
`625ec6a33f9f5722832eee6b9062994d680b48f6d1b6feba8e2db334629cbc9a`。

このstage1をLima `lsharp-linux-x86`（16 GiB RAM / 12 GiB disk）へ渡し、free `7,688,683,520`
bytes、required `4,294,967,296` bytesの条件で materializeした。full stage2を起動せず、
metadata `0..1`だけを実行したところ summary `status=diagnostic`、stdout `8,353` bytes、stderr `0`
bytesで終了した。これは current-source Linuxのstage1 materialize/native入口の診断証拠であり、
full stage2/stage3 fixed-point、Linux native stage0 source-file smoke、App.Cli release、package
acquisition/rollback、両 target parityは未検証である。VM、Cargo target、stage1/診断artifactは
検証後に削除し、VMは停止した。ADR: `docs/adr/decisions-v0.3-native-linux-stage1-metadata-e2e.md`。

同日、docs-only commit後の current `HEAD=41be4f2b28a329addffd3cd4de55f075b76a9ec2`から stage1を
再生成し、`1 passed`、`347.89s`、source commit `41be4f2b...`、code `4,393,425` bytes、data
`2,757` bytes、entrypoint `4,390,965`、function-start `3,409`、main function `3,418`を確認した。
同じ artifactをVMへ materializeし、full stage2を起動せず metadata `0..8`だけを実行した結果は
summary `status=diagnostic` / `phase=stage2-metadata`、stdout `53,484` bytes、stderr `0` bytesだった。
これは metadata prefixの現行 source境界を追加で識別した証拠であり、full stage2/stage3 fixed-pointや
payload materialization、Linux native stage0 source-file smoke、package acquisition/rollback、両 target
parityの証拠には拡張しない。VMは停止し、Cargo target、stage1、診断artifactを回収した。
ADR: `docs/adr/decisions-v0.3-native-linux-stage2-metadata-prefix-e2e.md`。

同じ current stage1 の entrypoint user index `3408`（actual function index `3418`）を metadata
`3408..3409`、prefix `128` で取得した。summaryは `status=diagnostic` / `phase=stage2-metadata`、
stdout `3,228` bytes、stderr `0` bytes。entrypoint IRの opcode `40` rowは3件で、次の通り emitted
bytes、signed rel32、function-relative targetが stage1の expected target diagnosticと一致した。

| IR idx | operand | offset | emitted bytes | rel32 | target |
| ---: | ---: | ---: | --- | ---: | ---: |
| 0 | 3416 | 11 | `e8 dd 9e ff ff` | `-24,867` | `-24,851` |
| 4 | 3417 | 49 | `e8 cd a8 ff ff` | `-22,323` | `-22,269` |
| 6 | 3415 | 59 | `e8 24 8b ff ff` | `-29,916` | `-29,852` |

この結果では entrypoint call rel32の opcode/bytes/target不一致は再現せず、広い append helper、spill
floor、offset-depth ctx/state refactor、または推測による call bundle修正は行わない。metadata-onlyのため、
full stage2 transport/materialize、target codeへの着地、stage3 fixed-point、Linux native stage0
source-file smoke、package acquisition/rollback、両 target parityは未検証である。ADR:
`docs/adr/decisions-v0.3-native-linux-stage2-entrypoint-rel32-diagnostic.md`。

この相関は `python3 scripts/ci/diagnose-native-linux-x86-entrypoint-metadata.py <metadata> \
--function-index 3418` で再現できる。fixtureの一致ケースと、bytesを1 byte変更した不一致ケースを
`bash scripts/ci/test-native-linux-x86-entrypoint-metadata-diagnostic.sh` で確認し、不一致時は
非ゼロ終了する。これは VM後処理の契約テストであり、native stage2/stage3の成功証拠には数えない。

### V2-16e / LEGACY-BOOT-01 Mac current-source stage0 package/source-file smoke (2026-08-02)

current `HEAD=80b299def6ca9b036b09bc427bc6d3e381fe63bf` で
`scripts/ci/native-macos-aarch64-stage0-release.sh` を実行し、
`test_e2e_native_macos_aarch64_actual_app_cli_release_program` を `1 passed` / `1,248.21s` で完走した。
生成した stage0 package `ci-artifacts/native-stage0/aarch64-apple-darwin/80b299de-current/` は
target `aarch64-apple-darwin`、current `source_commit`、relative な compiler / transport driver /
materializer pathを manifestへ保持し、`bin/compiler` は arm64 Mach-Oだった。

同じ packageを Mac native `scripts/ci/native-selfhost-dev-source-file-smoke.sh` に渡し、
`parse`、`check`、`fmt`、通常/metadata/property `test`、EC-M3 validation positive/negative fixture、
`compile`、`build`を実行した。runは `aarch64-apple-darwin native selfhost source-file smoke passed`、
exit `0` で終了した。成功経路では `cargo`、`rustc`、host `lsharp`を blocklistし、Rust fallbackを使わない
ことを smoke runner自身で確認した。positive outputは parse `decls:1` / `first-decl:defn` /
`first-body:int` / `diagnostics:0`、check `Int` / `diagnostics:0`、fmt `(defn main [] 42)`、通常test
`examples:0` / `invariants:0` / `failures:0`、metadata test `examples:2` / `invariants:1` /
`failures:0`。compile/buildの Wasm bytesは各 `2,559` bytes、SHA-256
`afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`で一致した。

source smoke evidenceは `ci-artifacts/native-stage0/aarch64-apple-darwin/80b299de-source-smoke-evidence/`
へ保存し、manifestは exit `0`、target/source commit、stage0 manifest SHA-256
`0452f04df1a6a0b9ec5d57e0833542983f4a6ccfbf4df86a2b505db19e5bd5ec`、payload SHA-256
`7d328772978bad0a3a0e4abf9703675cd9e01f5fa3c8c5bdc3461f1c0e8c938c`を記録した。初回 smokeで
macOS Bashの `set -u` 空配列展開が evidence writerを呼ぶ cleanupを失敗させたため、
`test-native-selfhost-source-file-smoke-evidence.sh` に REDを固定し、writer commandの配列化と optional
argumentの条件付き appendをGREENにした。focused test、`bash -n`、`git diff --check`、修正後 Mac source-file
smokeはすべて passし、task-owned Cargo target、source smoke stage directory、stale packageは回収した。

これは Mac current-source stage0 package/source-file smokeの verified sliceであり、Linux x86_64の同一
current commit package、公開 release asset acquisition、rollback実行、全公開 command、component sidecar、
Mac/Linux packaged artifact parity、`LEGACY-BOOT-01` aggregateの完了を意味しない。生成 package/evidenceは
ローカル `ci-artifacts/native-stage0/` に保持し、build差分として追跡しない。

### V2-16e / LEGACY-BOOT-01 packaged stage0 runner consumer contract (2026-08-01)

`bash scripts/ci/test-package-native-linux-x86-actual-stage1-vm.sh` は、current HEADの
`source_commit`、Linux x86_64 target、stage1 payload metadataを検証してから fake Lima materializerで
stage0 packageを作る。続けてその packageを `scripts/native-selfhost-dev.sh check` の `NATIVE_STAGE0_DIR`
へ渡し、package内の transport driver、decoder、materializer、programを runnerから実際に消費する。
出力は `Int` / `diagnostics:0`、stderrは空で、`cargo`、`rustc`、host `lsharp` の呼び出しは blocklist
で検出しない。REDでは host `timeout`、transport header、Mac host assemblerの境界を順に検出し、
テスト専用 shimと最小合法 transport fixtureでGREENへ進めた。

このテストは package manifest/payload/provenance と runnerの接続契約を閉じる fake harnessであり、
実 Linux VMの current-source stage1 package、source-file smoke、full stage2/stage3 fixed-point、
release acquisition/rollback、Mac/Linux artifact parityの証拠ではない。ADR:
`docs/adr/decisions-v0.3-native-linux-stage0-package-runner-contract.md`。

### V2-16e / LEGACY-BOOT-01 fetch install atomic rollback contract (2026-08-01)

`bash scripts/ci/test-fetch-stage0-atomic-install.sh` は local stage0 release archiveを作成し、
release/package checksum、target、current `source_commit`、native stage0 manifestを通常の
`scripts/fetch-stage0.sh` で検証する。checksumに登録されていない追加 payloadを rejectする REDと、
既存 `stage0/` を先に配置して最終 install moveだけを一度失敗させる REDを同じ fixtureで実行する。
GREENでは valid packageの install、checksum未登録 fileの reject、install error時の既存 package復元、
temporary packageと hidden previous backupの削除、partial replacementなしを確認した。

これは fetch package file-set と local atomic rollback boundaryであり、GitHub Release assetの取得、実 Linux
VM runtime、Mac/Linux rollback archiveの実 parity、provider/external release運用の証拠ではない。
ADR: `docs/adr/decisions-v0.3-native-stage0-fetch-atomic-install.md`。

### V2-16e / V2-13a-5 Linux x86 string-concat bounded heap allocation (2026-08-02)

Linux x86 selfhostの `emit-x86-selfhost-string-concat-helper` は、連結結果ごとに `mmap` syscallを発行していた。
current-source stage2 replayのメモリ境界を広げる前に、この局所的な per-allocation working-set要因を、materializerが
所有する `r14` native heapへ移した。heap先頭のcursor/limitを読み、`8 + lhs_length + rhs_length` を16-byte境界へ
alignしてbump allocateし、String header、lhs→rhs payload、high-bit tagを従来のABIのまま返す。cursorが未初期化の
場合は既存data frontier `8192` を使い、limit超過はnullを返す。helperは195 bytesから197 bytesへ増えたため、後続の
x86 trailer offsetとappend lengthはすべて実測byte vectorに合わせて `+2` へ同期した。

REDは `test_native_codegen_x86_string_concat_uses_bounded_heap_cursor` で旧 `mov eax,9; syscall` byte sequenceを
検出した。GREENは同テスト、197-byte slice/concat emitter回帰、call-site、CLI、write-file trailer offsetのfocused
testsを通過した。さらにLinux x86_64 Lima VMで、`r14` heapを初期化した最小native programへdynamic tagged `"ab"`
と `"Z"` を渡し、連結結果長 `3` を exit code `3` として確認した。Cargo target、VM一時ファイル、VMは検証後に回収・停止した。

これは string-concatのbounded allocation/copy/tagging ABIを閉じる verified sliceであり、full current-source stage2/
stage3 transport/materialize/fixed-point、Linux native stage0 source-file smoke、package acquisition/release/rollback、
`read-file` の残りのallocation helper、Mac/Linux release parityの証拠ではない。同じ
stage2 replayをこの局所修正だけで再実行せず、次は残りのallocation boundaryを一つずつRED化してから、必要なLinux
	stage2/stage3 gateを一回だけ実行する。ADR:
`docs/adr/decisions-v0.3-native-linux-string-concat-bounded-heap.md`。

### V2-16e / V2-13a-5 Linux x86 int-to-string bounded heap allocation (2026-08-02)

Linux x86 selfhostの `emit-x86-selfhost-int-to-string-helper` は、signed i64を最大20 bytesへ変換するたびに
固定32-byte String objectを `mmap` していた。helper先頭だけを、materializer-owned `r14` native heapのcursor
offset/limitへ置換した。cursorが0なら既存data frontier `8192` をレジスタへ置き、`cursor + 32 <= limit` を
確認してから `[r14]` を更新する。成功後に `r14 + cursor` をobject addressとして従来の十進変換、String header、
payload copy、high-bit tagへ渡し、失敗時は `rax=0` とcursor不変を保つ。`rbx` と `r14` のcallee-saved契約、import 6
dispatch、helper開始offset `base+2219` は維持した。helper長とtrailer append lengthは169から160へ同期した。

REDは `test_native_codegen_x86_int_to_string_uses_bounded_heap_cursor` で `48 c7 c0 09 00 00 00 0f 05`
（`mov rax,9; syscall`）を検出した。GREENは同テスト、既存の signed decimal/header/callee-saved ABI回帰、
`test_native_codegen_routes_int_to_string_import_to_target_helpers` を通過した。sourceのappend-encoded byte vectorと
clangで再構成したLinux objectは160 bytesで一致した。

Lima `lsharp-linux-x86` のVM-side lock下で最小native programを実行し、`r14` heapから `-42` を `-42` として出力した。
同じ生成helperについて `limit=cursor+31` は `rax=0` / cursor不変、`+32` は成功する境界も確認した。Cargo target、VM側
一時アセンブリ/実行ファイル/lockは検証後に回収し、VMは停止する。

これは signed int-to-string bounded allocationとlimit boundaryのverified sliceであり、full current-source stage2/
stage3 transport/materialize/fixed-point、Linux native stage0 source-file smoke、package acquisition/release/rollback、
`read-file` の残りのallocation helper、Mac/Linux release parityの証拠ではない。次は同じRED→focused
GREEN単位で残りのallocation境界を選び、full stage2/stage3 gateは複数の局所修正をまとめた一回だけ実行する。ADR:
`docs/adr/decisions-v0.3-native-linux-int-to-string-bounded-heap.md`。

### V2-16b / V2-16c / V2-16e Linux x86 current read-stdin fixed point (2026-08-03)

read-stdin helperを含む clean `HEAD=113d3785a54e0d4af0bc970edfe45c234a96449d` の actual Linux x86_64 stage1を
`302.68s` で生成した。stage1 manifestは target `x86_64-unknown-linux-gnu`、source commit同値、code `4,408,352` bytes、
data `2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、main function `3,427`。stage1は
`lsharp-native-selfhost-stage0` packageへ変換し、compiler、transport driver、materializer、manifest provenanceを確認した。
hostgen起動時の source commit envが未指定だったため、生成byte列と clean HEADを確認して保存 manifestの provenance fieldのみを
補正した。以後の package/source smokeとstage2/stage3 manifestは current commitを自動記録しているため、この補正済みstage1 manifestを
「生成時自動記録」の証拠とは分類しない。

packageを使った current-source native stage0 source-file smokeは exit `0` で、parse/check/fmt、通常/metadata test、compile/build、
validation positive/negativeを完走した。evidenceは
`ci-artifacts/native-linux-x86-hostgen-vm/113d3785-read-stdin-source-smoke-evidence/manifest.json` に保存し、
compile/buildは各 `2,559` bytes、SHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`、
stage0 manifest/payload SHA-256は `8f05341f559400a6df0de7c73cc357dcdd3caa8f5ebaf280888923a0c4522dd6` /
`f7d152eccbe02ff4bcf10bd8f48494a1d8fef2b6c94b5a58a795324ae0b30dda6` だった。成功経路は `cargo`、`rustc`、host `lsharp`、
Rust fallbackを使用しない。

同じstage1を再利用した `stage2 -> stage3 -> materialize -> compare` は Lima `lsharp-linux-x86` で status `pass` となった。
stage2/stage3 manifestは target/source commitとも一致し、code length各 `11,408,204`、stdout各 `1,434,730` lines、
stdout SHA-256各 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr各 `0` bytesで一致した。
stage2 seedの `int-to-string` / `string-concat` 経路と `42` tiny markerもstage3と一致した。`actual-selfregen-summary.json` は
旧schemaのため `source_commit=null` であり、provenance根拠には使わず、stage manifestを正本とする。

検証後は VM workdirを削除し、Lima VMを停止した。VMは 4 CPU / 16 GiB RAM / 12 GiB diskの既存構成を維持し、gate前のroot filesystem
availableは約 `7.68 GB`、使用率 `33%` だった。これは Linux current-source native coreの verified partialである。
Linux direct LSPのRust互換JSON/semantic projection、standalone Preview1 `io-read-stdin` runtime、component sidecar、公開 release
asset acquisition、rollback archive実行、Mac/Linux packaged artifact provenance parityは未完了のため、TODOの `[~]` を維持する。

### V2-16b / V2-16c / V2-16e current-source LSP wire projection (2026-08-02)

clean `HEAD=5db1c2a4f5147469f24eaca97976e2e62cfb6455` で、LSP wire projectionの4 operationをMac Apple SiliconとLinux
x86_64のcurrent-source native経路から確認した。Macは
`ci-artifacts/native-release/aarch64-apple-darwin/current-5db1c2a4-lsp/` の actual `App.Cli` release artifactを生成し、
gateは `1 passed` / `836.52s`、manifestは target `aarch64-apple-darwin`、`selfhost_fixed_point=true`、Mach-O arm64、
program SHA-256 `4b2650908f2e55037ab6e76a6129aed038133e78839a135525fb9f03be1ff4d8`、artifact size `4,676 KiB` を記録した。
direct `lsp --stdio` wire batchは exit `0`、stderr `0` bytesで、initialize capabilities object、hover range object、
completion item object、formatting `TextEdit` objectを返した。

Linuxは `ci-artifacts/native-linux-x86-hostgen-vm/5db1c2a4-lsp-stage1/` から stage0 packageを再生成し、manifestの target
`x86_64-unknown-linux-gnu`、source commit、code `4,408,352` bytes、data `2,757` bytes、entrypoint `4,405,792`、
function-start `3,418`、main function `3,427` を確認した。package/source-file smokeは exit `0` で、同packageを
Lima `lsharp-linux-x86` 内の `native-selfhost-dev.sh lsp --stdio` に渡した direct wire batchも exit `0`、stderr `0` bytesで、
同じ4種の object projectionを返した。stage2→stage3 fixed pointは
`ci-artifacts/native-linux-x86-hostgen-vm/5db1c2a4-lsp-stage23-current/actual-selfregen-summary.json` で status `pass`、
code length各 `11,408,204`、stdout各 `12,249,104` bytes、SHA-256各
`b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr空を記録した。

これは initialize/hover/completion/formattingの current-source native projection verified sliceである。definition/references/
renameのURI/location/workspace edit、diagnosticsの標準Diagnostic shape、position base、standalone Preview1 `io-read-stdin`
runtime、component sidecar、公開 release asset acquisition/rollback、Mac/Linux packaged artifact provenance parityは未完了で、
TODOの `V2-16b` / `V2-16c` / `V2-16e` と aggregateは `[~]` のまま維持する。direct LSP確認後はVM workdirを削除し、Lima VMを停止した。

同じ current Mac `program.native` に `native-selfhost-component.py --command compile` を渡した実native component boundaryは
exit `1`、artifactなしで終了した。stderrは `wasm-tools` の component encode時に
`wasi_snapshot_preview1::fd_write` importを解決できないことを示した。これは component sidecarを曖昧に成功扱いしない
fail-closed external packaging boundaryの証拠であり、Rust-free component実装やstandalone component runtimeの証拠ではない。

公開 provider assetも確認した。GitHub release `v0.1.0-native-rc1` は `2026-05-11T13:23:20Z` 公開の Mac arm64のみの
experimental native archiveで、Linux x86_64 assetは存在しない。取得した archiveの SHA-256は
`cd1a5df9db240eb155fe5b2fc9d6c24f721d6f078a870cfad4954724820725a0` だが、embedded `manifest.json` は stage hashだけで
current source commit/provenanceを持たないため、`HEAD=5db1c2a4` の release evidenceや両 target packaged parityには採用しない。
current-source release archive、実 provider asset acquisition、rollback archiveの実行は引き続き外部 release作業として残る。

### V2-16b / V2-16c / V2-16e current-source LSP zero-based position and Linux fixed point (2026-08-02)

current `HEAD=9175c6e50f4a6845ae97836b3ac6897102f3dd52` で、LSP wireの incoming Positionを内部1-based座標へ加算し、hover rangeと
formatting TextEdit rangeを外向き0-based座標へ減算する修正を反映した。focused REDは incoming `line=0,character=6` が mock
type-infoと0,0 rangeへ崩れ、formattingが1-based rangeを返すことを確認し、GREENは同fixtureの zero-based hover/formattingを確認した。

Mac Apple Siliconの `scripts/ci/native-macos-aarch64-selfhost-release.sh` は `1 passed` で、artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-9175c6e5-lsp-zero-based/` の manifestは target
`aarch64-apple-darwin`、source commit同値、`selfhost_fixed_point=true`、program SHA-256
`8106ebcb373da7d4b4183ee23b3a87b423afce5e2300e956fdb8915031865d18` を記録し、programは Mach-O arm64だった。同artifactの
direct `lsp --stdio` は exit `0`、stderr空で、numeric URIの hover range `0:6 -> 0:12` と formatting TextEdit range
`0:0 -> 1:3` を返した。

Linux x86_64では host actual stage1を `287.20s` で fresh生成し、manifestの target/source commit、code `4,408,352` bytes、data
`2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、main function `3,427` を実バイトと照合した。これを
`lsharp-native-selfhost-stage0` packageへ materializeし、package manifestの target/source commitを検証した。同packageの
`native-linux-x86-native-stage0-source-file-smoke.sh` は chunk `64` / timeout `900` で exit `0`、parse/check/fmt/test/validation/
compile/buildを成功させ、compile/buildは各 `2,559` bytes、SHA-256
`afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00` だった。成功経路では `cargo`、`rustc`、host `lsharp`、
Rust fallbackを使用していない。

同じ current stage1を `LSHARP_NATIVE_LINUX_X86_REUSE_ACTUAL_STAGE1_ARTIFACT_DIR` で再利用し、Lima
`lsharp-linux-x86` の VM-side lock下で stage2→stage3 transport/materialize/compareを一度だけ実行した。
`actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux x86_64`、`status=pass`、stage2/stage3 code
length各 `11,408,204`、stdout各 `12,249,104` bytes、stdout SHA-256一致
`b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr各 `0` bytesを記録した。stage2/stage3の full
stdoutは byte-for-byteで一致したが、別個の `stage2-tiny-summary.txt` は生成されていないため、tiny int/stringを独立した runtime
evidenceとしては扱わない。検証後は VM workdirと両 lockを回収し、Lima VMを停止した。

これは current-source Macの zero-based hover/formatting wire contractと、Linuxの stage1/package/source-file/stage2-stage3
fixed-point verified sliceである。Linux direct LSP semantic projection、definition/references/renameのURI/location/workspace
edit、diagnosticsの標準shape、standalone Preview1 `io-read-stdin` runtime、component sidecar、公開 release asset acquisition/
rollback、Mac/Linux packaged artifact provenance parity、`LEGACY-BOOT-01` aggregateの完了を意味しない。TODOの `[~]` と Rust
oracle/bootstrap/host integration境界は維持する。

### V2-16b / V2-16c / V2-16e current-source LSP standard URI navigation projection (2026-08-02)

current `HEAD=79bde035095e71a2baa85532c0430a9d76ea2ca7` で、標準 nested LSP params の string URIを自己ホスト LSP stateへ保持し、
definition / references の結果を URI付き `Location` object、renameの結果を URI-keyed `WorkspaceEdit` objectへ投影する変更を
RED→GREENで固定した。旧経路の integer URIと配列 locationをREDで検出し、`test_e2e_selfhost_cli_lsp_stdio_standard_uri_navigation_contract`
の同一実際 `lsp --stdio` fixtureで GREEN（`1 passed` / `319.24s`）を確認した。URI hashが負値になる入力を非負 map keyへ正規化し、
standard `didOpen` / `didChange` の parser parameter shapeを `[uri, source, path, uri-text]` に揃えた。state-aware rendererは保存した
wire URIをJSON stringとして escapeして再利用し、publish diagnostics / didOpen / didChangeにも同じ URI保持を適用した。numeric URIは
既存互換のlegacy rendererへfallbackする。

Evidence: `test_e2e_selfhost_lsp_state_preserves_wire_uri`、LSP docs/ops focused group（22 passed / 23 ignored）、
`python3 scripts/ci/test-native-selfhost-lsp-stdio.py`（5 passed）、commit `79bde035`。definition/referencesの内部 locationは
まだ point rangeであり、stateに実URIがない imported filesystem locationは `lsharp://document/<id>` synthetic URIへfallbackするため、
full symbol range、cross-document URI provenance、diagnosticsの標準Diagnostic shapeは未完了である。

これは current-source host-Wasm selfhost verified partialであり、current `HEAD=79bde035` のMac Apple Silicon / Linux x86_64
native artifactをこのURI変更込みで再生成したdirect LSP gateではない。直前 `9175c6e5` のMac zero-basedとLinux fixed-point evidenceを
current URI evidenceへ拡大解釈せず、native regeneration、standalone Preview1 runtime、component sidecar、release acquisition/rollback、
Mac/Linux packaged provenance parity、LSP aggregateは TODOの `[~]` と Rust oracle/bootstrap/host integration境界を維持する。

### V2-16b / V2-16c / V2-16e current-source LSP diagnostics URI refresh (2026-08-02)

current `HEAD=6f0238c240d12e5e0f4bbb31dc9ff75b774b4535` で、didOpen/didChange後に
`lsp-transport-maybe-append-diagnostics-frame` が追加する `publishDiagnostics` frameも、stateに保存した wire URIを
`lsp-render-publish-diagnostics-frame-with-state` へ渡すよう修正した。REDの
`test_e2e_selfhost_lsp_transport_diagnostics_preserves_wire_uri` は、didOpen notificationは string URIだが後続 diagnostics frameだけが
numeric URI hashになることを実際の `App.Cli` bundleで検出した。GREENは同じ fixtureで `1 passed` / `318.71s`、
`test_e2e_selfhost_lsp_state_preserves_wire_uri` は `1 passed` / `9.13s`、
`python3 scripts/ci/test-native-selfhost-lsp-stdio.py` は `5 passed` となった。

この結果は diagnostics refresh の URI ownership を閉じる host-Wasm selfhost verified partialである。標準 LSP Diagnosticの
field/schema parity、cross-document URI provenance、current `HEAD` のMac Apple Silicon / Linux x86_64 native artifact direct gate、
standalone Preview1 runtime、component sidecar、release acquisition/rollback、Mac/Linux packaged provenance parity、LSP aggregateは
未完了であり、TODOの `[~]` と Rust oracle/bootstrap/host integration境界を維持する。

### V2-16b / V2-16c / V2-16e current-source native LSP URI gate (2026-08-03)

clean `HEAD=d6517499b6b00287e901b278b1f549d56cc4fc2c` で、Mac Apple Siliconの
`scripts/ci/native-macos-aarch64-selfhost-release.sh` を一度だけ実行した。gateは `1 passed` / `941.46s`、manifestは
target `aarch64-apple-darwin`、同じ source commit、`selfhost_fixed_point=true`、program SHA-256
`ee884e4828e6263830e3b3d7908ad85444b012130e67f0d068b2bd360f5c072d`、Mach-O arm64、program `4,343,680` bytesだった。
artifactは `ci-artifacts/native-release/aarch64-apple-darwin/current-d6517499-lsp-diagnostics/` に保持した。

同artifactに標準 string URIの didOpen → definition / references / rename fixtureを直接渡した `lsp --stdio` は exit `0`、
stderr `0` bytes、5 framesで終了した。didOpen後の `publishDiagnostics`、definition / referencesの `Location`、renameの
URI-keyed `WorkspaceEdit` はすべて `file:///tmp/lsharp-uri-contract.ls` を保持した。これは current-source Mac native
artifactでの URI ownership / standard navigation projection の verified partialである。

同じ clean HEADから Linux x86_64 hostgen→Lima VM gateも一度だけ実行した。actual stage1 manifestと stage2/stage3 debug
manifestは target `x86_64-unknown-linux-gnu`、source commit `d6517499...` が一致し、stage1 code/dataは
`4,408,352` / `2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、main function `3,427`だった。
`actual-selfregen-summary.json` は `status=pass`、stage2/stage3 code lengthは各 `11,408,204` bytes、transport stdoutは
各 `12,249,104` bytes / SHA-256 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderrは各 `0` bytesで一致した。
artifactは `ci-artifacts/native-linux-x86-hostgen-vm/current-d6517499-lsp-diagnostics/` に保持した。

今回の Linux gateは host-generated stage1 payloadの stage2→stage3 transport/materialize/fixed-pointを対象とし、Linux
target-only App.Cli release binaryやLinux direct LSP semantic projectionまでは生成・実行していない。したがって標準
Diagnostic field/schema、full symbol range、cross-document URI provenance、Linux direct LSP、standalone Preview1 runtime、
component sidecar、公開 release asset acquisition/rollback、Mac/Linux packaged provenance parity、LSP aggregateは未完了である。
Lima `lsharp-linux-x86` は検証後に停止し、task-owned VM workdirと replay lockは回収した。TODOの `[~]` と Rust
oracle/bootstrap/host integration境界は維持する。

### V2-16b / V2-16c / V2-16e current-source parse diagnostics standard projection (2026-08-02)

current `HEAD=95656144` で parse-only diagnosticsの標準 LSP Diagnostic projectionを RED→GREENで固定した。
`test_e2e_selfhost_lsp_render_standard_parse_diagnostic_json` は enriched recordから zero-based `range`、LSP `severity`、
`LS0101`、`source="lsharp"`、messageを標準 objectへ投影し、legacy renderer/snapshotを含む render groupは `5 passed` / `17.05s`。
実際の `App.Cli` bundleで `lsp-source-parse-diagnostics ")"` を呼ぶ
`test_e2e_selfhost_lsp_parse_diagnostics_use_standard_projection` は `1 passed` / `321.40s`、
`scripts/ci/test-native-selfhost-lsp-stdio.py` は `5 passed` となった。

これは parse producerから JSON rendererまでの host-Wasm verified partialであり、type/lint diagnosticsの標準 fields、
複数診断の全文、full span/code/message parity、current HEADのMac Apple Silicon / Linux x86_64 native artifact direct gate、
cross-document URI provenance、standalone Preview1 runtime、component sidecar、公開 release asset acquisition/rollback、
Mac/Linux packaged provenance parity、LSP aggregateの完了を意味しない。TODOの `[~]` と Rust oracle/bootstrap/
host integration境界を維持する。

その後 current `HEAD=ab5122c18786f82d6277f36bbddaf6b31ff98f16` で Mac Apple Siliconの
`scripts/ci/native-macos-aarch64-selfhost-release.sh` を一度だけ実行し、`1 passed` / `922.36s`、target
`aarch64-apple-darwin`、`selfhost_fixed_point=true`、Mach-O arm64、program `4,360,192` bytes、program SHA-256
`d66c271e96545b0a2310424ee8a4f12c01bc56d1ad7180db154f53a9dc10d0e6` を確認した。artifactは
`ci-artifacts/native-release/aarch64-apple-darwin/current-ab5122c1-lsp-parse/` に保持した。同artifactへ invalid parseの
`lsp --stdio` wireを直接渡し、publishDiagnosticsの標準 `range` / severity / `LS0101` / source / messageと URIを一致確認した。

同じ source commitで Linux x86_64 hostgen→Lima VM gateも exit `0` となった。stage1 manifestは target
`x86_64-unknown-linux-gnu`、code/data `4,408,352` / `2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、
main function `3,427`。stage2/stage3 manifestの target/source commitが一致し、code length各 `11,408,204`、stdout各
`12,249,104` bytes / SHA-256 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr各 `0` bytes、
summary `status=pass` で一致した。artifactは `ci-artifacts/native-linux-x86-hostgen-vm/current-ab5122c1-lsp-parse/` に保持し、
VM workdir/lockを削除、Limaは停止した。

これは parse-only標準 DiagnosticのMac native direct evidenceと Linux stage2/stage3 fixed-pointを追加する verified partialである。
type/lint diagnosticsの標準 fields、複数診断の全文、full span/code/message parity、Linux direct LSP semantic projection、
cross-document URI provenance、standalone Preview1 runtime、component sidecar、公開 release asset acquisition/rollback、
Mac/Linux packaged provenance parity、LSP aggregateの完了を意味しない。TODOの `[~]` と Rust oracle/bootstrap/host integration境界を維持する。

### V2-16e / V2-13a-5 Linux x86 substring bounded heap allocation (2026-08-02)

Linux x86 selfhostの `emit-x86-selfhost-substring-helper` は、substring結果を per-allocation mmapで確保していた。
`r14` native heapのcursor/limitを使う bounded allocationへ置き換え、`8 + (end - start)` を16-byte境界へ alignし、
limit超過時はcursorを変更せず nullを返す。String header、payload copy、high-bit tag、signed range handling、
callee-saved ABIは維持した。helperは145 bytesから147 bytesへ増えたため、後続helper offset、append length、rel32
targetを実測byte vectorへ同期した。

REDは `test_native_codegen_x86_substring_uses_bounded_heap_cursor` で旧 mmap syscall byte sequenceを検出した。
GREENは同テスト、`end - start` length回帰、147-byte emitter、substring/concat call-site、trailer offset、function
metadata targetのfocused testsを通過した。host-side Linux x86 object smoke matrixにも substring objectを含む。

これは substring bounded allocation/copy/tagging ABIの verified sliceであり、current-source VM object runtime、full
stage2/stage3 transport/materialize/fixed-point、Linux native stage0 source-file smoke、package acquisition/release/rollback、
read-file runtime evidence、Mac/Linux release parityの証拠ではない。ADR:
`docs/adr/decisions-v0.3-native-linux-substring-bounded-heap.md`。

### EC-M1-01 current-source Mac Apple Silicon App.Cli native gate (2026-08-02)

current `HEAD=3f6c49976f75a5099d524f08ea85cc1698935cbb` の clean worktreeから、
`scripts/ci/native-macos-aarch64-selfhost-release.sh` を一度だけ実行して actual stage2/stage3 fixed-pointを経た
App.Cli native release programを生成した。manifestは target `aarch64-apple-darwin`、同じ `source_commit`、
`selfhost_fixed_point=true`、program SHA-256 `a1dac9ff7146fbfd012c6e299df786c3c6c00680e3849cfb98abdeb1efcd76de`
を記録し、programは Mach-O 64-bit arm64だった。`--version` は `lsharp 0.1.0`、stdout 12 bytes、stderr 0 bytes。

Evidence: `LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR=/tmp/lsharp-native-macos-aarch64-release-3f6c4997
LSHARP_NATIVE_MACOS_AARCH64_CARGO_TARGET_DIR=/tmp/lsharp-native-macos-aarch64-release-cargo-3f6c4997
LSHARP_NATIVE_MACOS_AARCH64_RELEASE_LOCK_DIR=/tmp/lsharp-native-macos-aarch64-release-3f6c4997.lock bash
scripts/ci/native-macos-aarch64-selfhost-release.sh`、
`test_e2e_native_macos_aarch64_actual_app_cli_release_program`（1 passed、896.66s）。artifactは
`ci-artifacts/native-release/aarch64-apple-darwin/current-3f6c4997/` に保持し、Cargo targetとlockは cleanup済み。

これは current-source Mac App.Cli fixed-point/native runtimeの verified sliceであり、Linux x86_64の同一
current-source gate、native stage0 compiler/package、全公開 command、release acquisition/rollback、両 targetの
provenance parity、EC-M1-01/`LEGACY-BOOT-01` aggregateの完了を意味しない。TODOの `[~]` と Rust oracle /
bootstrap / host integration境界は維持する。

### V2-16c actual selfhost CLI check file boundary (2026-08-02)

`App.Cli` の `check <file>` argv contractを ignored testから通常の actual Wasm E2Eへ昇格した。
REDでは bundle parse が `Cli.ls` の `run-test-source-json` と validation option loopの defn境界で
閉じ括弧不足により次の `defn` を `(` として拒否し、修正後は `Tools.Validation.ManifestInput`
の importが runtime bundle inventoryに欠けて `UndefinedVar` になった。CLI sourceの括弧を補正し、
support bundleの source-path / embedded source / module listを同期した。

Evidence: `test_e2e_selfhost_cli_main_with_args_check_file` は actual `Cli` Wasmを
`check input.ls` argvで実行し、stdout `Int` / `diagnostics:0`、exit successを `491.23s` で確認した。
`test_support_selfhost_cli_runtime_bundle_cached` では module marker、`ManifestInput`収録、cache identityを
確認した。これは Rust host compile/run の source bundle contractに限定され、native stage0 の
`check`、実 stage0の全公開 command、external tool、両 supported targetの release/provenance evidenceは
残る。`V2-16c` / `LEGACY-TOOL-01` と Rust oracle/bootstrap boundaryは維持する。

同日、`test_e2e_selfhost_cli_main_no_args_shows_help` の ignoreを外し、同じ actual `Cli` bundleを
引数なしで実行する gateを追加した。stdoutに `Usage: lsharp <command>` と `Commands:` が含まれ、
成功終了することを `447.61s` で確認した。これにより `main` の no-arg dispatch と help surface
serialization の Rust-host evidenceを追加したが、native stage0の no-arg parity、他の公開 command、
external helper、両 supported targetの release/provenance evidenceは引き続き未検証である。

同日、`test_e2e_selfhost_cli_main_batched_version_and_parse_argv` を追加し、同じ actual `Cli` bundleを
一度だけ compileして `--version`、`-v`、`parse input.ls` の3 argvケースを実行した。versionは
`lsharp 0.1.0`、parseは `decls:1` / `diagnostics:0`、全体は `1 passed`、`459.57s` だった。
この batch gate は長時間の bundle compile をケースごとに重複させない運用契約を示すが、native
stage0の version/parse parity、他の公開 command、external helper、両 supported targetの
release/provenance evidenceは未検証である。

### EC-M1-02 EmbeddedCli property diagnostic report forwarding slice (2026-07-20)

EmbeddedCli の実 argv `test input.ls --format json` について、non-Bool property precondition の failure boundary を Cli と同じ structured report へ転送することを追加検証した。stdout は JSON report 1 行、終了値は diagnostic failure の `2` とし、`implementation_conformance.status=fail`、`firstErrorCode=2`、`firstErrorSpan` は同一 fixtureを Rust canonical checkerへ渡して得た spanと比較した。

Evidence: `test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_property_precondition_span` は `1 passed`、`383.11s`。このテストは `selfhost_embedded_cli_runtime_bundle()` を実 argv で起動し、source file の読み込みから report serialization、exit codeまでを通過させる。既存の EmbeddedCli non-Bool invariant report test と合わせ、EmbeddedCli の structured diagnostic report の property precondition span forwardingを verified sliceとする。

これは EmbeddedCli の一つの property diagnostic boundaryに限定した evidenceであり、text report、全 property failure の report forwarding、EmbeddedCli の全形式・全公開 command、Rust/selfhost differential、Mac Apple Silicon / Linux x86_64 current-source artifact/runtime、EC-M1-06 / EC-M1-02 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-02 property text failure forwarding slice (2026-07-20)

selfhost CLI の text `test` について、non-Bool property precondition が property を実行済みの成功として隠さず、既存の text summary と diagnostic exit boundaryへ到達することを確認した。source runner は `properties:1`、`failures:1`、`diagnostics:1,LS1002` を順に返し、終了値は `2` となる。

Evidence: `test_e2e_selfhost_cli_text_reports_non_bool_property_precondition` は `1 passed`、`401.00s`。これは `run-test-source` の text preflight 経路を同一 fixtureで実行した verified sliceであり、property precondition の `LS1002` code、failure count、exit codeの forwardingを固定する。text 出力へ正確な source spanを追加する仕様は未確定のため、この sliceでは現行の code-only text contractを維持した。

これは text failure boundaryの code/exit forwardingに限定した evidenceであり、text span表示、全 property failure report forwarding、actual Cli/EmbeddedCli の全 text形式、Rust/selfhost differential、Mac Apple Silicon / Linux x86_64 current-source artifact/runtime、EC-M1-02 / EC-M1-06 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-02 multiple property directive index slice (2026-07-20)

Rust の `generate_tests` が同一関数の複数 `:property` directive ごとに property index を 0 へ戻し、生成テスト名を重複させる不整合を修正した。directive をまたぐ関数単位の counter を使い、deterministic smoke profile として生成された property だけを `property_0`, `property_1`, ... の source order で採番する。unsupported profile は既存の明示的な外部境界を維持し、採番対象へ暗黙に混ぜない。

Evidence: RED `metadata_check::test_generation_tests::test_generate_multiple_property_forms_have_unique_names` は 2 件目が `identity_property_0` となって失敗し、GREEN は `1 passed`。関連する Rust metadata generation 9 tests と metadata checker 22 tests は全件 pass。selfhost 側も同一の二つの `:property` fixtureを `extract-property-test-cases` へ渡し、結果数 `2` と index `0, 1` を `test_e2e_selfhost_property_test_cases_assign_global_indices_across_forms`（`1 passed`、24.99s）で確認した。専用 cargo target は検証後に削除する。

これは property test name/index の directive-boundary parity に限定した verified sliceであり、一般 `TypeExpr`、全 ContractSuite evaluator、structured report、supported 2 targets の current-source artifact/runtime、EC-M1-02 / EC-M1-03 aggregate の完了を意味しない。TODO の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-01 strict match-arm Bool slice (2026-07-20)

legacy `:invariant` の strict Bool 契約について、実際に選択された arm だけが Bool なら成功するという selfhost の欠落を修正した。Rust `Infer::Match` は各 arm の `when` guard も `Bool` として推論し、selfhost `TestRunner` は `match` の全 arm body と guard の静的 Bool shapeを先に確認する。直接解決できる user-defined function が parameter body を返す場合は call argument の静的 shapeを一段だけ本体へ伝播し、`identity 1` のような `Int` guard が match evaluator の `value-truthy` で隠れないようにした。higher-order / 未解決 callee など静的に分類できない式は従来の sample実行と `value-tag` 検査へ委ねるため、残りの dynamic subset boundaryは維持する。

RED fixtureは `make-just x` が常に `Just` を返す一方、未選択の `Nothing` armが `(+ x 1)` を返す `:invariant` で、修正前の selfhost結果は `diagnostic=0` だった。GREENでは Rust oracle の `LS1002` 相当 diagnostic、selfhostの `diagnostic=2`、同一sourceの invariant spanを一致させた。別の Rust RED fixture `(match true [_ when (+ 1 2) true] [_ true])` では、従来 `Infer::Match` が guardを推論せず diagnostics `0` だったが、guardの Bool unify追加後は metadata contract testを通過する。

Evidence: `test_e2e_selfhost_test_runner_rejects_non_bool_unselected_match_arm`（`1 passed`、35.83s）、`test_e2e_selfhost_test_runner_rejects_non_bool_match_guard`（`1 passed`、35.24s、guard式の exact span 一致）、`test_e2e_selfhost_test_runner_rejects_non_bool_compound_match_guard_span`（`1 passed`、35.92s、`if` guard 全体の exact span 一致）、`test_e2e_selfhost_test_runner_rejects_non_bool_function_match_guard`（`1 passed`、35.17s、user-defined function の静的 body shape と guard call の exact span 一致）、`test_e2e_selfhost_test_runner_rejects_non_bool_dynamic_function_match_guard`（`1 passed`、35.10s、`identity [x] x` への `Int` argument shape propagation と guard call の exact span 一致）、既存 strict Bool 5件（`5 passed`、425.54s）、valid guarded-match differential（`1 passed`、35.86s）、`legacy_invariant_match_guard_requires_bool` と `metadata_contract_check` 30件（全件 pass）。これは Rust metadata checker と Rust-host Wasm selfhost runnerの match-arm/guard code、静的 non-Bool guard span、直接解決できる user-defined guard の parameter shape verified sliceに限定され、guardの exact message/report forwarding、higher-order / 未解決 callee の全 span parity、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime、EC-M1-01 aggregate、他の未分類 type-directed expression parityの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### LEGACY-ROOT-01 selfhost compiler root_set source/ftable IR slice (2026-07-20)

selfhost compiler の source-aware `compile-program-functions-with-source` と ftable `compile-program-functions` について、既存 root slotへ allocating `map-insert` の結果を渡す IR order を別々に固定した。3引数関数 `(m k v)` の fixtureで `map-insert` が先に出力され、`root_set` がその結果を消費し、最後の明示的な `root_pop` より前に root slotを更新することを確認した。source pathは local-count `>=4`、ftable pathは params `3` / local-count `7` の outputで、ftable内部の先行 `root_pop` と明示 slot popを混同しないよう最後の `root_pop` を比較した。

Evidence: `test_e2e_selfhost_compiler_root_set_consumes_allocating_map_insert_result` と `test_e2e_selfhost_compiler_ftable_root_set_preserves_allocating_map_insert` は各 `1 passed`（38.91s / 39.17s）。最初の ftable RED は first `root_pop` を観測して `['3','7','19','24','20']` と失敗したが、これは内部 map lowering popを明示 slot popと取り違えたテスト観測ミスであり、last-pop観測へ修正後 GREENとなった。今回の evidenceは Rust host上の selfhost compiler IRに限定され、Wasm artifact/runtime、Mac native source/ftable artifact、Linux x86_64 VM gate、全 heap root rule、`LEGACY-ROOT-01` aggregateの完了を意味しない。TODOは `[~]` のまま維持する。

### EC-M1-01 higher-order lambda match guard slice (2026-07-21)

legacy `:invariant` の match guard について、最小の higher-order lambda call `((fn [x] x) 1)` が後続の Bool armへの fall-through で成功に見えないようにした。selfhost `TestRunner` の AST static Bool classifier は identity lambda の parameter shape を call argumentへ一段だけ伝播し、token sidecar は同じ call expression 全体を non-Bool guard span として選択する。closure capture、let-bound function value、一般の function value runtime へは拡張していない。

Evidence: RED `test_e2e_selfhost_test_runner_rejects_non_bool_lambda_match_guard` は selfhost diagnostic `0`（後続 armの Bool を誤って採用）で失敗した。GREEN では Rust oracle と selfhost の diagnostic code `2`、lambda call 全体の source span `46..61` を一致させた。同テストを含む non-Bool predicate/match guard 回帰 10 件は `10 passed`（484.62s）。これは direct lambda identity の static shape/span verified sliceであり、higher-order closure capture、unresolved callee、exact diagnostic message/report forwarding、supported 2 targets の current-source artifact/runtime、EC-M1-01 aggregate の完了を意味しない。TODO の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。

### Current-source Mac Apple Silicon App.Cli release after EC-M1-01 guard slice (2026-07-21)

現行 `main` (`faa490837b52b3050b9e340c14966a48be611a59`) の selfhost stage2/stage3 fixed-point から `App.Cli` native release programを再生成した。`test_e2e_native_macos_aarch64_actual_app_cli_release_program` は `582.12s` で passし、生成 artifact の manifest は target `aarch64-apple-darwin`、`source_commit`=現行 HEAD、`selfhost_fixed_point=true`、`program_sha256=fd2088fd22e8852d71945c6d0ecde9fcd7b5f6a6b60783ccf4b792cae3d7de23` を記録した。`program.native` は Mach-O arm64、実行時 `--version` は `lsharp 0.1.0`、stderr は 0 bytesだった。artifact は `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/faa49083-app-cli-current` に保存している。

これは current-source Mac Apple Silicon の fixed-point App.Cli release program と version smoke の evidenceであり、Linux x86_64 current-source artifact/runtime、EmbeddedCli native execution、全公開 command、stage0 acquisition/release/rollback、EC-M1-07 aggregate の完了を意味しない。専用 Cargo target は検証後に削除し、Mac/Linux 対応 target と Rust oracle / bootstrap / host integration の境界は維持する。

### EC-M1-01 legacy invariant diagnostic message forwarding slice (2026-07-20)

legacy `:invariant` の non-Bool failureについて、selfhost TestRunnerの diagnostic result末尾へ messageを追加し、Cli / EmbeddedCli の structured test JSON `implementation_conformance.diagnostics.message`へ転送した。既存の result code/span indexは維持し、static arithmetic predicate `(+ x 1)` は Rust oracleと同じ `:invariant は Bool 必須ですが、Int が推論されました` を返す。未分類の expression shapeは `Unknown` として誤った具体型を出さない。

Evidence: RED `test_e2e_selfhost_cli_test_source_json_reports_non_bool_invariant_message` は report message `null` と Rust oracle message `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで `1 passed`（426.04s）となり、既存の Cli JSON invariant report regressionは `1 passed`（434.85s）、実 argv EmbeddedCliの `test --format json` non-Bool invariant regressionは `1 passed`（381.04s）だった。両入口で report 1行と診断 exit `2`、既存の code/span/coverage fieldsを維持した。

これは static arithmeticで得られる `Int` messageと両CLIの JSON forwardingに限定した verified sliceであり、literal/String/Float/Unit、match guard内部の inferred type、higher-order/closure capture、全診断本文の型推論 parity、text report、Mac Apple Silicon / Linux x86_64 current-source artifact/runtime、EC-M1-01 aggregateの完了を意味しない。今回同時に取得した Linux x86_64 VM fixed-point artifact `faa49083-stage2-stage3-current` は `status=pass`、stage2/stage3 stdout SHA-256一致、stderr 0 bytesだったが、manifestの `source_commit=faa49083` はこの sliceの現行 commitではないため current-source evidenceには採用しない。次の current commit gateで provenanceを取り直す。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source Linux x86_64 diagnostic message fixed-point gate (2026-07-21)

`941920a496c6e69bf6177de42102ab06bd7b2b45` の current checkoutから host-generated stage1 x86 payloadを生成し、Lima `lsharp-linux-x86` VM上で stage2 と stage3 の native self-regenerationを同じ 64-byte chunk transportで完走させた。stage1、stage2-debug、stage3-debug の manifestはすべて target `x86_64-unknown-linux-gnu` と同じ `source_commit`を記録し、stage2/stage3は同じ entrypoint offset、function table length、code/data lengthを持つ。VMの free spaceは実行前後とも約 `5,456,564,224` bytesで、必要量 `4,294,967,296` bytesを満たした。検証後は一時 VM workdir、replay lock、host Cargo targetを削除し、アイドル VMを停止した。

Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/941920a4-stage2-stage3-current/actual-selfregen-summary.json` は `status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` の一致を記録する。両 stageの stderrは `0` bytesである。stage1 manifestの `code_len=4,203,487`、`data_len=1,523`、`function_start_len=3,237`、stage2/stage3 manifestの `entrypoint_offset=10,828,220` と `main_func_idx=3,246`も一致した。artifact sizeは約 `100M`で、current-source Linux x86_64 fixed-point evidenceとして保存している。

これは Linux x86_64上の current-source stage1 -> stage2 -> stage3 self-regenerationと固定点に限定した evidenceであり、Mac Apple Siliconのこの commitに対する current-source再実行、EmbeddedCli native artifact、全公開 command、全診断本文の型推論 parity、stage0 acquisition/release/rollback、EC-M1-01 aggregateの完了を意味しない。したがって TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 literal Int diagnostic message projection slice (2026-07-21)

legacy `:invariant` の direct literal non-Bool failureについて、selfhost `TestRunner` の message projectionが `ast-lit-int` を `Int` として返すようにした。既存の static arithmetic `Int` projection、診断 code/span、result vectorの後方 message field、Cli / EmbeddedCli の JSON forwardingは変更していない。

Evidence: RED `test_e2e_selfhost_cli_test_source_json_reports_literal_non_bool_invariant_message` は Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` に対し、selfhostが `:invariant は Bool 必須ですが、Unknown が推論されました` を返して失敗した（`431.92s`）。GREENは同じ fixtureで `1 passed`（`445.75s`）となり、Rust metadata checkerとselfhost source runnerの JSON message、diagnostic exit `2`を一致させた。専用 Cargo targetは検証後に削除した。

これは direct `Int` literalの本文投影に限定した verified sliceであり、String/Float/Unit literal、match guard内部の inferred type、user-defined/higher-order functionの戻り型、全診断本文の型推論 parity、text report、今回の変更後の Mac Apple Silicon / Linux x86_64 current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。前段の `941920a4` Linux fixed-point artifactはこの sliceの変更前 provenanceであるため、新しい current commit gateが完了するまで本変更の target evidenceには採用しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after literal Int projection (2026-07-21)

`0f6bf7b46fbdd06c9dd04d98f4ae4a3381c67079` を current sourceとして、direct `Int` literal message projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Mac gateは `test_e2e_native_macos_aarch64_actual_app_cli_release_program` が `583.70s`で passし、Linux gateは全 host probes、actual stage1、stage2、stage3 replayを完走した。どちらも変更前の `941920a4` artifactではなく、literal projectionを含む現行 commitの provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/0f6bf7b4-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=0f6bf7b46fbdd06c9dd04d98f4ae4a3381c67079`、`selfhost_fixed_point=true`、program SHA-256 `ac9431b87427d57f2e98ef3cde1d062e2f3d97fb6d56e20a0dff650867186f93`を記録する。`program.native` は Mach-O arm64、サイズ `3,468,544` bytes、`--version` は `lsharp 0.1.0`、smoke stderrは `0` bytesだった。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/0f6bf7b4-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 stderrは双方 `0` bytesである。VM free spaceは `5,449,560,064` bytes、必要量は `4,294,967,296` bytesで、検証後に VMを停止し、一時 workdir、lock、host Cargo targetを削除した。

これは literal `Int` projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、String/Float/Unit literal、match guard内部の inferred type、user-defined/higher-order functionの戻り型、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 literal String diagnostic message projection slice (2026-07-21)

legacy `:invariant` の direct literal non-Bool failureについて、selfhost `TestRunner` の message projectionが `ast-lit-string` を `String` として返すようにした。既存の `Int` literal / static arithmetic projection、診断 code/span、result vectorの後方 message field、Cli / EmbeddedCli の JSON forwardingは維持している。

Evidence: RED `test_e2e_selfhost_cli_test_source_json_reports_string_non_bool_invariant_message` は Rust oracleの `:invariant は Bool 必須ですが、String が推論されました` に対し、selfhostが `:invariant は Bool 必須ですが、Unknown が推論されました` を返して失敗した（`403.74s`）。GREENは同じ fixtureで `1 passed`（`435.14s`）、既存 `test_e2e_selfhost_cli_test_source_json_reports_literal_non_bool_invariant_message` の回帰は `1 passed`（`405.59s`）となり、Rust metadata checkerとselfhost source runnerの JSON message、diagnostic exit `2`を一致させた。専用 Cargo targetは検証後に削除した。

これは direct `String` literalの本文投影に限定した verified sliceであり、Float/Unit literal、match guard内部の inferred type、user-defined/higher-order functionの戻り型、全診断本文の型推論 parity、text report、今回の変更後の Mac Apple Silicon / Linux x86_64 current-source artifact/runtime gate、EmbeddedCli native release artifact、EC-M1-01 aggregateの完了を意味しない。次の target gateではこの String projectionを含む current source provenanceを確認する。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 literal Float/Unit diagnostic message projection slices (2026-07-21)

legacy `:invariant` の direct literal non-Bool failureについて、selfhost `TestRunner` の message projectionが `ast-lit-float` を `Float`、`ast-lit-unit` を `Unit` として返すようにした。既存の `Int` / `String` literal、static arithmetic projection、診断 code/span、result vectorの後方 message field、Cli / EmbeddedCli の JSON forwardingは維持している。

Evidence: RED `test_e2e_selfhost_cli_test_source_json_reports_float_non_bool_invariant_message` は Rust oracleの `:invariant は Bool 必須ですが、Float が推論されました` に対し、selfhostが `:invariant は Bool 必須ですが、Unknown が推論されました` を返して失敗した（`420.14s`）。RED `test_e2e_selfhost_cli_test_source_json_reports_unit_non_bool_invariant_message` も Rust oracleの `:invariant は Bool 必須ですが、Unit が推論されました` に対し selfhostが `Unknown` を返して失敗した（`407.18s`）。GREENは Float `1 passed`（`424.64s`）、Unit `1 passed`（`436.07s`）となり、各 Rust metadata checkerとselfhost source runnerの JSON message、diagnostic exit `2`を一致させた。専用 Cargo targetは検証後に削除した。

これは direct `Float` / `Unit` literalの本文投影に限定した verified slicesであり、match guard内部の inferred type、user-defined/higher-order functionの戻り型、全診断本文の型推論 parity、text report、今回の変更後の Mac Apple Silicon / Linux x86_64 current-source artifact/runtime gate、EmbeddedCli native release artifact、EC-M1-01 aggregateの完了を意味しない。次の dual-target gateでは `Int` / `String` / `Float` / `Unit` projectionを含む current source provenanceを確認する。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after literal Float/Unit projection (2026-07-21)

`a8d91da86dc359cfb1d1c987005c8b7451de43b6` を current sourceとして、`Int` / `String` / `Float` / `Unit` literal message projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Mac gateは `test_e2e_native_macos_aarch64_actual_app_cli_release_program` が `581.20s`で passし、Linux gateは全 host probes、actual stage1、stage2、stage3 replayを完走した。両 targetとも変更前 artifactではなく、literal projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/a8d91da8-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=a8d91da86dc359cfb1d1c987005c8b7451de43b6`、`selfhost_fixed_point=true`、program SHA-256 `f29f1e920c152a9a133133538feeb4d950bc05c2a285a3c59f04e3bad41ea25c`を記録する。`program.native` は Mach-O arm64、サイズ `3,468,544` bytes、`--version` は `lsharp 0.1.0`、smoke stderrは `0` bytesだった。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/a8d91da8-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。actual-stage1、stage1-debug、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 stderrは双方 `0` bytesである。VM free spaceは `5,443,948,544` bytes、必要量は `4,294,967,296` bytesで、検証後に VMを停止し、一時 workdir、lock、host Cargo targetを削除した。artifact sizeは `100M`だった。

これは literal `Int` / `String` / `Float` / `Unit` projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、match guard内部の inferred type、user-defined/higher-order functionの戻り型、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 direct arithmetic match guard diagnostic message slice (2026-07-21)

legacy `:invariant` の match guardについて、root `match` の static non-Bool messageが `Unknown` に落ちるため、direct arithmetic guardの inferred typeと token sidecarで得た guard spanを Rust oracle の E0002型推論失敗本文へ投影した。`(+ 1 2)` guardは `expected Int, found Bool` と guard expressionの spanを保持し、higher-order、user-defined function、未解決 calleeは既存の未分類境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_match_guard_diagnostic_message` は Rust oracleの `:invariant の型推論に失敗しました: [E0002] 型の不一致: expected Int, found Bool (46..53)` に対し、selfhostが `:invariant は Bool 必須ですが、Unknown が推論されました` を返して失敗した（`27.64s`）。GREENは同じ fixtureで `1 passed`（`27.37s`）となり、既存 non-Bool match guard regression 10件も `10 passed`（`712.47s`）で code/span と dynamic/lambda境界を維持した。専用 Cargo targetは検証後に削除した。

これは direct arithmetic match guardの diagnostic message projectionに限定した verified sliceであり、compound/root `if` の型推論失敗本文、user-defined/higher-order functionの inferred type本文、full diagnostic parity、text/structured reportの全境界、今回の変更後の Mac Apple Silicon / Linux x86_64 current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。次の current-source dual-target gateでこの match message projectionの provenanceを確認する。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 direct branch `if` diagnostic message slice (2026-07-21)

legacy `:invariant` の root `if` について、non-Bool branchの direct inferred typeを Rust oracle の E0003型推論失敗本文へ投影した。`(if true (+ 1 2) false)` は `expected Int, found Bool` と root `if` spanを保持し、condition non-Bool、dynamic branch、nested control expressionは今回の scope外に残している。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_if_diagnostic_message` は Rust oracleの `:invariant の型推論に失敗しました: [E0003] 型の不一致: expected Int, found Bool (26..49)` に対し、selfhostが `:invariant は Bool 必須ですが、Unknown が推論されました` を返して失敗した（`28.63s`）。GREENは同じ fixtureで `1 passed`（`27.50s`）、match/if message projection regression 3件は `3 passed`（`84.33s`）となり、既存 direct literal messageの境界も維持した。専用 Cargo targetは検証後に削除した。

これは direct branch `if` の E0003 message projectionに限定した verified sliceであり、condition non-Bool、nested/compound control expression、user-defined/higher-order functionの inferred type本文、full diagnostic parity、今回の変更後の Mac Apple Silicon / Linux x86_64 current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。次の current-source dual-target gateで match/if message projectionの provenanceを確認する。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after match/if diagnostic message projection (2026-07-21)

`e75cafeeae4bdb9ae56fa1e19e5517d716c6f5b4` を current sourceとして、direct arithmetic match guard と direct branch `if` の diagnostic message projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、match/if projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/e75cafee-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=e75cafeeae4bdb9ae56fa1e19e5517d716c6f5b4`、`selfhost_fixed_point=true`、program SHA-256 `a10b743b0f9dfe4b61e1bfbaf6c83a2d93c7934a90a8f3e56b1d5796ddfaf394` を記録する。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stderrは `0` bytesだった。focused release gateは `586.48s`で passした。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/e75cafee-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 stderrは双方 `0` bytesである。VM free spaceは `5,438,410,752` bytes、必要量は `4,294,967,296` bytesで、検証後に VMを停止し、一時 workdir、lock、host Cargo targetを削除した。artifact sizeは約 `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passした。

これは match guard / branch `if` diagnostic message projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、condition non-Bool、nested/compound control expression、user-defined/higher-order functionの inferred type本文、full diagnostic parity、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation diagnostic message projection slice (2026-07-21)

legacy `:invariant` の computation式について、最終 `return` stepが Bool でない場合の selfhost TestRunner message projectionを追加した。既存の computation evaluator は `let!` bindingを環境へ渡していたが、static preflightの diagnostic messageは computation node全体を未分類として `Unknown` と報告していた。最終 stepの direct arithmetic/literal shapeだけを既存の型文字列 projectionへ委譲し、nested controlやdynamic shapeは `Unknown` のままにした。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_diagnostic_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message と diagnostic spanを一致させて `1 passed`（29.24s）。match/if/computation message regressionは `4 passed`（119.98s）、既存 valid computation evaluatorは `2 passed`（57.67s）だった。

これは computation の最終 direct stepに限定した verified sliceであり、nested/compound control expression、dynamic/user-defined/higher-order functionの inferred type本文、computation全体の diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after computation diagnostic message projection (2026-07-21)

`d30ef1b9cd3d542475f30fbcfa9d1bc6ff812f36` を current sourceとして、computation diagnostic message projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、computation projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/d30ef1b9-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=d30ef1b9cd3d542475f30fbcfa9d1bc6ff812f36`、`selfhost_fixed_point=true`、program SHA-256 `890965190841154e76670bfd2ba882db23ea014dc57df271c6f409590245f68b` を記録する。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stderrは `0` bytesだった。focused release gateは `606.30s`で passした。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/d30ef1b9-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 stderrは双方 `0` bytesである。VM free spaceは `5,432,619,008` bytes、必要量は `4,294,967,296` bytesで、検証後に VMを停止し、一時 workdir、lock、host Cargo targetを削除した。artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passした。

これは computation message projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、nested/dynamic computationの診断本文、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct user-defined diagnostic message projection slice (2026-07-21)

computation式の最終 `return` stepが引数なし user-defined functionを呼ぶ場合について、function bodyが既存の direct arithmetic/literal shapeなら、その一段だけ inferred type本文へ投影するようにした。引数付き function、higher-order callee、nested/dynamic bodyは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message/spanを一致させて `1 passed`（27.77s）。match/if/direct/user-defined computation message regressionは `5 passed`（141.64s）、valid computation evaluatorは `2 passed`（55.42s）だった。

これは引数なし user-defined functionの direct bodyを一段だけ投影する verified sliceであり、引数付き・higher-order・closure capture・nested controlの inferred type本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct user-defined computation projection (2026-07-21)

`50ea2fc82d8b4c5fa959dbfa27ff52b1fc5f7efd` を current sourceとして、direct user-defined computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、user-defined projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/50ea2fc8-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=50ea2fc82d8b4c5fa959dbfa27ff52b1fc5f7efd`、`selfhost_fixed_point=true`、program SHA-256 `e162a18beb9c4e8235a301a94f13e914f35a96cca2dd6de2c7782849405a80cc` を記録する。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stderrは `0` bytesだった。focused release gateは `599.93s`で passした。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/50ea2fc8-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 stderrは双方 `0` bytesである。VM free spaceは `5,426,855,936` bytes、必要量は `4,294,967,296` bytesで、検証後に VMを停止し、一時 workdir、lock、host Cargo targetを削除した。artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passした。

これは direct user-defined computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、引数付き・higher-order・closure capture・nested computationの診断本文、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation one-argument user-defined diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、一引数の user-defined function `increment [x] (+ x 1)` を呼ぶ場合について、function bodyが既存の direct arithmetic/literal shapeなら inferred type本文へ一段だけ投影するようにした。`let! delta 1` で得た値を引数として渡す形を対象にし、二引数以上、identity/argument variable body、closure capture、higher-order callee、nested/dynamic bodyは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_one_arg_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message/spanを一致させて `1 passed`（30.85s）。既存の non-Bool message projection regressionは `6 passed`（212.96s）、valid computation evaluatorは `2 passed`（58.03s）だった。

これは一引数 user-defined functionの direct bodyを computation の最終 stepから一段だけ投影する verified sliceであり、二引数以上、identity/argument variable、closure capture、higher-order、nested/dynamic computationの inferred type本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after one-argument user-defined computation projection (2026-07-22)

`8b33947073ac6f3e57c65985b0496f9e52fc113c` を current sourceとして、一引数 user-defined computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、one-argument projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/8b339470-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=8b33947073ac6f3e57c65985b0496f9e52fc113c`、`selfhost_fixed_point=true`、program SHA-256 `f59fcb2113232ae444c1a915da110359784b271bfeb969602e90db7e1b582121`を記録する。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stderrは `0` bytesだった。focused release gateは `619.92s`で passした。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/8b339470-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。VM free spaceは `5,421,346,816` bytes、必要量は `4,294,967,296` bytesで、検証後に VMを停止し、一時 workdir、replay lock、host Cargo targetを削除した。artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passした。

これは one-argument computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、二引数以上、identity/argument variable、closure capture、higher-order、nested/dynamic computationの診断本文、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation one-argument identity diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、一引数の identity function `identity [x] x` を direct `Int` literalに適用する場合について、body variableをcall argumentのstatic type textへ投影するようにした。既存の parameter scope解決と program-aware projectionを再利用し、引数がlet-bound/dynamicで型本文を確定できない場合、二引数以上、closure capture、higher-order callee、nested bodyは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_one_arg_identity_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（27.82s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（39.78s）。computation diagnostic projection regressionは `4 passed`（47.35s）、valid computation evaluatorは `2 passed`（38.36s）だった。

これは一引数 identity bodyを direct literal argumentから一段だけ投影する verified sliceであり、let-bound/dynamic argument、二引数以上、closure capture、higher-order、nested/dynamic computationの inferred type本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after one-argument identity computation projection (2026-07-22)

`98d0160994bd4d887824da2ca69f779b49942028` を current sourceとして、一引数 identity computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Linuxは通常の native-linux-x86 selfregen scriptを完走し、Macは共有 worktreeの無関係な dirty filesを保全するため clean-worktreeを要求する wrapperを実行せず、同じ ignored E2Eとrelease後段検証を専用 artifact/Cargo targetで実施した。両 targetの生成物は現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/98d01609-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=98d0160994bd4d887824da2ca69f779b49942028`、`selfhost_fixed_point=true`、program SHA-256 `ec2942a0f6a3dad94e16065a9d4cadc484dc7c22ff27f9677910d9fba73db646`を記録する。underlying ignored E2Eは `1 passed`（566.29s）。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。4 filesの native input bundleを作成し、artifact sizeは `3.7M`、専用 Cargo target（実行中最大 `1.7G`）は削除した。clean-worktree wrapper自体は既存の無関係な dirty filesにより拒否されたため、その状態をrelease producerの完全なwrapper passとは扱わない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/98d01609-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。actual stage1は `514.95s`、VM free spaceは `5,407,244,288` bytes、必要量は `4,294,967,296` bytesで、検証後にVMを停止し、一時 workdir、replay lock、host Cargo targetを削除した。artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passした。

これは one-argument identity computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、let-bound/dynamic argument、二引数以上、closure capture、higher-order、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation two-argument direct-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、二引数の user-defined function `add [x y] (+ x y)` を direct `Int` literalsに適用する場合について、direct arithmetic bodyの inferred type本文を `Int` へ投影するようにした。一引数までに限定していた既存の arity guardを二引数まで広げ、dynamic/higher-order/closure/nested bodyは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_two_arg_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（27.69s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（44.09s）。computation diagnostic projection regressionは `5 passed`（49.55s）、valid computation evaluatorは `2 passed`（37.93s）だった。

これは二引数 user-defined functionの direct arithmetic bodyを computation の最終 stepから一段だけ投影する verified sliceであり、let-bound/dynamic argument、二引数の identity/variable body、三引数以上、closure capture、higher-order、nested/dynamic computationの inferred type本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after two-argument direct-body computation projection (2026-07-22)

`69cd1daa6f2f165d1a4f2e74d1d4db9fcd68fa81` を current sourceとして、二引数 direct-body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Linuxは通常の native-linux-x86 selfregen scriptを完走し、Macは共有 worktreeの無関係な dirty filesを保全するため clean-worktreeを要求する wrapperを実行せず、同じ ignored E2Eとrelease後段検証を専用 artifact/Cargo targetで実施した。両 targetの生成物は現行 commit provenanceを持つ。

Mac Evidence: `crates/lsharp-wasm/ci-artifacts/native-release/aarch64-apple-darwin/69cd1daa-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=69cd1daa6f2f165d1a4f2e74d1d4db9fcd68fa81`、`selfhost_fixed_point=true`、program SHA-256 `06f49481ef6b6038cab2dc8ed2379d1012ffcf9ba2652d9a1c7939d32d86421d`を記録する。underlying ignored E2Eは `1 passed`（593.44s）。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。4 filesの native input bundleを作成し、artifact sizeは `3.7M`、専用 Cargo target（実行中最大 `1.7G`）は削除した。clean-worktree wrapper自体は既存の無関係な dirty filesにより拒否されたため、その状態をrelease producerの完全なwrapper passとは扱わない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/69cd1daa-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。actual stage1は `494.76s`、VM free spaceは `5,402,439,680` bytes、必要量は `4,294,967,296` bytesで、検証後にVMを停止し、一時 workdir、replay lock、host Cargo targetを削除した。artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passした。

これは two-argument direct-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、let-bound/dynamic argument、二引数 identity/variable body、三引数以上、closure capture、higher-order、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound identity diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、一引数の identity function `identity [x] x` を `let! delta 1` で得た値に適用する場合について、let-bound valueの既知の static type textを computation diagnostic messageへ引き継ぐようにした。`value-string` を通じた `let!` bindingの一段解決を追加し、既存の direct literal / arithmetic、user-defined function、one-argument identity projectionを維持する。未解決 binding、dynamic value、二引数以上の identity/variable body、closure capture、higher-order callee、nested computationは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_identity_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（27.74s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（28.03s）。computation diagnostic projection regressionは `6 passed`（72.53s）、valid computation evaluatorは `2 passed`（35.64s）だった。

これは computation内の let-bound one-argument identityに対する既知の static type textを一段だけ投影する verified sliceであり、let-bound/dynamic/unresolved bindingの全 parity、二引数以上の identity/variable body、closure capture、higher-order、nested/dynamic computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound identity computation projection (2026-07-22)

`518783b399faf4b46dfbed544b33572db77221d6` を current sourceとして、let-bound identity computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Linuxは通常の native-linux-x86 selfregen scriptを完走し、Macは共有 worktreeの無関係な dirty filesを保全するため clean-worktreeを要求する wrapperを実行せず、同じ ignored E2Eとrelease後段検証を専用 artifact/Cargo targetで実施した。両 targetの生成物は現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/518783b3-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=518783b399faf4b46dfbed544b33572db77221d6`、`selfhost_fixed_point=true`、program SHA-256 `7f248798aa5a1a6f283dd1822c8b9d59564f8d36cc0145b2ac098777c21dfca5`を記録する。underlying ignored E2Eは `1 passed`（586.16s）。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。4 filesの native input bundleを作成し、artifact sizeは `3.7M`、専用 Cargo target（実行中最大約 `1.6G`）は削除した。clean-worktree wrapper自体は既存の無関係な dirty filesにより拒否されたため、その状態をrelease producerの完全なwrapper passとは扱わない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/518783b3-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。actual stage1は `532.13s`、VM free spaceは `5,397,004,288` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持し、ホスト `/` の空きは `141GiB` だった。

これは let-bound identity computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、let-bound/dynamic argumentの全 parity、二引数以上の identity/variable body、closure capture、higher-order、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound user-defined if-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を受ける user-defined function `choose-int [x] (if true (+ x 1) 0)` を呼ぶ場合について、function bodyの `if` branchから inferred type textを一段だけ投影するようにした。then/else branchの既存 static kind判定と env-aware type projectionを再利用し、既存の direct literal / arithmetic、user-defined function、identity、let-bound identity projectionを維持する。conditionが未分類、branchが未解決、closure capture、higher-order callee、nested/dynamic computationは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_if_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（28.06s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（28.07s）。computation diagnostic projection regressionは `7 passed`（52.21s）、valid computation evaluatorは `2 passed`（32.65s）だった。

これは let-bound user-defined functionの `if` bodyを computation diagnostic本文へ投影する verified sliceであり、condition/branchの全 type inference、nested control、closure capture、higher-order、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound user-defined if-body computation projection (2026-07-22)

`b37423c16e403b02826317f0e3eac973a64370ec` を current sourceとして、let-bound user-defined `if` body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Linuxは通常の native-linux-x86 selfregen scriptを完走し、Macは共有 worktreeの無関係な dirty filesを保全するため clean-worktreeを要求する wrapperを実行せず、同じ ignored E2Eとrelease後段検証を専用 artifact/Cargo targetで実施した。両 targetの生成物は現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/b37423c1-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=b37423c16e403b02826317f0e3eac973a64370ec`、`selfhost_fixed_point=true`、program SHA-256 `d92b2c773dc3b235119f59646a9779cd52155d1e06c35e0bd9bdc0d07b6cf2f6`を記録する。underlying ignored E2Eは `1 passed`（585.75s）。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。manifest、program、smoke stdout/stderrの4 filesを artifactへ保存し、artifact sizeは `3.3M`、専用 Cargo target（実行中最大 `1.6G`）は削除した。clean-worktree wrapper自体は既存の無関係な dirty filesにより拒否されたため、その状態をrelease producerの完全なwrapper passとは扱わない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/b37423c1-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `523.65s`、VM free spaceは `5,391,491,072` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound user-defined if-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、condition/branchの全 parity、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound user-defined let-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を受ける user-defined function `choose-int [x] (let [next (+ x 1)] next)` を呼ぶ場合について、function bodyの local `let` initializerの既知の type textを envへ bindし、bodyを一段だけ再評価するようにした。既存の direct literal / arithmetic、user-defined function、identity、let-bound identity/if projectionを維持する。未解決 initializer、dynamic binding、closure capture、higher-order callee、nested/dynamic computationは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_let_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（28.09s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（27.96s）。computation diagnostic projection regressionは `8 passed`（55.84s）、valid computation evaluatorは `2 passed`（32.80s）だった。

これは let-bound user-defined functionの local `let` bodyを computation diagnostic本文へ投影する verified sliceであり、initializerの全型、複数束縛、nested local binding、condition/branchの全 type inference、closure capture、higher-order、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound user-defined let-body computation projection (2026-07-22)

`3f5133d0214f6900a9909d9ed9ddd7ff2322ada9` を current sourceとして、let-bound user-defined local `let` body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Linuxは通常の native-linux-x86 selfregen scriptを完走し、Macは共有 worktreeの無関係な dirty filesを保全するため clean-worktreeを要求する wrapperを実行せず、同じ ignored E2Eとrelease後段検証を専用 artifact/Cargo targetで実施した。両 targetの生成物は現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/3f5133d0-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=3f5133d0214f6900a9909d9ed9ddd7ff2322ada9`、`selfhost_fixed_point=true`、program SHA-256 `761997efb2de368049820669684de81c745b3b6ef665c425723c6cb604325ceb`を記録する。underlying ignored E2Eは `1 passed`（592.90s）。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。manifest、program、smoke stdout/stderrの4 filesを artifactへ保存し、artifact sizeは `3.3M`、専用 Cargo target（実行中最大 `1.6G`）は削除した。clean-worktree wrapper自体は既存の無関係な dirty filesにより拒否されたため、その状態をrelease producerの完全なwrapper passとは扱わない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/3f5133d0-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `516.13s`、VM free spaceは `5,385,973,760` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound user-defined let-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、initializerの全型、複数束縛、nested local binding、condition/branchの全 parity、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound user-defined match-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を受ける user-defined function `choose-int [x] (match true [_ (+ x 1)] [_ 0])` を呼ぶ場合について、function bodyの match armから inferred type textを一段だけ投影するようにした。guard付き armと通常 armを順に走査し、既知の non-Bool type textを最初に採用する。既存の direct literal / arithmetic、user-defined function、identity、let-bound identity/if/let projectionを維持し、constructor/record patternの環境束縛、higher-order callee、nested/dynamic computationは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_match_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（28.65s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（28.60s）。match projectionを含む non-Bool computation diagnostic regressionは `9 passed`（58.71s）、valid computation evaluatorは `2 passed`（36.23s）だった。

これは let-bound user-defined functionの match bodyを computation diagnostic本文へ一段だけ投影する verified sliceであり、guard/patternの全型推論、constructor/record/GADT pattern環境、nested match、closure capture、higher-order、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound user-defined match-body computation projection (2026-07-22)

`0bdbebf83b16e39bea5dd5674c5216130f59174a9` を current sourceとして、let-bound user-defined match body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。Linuxは通常の native-linux-x86 selfregen scriptを完走し、Macは共有 worktreeの無関係な dirty filesを保全するため clean-worktreeを要求する wrapperを実行せず、同じ ignored E2Eとrelease後段検証を専用 artifact/Cargo targetで実施した。両 targetの生成物は現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/0bdbebf8-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=0bdbebf83b16e39bea5dd5674c5216130f59174a`、`selfhost_fixed_point=true`、program SHA-256 `2c36abee170cb76e95a8d1c3deba8749392e27f391195021ec8cd32dca5a9887` を記録する。underlying ignored E2Eは `1 passed`（592.08s）。`program.native` は Mach-O arm64、サイズ `3,485,056` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。manifest、program、smoke stdout/stderrの4 filesを artifactへ保存し、artifact sizeは `3.3M`、専用 Cargo target（実行中最大 `1.6G`）は削除した。clean-worktree wrapper自体は既存の無関係な dirty filesにより拒否されたため、その状態をrelease producerの完全なwrapper passとは扱わない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/0bdbebf8-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。actual stage1は `511.03s`、VM free spaceは `5,380,521,984` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound user-defined match-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、guard/patternの全 parity、constructor/record/GADT pattern環境、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound user-defined do-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を受ける user-defined function `choose-int [x] (do 0 (+ x 1))` を呼ぶ場合について、function bodyの `do` の最終 expressionから inferred type textを一段だけ投影するようにした。既存の direct literal / arithmetic、user-defined function、identity、let-bound identity/if/let/match projectionを維持する。複数 expressionの途中値、closure capture、higher-order callee、nested/dynamic computationは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_do_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（51.59s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（36.27s）。computation diagnostic projection regressionは `10 passed`（78.34s）、valid computation evaluatorは `2 passed`（39.58s）だった。

これは let-bound user-defined functionの `do` bodyを computation diagnostic本文へ一段だけ投影する verified sliceであり、複数 expressionの全型、nested do、closure capture、higher-order、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound user-defined do-body computation projection (2026-07-22)

`ed66349aedb3a1921f63408ba5fca0f10d5f0cad` を current sourceとして、let-bound user-defined `do` body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、do projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/ed66349a-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=ed66349aedb3a1921f63408ba5fca0f10d5f0cad`、`selfhost_fixed_point=true`、program SHA-256 `61aa9eafafb319a2774d33943a18b2b73a5cacc97724a4fe9c85bad028d21f58`を記録する。underlying ignored E2Eは `1 passed`（637.39s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Cargo targetは削除した。共有 worktreeの無関係な dirty filesを保全するため、clean-worktree wrapper自体は実行していない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/ed66349a-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffecf0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `539.52s`、VM free spaceは `5,375,107,072` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound user-defined do-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、複数 expressionの全 parity、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound inline lambda-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を `((fn [x] (+ x 1)) delta)` の inline lambdaへ渡す場合について、lambda bodyの direct arithmetic shapeから inferred type textを一段だけ投影するようにした。lambda calleeの既存 AST 引数位置と 0〜2引数の arity境界を再利用し、body variableも既知の argument type textへ接続する。nested lambda、closure capture、higher-order/dynamic callee、未分類 bodyは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_lambda_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（31.03s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（29.00s）。computation diagnostic projection regressionは `11 passed`（123.93s）、valid computation evaluatorは `2 passed`（35.52s）だった。

これは let-bound inline lambdaの direct bodyを computation diagnostic本文へ一段だけ投影する verified sliceであり、lambda bodyの全型推論、複数引数の identity/variable body、nested lambda、closure capture、higher-order、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound inline lambda-body computation projection (2026-07-22)

`99c0534643b7660aadf7e4fb1de21c1f4646441c` を current sourceとして、let-bound inline lambda-body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、inline lambda projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/99c05346-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=99c0534643b7660aadf7e4fb1de21c1f4646441c`、`selfhost_fixed_point=true`、program SHA-256 `0145677d810b9b9a9aeb2268cf4d8d339549913584a8ed49bb329ba558b6a3aa`を記録する。underlying ignored E2Eは `1 passed`（620.57s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Cargo targetは削除した。共有 worktreeの無関係な dirty filesを保全するため、clean-worktree wrapper自体は実行していない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/99c05346-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `512.98s`、VM free spaceは `5,333,483,520` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound inline lambda-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、lambda bodyの全型推論、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound inline lambda control-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を `((fn [x] (if true x x)) delta)` の inline lambdaへ渡す場合について、lambda parameterを既知の static type textとして局所 envへ束縛し、control bodyの両 branchから inferred type textを一段だけ投影するようにした。両 branchが同じ既知型の場合だけ if projectionを許可し、未分類の arity、nested lambda、closure capture、higher-order/dynamic calleeは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_lambda_control_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（40.10s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（28.54s）。computation diagnostic projection regressionは `12 passed`（92.24s）、valid computation evaluatorは `2 passed`（38.78s）だった。

これは let-bound inline lambdaの parameter env と同型 control branchを一段だけ投影する verified sliceであり、lambda bodyの全型推論、異なる branch型の推論、複数引数の identity/variable body、nested lambda、closure capture、higher-order、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、両対応 targetの current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound inline lambda control-body computation projection (2026-07-22)

`2ad28b46883e06d7720556b3018033fb056f7b77` を current sourceとして、let-bound inline lambda control-body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、lambda parameter env/control projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/2ad28b46-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=2ad28b46883e06d7720556b3018033fb056f7b77`、`selfhost_fixed_point=true`、program SHA-256 `f3af04f5c708665c1894d0fd22f5fa8fcaf5c85161c6012730b24c5a3eca8fb0`を記録する。underlying ignored E2Eは `1 passed`（584.99s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Cargo targetは削除した。共有 worktreeの無関係な dirty filesを保全するため、clean-worktree wrapper自体は実行していない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/2ad28b46-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `586.64s`、VM free spaceは `5,327,966,208` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound inline lambda control-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、lambda bodyの全型推論、異なる branch型、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound user-defined control-body diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を user-defined function `choose-int [x] (if true x x)` へ渡す場合について、function parameterを既知の static type textとして局所 envへ束縛し、control bodyの両 branchから inferred type textを一段だけ投影するようにした。対応 arityは 0〜2 に限定し、未分類の arity、nested function、closure capture、higher-order/dynamic calleeは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_user_function_control_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（30.50s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（29.00s）。non-Bool computation projection regressionは `13 passed`（100.46s）、valid computation evaluatorは `2 passed`（34.60s）だった。

これは let-bound user-defined functionの parameter env と同型 control branchを一段だけ投影する verified sliceであり、function bodyの全型推論、異なる branch型の推論、三引数以上、constructor/record pattern環境、closure capture、higher-order、nested/dynamic computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound user-defined control-body computation projection (2026-07-22)

`4d56f9c5706668570a6395fea18b393797056093` を current sourceとして、let-bound user-defined control-body computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、user-function parameter environment projectionを含む現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/4d56f9c5-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=4d56f9c5706668570a6395fea18b393797056093`、`selfhost_fixed_point=true`、program SHA-256 `5b6c58755812584afdecf329ea9c96d044fcfb8a874143bf3814073878f13b60`を記録する。underlying ignored E2Eは `1 passed`（587.38s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Cargo targetは削除した。共有 worktreeの無関係な dirty filesを保全するため、clean-worktree wrapper自体は実行していない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/4d56f9c5-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `529.96s`、VM free spaceは `5,322,387,456` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound user-defined control-body computation projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、function bodyの全型推論、異なる branch型、三引数以上、constructor/record pattern環境、closure capture、higher-order、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound higher-order user-defined diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、top-level function `increment [x] (+ x 1)` を higher-order parameter `f` へ渡し、user-defined function `apply-one [f x] (f x)` の bodyから `Int` を返す場合について、static envへ top-level function hashを保持し、bound calleeを declarationへ再解決するようにした。通常の type text bindingと function-value bindingを分離し、未解決の inline lambda function value、dynamic callee、closure capture、nested/higher-order bodyは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_higher_order_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（39.09s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（37.56s）。non-Bool computation projection regressionは `14 passed`（126.03s）、valid computation evaluatorは `2 passed`（34.47s）だった。

これは top-level function argumentを一段だけ static envへ投影する verified sliceであり、inline lambda function value、closure capture、partial application、higher-order functionの再帰的な全 body推論、dynamic/unresolved calleeの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound higher-order user-defined projection (2026-07-22)

`e74c1bdab2aef4f87ea5c0a7e06ecebe03aaa52b` を current sourceとして、let-bound higher-order user-defined computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、function-value env/callee resolutionを含む現行 commit provenanceを持つ。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/e74c1bda-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=e74c1bdab2aef4f87ea5c0a7e06ecebe03aaa52b`、`selfhost_fixed_point=true`、program SHA-256 `eb67957239f5b44764493592312c849dbeb66a2a17756f6eba8e4c1dbcafebd7`を記録する。underlying ignored E2Eは `1 passed`（623.02s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Cargo targetは削除した。共有 worktreeの無関係な dirty filesを保全するため、clean-worktree wrapper自体は実行していない。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/e74c1bda-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stdoutは双方 `11,646,271` bytes、stderrは双方 `0` bytesである。actual stage1は `746.84s`、VM free spaceは `5,316,964,352` bytes、必要量は `4,294,967,296` bytesで、artifact sizeは `100M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、一時 workdir、replay lock、host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound higher-order user-defined projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、inline lambda function value、closure capture、partial application、higher-order functionの再帰的な全 body推論、dynamic/unresolved calleeの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound inline lambda higher-order diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値と inline lambda `(fn [x] (+ x 1))` を higher-order user-defined function `apply-one [f x] (f x)` へ渡す場合について、lambda ASTを static envへ保持し、bound calleeの呼び出し時に lambda parameter envを作って bodyの inferred type textを一段だけ投影するようにした。lambda arityは 0〜2 に限定し、inline lambda以外の top-level function value、既知の type text value、未解決の calleeは既存の境界を維持する。三引数以上、closure capture、partial application、nested/dynamic bodyは `Unknown` のままとする。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_inline_lambda_higher_order_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（38.50s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（32.07s）。non-Bool computation projection regressionは `15 passed`（125.53s）、valid computation evaluatorは `2 passed`（36.26s）だった。実装中に `and` の arity契約違反を検出し、二引数の nested formへ修正してから GREEN を再確認した。

これは let-bound inline lambdaを higher-order user-defined callへ渡す verified sliceであり、lambdaの全型推論、三引数以上、closure capture、partial application、nested/dynamic computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound inline lambda higher-order projection (2026-07-22)

`5c6b372da28dbb7b641c8ace8744e7ec7347a76e` を current sourceとして、let-bound inline lambda higher-order computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。両 targetとも変更前 artifactではなく、inline lambda AST env/call projectionを含む現行 commit provenanceを持つ。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/5c6b372d-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=5c6b372da28dbb7b641c8ace8744e7ec7347a76e`、`selfhost_fixed_point=true`、program SHA-256 `cb3743ef7843519b18f9fc23c3c18fc6c22155c6a00d63a4ace68410a01aecc2`を記録する。underlying ignored E2Eは `1 passed`（628.96s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Cargo targetは削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/5c6b372d-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stage2/stage3 stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1、stage2-debug、stage3-debug manifestは同じ source commitを持ち、stage1 code length `4,203,487`、data length `1,523`、entrypoint offset `4,201,104`、stage2/stage3 code length `10,832,651`、data length `1,523`、entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`が一致した。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。actual stage1は `560.38s`、VM free spaceは `5,311,479,808` bytes、必要量は `4,294,967,296` bytesだった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持し、artifact sizeは `100M`だった。

これは let-bound inline lambda higher-order projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、inline lambdaの全型推論、三引数以上、closure capture、partial application、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound factory closure capture higher-order diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を factory function `make-constant [delta] (fn [x] (if true delta delta))` へ渡し、factory が返した closure を `apply-one [f x] (f x)` へ渡す場合について、lambda AST と factory 呼び出し時の static environment を closure value として保持するようにした。呼び出し時は argument environment と captured environment を分離し、捕捉した `delta` を lambda body の inferred type textへ一段だけ投影する。closure arityは 0〜2 に限定し、partial application、三引数以上、nested closure、dynamic/unresolved calleeは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_closure_capture_higher_order_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（29.23s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（29.64s）。既存を含む non-Bool computation projection regressionは `16 passed`（124.40s）、valid computation evaluatorは `2 passed`（34.83s）だった。実装中に direct lambda control regressionを検出したが、argument/captured environment helperの引数順を修正後、lambda control単体（38.30s）を含む回帰一式を再度 `16/16` で確認した。

これは let-bound factory closure captureを higher-order user-defined callへ渡す verified sliceであり、closure内の全型推論、partial application、三引数以上、nested closure、異なる branch型、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound factory closure capture projection (2026-07-22)

`a0b733e9db7d6552a0c84cc70918056c5da5eae5` を current sourceとして、let-bound factory closure capture higher-order computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、専用 worktree・Cargo target・artifact pathで検証し、clean-worktree wrapperは実行していない。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/a0b733e9-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=a0b733e9db7d6552a0c84cc70918056c5da5eae5`、`selfhost_fixed_point=true`、program SHA-256 `3b368e2b3a52449baf141dac1947c00f2875d323c589086e4d74054a24097d11`を記録する。underlying ignored E2Eは `1 passed`（589.63s）。`program.native` は Mach-O arm64、サイズ `3,501,568` bytes、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.3M`、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/a0b733e9-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stage2/stage3 stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。actual stage1 manifestは source commit `a0b733e9db7d6552a0c84cc70918056c5da5eae5`、code `4,203,487`、data `1,523`、entrypoint offset `4,201,104`、function table length `3,237`、main function index `3,246`を持ち、stage2-debug / stage3-debug manifestは同じ source commit、code `10,832,651`、data `1,523`、entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`で一致した。actual stage1 bundle生成は `524.37s`、VM free spaceは `5,306,048,512` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、stage2/stage3 stderrは `0` bytes、検証後に VM、temporary workdir、replay lock、host Cargo targetを停止・削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound factory closure capture projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure内の全型推論、partial application、三引数以上、nested/dynamic computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation let-bound nested closure diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を受けて inline lambda `(fn [x] (fn [y] (+ x y)))` を二段適用する場合について、inner lambda callの結果を lambda AST と captured environment の closure valueとして保持し、outer callの引数 environmentと組み合わせて最終 bodyの inferred type textを一段だけ投影するようにした。calleeが nested apply の場合だけ closure valueを再帰解決し、既存の factory closure capture、bound closure、direct lambdaの経路は維持する。closure arityは 0〜2 に限定し、partial application、三引数以上、dynamic/unresolved calleeは `Unknown` の境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_bound_nested_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message/spanを一致させて `1 passed`。factory closure captureを含む既存の computation projection regressionは `12 passed`、valid computation evaluatorは `2 passed`、factory closure capture focused testも `1 passed` だった。

これは let-bound nested closureの最終 non-Bool diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、partial application、三引数以上、異なる branch型、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after let-bound nested closure projection (2026-07-22)

`65dcdeae53a553c8b9062287b44071e474e048e3` を current sourceとして、let-bound nested closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/65dcdeae-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=65dcdeae53a553c8b9062287b44071e474e048e3`、`selfhost_fixed_point=true`、program SHA-256 `294c4f515b35cea058a901fd9dc3625cea13073892a9e3fd02c74537290391fa`を記録する。underlying ignored E2Eは `1 passed`（728.63s）。`program.native` は Mach-O arm64、`--version` は `lsharp 0.1.0`、smoke stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.4M`、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/65dcdeae-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage1 manifestは同じ source commit、code `4,203,487`、data `1,523`、entrypoint offset `4,201,104`、function table length `3,237`、main function index `3,246`を持ち、stage2-debug / stage3-debug manifestも同じ source commit、code `10,832,651`、data `1,523`、entrypoint offset `10,828,220`、function table length `3,237`、main function index `3,246`で一致した。stage2/stage3 stderrは双方 `0` bytes、final `summary.json` は expected/actual exit code `42` の一致を記録する。actual stage1 bundle生成は `604.22s`、VM free spaceは `5,300,523,008` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probeと actual stage1 -> stage2 -> stage3 replayも passし、検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは let-bound nested closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure内の全型推論、partial application、三引数以上、異なる branch型、dynamic/unresolved computationの診断本文、Mac release wrapperのclean-worktree pass、EmbeddedCliの native release artifact、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct three-argument lambda diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を `(fn [x y z] (+ x (+ y z)))` の direct three-argument lambdaへ一度に渡す場合について、既存の lambda parameter environment loopを3引数まで許可し、Rust oracleの non-Bool diagnostic type textを `Int` として投影するようにした。部分適用ではない direct callだけを対象とし、nested closure、closure capture、higher-order callee、partial application、4引数以上、user-defined functionの3引数以上は既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_three_arg_lambda_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message/spanを一致させて `1 passed`。nested closure、factory closure capture higher-order、bound lambdaの既存 focused regressionも各 `1 passed` である。

これは direct three-argument lambdaの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、lambda bodyの全型推論、closure/higher-order経路の3引数以上、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct three-argument lambda projection (2026-07-22)

`77072d7cc5dc765e940f1815a555250a23a58c52` を current sourceとして、direct three-argument lambda computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、専用 worktree・Cargo target・artifact pathで検証し、Mac release wrapperではなく underlying ignored E2E producerを実行した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/77072d7c-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=77072d7cc5dc765e940f1815a555250a23a58c52`、`selfhost_fixed_point=true`、program SHA-256 `3644b7e7844ddad251badf202df7fa3e648e2d8a039f48e9b47e8caab29e8a5f`を記録する。underlying ignored E2Eは `1 passed`（651.49s）。`program.native` は Mach-O arm64、サイズ `3.4M`、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致し、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/77072d7c-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `734.31s`、VM free spaceは `5,294,845,952` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct three-argument lambda projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの3引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct three-argument user-function diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct user-defined function `sum3 [x y z] (+ x (+ y z))` へ一度に3つ渡す場合について、user-function bodyの static environment projectionを3引数まで許可し、Rust oracleの non-Bool diagnostic type textを `Int` として投影するようにした。部分適用ではない direct callだけを対象とし、closure value、higher-order callee、partial application、4引数以上、未解決 calleeは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_three_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message/spanを一致させて `1 passed`。direct 3-arg lambda、nested closure、two-arg user functionの既存 focused regressionも各 `1 passed` である。

これは direct three-argument user-defined functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、closure/higher-order経路の3引数以上、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct three-argument user-function projection (2026-07-22)

`e7e127ef0f4747fe04cd1177edc48f910d77d9bf` を current sourceとして、direct three-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、専用 worktree・Cargo target・artifact pathで検証し、Mac release wrapperではなく underlying ignored E2E producerを実行した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/e7e127ef-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=e7e127ef0f4747fe04cd1177edc48f910d77d9bf`、`selfhost_fixed_point=true`、program SHA-256 `f825d87b3645475502e6bb43b19bc7a726ecf578a057e4125958587f60de2ecf`を記録する。underlying ignored E2Eは `1 passed`（615.64s）。`program.native` は Mach-O arm64、サイズ `3.4M`、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致し、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/e7e127ef-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `528.11s`、VM free spaceは `5,289,250,816` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct three-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの4引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct three-argument lambda-returning closure diagnostic message projection slice (2026-07-22)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct three-argument lambda `(fn [x y z] (fn [w] (+ x w)))` へ一度に渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`lambda-call-closure` の parameter environment projectionを3引数まで許可した。partial applicationではなく外側lambdaへの3引数 direct callを対象とし、closure bodyの捕捉値と最終引数を使った Rust oracleの non-Bool diagnostic type textを `Int` として投影する。4引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_three_arg_lambda_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した。GREENは同じ fixtureで message/spanを一致させて `1 passed`。direct three-argument lambda、direct three-argument user-function、nested closure、bound closureの focused regressionも各 `1 passed` である。

これは direct three-argument lambdaがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの4引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct three-argument lambda-returning closure projection (2026-07-22)

`6c14b10e3d01b4768c427796ad360145dce7d8c2` を current sourceとして、direct three-argument lambda-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/6c14b10e-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=6c14b10e3d01b4768c427796ad360145dce7d8c2`、`selfhost_fixed_point=true`、program SHA-256 `5db272930d4187c77bde5e95a5aa3bbdaf4b4449546b073ce64d996be681f0db`を記録する。underlying ignored E2Eは `1 passed`（850.22s）。`program.native` は Mach-O arm64、サイズ `3.4M`、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/6c14b10e-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `549.87s`、VM free spaceは `5,283,753,984` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct three-argument lambda-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの4引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct four-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct four-argument user-defined function `make-constant4 [x y z q] (fn [w] (+ x (+ y (+ z (+ q w)))))` へ渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`invariant-static-user-function-closure-value-with-env` の parameter environment projectionを4引数まで許可した。partial applicationではなく user functionへの4引数 direct callを対象とし、closure bodyの捕捉値と最終引数を使った Rust oracleの non-Bool diagnostic type textを `Int` として投影する。5引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_four_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（32.78s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`。three-argument user-function-returning closure、three-argument lambda-returning closure、nested closure、direct three-argument user functionの既存 focused regressionも各 `1 passed` である。

これは direct four-argument user functionがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの5引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct four-argument user-function-returning closure projection (2026-07-23)

`f1592ecb379686640d7d8f0b47869b5fa93411a6` を current sourceとして、direct four-argument user-function-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/f1592ecb-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=f1592ecb379686640d7d8f0b47869b5fa93411a6`、`selfhost_fixed_point=true`、program SHA-256 `2f2a808f39d216a4c7d099b4d39e75e5ef221e4e6fe9914c02a6e80e09da33de`を記録する。underlying ignored E2Eは `1 passed`（650.35s）。`program.native` は Mach-O arm64、サイズ `3.4M`、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/f1592ecb-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `558.68s`、VM free spaceは `7,677,603,840` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct four-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの5引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct three-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct three-argument user-defined function `make-constant3 [x y z] (fn [w] (+ x w))` へ渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`invariant-static-user-function-closure-value-with-env` の parameter environment projectionを3引数まで許可した。partial applicationではなく user functionへの3引数 direct callを対象とし、closure bodyの捕捉値と最終引数を使った Rust oracleの non-Bool diagnostic type textを `Int` として投影する。4引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_three_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（30.55s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`。closure capture、nested closure、three-argument lambda-returning closure、three-argument user functionの既存 focused regressionも各 `1 passed` である。

これは direct three-argument user functionがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの4引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct three-argument user-function-returning closure projection (2026-07-23)

`0109c0e56fa8501e8dbad16b0c2323ee02683748` を current sourceとして、direct three-argument user-function-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/0109c0e5-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=0109c0e56fa8501e8dbad16b0c2323ee02683748`、`selfhost_fixed_point=true`、program SHA-256 `4fc628611f38917e744280fc18948c96b73e187dd705614d62b3549c4946b858`を記録する。underlying ignored E2Eは `1 passed`（685.94s）。`program.native` は Mach-O arm64、サイズ `3.4M`、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/0109c0e5-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `531.78s`、VM free spaceは `7,683,039,232` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct three-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの4引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct five-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct five-argument user-defined function `make-constant5 [x y z q r] (fn [w] (+ x (+ y (+ z (+ q (+ r w))))))` へ渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`invariant-static-user-function-closure-value-with-env` の parameter environment projectionを5引数まで許可した。partial applicationではなく user functionへの5引数 direct callを対象とし、closure bodyの捕捉値と最終引数を使った Rust oracleの non-Bool diagnostic type textを `Int` として投影する。6引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_five_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（30.97s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`。既存の computation non-Bool prefix回帰23件（3/4/5引数 user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `23 passed`（203.87s）だった。

これは direct five-argument user functionがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの6引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct five-argument user-function-returning closure projection (2026-07-23)

`21377257ef317f0dac9aedd12e9798f726f8a8bb` を current sourceとして、direct five-argument user-function-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/21377257-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=21377257ef317f0dac9aedd12e9798f726f8a8bb`、`selfhost_fixed_point=true`、program SHA-256 `27ec71b784fe73e53149974174070f6515df1f6786f0b1f1f7f1849b282fafa7`を記録する。underlying ignored E2Eは `1 passed`（673.90s）。`program.native` は Mach-O arm64、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/21377257-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `519.70s`、VM free spaceは `7,672,152,064` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct five-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの6引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct six-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct six-argument user-defined function `make-constant6 [x y z q r s] (fn [w] (+ x (+ y (+ z (+ q (+ r (+ s w)))))))` へ渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`invariant-static-user-function-closure-value-with-env` の parameter environment projectionを6引数まで許可した。partial applicationではなく user functionへの6引数 direct callを対象とし、closure bodyの捕捉値と最終引数を使った Rust oracleの non-Bool diagnostic type textを `Int` として投影する。7引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_six_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（30.43s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（30.38s）。既存の computation non-Bool prefix回帰24件（3〜6引数 user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `24 passed`（173.89s）だった。

これは direct six-argument user functionがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの7引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct six-argument user-function-returning closure projection (2026-07-23)

`5d67aee491e1bbb994f805fc5874fadce0f0a744` を current sourceとして、direct six-argument user-function-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/5d67aee4-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=5d67aee491e1bbb994f805fc5874fadce0f0a744`、`selfhost_fixed_point=true`、program SHA-256 `93acac77f52c225f68feba7a14c9a811f7442ca1944b133842d483c566aa8d07`を記録する。underlying ignored E2Eは `1 passed`（609.67s）。`program.native` は Mach-O arm64、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/5d67aee4-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `507.89s`、VM free spaceは `7,666,716,672` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct six-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの7引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct seven-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct seven-argument user-defined function `make-constant7 [x y z q r s t] (fn [w] (+ x (+ y (+ z (+ q (+ r (+ s (+ t w)))))))))` へ渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`invariant-static-user-function-closure-value-with-env` の parameter environment projectionを7引数まで許可した。partial applicationではなく user functionへの7引数 direct callを対象とし、closure bodyの捕捉値と最終引数を使った Rust oracleの non-Bool diagnostic type textを `Int` として投影する。8引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_seven_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（32.38s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（30.94s）。既存の computation non-Bool prefix回帰25件（3〜7引数 user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `25 passed`（195.87s）だった。

これは direct seven-argument user functionがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの8引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct seven-argument user-function-returning closure projection (2026-07-23)

`768c277455bf4321d04f476593d36bdb8409cc3d` を current sourceとして、direct seven-argument user-function-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/768c2774-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=768c277455bf4321d04f476593d36bdb8409cc3d`、`selfhost_fixed_point=true`、program SHA-256 `3a7dca24e84849fc117126694726ae6ab11605457ba17ae29f8692ecd02db581`を記録する。underlying ignored E2Eは `1 passed`（601.25s）。`program.native` は Mach-O arm64、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/768c2774-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。`summary.json` は expected/actual exit code `42` の一致を記録し、stage2/stage3 stderrは双方 `0` bytesだった。actual stage1 bundle生成は `519.46s`、VM free spaceは `7,661,223,936` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct seven-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの8引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct eight-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct eight-argument user-defined function `make-constant8 [x y z q r s t u] (fn [w] (+ x (+ y (+ z (+ q (+ r (+ s (+ t (+ u w)))))))))` へ渡し、その戻り値の one-argument closureを同じ computation内で適用する場合について、`invariant-static-user-function-closure-value-with-env` の static parameter environment projectionを bounded `<= 8` 判定へ置き換えた。既存の0〜7引数列挙を一段増やすのではなく単一比較へ整理し、9引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_eight_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（30.23s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（42.69s）。直前の seven-argument regressionも `1 passed`（31.14s）で bounded comparisonへの変更後も維持された。

これは direct eight-argument user functionがclosureを返す computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、closure内の全型推論、closure/higher-order/user-functionの9引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct eight-argument user-function-returning closure projection (2026-07-23)

`8bebb8645eb24ebfebfb0794295894927924a1c2` を current sourceとして、direct eight-argument user-function-returning closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/8bebb864-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=8bebb8645eb24ebfebfb0794295894927924a1c2`、`selfhost_fixed_point=true`、program SHA-256 `688210434fca6d4bcac66ff1a7944f403f459a0fe4462b7deb4a178682346857`を記録する。underlying ignored E2Eは `1 passed`（593.66s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。artifact sizeは `3.4M`、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/8bebb864-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `520.43s`、VM free spaceは `7,689,248,768` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct eight-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの9引数以上、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct four-argument user-function and nine-argument user-function-returning closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepについて、direct four-argument user-defined functionの通常の戻り値投影と、direct nine-argument user-defined function `make-constant9 [x y z q r s t u v] (fn [w] (+ x (+ y (+ z (+ q (+ r (+ s (+ t (+ u (+ v w))))))))))` の戻り値の one-argument closure投影を閉じた。前者は static user-function body projectionを4引数まで単一の bounded comparisonで許可し、後者は `invariant-static-user-function-closure-value-with-env` の parameter environment projectionを9引数まで許可する。いずれも partial applicationではなく direct callだけを対象とし、通常の user functionは5引数以上、closure-returning user functionは10引数以上、partial application、closure/higher-orderの未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_four_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（29.97s）。RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_nine_arg_user_function_returning_closure_message` も同じ `Unknown` / `Int` 差分で失敗した（31.64s）。GREENは各 fixtureで message/spanを一致させてそれぞれ `1 passed`（30.38s / 30.31s）。既存の computation non-Bool prefix回帰28件（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `28 passed`（230.83s）だった。

これは direct four-argument user functionとdirect nine-argument user function-returning closureの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、全型推論、10引数以上、partial application、nested closureの任意深さ、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct four-argument user-function and nine-argument user-function-returning closure projection (2026-07-23)

`7b3e8a979d4dacddfb137d20fe0fee3e644471af` を current sourceとして、direct four-argument user-functionおよびdirect nine-argument user-function-returning closureの computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/7b3e8a97-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=7b3e8a979d4dacddfb137d20fe0fee3e644471af`、`selfhost_fixed_point=true`、program SHA-256 `db117e21e1380e8d6325e6af8f1185a3ae597d1ffae350411d83f8fe6ce0b121`を記録する。underlying ignored E2Eは `1 passed`（611.90s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致し、専用 Mac Cargo targetは検証後に削除する。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/7b3e8a97-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `531.97s`、VM free spaceは `7,683,764,224` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct four-argument user-functionとdirect nine-argument user-function-returning closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct five-argument user-function diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct five-argument user-defined function `sum5 [x y z q r] (+ x (+ y (+ z (+ q r))))` へ一度に渡す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを5引数まで許可した。partial applicationではなく direct callだけを対象とし、6引数以上、closure value、higher-order callee、partial application、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_five_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（31.14s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（31.19s）。既存の computation non-Bool prefix回帰29件（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `29 passed`（213.74s）だった。

これは direct five-argument user functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、6引数以上、closure/higher-order経路、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct five-argument user-function projection (2026-07-23)

`6674af6d5911721140db7cf56c08ff38114ad607` を current sourceとして、direct five-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/6674af6d-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=6674af6d5911721140db7cf56c08ff38114ad607`、`selfhost_fixed_point=true`、program SHA-256 `b202c95d1dbe66e45c59f81e22fe9c291c2f8951cb073b0dcd8b3bd81f3ad716`を記録する。underlying ignored E2Eは `1 passed`（602.55s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致し、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/6674af6d-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `529.72s`、VM free spaceは `7,678,230,528` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct five-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct six-argument user-function diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct six-argument user-defined function `sum6 [x y z q r s] (+ x (+ y (+ z (+ q r))))` へ一度に渡す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを6引数まで許可した。partial applicationではなく direct callだけを対象とし、7引数以上、closure value、higher-order callee、partial application、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_six_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（30.73s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（30.77s）。既存の computation non-Bool prefix回帰30件（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `30 passed`（199.53s）だった。

これは direct six-argument user functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、7引数以上、closure/higher-order経路、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct six-argument user-function projection (2026-07-23)

`15d9776168a2909c62b9426b0c77ea6b6dc2080a` を current sourceとして、direct six-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/15d97761-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=15d9776168a2909c62b9426b0c77ea6b6dc2080a`、`selfhost_fixed_point=true`、program SHA-256 `3163d2fee1ee23aac384dc36e0a559bcee991d69bb36a3da03fcd34a8b4d63a3`を記録する。underlying ignored E2Eは `1 passed`（588.63s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、exit codeは `0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致し、専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/15d97761-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `586.97s`、VM free spaceは `7,670,706,176` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct six-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct seven-argument user-function diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct seven-argument user-defined function `sum7 [x y z q r s t] (+ x (+ y (+ z (+ q (+ r s)))))` へ一度に渡す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを7引数まで許可した。partial applicationではなく direct callだけを対象とし、8引数以上、closure value、higher-order callee、partial application、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_seven_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（31.29s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（52.36s）。既存の computation non-Bool prefix回帰31件（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `31 passed`（317.28s）だった。

これは direct seven-argument user functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、8引数以上、closure/higher-order経路、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct seven-argument user-function projection (2026-07-23)

`05eea9a1bbfb4a3e2a5f7f84ad028894e33797c2` を current sourceとして、direct seven-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/05eea9a-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=05eea9a1bbfb4a3e2a5f7f84ad028894e33797c2`、`selfhost_fixed_point=true`、program SHA-256 `bd8574ff7e548fb0df17a9f468a47b2e13df31ac12ec23e0fb1ede2b1477c2bc`を記録する。underlying ignored E2Eは `1 passed`（698.11s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除する。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/05eea9a-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `528.36s`、VM free spaceは `7,665,205,248` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct seven-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct eight-argument user-function diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct eight-argument user-defined function `sum8 [x y z q r s t u] (+ x (+ y (+ z (+ q (+ r (+ s (+ t u)))))))` へ一度に渡す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを8引数まで許可した。partial applicationではなく direct callだけを対象とし、9引数以上、closure value、higher-order callee、partial application、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_eight_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（31.42s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（34.74s）。既存の computation non-Bool prefix回帰32件（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `32 passed`（297.27s）だった。

これは direct eight-argument user functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、9引数以上、closure/higher-order経路、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct eight-argument user-function projection (2026-07-23)

`463dd0c87d4db782bcb57edc80f78a3ff079b0a9` を current sourceとして、direct eight-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/463dd0c-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=463dd0c87d4db782bcb57edc80f78a3ff079b0a9`、`selfhost_fixed_point=true`、program SHA-256 `864105f08b5fc02486a56ad1e732304502dbc79a5744ba88c77f0e5c050228e3`を記録する。underlying ignored E2Eは `1 passed`（697.17s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除する。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/463dd0c-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `669.31s`、VM free spaceは `7,679,660,032` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct eight-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct nine-argument user-function diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct nine-argument user-defined function `sum9 [x y z q r s t u v] (+ x (+ y (+ z (+ q (+ r (+ s (+ t u v)))))))` へ一度に渡す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを9引数まで許可した。partial applicationではなく direct callだけを対象とし、10引数以上、closure value、higher-order callee、partial application、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_nine_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（37.69s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（35.17s）。既存の computation non-Bool prefix回帰33件（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `33 passed`（309.04s）だった。

これは direct nine-argument user functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、10引数以上、closure/higher-order経路、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct nine-argument user-function projection (2026-07-23)

`4054faf2251a5f21be54abb209b8f8a5fd270ecf` を current sourceとして、direct nine-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/4054faf-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=4054faf2251a5f21be54abb209b8f8a5fd270ecf`、`selfhost_fixed_point=true`、program SHA-256 `5d0361e71f7e022bb7972e5c3c63f8f483bb849bbaf4e1cb9b15f5be0e705417`を記録する。underlying ignored E2Eは `1 passed`（681.40s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除する。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/4054faf-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `517.65s`、VM free spaceは `7,683,231,744` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct nine-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct ten-argument user-function diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct ten-argument user-defined function `sum10 [x y z q r s t u v w] (+ (+ (+ (+ x y) (+ z q)) (+ (+ r s) (+ t u))) (+ v w))` へ一度に渡す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを10引数まで許可した。partial applicationではなく direct callだけを対象とし、11引数以上、closure value、higher-order callee、partial application、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_ten_arg_user_function_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（34.41s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（30.79s）。既存の computation non-Bool prefix回帰（direct user function、user-function-returning closure、lambda-returning closure、nested/capture/higher-orderを含む）は `34 passed`（277.54s）だった。

これは direct ten-argument user functionの computation diagnostic message/span projectionを一段だけ閉じる verified sliceであり、user-function bodyの全型推論、11引数以上、closure/higher-order経路、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct ten-argument user-function projection (2026-07-23)

`bbcc1d73a1e865fcc7767cea5b51896d5085c072` を current sourceとして、direct ten-argument user-function computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/bbcc1d7-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=bbcc1d73a1e865fcc7767cea5b51896d5085c072`、`selfhost_fixed_point=true`、program SHA-256 `adfd2bdd09702eecd607cbf98efa2fa1bf4f6b41a29b84125f6f3f880b0f6a1b`を記録する。underlying ignored E2Eは `1 passed`（799.14s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。専用 Mac Cargo targetは検証後に削除した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/bbcc1d7-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length `10,832,651` bytes、stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code artifactの SHA-256は双方 `52f2c3e8c315c009d9afc2cffec0f0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `603.49s`、VM free spaceは `7,689,900,032` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct ten-argument user-function projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 computation direct ten-argument user-function-returning-closure diagnostic message projection slice (2026-07-23)

computation式の最終 `return` stepが、`let! delta 1` で得た値を direct ten-argument user-defined function `make-constant10 [x y z q r s t u v w]` に渡し、その戻り値である closureをさらに呼び出す場合について、`invariant-static-user-function-non-bool-type-text-with-env` の static parameter environment projectionを10引数まで許可した。direct ordinary callと同じく11引数以上、partial application、higher-order callee、未解決 callee、dynamic computationでは既存の `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_computation_ten_arg_user_function_returning_closure_message` は selfhostの `:invariant は Bool 必須ですが、Unknown が推論されました` と Rust oracleの `:invariant は Bool 必須ですが、Int が推論されました` の差分で失敗した（31.70s）。GREENは同じ fixtureで message/spanを一致させて `1 passed`（32.36s）。既存の computation non-Bool prefix回帰35件（direct ordinary、direct closure、lambda-returning closure、nested/capture/higher-orderを含む）は `35 passed`（369.73s）だった。

これは direct ten-argument user-function-returning-closure の diagnostic message/span projectionを一段だけ閉じる verified sliceであり、11引数以上、closure/higher-order経路全体、partial application、dynamic/unresolved computationの診断本文、full diagnostic/span parity、structured/text reportの全境界、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after direct ten-argument user-function-returning-closure projection (2026-07-23)

`b5760ddc8c246e5a5e50be2460bcd520600e9efc` を current sourceとして、direct ten-argument user-function-returning-closure computation diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証し、完了後に専用 target/workdirを削除した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/b5760dd-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=b5760ddc8c246e5a5e50be2460bcd520600e9efc`、`selfhost_fixed_point=true`、program SHA-256 `5be8afbf8cbf02170c9de2b6041d1666d0790ded2cbb7a8a620ad03d4d20202e`を記録する。underlying ignored E2Eは `1 passed`（677.48s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/b5760dd-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6`の一致を記録する。stage2/stage3 code lengthは双方 `10,832,651` bytes、SHA-256は双方 `52f2c3e8c315c009d9afc2cffecf0c0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、actual stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `590.19s`、VM free spaceは `7,690,383,360` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `101M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後に VMを停止し、VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct ten-argument user-function-returning-closure projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、closure/higher-order/user-functionの未対応引数範囲、partial application、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 three-argument user-function match guard diagnostic message projection slice (2026-07-23)

通常の match guard が direct three-argument user-defined function `sum3 [x y z] (+ x (+ y z))` を呼び出す場合について、`invariant-static-non-bool-type-text-with-program` を使って guard の non-Bool 型本文を program-aware に投影する経路を閉じた。従来の static-only helperでは user-function bodyの parameter environmentを解決できず `Unknown` になっていたため、`sum3 1 2 3` の実型 `Int` と Rust oracleの診断本文を一致させた。直接呼び出しの parameter projectionは3引数までに限定し、未解決 shapeや4引数以上では既存の fail-closed な `Unknown` 境界を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_three_arg_function_match_guard_message` は selfhostの `expected Unknown, found Bool` と Rust oracleの `expected Int, found Bool` の差分で同一 span `80..92` のまま失敗した。GREENは同じ fixtureで `1 passed`（30.11s）。非Bool match guard rejection回帰は `10 passed`（530.11s）、既存の literal guard diagnostic regressionは `1 passed`（34.89s）だった。実装は match guard の message projectionだけを program-aware helperへ切り替え、既存の diagnostic code/span と unresolved/higher-arity の fail-closed 契約を保持する。

これは direct three-argument user-function match guardの diagnostic message/span projectionを一段だけ閉じる verified sliceであり、全 match guard の型推論、dynamic predicate、structured/text reportの全境界、full diagnostic/span parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source dual-target gates after three-argument user-function match guard message projection (2026-07-23)

`eb6f053b4ff4f7ac0d638b78b510ef003771bbb2` を current sourceとして、direct three-argument user-function match guard diagnostic projection後の Mac Apple Silicon / Linux x86_64 native evidenceを取り直した。共有 root worktreeの無関係な dirty filesを保全するため、clean-worktree wrapperは実行せず、専用 worktree・Cargo target・artifact pathで検証した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/eb6f053-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=eb6f053b4ff4f7ac0d638b78b510ef003771bbb2`、`selfhost_fixed_point=true`、program SHA-256 `252b096e802d7ef278962c27c7d0df41b75bb36f6ab7022645a202af08d14d56` を記録する。underlying ignored E2Eは `1 passed`（803.99s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes（artifact directory `3,804 KiB`）、`--version` は `lsharp 0.1.0`、stdoutは `12` bytes、stderrは `0` bytesだった。manifestとprogramのSHA-256は一致した。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/eb6f053-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` の一致を記録する。stage2/stage3 code lengthは双方 `10,832,651` bytes、SHA-256は双方 `52f2c3e8c315c009d9afc2cffecf0c0c7aee0eba8e2f651a3caa7d4cc1896d819`、stderrは双方 `0` bytesである。`summary.json` は expected/actual exit code `42` の一致を記録し、actual stage1、stage2-debug、stage3-debug manifestは同じ source commitを持つ。stage1 code/data/entrypointは `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 code/data/entrypointは `10,832,651` / `1,523` / `10,828,220` bytes、function table lengthは `3,237`、main function indexは `3,246`で一致した。actual stage1 bundle生成は `572.96s`、VM free spaceは `7,693,627,392` bytes、必要量は `4,294,967,296` bytes、artifact sizeは `103M`だった。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compareが passし、生成された `program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64だった。検証後にVMを停止し、VM workdir、replay lock、専用 host Cargo targetを削除した。VM設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは direct three-argument user-function match guard message projectionを含む current-source native artifact/runtime fixed-pointの両対応 target evidenceであり、全 match guard predicate、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 dynamic user-function match guard message regression slice (2026-07-23)

return shapeが未知の user-defined function `identity [x] x` を match guardから呼び出す場合について、既存の `LS1002` rejection と diagnostic spanに加えて、selfhost runnerの診断本文が Rust oracle と一致することを固定した。`test_e2e_selfhost_test_runner_preserves_non_bool_dynamic_function_match_guard_message` は `identity 1` の同一 fixtureを使い、結果件数、diagnostic code、message、spanを比較して `1 passed`（87.96s）となった。今回の差分は回帰テストと証跡のみで、selfhost実装の変更はない。

これは Rust oracle / bootstrap 経路の dynamic guard message parity regressionであり、current-source native artifact/runtime gate、dynamic predicate全体、全診断本文の型推論 parity、EC-M1-01 aggregateの完了を意味しない。TODOの `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 compound match guard branch diagnostic projection slice (2026-07-23)

match guard 内の既知の `if` branch mismatch を selfhost 側で識別し、Rust oracle と同じ non-Bool diagnostic message を返す narrow slice を追加した。`invariant-static-if-branch-mismatch` と match-arm scan は、condition が Bool で then/else の既知型が異なる場合だけ `E0003` と非Bool側の型名を選び、それ以外の既存 `E0002` / `Unknown` fallback を維持する。

Evidence: RED `test_e2e_selfhost_test_runner_preserves_non_bool_compound_match_guard_message` は selfhost `E0002` / `Unknown` と Rust `E0003` / `Int` の差分（同一 span `46..69`）で失敗した。GREEN は `1 passed`（31.58s）。valid constructor guard evaluator は `1 passed`（31.17s）、compound guard span regression は `1 passed`（42.46s）、既存 direct three-argument function message regression は `1 passed`（52.05s）、dynamic function message regression は `1 passed`（34.72s）だった。full selfhost CLI JSON report の message/code/span/exit 比較も `1 passed`（558.03s）となった。

この slice は known compound match guard の message projectionだけを閉じるもので、全 match predicate、全診断本文の型推論 parity、structured/text report の全境界、EC-M1-01 aggregate の完了を意味しない。Standalone source check は `cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` を試行したが、既存の `undefined symbol` 289 diagnostics で失敗したため、この slice の focused bundle GREEN を source-file check 全体の成功へ拡大解釈しない。TODO の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。

### EC-M1-01 standalone TestRunner source-check failure classification (2026-07-23)

current source の `cargo run --quiet --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` は、復元した実装で `diagnostics.count=289`、`firstErrorCode=1`、exit `1` を返した。専用 worktree 内の識別実験で、新規の `invariant-static-if-branch-mismatch` と `invariant-match-branch-mismatch-index` の本体を両方 no-op にすると `287`、片方だけ本体を戻すと `288`、両方を戻すと `289` になった。追加の2件は各 helper 定義に対応する。

definition-index trace では、両 helper が呼ぶ既存の `invariant-static-bool-kind-with-program` 自体が standalone checker の既存 failure definition であり、failed definition を環境から除去する現在の単一ファイル解析により、新しい caller 側が `undefined symbol` として数えられることを確認した。`ast-if` / `ast-match-guard` を数値 tagへ置換する識別実験でも `289` は変わらず、import symbolだけを置換してもこの境界は解消しない。

これは compound guard の focused runtime / full CLI JSON / Mac Apple Silicon / Linux x86_64 native gateの失敗ではなく、standalone source-check の定義別 provenance・import-aware resolution不足である。次の RED はこの集計を広く書き換えず、失敗した定義名・依存先・source spanを一件ずつ観測できる診断契約に固定する。source-file checkが `0` になるまで、EC-M1-01 aggregateの完了やRust-free全体完了とは扱わない。

### EC-M1-01 current-source dual-target gates after compound match guard branch diagnostic projection (2026-07-23)

`fafd8063b39985a098bbe388b8c079caebdd169e` を current source として、compound match guard branch diagnostic projection 後の Mac Apple Silicon / Linux x86_64 native evidence を取り直した。共有 root worktree の無関係な dirty files を保全するため、専用 worktree・Cargo target・artifact path で検証し、完了後に専用 worktree と target を削除した。

Mac Evidence: `ci-artifacts/native-release/aarch64-apple-darwin/fafd806-app-cli-current/manifest.json` は target `aarch64-apple-darwin`、`source_commit=fafd8063b39985a098bbe388b8c079caebdd169e`、`selfhost_fixed_point=true`、program SHA-256 `c365079bb835d7493f67a3eeedb832be4753073f409f9cdb3c1317c45b2fc84f` を記録する。underlying ignored E2E は `1 passed`（624.57s）。`program.native` は Mach-O arm64、サイズ `3,518,080` bytes、artifact directory `3,804 KiB`、stdout `12` bytes、stderr `0` bytes だった。

Linux Evidence: `ci-artifacts/native-linux-x86-hostgen-vm/fafd806-stage2-stage3-current/actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 stdout length `11,646,271` bytes、両 stdout SHA-256 `2ae6f1406e5c0484a94282a17edac85fad9f3f5649352c43db1826979a576ed6` の一致を記録する。stage2/stage3 code length は双方 `10,832,651` bytes、SHA-256 は双方 `52f2c3e8c315c009d9afc2cffefc0c0c7aee0eba8e2f651a3caa7d4cc1896d819`、`summary.json` の expected/actual exit code は `42` で一致した。actual stage1 manifest の code/data/entrypoint は `4,203,487` / `1,523` / `4,201,104` bytes、stage2/stage3 は `10,832,651` / `1,523` / `10,828,220` bytes、function table length は `3,237`、main function index は `3,246`で一致した。actual stage1 bundle生成は `675.55s`、VM free space は `7,694,491,648` bytes、必要量は `4,294,967,296` bytes、artifact size は `101M`だった。`program.native` は ELF 64-bit x86-64、`program.o` は ELF 64-bit relocatable x86-64。全 Linux host probe、actual stage1 -> stage2 -> stage3 transport、materialize、stage2/stage3 byte compare が pass し、検証後に VM を停止した。VM 設定は `4 CPU / 16GiB memory / 12GiB disk` のまま維持した。

これは compound match guard projection を含む current-source native artifact/runtime fixed-point の両対応 target evidence であり、全言語機能、全公開 command、stage0 acquisition/release/rollback、全診断本文の型推論 parity、EC-M1-01 aggregate の完了を意味しない。次の未完条件は standalone `TestRunner.ls` source check の 289 diagnostics の分類と、残る match predicate / diagnostic boundary の要件別 parity である。

### EC-M1-01 standalone source-check first failed definition index provenance slice (2026-07-23)

standalone source-check の失敗定義を後続の name / dependency / span 観測へ渡すため、`infer-program-analysis` の既存 state index 0〜5 を維持したまま末尾に `first-error-index` を追加した。top-level defn の推論結果が失敗した最初の `program idx` だけを保存し、後続の失敗で上書きしない。recursive-alias の early return と成功時の初期値は `-1` とする。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_first_failed_definition_index` は未定義 accessorで失敗した。GREEN は `(defn ok [] 42) (defn fail [] missing)` の selfhost fixtureで diagnostics `1`、first-error-index `1` を確認した。既存 typed-defn signature rejection と mutual-recursion program analysis は `RUST_MIN_STACK=128MiB` の focused E2E で各 `1 passed` となり、既存 state accessorと recursive branchの互換性を確認した。通常 stackでの typed-defn testは selfhost bundle実行中に stack overflowとなるため、expanded stackを使う既存運用に合わせた。

これは standalone `TestRunner.ls` の 289 diagnostics を減らす修正ではなく、失敗定義を一件ずつ分類するための観測契約である。失敗定義名、依存先、source spanの native projection、source-file check `0`、EC-M1-01 aggregate、Rust-free全体完了は未達であり、TODOの `[~]` と Rust oracle / bootstrap / host integration境界を維持する。次の RED はこの indexから定義名・依存先・spanを同じ fixtureで取得する narrow contractとする。

### EC-M1-01 standalone source-check first failed definition name-hash provenance slice (2026-07-23)

`infer-program-analysis` の state 末尾へ `first-error-name-hash` を追加し、最初に失敗した top-level defn の name hash を index と同じ provenance として保持する。後続の失敗では既存の index/name hashを維持し、recursive-alias early return と成功時は `-1` とする。既存 state index 0〜6 は変更しない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_first_failed_definition_name_hash` は未定義 accessorで失敗した。GREEN は `(defn ok [] 42) (defn fail [] missing) (defn later [] missing-later)` の fixtureで diagnostics `2`、first-error-index `1`、AST index `1` の name hashとの一致を確認し、provenance test 2件が `2 passed`（single-thread）となった。typed-defn signature rejection と mutual-recursion program analysisも state 8要素化後に各 `1 passed`（`RUST_MIN_STACK=128MiB`）だった。

これは name hashの観測契約だけを追加する verified sliceであり、失敗定義名の文字列化、依存先、source spanの native projection、standalone source-check `0`、EC-M1-01 aggregate、Rust-free全体完了を意味しない。次は name hashを依存先・source spanへ結び付ける狭い診断契約を分離して進める。

### EC-M1-01 import-aware selfhost file check slice (2026-07-23)

`run-check-source` の source-only 経路と `run-check` の file 経路を分離し、後者は既存の `compile-file-pairs-with-cache` が作る依存閉包を使って、依存 module の declaration vector を依存順に一つの TypeInfer program へ畳み込むようにした。`Cli` と `EmbeddedCli` の両方を同期し、bounded loop を使って declaration / pair の走査を行う。

Evidence: RED `test_e2e_selfhost_cli_check_file_resolves_imported_definition` は、`Main.ls` から import した `Lib.helper` を解決できず `undefined symbol` になる failure を固定した。GREEN は同じ Preview1 selfhost bundle で `Fn`、`diagnostics:0`、exit `0` を確認した（1 passed、445.78s）。依存 module のない source-only `run-check-source` の既存契約も維持する。`run_wasm_component_capture` は component trap 時にも既に捕捉した stdout を error に残すようにし、`test_component_trap_error_preserves_captured_stdout` で `stdout_lossy` の出力を固定した。

この slice は module/import declaration の flatten と unqualified name resolution に限定される。`import` の `:only` / `:as` / `:open`、qualified name、private visibility、依存先 source span の診断 projection、standalone source-check の全定義成功、supported 2 targets の current-source native gateは未完了である。Preview2 の Rust driver で `selfhost/src/Syntax/Parser.ls` を直接 check すると、temporary marker により import traversal 前の初期 source parse で component trap することを確認した。Preview1 の core harness で到達できることを Preview2 component parityへ拡大解釈せず、root/stack/spill の広い変更は保留する。次の RED は、失敗定義の name hashを依存先・source spanへ結び付ける狭い診断契約である。

### EC-M1-01 first failed imported module provenance slice (2026-07-23)

file-check の flatten 結果と同じ declaration 順で module hash の平行ベクタを作り、`infer-program-analysis-first-error-index` が示す base type error を依存 moduleへ結び付けた。`run-check-source` は空の provenance contextを使うため、source-only の text/JSON 契約は変えず、file-check の text error outputにだけ `first-module-hash:<hash>` を追加する。

Evidence: RED `test_e2e_selfhost_cli_check_file_reports_first_failed_module_hash` は `Lib.helper` の `missing` を使い、出力が module hash `76389`、`Int`、`diagnostics:1,T0001@1:1,first-body:undefined symbol`、exit `1` までで provenance 行がない failure を固定した。GREEN は同じ fixtureで `first-module-hash:76389` を確認し `1 passed`（437.91s）となった。`Cli` と `EmbeddedCli` の context wiringを同期し、component targetの JSON reportへ未検証の fieldは追加していない。

これは module hashの dependency provenanceだけを閉じる verified sliceであり、module name/pathの文字列化、qualified import/private visibility、失敗式の source span、canonical/case/property diagnosticsとの統合、standalone source-check `0`、supported 2 targetsの current-source native gate、EC-M1-01 aggregateの完了を意味しない。次の RED はこの module hashを source pathと失敗式 spanへ分解して観測する narrow contractである。

### EC-M1-01 first failed imported module name provenance slice (2026-07-23)

module declaration の既存 `[tag, hash, body]` slotを維持したまま、selfhost parser が module name の source start/end を追加 slotへ保存するようにした。file-check の owner は `[module-hash, module-name]` となり、`first-error-index` から hash と source name を同じ declarationへ投影する。source-only context は空 ownerのままで、JSON schemaや既存の module body consumerは変更しない。

Evidence: RED は既存 `test_e2e_selfhost_cli_check_file_reports_first_failed_module_hash` に `first-module-name:Lib` の assertion を追加し、hash行までの出力 `76389`, `Int`, `diagnostics:1,T0001@1:1,first-body:undefined symbol`, `first-module-hash:76389`, exit `1` で name 行がないことを確認した。GREEN は同じ Preview1 fixtureで `first-module-name:Lib` を含めて `1 passed`（436.06s）となった。

これは module hashから source module nameへ進めた verified sliceであり、実 filesystem pathの表示、qualified import/private visibility、失敗式の source span、canonical/case/property diagnosticsとの統合、standalone source-check `0`、supported 2 targetsの current-source native gate、EC-M1-01 aggregateの完了を意味しない。次の RED は module nameを resolved source pathと失敗 expression spanへ結び付ける narrow contractである。

### EC-M1-01 first failed imported module resolved path provenance slice (2026-07-23)

file-check の module owner を `[module-hash, module-name, module-path]` とし、`load-check-program` が既存の `ModuleResolver` の source root / package root / cache を使って module name から resolved source pathを保存するようにした。`Cli` と `EmbeddedCli` の text reportは、最初の base type errorに対応する ownerへ `first-module-path:<path>` を追加する。source-only check の空 owner、JSON report、既存の resolver優先順位は変更しない。

Evidence: RED は既存の `test_e2e_selfhost_cli_check_file_reports_first_failed_module_hash` に path assertionを追加し、実装前の出力に hash/name までしかないことを固定した。実装後の同じ Preview1 fixtureは `76389`、`Int`、`diagnostics:1,T0001@1:1,first-body:undefined symbol`、`first-module-hash:76389`、`first-module-name:Lib`、`first-module-path:./Lib.ls`、exit `1` を返し、`1 passed`（603.01s）となった。相対 entry pathに対して既存 resolverが返す `./Lib.ls` をそのまま契約とし、診断側で未検証の絶対 path正規化は追加していない。通常の `check examples/fib.ls` も diagnostics `0`、exit `0` で確認した。

これは resolved module pathの dependency provenanceだけを閉じる verified sliceであり、qualified import/private visibility、絶対 pathの canonicalization、失敗 expressionの source span、canonical/case/property diagnosticsとの統合、standalone source-check `0`、supported 2 targetsの current-source native gate、EC-M1-01 aggregateの完了を意味しない。次の RED は同じ first-error indexから失敗式の source spanを取得する narrow contractである。

### EC-M1-01 first failed imported expression span provenance slice (2026-07-23)

parser 由来の変数参照 AST を既存の `[tag, name-hash]` consumer と互換な `[tag, name-hash, start, end]` に拡張し、未定義変数の type-infer error result と `propagate-error-result-with-span` が失敗式の byte span を後置 slotへ保持するようにした。既存の `propagate-error-result` は Pattern などの旧 result shape向けに維持する。`infer-program-analysis` は既存 state index `0..7` を維持したまま `first-error-start` / `first-error-end` を末尾へ追加し、`Cli` と `EmbeddedCli` の file-check text output は既存の module hash/name/path と exit codeを保ったまま `first-error-span:<start>:<end>` を追加する。JSON、source-only の空 owner、spanを持たない既存の成功 result は変更しない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_first_failed_expression_span` は未定義 accessorで失敗した（27.92s）。GREEN は `(defn fail [] missing)` の selfhost TypeInfer fixtureで diagnostics `1`、span `14..21` を確認し、既存の first-error index/name-hash 回帰を含む3件が `1 passed`（51.94s）となった。file-check の RED `test_e2e_selfhost_cli_check_file_reports_first_failed_module_hash` は既存の `76389`、`Lib`、`./Lib.ls` provenance までで span 行がないことを固定した（455.46s）。GREEN は同じ Preview1 fixtureで `first-error-span:29:36`、既存 diagnostics、module hash/name/path、exit `1` を確認した（444.71s）。途中で Pattern の `[result, env]` slot 3 と span slotの型衝突を検出し、Pattern failureは従来の code-only propagationへ戻して build-script回帰を解消した。EmbeddedCli の source contract test、`CARGO_TARGET_DIR=/tmp/lsharp-analysis-failure-span-target cargo run --quiet --bin lsharp -- check examples/fib.ls`（`Fn`、diagnostics `0`）、`bash scripts/audit_docs.sh`（error/warning `0`）も passした。

これは最初の imported undefined-variable expression の byte-span provenanceだけを閉じる verified sliceであり、line/column projection、qualified import・`:only`・`:as`・`:open`・private visibility、絶対 path canonicalization、if/apply/record/canonical/case/property の全 error constructorに対する span parity、pattern failureの span propagation、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。次の RED は direct nested diagnostic の span propagation または standalone source-check の失敗定義分類から一つに絞る。

### EC-M1-01 nested if error span propagation slice (2026-07-23)

`infer-if` の condition / then / else failure forwarding が error codeだけを再構築していたため、内側の未定義変数の spanを失っていた。3つの通常 infer-result branchを `propagate-error-result-with-span` に接続し、branch type mismatchなど新しい error constructorの spanは未検証のまま既存境界を維持する。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_nested_if_failure_span` は `(defn fail [] (if missing 1 2))` で first-error span `-1` を返した（28.51s）。GREEN は同じ selfhost TypeInfer fixtureで diagnostics `1`、`missing` の span `18..25` を確認した（29.04s）。既存 first-error index/name/span を含む analysis 4件は `1 passed`（37.60s）、変更後の `cargo run --quiet --bin lsharp -- check examples/fib.ls` は `Fn`、diagnostics `0` で passした。

これは nested `if` の内側 error span forwardingだけを閉じる verified sliceであり、apply / let / do / record / pattern / computation、branch mismatchの診断 span、line/column projection、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。次は direct apply / let propagation と standalone source-check failure-definition分類を重複しない REDとして進める。

### EC-M1-01 apply callee error span propagation slice (2026-07-23)

`TypeInferApply` の legacy apply pathで callee failureを code-only resultへ再構築していたため、parser由来の未定義 calleeの spanを失っていた。引数なしと引数ありの callee forwardingだけを `propagate-error-result-with-span` へ接続し、argument forwarding、高 arity、unify failureの spanは別境界として残す。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_apply_callee_failure_span` は `(defn fail [] (missing 2))` で first-error span `-1` を返した（30.08s）。GREEN は同じ selfhost TypeInfer fixtureで diagnostics `1`、`missing` の span `15..22` を確認した（30.77s）。既存 apply error-code 2件は通常 stackでは既知の stack overflowになるが、`RUST_MIN_STACK=134217728` で `2 passed`（46.11s）。変更後の `cargo run --quiet --bin lsharp -- check examples/fib.ls` は `Fn`、diagnostics `0` で passした。

これは apply callee failureの byte-span forwardingだけを閉じる verified sliceであり、argument failure、lambda body、let/do/computation、record/pattern、0〜7以外の arity、unify failureの diagnostic span、line/column projection、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。次は apply argumentまたは let initializerのどちらか一つを REDに固定する。

### EC-M1-01 arity-1 apply argument error span propagation slice (2026-07-23)

arity-1 applyの argument failure forwardingを `propagate-error-result-with-span` へ接続し、callee成功後に未定義 argumentが失敗した場合も source spanを保持するようにした。既存の callee forwarding、arg mismatch code、2引数以上の arity分岐は変更しない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_apply_argument_failure_span` は `(defn fail [] (not missing))` で first-error span `-1` を返した（28.54s）。GREEN は同じ selfhost TypeInfer fixtureで diagnostics `1`、`missing` の span `19..26` を確認した（29.59s）。変更後の `cargo run --quiet --bin lsharp -- check examples/fib.ls` は `Fn`、diagnostics `0` で passした。

これは arity-1 apply argumentの byte-span forwardingだけを閉じる verified sliceであり、2〜7引数の argument forwarding、lambda body、let/do/computation、record/pattern、unify failureの diagnostic span、line/column projection、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。次は let initializerの span propagationまたは standalone source-check failure-definition分類を REDに固定する。

### EC-M1-01 let initializer error span propagation slice (2026-07-23)

`TypeInferBlock.infer-let` の initializer failure forwardingを `propagate-error-result-with-span` へ接続し、binding bodyへ進む前に失敗した未定義 initializerの byte spanを保持する。let body、do/computationの step、generalization/unify failureは既存 code-only境界のまま残す。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_let_initializer_failure_span` は `(defn fail [] (let [value missing] value))` で first-error span `-1` を返した（28.46s）。GREEN は同じ selfhost TypeInfer fixtureで diagnostics `1`、`missing` の span `26..33` を確認した（29.01s）。変更後の `cargo run --quiet --bin lsharp -- check examples/fib.ls` は `Fn`、diagnostics `0` で passした。

これは let initializerの byte-span forwardingだけを閉じる verified sliceであり、let body、do/computation step、2〜7引数 apply、lambda、record/pattern、unify failureの diagnostic span、line/column projection、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration 境界は維持する。次は let bodyまたは do/computation stepのどちらか一つを REDに固定する。

### EC-M1-01 computation let-bang step error span propagation slice (2026-07-23)

2-step computation の `let!` step で initializer が失敗した場合の forwardingだけを `propagate-error-result-with-span` へ接続し、後続の `return` 式へ進む前に未定義式の byte spanを保持するようにした。step 2以降、`do!`、3-step computation、let body、generalization/unify failureは既存の code-only境界のまま残す。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_reports_computation_step_failure_span` は `(computation maybe-builder (let! x missing) (return x))` で diagnostics `1` を返したが、first-error spanの startが `-1` だった（29.66s）。GREEN は同じ selfhost TypeInfer fixtureで `missing` の span `49..56` を確認した（28.76s）。

これは computation の最初の `let!` step の byte-span forwardingだけを閉じる verified sliceであり、computation全体、do/computationの他step、2〜7引数 apply、lambda、record/pattern、unify failureの diagnostic span、line/column projection、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 current-source native artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は computation の残る step または standalone source-check failure-definition分類を REDに固定する。

### EC-M1-01 standalone source-check definition failure-kind classification slice (2026-07-24)

standalone source-check の連鎖失敗を分類するため、undefined-variable error resultに参照先 name hashを保持し、span-aware forwardingでもその hashを失わないようにした。`infer-program-analysis` は top-level defn の推論順に `failure-kinds` を保存し、`0=success`、`1=direct failure`、`2=dependency failure` と定義する。先行して失敗した defn の name hashを参照する `undefined symbol` だけを dependency failure とし、それ以外の error code / name は direct failureに分類する。

Evidence: `test_e2e_selfhost_typeinfer_analysis_classifies_definition_failure_kinds` は `(defn primary [] missing) (defn dependent [] primary) (defn independent [] missing-later)` を同一 selfhost TypeInfer fixtureで実行し、diagnostics `3` と failure kinds `1,2,1` を確認した（`RUST_MIN_STACK=128MiB`、1 passed、43.67s）。既存の first-error index/name-hash/span、nested if、apply、let、computation の analysis regression群も同じ変更後 targetで `9 passed`（67.94s）となった。`test_e2e_selfhost_cli_check_source_json_reports_definition_failure_kinds` は JSON の `failureKinds:[1,2,1]` と診断時 exit code `1` を確認した。RED は未投影の JSON 値 `null`（490.86s）、GREEN は 1 passed（506.03s）だった。`test_e2e_selfhost_embedded_cli_check_json_contract_is_present` で EmbeddedCli の同じ JSON builder / field contract も確認した。

これは TypeInfer analysis と `Cli` / `EmbeddedCli` の text/JSON reportへの定義別分類投影、および通常の複数段 dependency chainの regression を閉じる verified sliceであり、qualified/private import、非-`undefined symbol` の dependency分類、standalone `TestRunner.ls` source-check の 289 diagnostics 解消、Mac Apple Silicon / Linux x86_64 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`test_e2e_selfhost_typeinfer_analysis_classifies_multilevel_definition_failure_kinds` は `1,2,2,1` を確認し（1 passed、30.08s）、analysis regression群は `10 passed`（80.13s）だった。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified/private import または非-`undefined symbol` の failure boundaryを一つ REDに固定する。

### EC-M1-01 current-source Mac Apple Silicon App.Cli native gate (2026-07-24)

専用 clean worktree の current source commit `d249f517263181fb471aee4c8bdfc2865b574d74` から、actual stage2/stage3 self-regenerationを経て App.Cli の native release program を生成した。manifest は target `aarch64-apple-darwin`、`source_commit` が同じ current commit、`selfhost_fixed_point=true` を記録し、生成 program は Mach-O arm64 だった。`--version` は `lsharp 0.1.0`、stdout 12 bytes、stderr 0 bytes、program SHA-256 は `4d182d50c5fc0b5789a9b96bce773e3720aa36579e88b6a26d1bc4a210838e53` である。

Evidence: `LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR=/tmp/lsharp-native-macos-aarch64-release-d249f517 LSHARP_NATIVE_MACOS_AARCH64_CARGO_TARGET_DIR=/tmp/lsharp-native-macos-aarch64-cargo-d249f517 bash scripts/ci/native-macos-aarch64-selfhost-release.sh`、`test_e2e_native_macos_aarch64_actual_app_cli_release_program`（1 passed、651.17s）。artifact は 3,856 KiB で、検証後に一時 artifact と Cargo target を削除した。

これは current-source Mac Apple Silicon の App.Cli fixed-point/native runtime boundaryだけを閉じる evidenceであり、Linux x86_64 の同一 commit gate、EmbeddedCli native artifact、stage0 acquisition/package/rollback、全公開 command、EC-M1-01/EC-M1-07 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 current-source Linux x86_64 actual self-regeneration gate (2026-07-24)

専用 clean worktree の current source commit `a4b69b795085d218d480a4f68102244af3aa91fd` から、host-generated stage1 x86 payloadを Lima VM `lsharp-linux-x86`（x86_64、4 CPU、16 GiB memory、12 GiB disk）で実行し、stage2/stage3 の native self-regenerationを完走した。VM 側は 64-byte chunk、actual timeout 1200 seconds、4 GiB heap、fail-fast-on-OOM で実行し、ゲート中の `/tmp` 使用量は 33%（空き約 7.5 GiB）だった。stage1 manifest は target `x86_64-unknown-linux-gnu`、code `4,229,966` bytes、data `1,571` bytes、function-start length `3,267`、entrypoint offset `4,227,583` を記録した。stage2 と stage3 の manifest は同じ source commitを指し、両方とも code `10,920,570` bytes、data `1,571` bytes、function-start length `3,267`、entrypoint offset `10,916,139` となった。

Evidence: `NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=main-current-a4b69b79 NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_DIR=ci-artifacts/native-linux-x86-hostgen-vm/main-current-a4b69b79 NATIVE_LINUX_X86_ACTUAL_TIMEOUT=1200 NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE=64 NATIVE_LINUX_X86_REJECT_DIRTY_STAGE1_SEED=1 bash scripts/ci/native-linux-x86-selfregen.sh`、host probe 13件、`test_e2e_native_linux_x86_host_generates_actual_selfregen_stage1_bundle_artifact`（1 passed、601.52s）、actual self-regeneration summary（`status=pass`）。stage2/stage3 stdout SHA-256 はともに `4facb5f6da344ad22e1bb048683182e6b3624a8cc2c71421f5e5bcfca35edee3`、stdout は各 `11,727,461` bytesで一致した。VM workdir、host artifact、Cargo targetを検証後に回収し、VM は `Stopped` に戻した。

これは current-source Linux x86_64 の host-generated stage1 → stage2 → stage3 fixed-point evidenceだけを閉じるものであり、EmbeddedCli native artifact、stage0 acquisition/package/rollback、全公開 command、qualified/private import、非-`undefined symbol` の dependency分類、EC-M1-01/EC-M1-07 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 selfhost private definition local visibility slice (2026-07-24)

`private` wrapper を同一プログラム内の TypeInfer 入力として扱い、predeclare、pending-env、analysis、failure-kind の各ループが wrapper 内の `defn` を登録・推論するようにした。対象は `(private (defn helper [value] (+ value 1)))` と同一プログラム内の `(helper 1)` に限定し、private declaration を公開 import へ漏らす処理や no-argument apply は変更していない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_accepts_private_definition_call` は Rust oracle の `Infer::infer_program` 成功に対し、selfhost が `diagnostics=1`、`failureKinds=[1]` を返した（34.57s）。GREEN は同じ fixtureで selfhost `diagnostics=0`、`failureKinds=[0,0]` を確認した（35.28s）。既存の span/classification を含む analysis regression 11件も `11 passed`（133.65s）となった。

これは同一 flattened program 内の private `defn` local visibilityだけを閉じる verified sliceであり、import 先 private symbol の非公開境界、qualified import、`:only` / `:as` / `:open`、private type/record/ADT、no-argument apply、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は imported private function の module boundary または qualified import parser/lookupのどちらか一つを RED に固定する。

### EC-M1-01 two-expression do dependency failure-kind slice (2026-07-24)

2式の `do` ブロックの先頭式で失敗した undefined symbol を `propagate-error-result-with-span-and-name` へ渡し、先行して失敗した top-level 定義の name hash を failure-kind 分類まで保持する経路を閉じた。対象は `(defn primary [] missing) (defn dependent [] (do primary 42))` に限定し、3式以上の `do`、後続式、computation、import 境界は変更していない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_classifies_do_dependency_failure_kind` は Rust oracle の拒否に対し、selfhost が `diagnostics=2`、`failureKinds=[1,1]` を返した（35.68s）。GREEN は同じ fixtureで `diagnostics=2`、`failureKinds=[1,2]` を確認した（1 passed、33.09s）。既存の span/classification を含む analysis regression 12件も `12 passed`（118.62s）となり、`CARGO_TARGET_DIR=/tmp/lsharp-do-failure-red-target cargo run --quiet --bin lsharp -- check examples/fib.ls` は `Fn`、diagnostics `0`、failureKinds `[0,0]` で成功した。

これは2式 `do` の先頭 failure name-hash propagationだけを閉じる verified sliceであり、`do` の他の式数・位置、非-`undefined symbol` の dependency分類、import/private module boundary、standalone `TypeInfer.ls` source-check `0`、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は imported private function の module boundary または qualified import parser/lookupのどちらか一つを RED に固定する。

### EC-M1-01 imported private definition module-boundary slice (2026-07-24)

file-check の flatten 順で `module` 境界に到達したとき、先行 module の `private (defn ...)` name hashだけを selfhost TypeInfer の環境から除去するようにした。private declaration自体は flatten から削除せず、同じ module 内では public definitionの推論に利用できる状態を維持する。対象は `Lib` の arity-1 private `secret` と `Main` からの unqualified参照に限定した。

Evidence: RED `test_e2e_selfhost_cli_check_file_blocks_imported_private_definition` は、Rust oracle が import 先 private symbolを拒否する fixtureに対して selfhost が `Fn`、`diagnostics:0` を返した（548.95s）。GREEN は module 境界 cleanup後、同じ file-check fixtureで `1 passed`（484.90s）となり、`diagnostics:1,T0001@1:1,first-body:undefined symbol`、`first-module-name:Main`、`failure-kinds:0,1` を確認した。既存の同一プログラム内 private visibility focused testも変更後 `1 passed`（35.73s）である。

これは imported private functionの unqualified参照を module 境界で拒否する verified sliceであり、qualified import、`:only` / `:as` / `:open`、private type/record/ADT、複数 module の forward visibility、standalone `TypeInfer.ls` source-check `0`、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified import parser/lookupまたは `:only` export filteringのどちらか一つを RED に固定する。

### EC-M1-01 selfhost `import :as` parser/formatter slice (2026-07-24)

selfhost `Syntax.Parser` が `(import Lib :as L)` の `:as` optionを消費し、既存の module name hash / source span slotを維持したまま alias hashを AST index `4` へ保持するようにした。`Syntax.AST` に alias constructor/accessorを追加し、`FormatterDecl` の canonical formatterも aliasを `:as` textへ復元する。対象は一つの `:as` optionだけであり、import lookupはまだ unqualified nameのままである。

Evidence: RED `test_e2e_selfhost_parser_import_alias` は現在の parserが同じ sourceを `program length=5` として返した（7.88s）。実装後の GREENは同じ fixtureで `1 passed`（7.41s）となり、import tag `26`、module hash、alias slot、`L` の alias hashを確認した。formatter RED `test_e2e_selfhost_formatter_preserves_import_alias` は canonical output `"(import Lib)"` へ aliasが落ちた（11.83s）。formatter GREENは `"(import Lib :as L)"` を返して `1 passed`（12.08s）となった。既存 parser focused regressionは 9件中 8件が passし、nested module body-countの既存失敗（期待 `1`、実測 `3`）は今回の import fixtureを使わないため変更対象外として残した。

これは `import :as` の parser AST保持と formatter roundtripだけを閉じる verified sliceであり、alias経由の qualified lookup、直接 module名 lookup、`:only` / `:open`、private type/record/ADT、複数 moduleの export filtering、Rust oracleとの import type environment parity、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は alias-qualified name lookupまたは `:only` export filteringを、parser sliceと分離した REDに固定する。

### EC-M1-01 selfhost alias-qualified definition lookup slice (2026-07-24)

selfhost parser が保持する `(import Lib :as L)` の alias hashと、`L.helper` の prefix/suffix hashから同じ qualified keyを作り、依存 moduleの public top-level `defn` にだけ alias-qualified type schemeを追加するようにした。raw name lookupは先に維持し、既存の record field accessなどの直接 lookupを変えない。private wrapperはqualified exportへ追加せず、import前の declarationだけを対象にする。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_definition` は Rust oracleへ `Lib.helper` の外部型を注入した同一 fixtureに対し、selfhostが diagnostics `1`、failure kinds `0,1` を返した（32.43s）。GREENは同じ selfhost TypeInfer fixtureで diagnostics `0`、failure kinds `0,0` を返した（1 passed、31.03s）。既存の private visibility、`do` dependency failure-kind、parser field-access regressionも各 `1 passed` となった。clean treeと root `main` の既存 `test_e2e_selfhost_typeinfer_field_access` は今回の差分を外した状態でも標準 stack overflowとなるため、新規回帰ではなく既存 test harnessの制約として分離した。

これは alias-qualified public top-level function lookupだけを閉じる verified sliceであり、direct module prefix lookup、`:only` / `:open`、qualified type/record/ADT、private import filteringの全 surface、複数 moduleの forward visibility、standalone source-check `0`、fresh EmbeddedCli full check、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は `:only` export filteringまたは direct module-qualified lookupを一つの REDに固定する。

### EC-M1-01 selfhost `import :only` parser/formatter slice (2026-07-25)

selfhost `Syntax.Parser` が `(import Lib :only [helper extra])` の選択 symbol を hash vector として AST index `5` に保持し、既存の module name hash / source span / alias slotを維持するようにした。`Syntax.AST` に `:only` constructorを追加し、`FormatterDecl` の canonical formatterも選択 symbol を `:only [...]` textへ復元する。Rust parserの `ImportDecl { module, alias, only, open }` と同じ fixtureで module名、symbol順、空 alias、`open=false` を照合した。

Evidence: RED `test_e2e_selfhost_parser_import_only` は同じ sourceを selfhost parserで 7 nodeへ分解し、`:only` optionを消費できなかった。RED `test_e2e_selfhost_formatter_preserves_import_only` は canonical outputを `"(import Lib)"` と選択 symbolの残余へ分解した。GREEN は `test_e2e_selfhost_parser_import` の 2件、`test_e2e_selfhost_formatter_preserves_import` の 2件が全て passし、AST tag `26`、module hash、alias slot `0`、only hash vector `[helper, extra]`、formatter output `"(import Lib :only [helper extra])"` を確認した。

これは `import :only` の parser AST保持と formatter roundtripだけを閉じる verified sliceであり、`:only` export filtering / qualified lookup、`:as` と `:only` の複合 option順序、`:open`、qualified type/record/ADT、private import filtering、複数 moduleの forward visibility、Rust/native type environment parity、standalone source-check `0`、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は direct module-qualified lookupまたは `:only` export filteringを parser/formatter sliceと分離した REDに固定する。

### EC-M1-01 selfhost direct module-qualified definition lookup slice (2026-07-25)

selfhost `TypeInfer` の import qualified key生成で、`:as` aliasが無い `(import Lib)` の場合も module name hashを prefixとして使うようにした。これにより parser が保持する `Lib.helper` の prefix/suffix hashから、依存 module `Lib` の public top-level `helper` に対応する type schemeを同じ環境へ登録する。alias経路、同一 module内の private visibility、先行定義の範囲は維持した。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_module_qualified_definition` は Rust oracleへ `Lib.helper` の外部型を注入した同じ fixtureに対し、selfhostが diagnostics `1`、failure kinds `0,1` を返した。GREEN は同じ selfhost TypeInfer fixtureで diagnostics `0`、failure kinds `0,0` を確認した（1 passed）。既存の alias-qualified lookup 2件、private definition local visibility、2-expression `do` dependency failure-kind の focused regressionも各 passした。追加の source-check は `infer-recordlit` の未変更行（`11255..11295`）で `E0004 expected Vector, found Map` となったため、この sliceの source-check `0` evidenceには数えない。

これは direct module prefix経由の public top-level function lookupだけを閉じる verified sliceであり、`:only` export filtering、`:as` と `:only` の複合 option、`:open`、qualified type/record/ADT、private import filtering、複数 moduleの forward visibility、`infer-recordlit` source-check blockerを含む standalone source-check全体、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は `:only` export filteringを direct/alias qualified lookupから分離した REDに固定する。

### EC-M1-01 selfhost `import :only` qualified export filtering slice (2026-07-25)

selfhost `TypeInfer` が `Syntax.AST` の only hash vectorを読み、空 vectorの importは従来どおり全公開、非空 vectorの importは選択された public top-level `defn` だけを qualified type environmentへ登録するようにした。除外された symbolは qualified lookupで未定義として明示的に失敗し、alias/direct prefixの既存 lookup、private visibility、failure-kindの既存分類は変更していない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_filters_import_only_qualified_definition` は Rust の `inject_external_types_for_import` oracleでは selected `Lib.helper` が成功し、excluded `Lib.hidden` が拒否された一方、selfhostは selected/excludedとも diagnostics `0` を返した。GREEN は同じ selfhost flattened fixtureで selected `diagnostics=0, failureKinds=[0,0,0]`、excluded `diagnostics=1, failureKinds=[0,0,0]` を確認した（1 passed）。excluded qualified nameは top-level `hidden` name hashと異なるため failure-kindは direct `0` のままであり、visibility failureと dependency分類を混同しない。alias/direct qualified lookup 2件、private visibility、2-expression `do` dependency failure-kindも各 passした。

これは `:only` による direct module prefixの public top-level function filteringだけを閉じる verified sliceであり、`:as` と `:only` の複合 option順序、`:open`、unqualified open import、qualified type/record/ADT、private type/record/ADT、複数 moduleの forward visibility、`infer-recordlit` の既存 source-check blockerを含む standalone source-check全体、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は `:as` と `:only` の複合 import optionまたは `:open` unqualified export filteringを一つの REDに固定する。

### EC-M1-01 selfhost `import :as + :only` compound option slice (2026-07-25)

selfhost `Syntax.Parser` が canonical な `(import Lib :as L :only [helper extra])` を一つの import 宣言として連続消費し、`Syntax.AST` の6 slot（tag、module hash、module span、alias hash、only hash vector）へ alias と選択 symbol を同時に保持するようにした。`FormatterDecl` は両 optionを `:as L :only [helper extra]` の順で復元する。TypeInfer は同じ AST の alias prefix と only export filterを組み合わせ、選択された `L.helper` を解決し、除外された `L.hidden` を qualified lookupの明示的な失敗として扱う。

Evidence: RED の parser fixtureは optionを aliasだけとして扱い `8` node と残余 `only` tokenを返し、formatter fixtureは `:only` の残余を含む複数行へ崩れ、TypeInfer fixtureは除外された `L.hidden` も diagnostics `0` で通していた。GREEN は Rust parser oracleと module/alias/only/open=false を照合した parser 1件、canonical formatter 1件、Rust oracleへ `Lib.helper` / `Lib.hidden` の外部型を注入した TypeInfer 1件を passし、selected diagnostics `0`、excluded diagnostics `1` と failure-kind `0` を確認した。既存の import parser 3件、formatter 3件、TypeInfer関連回帰16件も全て passした。

これは canonical `:as` + `:only` compound importの parser AST保持、formatter roundtrip、public top-level functionの alias-qualified export filteringだけを閉じる verified sliceである。optionの逆順、`:open` / unqualified open import、qualified type/record/ADT、private type/record/ADT、複数 moduleの forward visibility、`infer-recordlit` の既存 `E0004 expected Vector, found Map (11255..11295)` source-check blocker、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は `:open` unqualified export filteringまたは qualified type/record/ADT importを一つの REDに固定する。

### EC-M1-01 selfhost `import :open` unqualified export filtering slice (2026-07-25)

selfhost `TypeInfer` が module 境界で先行 module の raw top-level `defn` 名を型環境から除去し、qualified import keyを保持するようにした。`(import Lib :open)` を処理した時だけ、依存 moduleの public top-level `defn` を source moduleの qualified keyから取得して raw nameへ戻す。これにより `:open` の public functionは `helper` として解決でき、通常の `(import Lib)` では `helper` を unqualified に参照できず、private wrapper内の `secret` は `:open` でも公開されない。`:open` ASTの only slot `0` も空選択として正規化し、既存の alias/direct qualified と `:only` filteringの経路を維持した。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_filters_import_open_unqualified_definition` は同じ flattened fixtureで `:open` なしの `helper` が diagnostics `0` になる誤った raw visibilityと、public/private境界を同時に固定した。GREEN は Rust oracleで public raw external type、closed import、private symbolの成功/失敗を確認し、selfhost native harnessで `:open` selected `diagnostics=0`、`:open` なし `diagnostics=1, failure-kind=1`、private `diagnostics=1` を確認した。既存の alias/direct qualified 2件、`:only` / `:as + :only` 3件、同一 moduleの private visibility 1件も passした。

これは `:open` による public top-level functionの unqualified visibilityだけを閉じる verified sliceである。`:open + :only`、`:as + :open`、同名 raw symbolの衝突方針、qualified type/record/ADT、private type/record/ADT、複数 moduleの forward visibility、`infer-recordlit` の既存 `E0004 expected Vector, found Map (11255..11295)` source-check blocker、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified type/record/ADT import、または open optionの組み合わせを一つの REDに固定する。

### EC-M1-01 selfhost `import :open + :only` option combination slice (2026-07-25)

selfhost `Syntax.Parser` の import option state に `open-present` を追加し、`open` の予約 token kind `49` と通常の option symbol kind `20` を同じ option parserで処理するようにした。`Syntax.AST` は `:open + :only` を `[26, module-hash, start, end, 0, only-hashes, 1]`、aliasを含む組み合わせを `[26, module-hash, start, end, alias-hash, only-hashes, 1]` として保持する。optionの入力順に依存せず、`FormatterDecl` は `:only [...] :open` へ canonicalizeする。TypeInferでは既存の `:only` export filtering と `:open` unqualified visibilityを同じ AST slotから適用する。

Evidence: RED `test_e2e_selfhost_parser_import_open_only` は `(import Lib :open :only [helper])` を import 1件ではなく8 nodeへ分解し、open tokenと only tokenを残した。RED `test_e2e_selfhost_formatter_preserves_import_open_only` は `"(import Lib)"` と残余 tokenの複数行へ崩れ、RED `test_e2e_selfhost_typeinfer_analysis_filters_import_open_only_unqualified_definition` は selected `helper` と excluded `hidden` の両方を diagnostics `0` で受理した。GREEN は Rust parser oracleで `alias=None`、`only=[helper]`、`open=true` を照合し、selfhost parser 1件、formatter 1件、TypeInfer selected/excluded 1件を passした。TypeInfer は selected `diagnostics=0`、excluded `diagnostics=1` を確認した。既存 import parser 5件、formatter 5件、TypeInfer filtering 4件も全て passした。

これは `:open + :only` の parser AST保持、formatter roundtrip、public top-level functionの unqualified export filteringだけを閉じる verified sliceである。`:as + :open` の独立回帰、optionの全順序、同名 raw symbolの衝突方針、qualified type/record/ADT、private type/record/ADT、複数 moduleの forward visibility、`infer-recordlit` の既存 `E0004 expected Vector, found Map (11255..11295)` source-check blocker、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified ADT constructor lookupまたは `:as + :open` を一つの REDに固定する。

### EC-M1-01 selfhost qualified ADT constructor export slice (2026-07-25)

selfhost `TypeInfer` の import export scan が、対象 module の ADT type declaration の variant constructorを raw constructor schemeから qualified keyへ写すようにした。`(import Lib)` の `Lib.Some` は module prefixと constructor hashから `ast-qualified-name-hash` を作り、既存の `:only` filtering と `:open` unqualified insertionを同じ経路で適用する。variantを持たない nominal type declarationは明示的に skipし、既存の function export scanと constructor registryを変更していない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_adt_constructor` は Rust oracleへ `Lib.Some : Int -> Lib.Option` を注入した同じ fixtureで、selfhostが `diagnostics=1, failureKinds=[1]` を返した。GREEN は selfhostの実 `Lib` ADT declarationと `(Lib.Some 42)` を同じ harnessで実行し、`diagnostics=0, failureKinds=[0]` を確認した。Rust oracle `Infer::infer_program` の成功、既存 qualified import resolve 4件、import export filtering 4件、parametric ADT 2件、Rust `test_import_open`、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInfer.ls` の diagnostics `0` も確認した。

これは public ADT constructorの module-qualified function-position lookupだけを閉じる verified sliceである。`:only` による constructor選択/除外の独立 RED、`:open` constructorの unqualified境界、alias-qualified ADT constructor、qualified type annotation、constructor pattern、record constructor/accessor、private type/ADT、複数 moduleの forward visibility、standalone source-check `0`、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified ADTの `:only` filteringまたは alias-qualified ADT constructorを一つの REDに固定する。

### EC-M1-01 selfhost alias-qualified ADT `:only` filtering contract (2026-07-25)

既存の qualified ADT export helper が `alias-hash` と `only-hashes` を同時に適用することを、`(import Lib :as L :only [Some])` の実 fixtureで確認した。`Lib` の `Some` と `Other` を同じ ADTへ登録し、`L.Some` は選択 exportとして解決し、`L.Other` は除外 exportとして拒否する。実装差分は不要で、先行した qualified ADT constructor sliceの filtering contractを追加で固定した。

Evidence: `test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_adt_constructor` は Rust oracleへ `Lib.Some : Int -> Lib.Option` / `Lib.Other : Bool -> Lib.Option` を注入し、selected fixtureが成功、excluded fixtureが失敗することを確認した。selfhost native harnessの結果は selected `diagnostics=0, failureKinds=[0]`、excluded `diagnostics=1, failureKinds=[1]` である。これは parser/formatterではなく、ADT variant schemeの alias-qualified export filtering boundaryだけを対象にする。

この contractで `:as` + `:only` の public ADT function-position lookupを閉じたが、`:open` constructorの unqualified境界、qualified type annotation、constructor pattern、record constructor/accessor、private type/ADT、複数 moduleの forward visibility、standalone source-check `0`、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified record constructor/accessorまたは constructor pattern importを一つの REDに固定する。

### EC-M1-01 selfhost qualified record constructor export slice (2026-07-25)

selfhost `TypeInfer` の import export scan に `ast-recorddef` を追加し、`TypeInferRecordDecl` が raw record nameへ登録した constructor schemeを module/alias prefix付き qualified keyへ写すようにした。`:only` filtering と `:open` raw insertionは既存の named export helperを共有する。対象は function-positionの `(Lib.Point 1 2)` だけで、record literal、field accessor、qualified type名は変更していない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_constructor` は Rust oracleへ `Lib.Point : Int -> Int -> Lib.Point` を注入した同じ fixtureで、selfhostが `diagnostics=1, failureKinds=[1]` を返した。GREENは selfhostの実 `Lib.Point` record declarationと `(Lib.Point 1 2)` を実行し、`diagnostics=0, failureKinds=[0]` を確認した。Rust parser/source oracle、qualified import resolve 5件（ADT/record含む）、import filtering 5件、parametric ADT 2件、Rust `test_import_open`、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInfer.ls` の diagnostics `0`、`git diff --check` を確認した。

これは public record constructorの module-qualified function-position lookupだけを閉じる verified sliceである。alias-qualified record constructor、`:only` record filtering、record field accessor、qualified record literal/type annotation、private record、constructor pattern、複数 moduleの forward visibility、standalone source-check `0`、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は alias-qualified record constructorまたは record field accessorを一つの REDに固定する。

### EC-M1-01 selfhost qualified record field accessor export slice (2026-07-25)

selfhost `TypeInfer` の record import export scan が、`TypeInferRecordDecl` の raw accessor scheme (`Point.x`) を対象 module prefix付き (`Lib.Point.x`) または alias prefix付き (`L.Point.x`) qualified keyへ写すようにした。record constructorの exportは既存 helperを維持し、field accessorだけは declaration field tripleの accessor hashを走査する。`:only [Point.x]` は accessor単位で selected / excluded を判定し、`:open` の場合だけ raw accessor keyも追加する。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_accessor` は Rust oracleへ `Lib.Point.x : Lib.Point -> Int` を注入した同じ fixtureで、selfhostが `diagnostics=1, failureKinds=[1]` を返した。GREEN は selfhostの実 `Lib.Point` record declarationと bare `Lib.Point.x` lookupを実行し、`diagnostics=0, failureKinds=[0]` を確認した。`test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_record_accessor` は `(import Lib :as L :only [Point.x])` で `L.Point.x` を受理し `L.Point.y` を拒否する Rust oracle / selfhost fixtureを確認し、selected `diagnostics=0, failureKinds=[0]`、excluded `diagnostics=1, failureKinds=[1]` を得た。focused `record_accessor` 2件、qualified import regression 3件、alias + `:only` regression 3件、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInfer.ls` の `decls:101 / diagnostics:0`、`git diff --check` が passした。

これは public record field accessorの module/alias-qualified function-position lookupと `:only` filteringだけを閉じる verified sliceである。qualified record literal/type annotation、unqualified `:open` accessorの独立 fixture、private record、constructor pattern、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified record constructorの alias + `:only` 契約、または record accessorの `:open` unqualified 境界を一つの REDに固定する。

### LEGACY-ROOT-01 stateful root slot invariant diagnostic slice (2026-07-25)

stateful REPL/LSP telemetry の Wasm backtrace `<wasm function 24>` が core WASI helper `root_set` に対応することを固定し、allocator (`__alloc`) / root-stack capacity (`root_push`) の `LS4002` と compiler-side safe-point spill の slot 不整合を分類上分離した。function 24 は trap text が省略された Wasmtime backtraceでも `LS4003: GC root slot の整合性が壊れました` へ分類し、元の backtraceを保持する。function 27 など user code の trap と Component Model の generic boundaryは変更していない。

Evidence: `cargo test -p lsharp-wasm wasi_runner::tests::test_classify_wasi_runtime_failure -- --nocapture`（5 passed）、`cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_error_reference_doc_mentions_all_mcp_error_codes -- --exact --nocapture`（1 passed）、`bash scripts/audit_docs.sh`（error 0 / warning 0）。128 MiB host/Wasmtime stackでも stateful REPL failureは function 24で再現したため、これは観測可能性の verified sliceであり、compiler safe-point ledger、REPL/LSP stateful runtime、Mac Apple Silicon / Linux x86_64 native stage0、`LEGACY-ROOT-01` aggregateの完了ではない。次は `root_push` が返す slot、`root_set` の更新対象、`root_pop` の lexical lifetimeを compiler-side ledger/contract testで固定する。

### LEGACY-ROOT-01 root_set failure ledger slice (2026-07-25)

`root_set` が `root_stack_top` より大きい slot を受け取り `unreachable` へ到達する直前に、runtime が要求 slot、観測時の root stack top、失敗回数を mutable global へ保存するようにした。WASI Preview1 では 3 globals を内部 export し、HTTP core でも同じ global layout と helper contract を保つ。これにより compiler-side safe-point spill の不整合を Wasmtime trap 後に slot/top/count として回収できる。

Evidence: RED `test_root_set_invalid_slot_records_failure_ledger_before_trap` は未実装時に failure ledger export の `get_global` が `None` で失敗した。GREEN は `(defn main [] (root_set 0 42))` で Wasmtime function 24 trap 前に `slot=0`、`top=0`、`count=1` を確認した。既存の `wasi_runner` classifier 5件も維持して passしている。

これは runtime の failure observability だけを閉じる verified sliceであり、compiler safe-point ledger、REPL/LSP stateful runtime、Mac Apple Silicon / Linux x86_64 native stage0、HTTP component runtime、`LEGACY-ROOT-01` aggregateの完了を意味しない。次は compiler が各 safe-point で生成する `root_push` slot、`root_set` target、`root_pop` lexical lifetime を同じ failure ledgerへ対応づける contract testである。

### EC-M1-01 selfhost alias-qualified record constructor `:only` contract (2026-07-25)

既存の record named export helper が `alias-hash` と `only-hashes` を同時に適用することを、`(import Lib :as L :only [Point])` の実 fixtureで固定した。`L.Point` は qualified constructorとして解決し、同じ moduleの `Hidden` は `:only` から除外して拒否する。実装差分は不要で、qualified record constructor export sliceの filtering contractを追加で検証した。

Evidence: `test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_record_constructor` は Rust oracleへ `Lib.Point : Int -> Int -> Lib.Point` / `Lib.Hidden : Int -> Int -> Lib.Hidden` を import visibility付きで注入し、selected fixtureが成功、excluded fixtureが失敗することを確認した。selfhost native harnessの結果は selected `diagnostics=0`、excluded `diagnostics=1` である。これは parser/formatterや record field accessorではなく、record constructor schemeの alias-qualified `:only` export boundaryだけを対象にする。

この contractで public record constructorの alias + `:only` function-position lookupを閉じたが、qualified record literal/type annotation、constructor pattern、private record、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了は未検証のまま残る。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified record type annotation / literal、または private record export filteringを一つの REDに固定する。

### EC-M1-01 selfhost `:open` record field accessor boundary slice (2026-07-25)

selfhost `TypeInfer` が module 境界で先行 record declaration の raw constructor/accessor schemeを除去し、import export scan中だけ record schemaから一時的に schemeを再構築するようにした。qualified key (`Lib.Point`, `Lib.Point.x`) は保持し、closed importでは raw keyを残さず、`:open` では `:only` の選択範囲に限って raw constructor/accessorを戻す。これにより record accessorの unqualified visibilityを function exportと同じ module boundaryへ揃えた。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_filters_import_open_record_accessor` は `:open` と closed importを同じ flattened fixtureで比較し、実装前は open/closedとも `diagnostics=0` となった。GREEN は open `Point.x` を `diagnostics=0`、closed `Point.x` を `diagnostics=1` と確認した。`test_e2e_selfhost_typeinfer_analysis_filters_import_open_only_record_accessor` も selected `Point.x` / excluded `Point.y` を同じ境界で確認した。既存の `import_open` 9件（parser / formatter / function visibility / `:open + :only` / record accessor）、focused `record_accessor` 4件（open + only含む）、qualified import 3件、alias + `:only` 3件、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInfer.ls` (`decls:101`, `diagnostics:0`)、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInferRecordDecl.ls` (`decls:34`, `diagnostics:0`)、`git diff --check` が passした。

これは record constructor/accessorの raw module visibilityと `:open` boundaryだけを閉じる verified sliceである。private record、qualified record literal/type annotation、constructor pattern、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は record constructorの alias + `:only` contract、または private record export filteringを一つの REDに固定する。

### EC-M1-01 selfhost qualified record type annotation slice (2026-07-25)

selfhost parser の dotted type name を、qualified value lookup と同じ `ast-qualified-name-hash` で保持するようにした。式 annotation は visible な qualified record constructor scheme (`Lib.Point`) の戻り record typeを参照し、`(Lib.Point 1 2)` と `Lib.Point` の annotationを同じ record typeとして unify する。constructor export、record literal、defn signature の qualified type annotationはこの変更で拡張していない。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_type_annotation` は Rust oracleへ `Lib.Point : Int -> Int -> Lib.Point` を注入した Main fixtureで、selfhostが `diagnostics=1` を返した。GREEN は qualified type hashの parser変更と visible constructor schemeの record-return projection後に `diagnostics=0` を確認した。qualified record accessor 3件、record accessor visibility 4件、alias + `:only` record constructor 1件、`cargo run --bin lsharp -- parse selfhost/src/Syntax/Parser.ls` (`decls:241`, `diagnostics:0`)、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInfer.ls` (`decls:103`, `diagnostics:0`)、`git diff --check` が passした。

これは module-qualified record の式 annotationだけを閉じる verified sliceである。qualified record literal、alias-qualified `L.Point` annotation、defn parameter/return signature、constructor pattern、private record、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified record literalの parser/oracle contractまたは private record export filteringを一つの REDに固定する。
### LEGACY-ROOT-01 compiler safe-point root lifetime ledger slice (2026-07-25)

Rust IR lowering に `crates/lsharp-ir/src/root_lifetime.rs` の抽象 ledger を接続し、`root_push` が返す
slot identity、`root_set` の active target、`root_pop` の lexical lifetime、structured branch の root depth を
codegen 前に検証するようにした。違反は `LS3003` として fail-closed にし、driver error table / error referenceへ登録した。

RED では selfhost Compiler.ls の `compile-user-call-arg-instrs-step-with-source` の不足 pop、
`compile-recordupdate-with-ftable` の過剰 pop、`register-adt-variants` と
`compile-let-with-ftable-impl-body-impl-3` の不足 popを ledger が実際の IR body で検出した。
修正後は selfhost nested map safe-point fixtureで、生成 IRの local slotを追跡しながら `root_set` 前後と関数 exit
の root depth 0 を確認した。

Evidence: `cargo test -p lsharp-ir --lib`（255 passed）、
`test_e2e_selfhost_compiler_root_lifetime_ledger_tracks_nested_map_safe_point`（1 passed）、
`git diff --check`。rooting parity の並列全件は multifile stateful fixtureの stack overflowで終了したため、
REPL/LSP stateful runtime、Mac/Linux native stage0、全 control-flow / indirect-call coverage、
runtime failure ledgerとの actual slot/top/count differentialは未完了である。TODOの `[~]` と Rust/bootstrap/host境界を維持する。

### EC-M1-01 selfhost qualified record literal schema slice (2026-07-25)

Rust parser が dotted record name (`{Lib.Point ...}`) を record literal として受理し、selfhost parser も record type hashを qualified export keyへ揃えるようにした。selfhost の qualified record literal は visible な constructor schemeの戻り record typeを schemaとして使い、field count と各 field 型を宣言 schemaへ unify する。未可視名は従来の未宣言 literal fallbackを維持し、record visibility全体をこの変更で再設計していない。

Evidence: RED は `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_literal` の Rust parserで dotted record nameが拒否され、parser修正後は Rust oracleの qualified declarationと selfhost の import fixtureを受理した。GREEN は selected `{Lib.Point x 1 y 2}` の diagnostics `0` と、invalid `{Lib.Point x true y 2}` の diagnostics `1` を同じ native harnessで確認した。qualified record regression 4件、record accessor regression 4件、`cargo run --bin lsharp -- parse selfhost/src/Syntax/Parser.ls` (`decls:241`, `diagnostics:0`)、`cargo run --bin lsharp -- parse selfhost/src/Types/TypeInferRecord.ls` (`decls:18`, `diagnostics:0`)、`git diff --check` が passした。

これは module-qualified record literalの parser と field schema validationだけを閉じる verified sliceである。alias-qualified `L.Point` literal、unqualified `:open` literal、record update、parametric record literal、constructor pattern、private record、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。Rust oracleは flattened module importの既存境界を避ける qualified record declarationで型を構築しており、import visibility parityの証拠とは分離して扱う。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は alias-qualified record annotation/literalまたは private record export filteringを一つの REDに固定する。

### LEGACY-ROOT-01 selfhost type inference root balance slice (2026-07-25)

最新の compiler-side root lifetime ledger が検出した selfhost TypeInfer の underflow を、余分な `root_pop` の除去と不足していた ADT variant loop の `root_pop` 追加で修正した。対象は `typeinfer-finalize-defn-result-with-env-vars`、`infer-var`、`typeinfer-program-analysis-state-base`、`typeinfer-register-adt-variants-loop` の root push/pop 対称性であり、型推論の意味論や Rust fallback の挙動は変更していない。

Evidence: `test_e2e_selfhost_typeinfer_finalize_defn_root_lifetime_is_balanced`、`test_e2e_selfhost_infer_var_root_lifetime_is_balanced`、`test_e2e_selfhost_typeinfer_program_analysis_state_base_root_lifetime_is_balanced`、`test_e2e_selfhost_typeinfer_register_adt_variants_root_lifetime_is_balanced` は全て passした。加えて `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_literal` が root ledger の underflow / branch-depth failureなしに selected `diagnostics=0`、invalid `diagnostics=1` を確認した。

これは selfhost TypeInfer の4関数と qualified record literal fixtureに対する current Rust oracle/native harness の回帰修正であり、全 selfhost sourceの root lifetime parity、stateful REPL/LSP、Mac Apple Silicon / Linux x86_64 native stage0、Wasm artifact/runtime、`LEGACY-ROOT-01` aggregateの完了を意味しない。残る不均衡候補は対象 fixtureで実行された範囲に限定して次の RED で切り出す。

### EC-M1-01 selfhost alias-qualified record literal visibility slice (2026-07-25)

selfhost Parser は named record literal の既存 field index を維持したまま、末尾 marker で type name が module/alias-qualified かを保持するようにした。TypeInferRecord は visible な alias-qualified constructor schemaを使う場合だけ field 型検査を行い、`:only [Point]` で除外された `L.Hidden` は未知 record fallbackへ落とさず明示的な診断へ進める。unqualified と anonymous record literal の既存 fallbackは維持する。

Evidence: RED `test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_record_literal` は selected `L.Point` と excluded `L.Hidden` をともに diagnostics `0` で受理していた。GREEN は selected `diagnostics=0`、excluded `diagnostics=1` を Rust oracleの可視 registry縮約と同じ fixtureで確認した。`test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_record_literal`、既存 qualified record regression 5件、`test_e2e_selfhost_parser_record_literal` も passした。

これは alias-qualified record literal の `:only` visibility と field schema validationだけを閉じる verified sliceである。unqualified `:open` literal、record update、parametric record literal、qualified/alias-qualified annotation、constructor pattern、private record、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次は qualified record annotationの alias parityまたは private record export filteringを一つの REDに固定する。
### EC-M1-01 selfhost alias-qualified record type annotation/signature slice (2026-07-25)

selfhost `TypeInferFunctions` の defn signature resolverに、値環境へ登録済みの named record constructor schemeを参照する narrow pathを追加した。`L.Point` のような alias-qualified named typeは constructor schemeを instantiateし、その戻り record typeを param/return annotationの unifyへ渡す。未登録名、非-record scheme、nested `TypeApp` は既存の nominal/alias resolverへ戻し、record全体の型名解決を広げていない。

RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_record_defn_signature` は、`(import Lib :as L :only [Point])` と `(defn make [] : L.Point (L.Point 1 2))` で実装前の selected diagnostics `1` を確認し、invalid constructor field mismatch は `1` を維持した。GREEN は signature resolverが visible constructorの戻り record typeを使うようにした後、selected `diagnostics=0`、invalid `diagnostics=1` を同じ selfhost harnessで確認した。式 annotationの alias + `:only` contract `test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_only_qualified_record_type_annotation` は selected `0`、mismatch `1` を確認し、qualified record regression 7件も再実行して passした。`TypeInferFunctions.ls` / `TypeInfer.ls` の parser diagnosticsはともに `0`、`git diff --check` も passした。

これは direct named record の alias-qualified expression annotationと defn return signatureだけを閉じる verified sliceである。nested `TypeApp` 内の record、constructor pattern、private record、record update、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。次は nested record type expressionまたは private record export filteringを一つの REDに固定する。

### EC-M1-01 selfhost nested alias-qualified record function signature slice (2026-07-25)

`TypeInferFunctions` の signature resolverを `TypeFun` の raw parameter/return expressionへ再帰させ、`(-> L.Point Int)` の `L.Point` を visible な record constructor schemeの戻り record typeへ解決するようにした。`(fn [point] (L.Point.x point))` の bodyと組み合わせることで、nominal hashだけを比較する実装では通らない parameter unifyを固定している。未登録名、非-record scheme、nested `TypeApp`、constructor pattern、private record、record updateは既存の境界を維持する。

RED `test_e2e_selfhost_typeinfer_analysis_resolves_nested_alias_qualified_record_signature` は、Rust oracleが受理する `(defn get-x [] : (-> L.Point Int) ...)` を selfhostへ渡し、実装前の selected fixtureで diagnostics `1`、return annotationを `Bool` にした invalid fixtureで diagnostics `1` を確認した。GREEN は recursive `TypeFun` resolver追加後に selected `0`、invalid `1` を同じ harnessで確認した。qualified record regression 8件、typed defn signature regression 6件（`RUST_MIN_STACK=33554432`）、`cargo run --quiet --bin lsharp -- parse selfhost/src/Types/TypeInferFunctions.ls` (`diagnostics:0`)、`git diff --check` が passした。

これは direct nested `TypeFun` 内の alias-qualified record parameter/returnだけを閉じる verified sliceである。nested `TypeApp`、constructor pattern、private record、record update、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。次は Rust oracleとの契約が成立する nested `TypeApp` または別の未対応 type-expression boundaryを一つの REDに固定する。

### EC-M1-01 selfhost nested alias-qualified record TypeApp signature slice (2026-07-25)

signature 専用 resolverに `TypeApp` argumentの再帰経路を追加し、`(Ref L.Point)` の `L.Point` を visible な record constructor schemeの戻り record typeへ解決するようにした。outer app nameの正規化と parametric alias expansionは既存 helperを再利用し、unknown nameや non-record schemeの fallbackは変えていない。`(L.Point.x (ref-get point))` を同じ fixtureへ置くことで、`Ref` の inner typeを nominal constructorのまま扱う実装では通らない schema unifyを検証した。

RED `test_e2e_selfhost_typeinfer_analysis_resolves_nested_alias_qualified_record_type_app_signature` は、Rust oracleが受理する `(defn read-x [(: point (Ref L.Point))] : Int (L.Point.x (ref-get point)))` で実装前の selected diagnostics `1`、return annotationを `Bool` にした invalid fixtureで diagnostics `1` を確認した。GREEN は signature TypeApp argument recursion追加後に selected `0`、invalid `1` を同じ harnessで確認した。qualified record regression 9件、typed defn signature regression 6件（`RUST_MIN_STACK=33554432`）、`cargo run --quiet --bin lsharp -- parse selfhost/src/Types/TypeInferFunctions.ls` (`diagnostics:0`)、`git diff --check` が passした。

これは direct nested `TypeApp` の alias-qualified record argumentだけを閉じる verified sliceである。さらに深い TypeApp、constructor pattern、private record、record update、複数 moduleの forward visibility、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。次は同じ Rust oracle contractの範囲でさらに深い TypeAppを追加するか、constructor patternの record schema boundaryを一つの REDに固定する。

### EC-M1-01 selfhost alias-qualified record pattern slice (2026-07-25)

Parserの record pattern type hashを通常の symbol hashから `current-type-name-hash-v3`へ揃え、`{L.Point ...}` を `ast-qualified-name-hash(L, Point)` として保持するようにした。TypeInferPatternは raw record-envに qualified keyがない import境界でも、envの visible constructor schemeを instantiateして戻り record typeを schemaとして使う。unknown keyは従来どおり明示的な undefined errorにし、record pattern全体の runtime/lowering parityは広げていない。

RED `test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_record_pattern` は、Rust oracleが受理する `{L.Point x x}` で実装前の selected diagnostics `1`、field mismatchを含む `{L.Point x true}` で diagnostics `1` を確認した。GREENは record patternの qualified hash保持と visible constructor schema fallback追加後に selected `0`、invalid `1` を同じ harnessで確認した。qualified record regression 10件、record pattern parser 2件、`cargo run --quiet --bin lsharp -- parse selfhost/src/Syntax/Parser.ls` (`decls:242 / diagnostics:0`)、`cargo run --quiet --bin lsharp -- parse selfhost/src/Types/TypeInferPattern.ls` (`decls:23 / diagnostics:0`)、`git diff --check` が passした。

この時点の actual compiler-mode/ftable record-pattern regressionは `RUST_MIN_STACK=33554432` で実行しても `load-imports-from-decls-step` instruction 78の root ledger `BranchDepthMismatch` (`then_depth=6`, `else_depth=7`) に到達した。変更なしの `origin/main` baselineでも同じ failure valueを再現したため、これは今回の parser/TypeInferPattern差分起因ではない既存 blockerとして履歴化した。後続の root lifetime修正で actual compiler-modeまで進められるようになり、qualified nominal markerの mismatchは次の runtime sliceで別の REDとして確定した。この時点の sliceは qualified record patternの parser/type-inference境界だけを閉じたものであり、constructor pattern、private record、record update、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。

### EC-M1-01 selfhost alias-qualified record pattern `:only` visibility slice (2026-07-25)

`(import Lib :as L :only [Point])` の export filteringと record pattern schema lookupを同じ fixtureで固定した。`{L.Point x x}` は visible constructor schemeの戻り record typeから field schemaを取得して diagnostics `0`、`:only` から除外された `{L.Hidden x x}` は qualified keyが環境へ登録されず diagnostics `1` になる。Rust oracleは flattened selected `L.Point` registryで selectedを受理し、`L.Hidden` の未登録 patternを拒否する。

Evidence `test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_qualified_record_pattern` は Rust oracleの selected/rejected結果と selfhost native harnessの `0/1` を一致させた。これは alias-qualified record patternの `:only` visibilityだけを閉じる evidence-only sliceであり、実装差分は不要だった。record pattern runtime/lowering、constructor pattern、private record、record update、actual compiler-mode/ftable gate、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。次は qualified record updateの type-inference contractを一つの REDに固定する。

### EC-M1-01 selfhost qualified record update type-inference slice (2026-07-25)

qualified record annotationで解決した `L.Point` parameterを record updateの baseへ渡し、更新 fieldの schema unifyまで同じ fixtureで固定した。`{point | x 42}` は selected diagnostics `0`、`{point | x true}` は expected `Int` との mismatchで diagnostics `1` になる。Rust oracleも flattened `L.Point` record declarationを使って selected/rejectedを同じ判定にする。

Evidence `test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_record_update` は Rust oracleと selfhost native harnessの `0/1` を一致させた。これは type-inference boundaryだけを閉じる evidence-only sliceであり、actual compiler-mode/ftable import-qualified update、record update runtime/lowering parity、standalone native stage0、Wasm artifact/runtime、Mac Apple Silicon / Linux x86_64 のこの変更後 current-source native gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。次は actual compiler-modeの import-qualified record updateを一つの REDに固定する。

### EC-M1-01 selfhost qualified record update compiler-mode/runtime slice (2026-07-25)

actual compiler-mode fixtureの RED で検出した `CompilerMode` の import traversal / compile probe の root push/pop 不均衡を、`load-imports-from-decls`、check、cache、progress、warm-target parity の局所差分として修正した。root ledger の branch depth / underflow verifierを通過した後、codegenでは qualified var node の source 全体 hashが raw ftable export lookupから外れた場合だけ suffix export hashへ解決する helperを追加し、alias + `:only [Point]` の record constructorを生成 helperへ接続した。raw lookupは先に維持し、未検出時に suffixもない場合は従来どおり target `0` を返すため、未解決名を成功扱いにしていない。

Evidence: `test_e2e_selfhost_compiler_mode_imported_record_update_runs` は alias-qualified `(S.Point 40 2)` と `{point | x 41}` を actual generated Wasmで実行し `41\n2\n` を返した（75.20s）。`test_e2e_selfhost_compiler_mode_imported_record_constructor_and_static_accessor_run` は unqualified imported constructor/accessorの既存 `41\n2\n` を再確認した（70.73s）。`cargo test -q -p lsharp-wasm --test e2e selfhost_compiler_mode_imported_record_update_runs --no-run`、`git diff --check` も passした。

これは alias-qualified record constructor callと record updateの source compiler-mode runtimeに対する verified sliceである。ftable direct alias/import target、複数 moduleの同名 export衝突、record pattern runtime、standalone native stage0、Wasm artifact/runtimeの Mac Apple Silicon / Linux x86_64 parity、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。次の ftable alias-qualified function sliceで、flat export/importの actual runtime contractを固定する。

### EC-M1-01 selfhost alias-qualified ftable function call slice (2026-07-25)

legacy `program-functions-base` の flat declaration sequenceに、`inc` の exportと `App.Math :as M :only [inc]` の importを置き、`(M.inc 41)` を actual Wasmで実行する fixtureを追加した。qualified var nodeの raw source hashが ftableの raw export hashに一致しない場合だけ suffix hashへ fallbackする `ftable-lookup-call-target` を source/ftable user-call両経路で共有するため、unqualified callの既存 lookup順序は変えていない。module declaration内の定義はこの legacy entryが flattenしないため、fixtureは意図的に flat scopeへ限定している。

REDは helperを除いた root-balanced comparison checkoutで `65536\n` となった。これは `M.inc` が未解決の target `0`、すなわち runtime `__alloc(41)` の戻り値を出力した値である。GREENは current `test_e2e_selfhost_ftable_compiler_alias_qualified_function_call_runs` で `42\n` を確認した。`RUST_MIN_STACK=33554432 cargo test -q -p lsharp-wasm --test e2e selfhost_ftable_compiler_alias_qualified_function_call_runs -- --nocapture` は current 129.44s、comparison 69.13s、`git diff --check` は passした。

これは flat legacy ftable compilerの alias-qualified function callだけを閉じる verified sliceである。CompilerModeの module/file import graph、module declaration flatten、qualified name collision防止、record pattern/updateの ftable import runtime、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 selfhost alias-qualified record pattern runtime slice (2026-07-25)

record constructorが raw `Point` hashを nominal markerへ格納する一方、`{S.Point ...}` の parser ASTは TypeInferの可視性 lookup用に `ast-qualified-name-hash(S, Point)` を保持するため、runtime nominal checkだけでは markerが一致しなかった。Parserは record pattern末尾に full qualified hashと raw suffix hashを並べて保持し、TypeInferは従来どおりfull hashを使い、Wasm compilerの nominal marker照合だけraw suffix hashを使う狭い分離にした。旧ASTにraw suffixがない場合はfull hashへfallbackする。

REDは `test_e2e_selfhost_compiler_mode_imported_alias_qualified_record_pattern_runs` で `0\n`、`test_e2e_selfhost_parser_match_qualified_record_pattern_retains_raw_type_hash` で `1\n0` となった。GREENは parser focused testの `1\n1`、`test_e2e_selfhost_compiler_mode_imported_alias_qualified_record_pattern_runs` の `41\n`（75.55s）、`test_e2e_selfhost_ftable_compiler_alias_qualified_record_pattern_runs` の `41\n`（70.33s）で確認した。CompilerModeは `App.Shapes` の `Point` を `App.Main` から alias + `:only [Point]` で importし、flat ftableは同じ alias contractを `compile-program-functions-with-base` から実行する。`cargo run -q --bin lsharp -- parse selfhost/src/Syntax/Parser.ls`（`decls:243 / diagnostics:0`）、`cargo run -q --bin lsharp -- parse selfhost/src/Backend/Wasm/CompilerSplit.ls`（`decls:47 / diagnostics:0`）、CompilerMode testの `--no-run`、qualified record TypeInfer/`:only` regression、`git diff --check` も passした。

これは alias-qualified record patternの nominal marker runtimeと source/file + flat ftableの二経路だけを閉じる verified sliceである。constructor pattern、private record、record update全形式、同名 export衝突、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 selfhost alias-qualified ftable record update runtime slice (2026-07-25)

qualified constructor codegenの suffix export lookupと record updateの patch/base fallbackを同じ flat ftable fixtureへ接続し、`(S.Point 40 2)` を `{point | x 41}` へ更新して field accessを実行した。CompilerMode file-import側の `41\n2\n` に加え、`compile-program-functions-with-base` の ftable側でも alias + `:only [Point]` の actual Wasm結果 `41\n2\n` を確認した。実装差分は追加していない。

Evidence: `test_e2e_selfhost_ftable_compiler_alias_qualified_record_update_runs` は `RUST_MIN_STACK=33554432` で 70.28s、stderrなしで passした。これは flat ftable alias-qualified record updateの constructor/patch/base runtimeだけを閉じる verified sliceであり、record update全形式、複数 moduleの同名 export衝突、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 selfhost alias-qualified record literal marker runtime slice (2026-07-25)

qualified record literal `{S.Point x 41 y 2}` は TypeInferの可視性 lookup用 full qualified hashと、record constructorの runtime nominal marker用 raw `Point` hashを分けて保持する必要がある。Parserは既存の qualified flagを維持したままraw suffix hashを末尾へ追加し、`compile-recordlit-with-source` / `compile-recordlit-with-ftable` は新しい raw hashを markerへ書く。旧ASTは record nodeの type hashへfallbackする。

REDは `test_e2e_selfhost_ftable_compiler_alias_qualified_record_literal_pattern_runs` で literalから `{S.Point ...}` patternへ渡した結果が `0\n`、`test_e2e_selfhost_parser_qualified_record_literal_retains_raw_type_hash` で `1\n1\n0` となった。GREENは parser focused testの `1\n1\n1`、ftable actual Wasmの `41\n`（70.54s）、CompilerMode file-import actual Wasmの `41\n`（68.89s）で確認した。qualified record TypeInfer/`:only` regressionと既存 record pattern runtimeも維持する。

これは flat ftableの qualified record literal markerと pattern nominal checkだけを閉じる verified sliceである。CompilerMode file-import literal、parametric/private record、record update全形式、同名 export衝突、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界は維持する。

### EC-M1-01 selfhost alias-qualified parametric record literal/pattern runtime slice (2026-07-25)

既存の raw nominal marker経路を、`(type (Box a) (record (: value a)))` の parametric recordへ concrete instantiationとして適用した。`App.Shapes :as S :only [Box]` の `{S.Box value 41}` literalを `{S.Box value x}` patternへ渡し、flat ftable compilerとCompilerMode file-import compilerの両方で生成した Wasmを実行して `41\n` を確認した。flat ftableでは同一 fixtureに `Int` と `Bool` を置いた複数 instantiationも実行し、`41\n1\n` を確認した。これは literalの qualified visibility用 full hashと、runtime nominal marker用 raw `Box` hashを分離した実装が、non-parametric `Point` 以外でも concrete typeごとに同じ境界を保つことを確認する。

REDは `test_e2e_selfhost_ftable_compiler_alias_qualified_parametric_record_literal_pattern_runs`、`test_e2e_selfhost_compiler_mode_imported_alias_qualified_parametric_record_literal_pattern_runs`、`test_e2e_selfhost_ftable_compiler_alias_qualified_parametric_record_multiple_instantiations_run` の追加前にはこの concrete instantiationに actual Wasm evidenceがなく、parametric marker parityは未検証だった。GREENは前者を `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-wasm --test e2e selfhost_ftable_compiler_alias_qualified_parametric_record_literal_pattern_runs -- --nocapture` で実行し `41\n`、1 passed、85.14s、file-import testを同じ `cargo test` filterで実行し `41\n`、1 passed、69.35s、複数 instantiation testを実行し `41\n1\n`、1 passed、69.50sを確認した。

これは flat ftableの parametric `Int`/`Bool` 複数 instantiationとCompilerMode file-importの `Int` instantiationを閉じる verified sliceである。CompilerMode file-import側の複数 concrete instantiation、private record、record update全形式、同名 export衝突、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。

### EC-M1-01 selfhost alias-qualified same-name record constructor collision slice (2026-07-25)

異なる module の同名 record constructorを raw suffix hashだけで解決すると、`L.Point` と `R.Point` が同じ ftable entryへ落ちる。`App.Left.Point` は `x` だけ、`App.Right.Point` は `x`/`y` の2 fieldという異なる arityの file-import fixtureでこの衝突を固定した。

RED `test_e2e_selfhost_compiler_mode_imported_alias_qualified_same_name_record_constructors_run` は、修正前に `L.Point` が `R.Point` の constructor indexへ誤解決され、Wasm translationが `offset 2929: type mismatch: expected i64 but nothing on stack` で失敗した。GREENは record preludeへ module-qualified constructor keyを追加し、CompilerModeの importごとに alias-qualified keyを登録したうえで、`ftable-lookup-call-target` の lookup順を raw full hash → `ast-qualified-name-hash(prefix, suffix)` → raw suffixへ変更し、同テストで `41\n2\n3\n`、1 passed、84.04sを確認した。既存の flat ftable alias-qualified function call `42\n` と CompilerMode qualified record literal/pattern `41\n` も再実行して passした。

これは CompilerMode file-import の alias-qualified record constructor collision一例だけを閉じる verified sliceである。same-name static accessor、unqualified同名 exportの曖昧性方針、private record、record update全形式、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。cross-module nominal markerの pattern境界は後続の verified sliceで固定した。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。

### EC-M1-01 selfhost cross-module same-schema record nominal marker slice (2026-07-25)

異なる module の同名 recordが同じ schemaでも、runtime nominal markerが raw suffix `Point` だけでは `L.Point` の値へ `R.Point` patternが誤って一致する。CompilerMode file-import fixtureで `App.Left.Point` と `App.Right.Point` をそれぞれ alias importし、同じ `x: Int` fieldを持つ constructor/patternを実行境界まで固定した。

RED `test_e2e_selfhost_compiler_mode_imported_alias_qualified_same_schema_record_patterns_are_nominal` は、修正前に `(L.Point 41)` と `{R.Point x x}` の matchが `1\n` となった。GREENは record constructorの markerを module-qualified hashへ変更し、CompilerMode import alias登録時に `alias-qualified key -> canonical module marker` を既存 ftableへ追加した。record literal/pattern compilerは marker mappingを優先し、flat ftableと旧ASTでは従来の raw hashへfallbackする。同テストは `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-wasm --test e2e selfhost_compiler_mode_imported_alias_qualified_same_schema_record_patterns_are_nominal -- --nocapture` で `0\n`、1 passed、69.49sとなった。追加した `test_e2e_selfhost_compiler_mode_imported_alias_qualified_same_schema_record_literals_are_nominal` は `{L.Point x 41}` の同 alias patternを `41\n`、`R.Point` patternを `0\n` とし、1 passed、72.89sを確認した。CompilerMode alias-qualified record関連の既存回帰5件も同じ focused filterで `5 passed`、138.82sを確認した。

これは同名 recordの cross-module nominal constructor/literal/pattern境界を閉じる verified sliceであり、same-name static accessor、unqualified同名 exportの曖昧性方針、private/local visibility、record update全形式、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。

### EC-M1-01 selfhost same-name record static accessor ftable target slice (2026-07-25)

record accessorは raw `Point.x` function keyだけでは、異なる moduleの同名 recordで alias-qualified targetを区別できない。record preludeに `module-qualified(accessor)` keyを登録し、CompilerModeの importごとに `alias-qualified(accessor) -> target index` を追加する。`:only [Point.x]` では constructor aliasを登録せず、accessor exportだけを登録する。

RED `test_e2e_selfhost_compiler_mode_imported_alias_same_name_record_accessor_ftable_keys_are_separate` は、`App.Left.Point.x` / `App.Right.Point.x` の `L` / `R` alias keyが未登録で `0` になった。GREENは module-qualified accessor keyと alias keyを分離して ftableへ登録し、両 targetが存在し異なる indexであることを `1` として確認した（1 passed、77.11s）。CompilerMode alias-qualified record 6件と flat ftable alias-qualified record 6件も再実行してすべて passした。

これは static accessorの ftable target分離だけを閉じる verified sliceであり、static accessor actual runtimeの nominal guard、unqualified同名 exportの曖昧性方針、private/local visibility、record update全形式、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。

### EC-M1-01 selfhost imported private record export filtering slice (2026-07-25)

`(private (type Secret (record (: x Int))))` を含む `Lib` から `:as L :only [Secret]` で importした `{L.Secret x 1}` を、selfhost TypeInferが公開 environmentへ漏らさず拒否する境界を追加した。Rust oracleは現在 private typeの同一 module local visibilityを表現できないため、oracle側は `Secret` を公開 registryへ注入しない synthetic sourceで qualified lookup拒否を確認し、selfhost側は private wrapperを含む実 sourceを解析する。

Evidence `test_e2e_selfhost_typeinfer_analysis_filters_imported_private_record` は Rust oracleの reject と selfhostの diagnostics `1` を確認した（17.54s）。これは export filteringの evidence-only sliceであり、private recordを同一 module内で使う type-inference contractや constructor/literal/pattern runtimeを実装完了した証拠ではない。

同一 module内の private record local visibility、private record constructor/literal/pattern、同名 export collisionの nominal marker、record update全形式、standalone native stage0、Mac Apple Silicon / Linux x86_64 の current-source artifact/runtime gate、EC-M1-01 aggregateは未完了のまま残す。`TODO.md` の `[~]` と Rust oracle / bootstrap / host integration境界を維持する。

### EC-M1-01 selfhost imported record update compiler-mode initial RED (2026-07-25)

actual Wasm runtimeの最小 fixtureとして `App.Shapes.Point` を `App.Main` から alias + `:only [Point]` で importし、`(S.Point 40 2)` を `{point | x 41}` へ updateして `(. updated x)` / `(. updated y)` を出力する testを追加した。初回は `RUST_MIN_STACK=33554432` で `load-imports-from-decls-step` instruction 78の root ledger `BranchDepthMismatch` (`then_depth=6`, `else_depth=7`) に到達し、Wasm runtimeへ進めなかった。同じ failure valueは変更なし `origin/main` の record-pattern compiler-mode baselineでも再現したため、fixtureの parser/type-inference差分とは分離した。

この RED は後続の `EC-M1-01 selfhost qualified record update compiler-mode/runtime slice` で解消済みである。現在は root lifetime修正と qualified constructor codegenを actual source compiler-mode runtimeまで検証済みだが、ftable direct alias/import target、artifact bytesの二 target parity、standalone native stage0、EC-M1-01 aggregateは未完了である。

### Current-source Mac Apple Silicon native stage0 smoke (2026-07-25)

source commit `f3e63270fb70d5a47a4e4ec4fe0ed60422950cf2` を checkoutした専用 worktreeで、`LSHARP_NATIVE_PROXY_KEEP_FAILED_DIR=1 CARGO_TARGET_DIR=... cargo test -q -p lsharp-wasm --test e2e e2e::selfhost_native_stage_chain::test_e2e_stage23_actual_native_self_regeneration_harness_stage2_stage3_match -- --exact --ignored --nocapture` を実行した。actual stage23は `1 passed`、`435.38s` で完了し、保存した `actual-stage3-native/program.native` は Mach-O arm64の低レベル compilerとして `src/App/Seed.ls` を直接受け付け、stderrなしで transport outputを返した。

このstage3 compilerを `scripts/ci/package-native-stage0.sh` で `aarch64-apple-darwin` stage0へ包み、`NATIVE_STAGE0_DIR=... NATIVE_SELFHOST_STAGE_DIR=... bash scripts/ci/native-selfhost-dev-source-file-smoke.sh` を実行した。smokeは `cargo`、`rustc`、host `lsharp` をPATH上で遮断し、`parse`、`check`、`fmt`、`test`、metadata/property test、`compile`、`build`、および明示的拒否ケースを通過して `aarch64-apple-darwin native selfhost source-file smoke passed` となった。source fingerprintとstage0 fingerprintも生成され、current sourceからの再生成と package入力の一致を確認した。

最初に `App.Cli` release programをstage0 compilerとして渡した packageは、`src/App/Cli.ls` を command名として解釈して exit 127 (`unknown command`) になった。これは実装失敗ではなく成果物の役割を取り違えたためである。以後、App.Cli配布 binaryと、source pathを直接受け付けるactual stage3 compilerを別の成果物として扱う。この検証はMac Apple Siliconのcurrent-source daily sliceだけを閉じるもので、Linux x86_64のcurrent-source stage0、Wasm artifact/runtimeの二 target parity、全公開surface、EC-M1-01 aggregateの完了を意味しない。

### EC-M3-03 EmbeddedCli manifest output wiring (2026-07-27)

`App.EmbeddedCli` の `validate --source --format json --emit-manifest` を、`App.Cli` と同じ
`validation-source-manifest-json` serializer と `write-file` builtinへ接続した。reportは stdout の
1 JSON lineを保ち、version 1 manifestの nodes/evidence/edges を指定 pathへ出力し、trace gap
を含む fixtureでは `unknown` / exit `2` を維持する。旧 `external-boundary:embedded-cli-manifest-output`
診断はこの output wiringで superseded された。

Evidence: RED `test_e2e_selfhost_embedded_cli_validate_source_emits_manifest` は実装前に
manifest fileが存在せず失敗した。GREEN は同じ test が `1 passed`（255.11s）。さらに既存の
`test_e2e_selfhost_embedded_cli_validate_source_reports_fail` / `...reports_pass` を直列で実行し、
2 passed（509.38s）で status/exit regression がないことを確認した。`bash scripts/audit_docs.sh`
は error/warning 0件、`git diff --check` も passした。

これは Rust-host actual Wasm の EmbeddedCli writer/output boundaryだけを閉じる verified sliceで
あり、native stage0 parity、atomic/durable replacement、write/provenance failure、MCP parity、
Mac Apple Silicon / Linux x86_64 artifact/runtime matrix、EC-M3 aggregateの完了を意味しない。

### EC-M3-03 EmbeddedCli manifest write failure boundary (2026-07-27)

`EmbeddedCli` の `validate --source --format json --emit-manifest` は、manifest の親ディレクトリ
など filesystem write が失敗した場合に report を出力せず、`source validation manifest write
failed` と exit `1` を返す fail-closed 契約へ揃えた。成功した write の後だけ report を stdoutへ
出すため、validation status と artifact write failure を混同しない。

Evidence: RED `cargo test -p lsharp-wasm --test e2e
selfhost_cli_manifest_output::test_e2e_selfhost_embedded_cli_validate_source_rejects_manifest_write_failure
-- --nocapture` は実装前に exit `2` と report outputを返して失敗した（`0 passed; 1 failed`,
252.02s）。GREEN は同じ実 Wasm fixtureで `1 passed`（254.32s）となり、diagnostic、exit `1`、
`"status"` の不在、manifest file の不在を確認した。

これは Rust-host actual Wasm の write-error boundaryだけを閉じる verified sliceであり、native
stage0 の atomic/durable replacement、source/release provenance、MCP parity、Mac Apple Silicon /
Linux x86_64 artifact/runtime matrix、EC-M3 aggregateの完了を意味しない。

### LEGACY-LANG-01 selfhost bounded record field inference slice (2026-07-31)

Linux x86_64 native self-hostingで大きな recordを処理するとき、`TypeInferRecord.ls` に残っていた
record field/value inference、record literal field lookup、declared record literal/update inferenceの
直接再帰 scanを、64要素の bounded loopとchunk境界の rooted continuationへまとめて移行した。一要素の
stepとchunk単位の再帰を分離し、既存の result/substitution semanticsとroot lifetimeを維持する。
`TypeInferRecord.ls` は移行後も731行で、500〜800行のファイルサイズ境界内に収まる。

Evidence: `test_e2e_selfhost_typeinfer_record_inference_uses_bounded_chunks` と
`test_e2e_selfhost_typeinfer_large_record_inference_preserves_results` の2 tests、既存
`selfhost_typeinfer_records_computation` の10 tests、`git diff --check`、Rust test fixtureの
`rustfmt --edition 2021 --check` が passした。既存の
`test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type` は変更前の baseline と同じく
`left "1" right "0"` で失敗するため、今回の bounded scanが record schema pattern parityを解決したとは扱わない。

source commit `a2e609f4` に対して、`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=a2e609f4-typeinfer-record-inference`
の local Lima `lsharp-linux-x86` gateを一度だけ実行した。host-generated stage1 x86 payloadがLinux
x86_64 VM内でstage2/stage3 native self-regenerationを完走し、summaryは `status=pass`、stage2/stage3
code lengthは双方 `11168596`、stdout SHA256は双方
`dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。証跡は
`ci-artifacts/native-linux-x86-hostgen-vm/a2e609f4-typeinfer-record-inference/actual-selfregen-summary.json`
へ summaryだけを残し、11 MiB級 stdout、stage debug、stage1中間物は回収後に削除した。VMはgate後に停止し、
root空き容量は約7.1 GiBを維持した。

これは record field/value inferenceのbounded scanとLinux x86_64 stage2/stage3 fixed-pointだけを閉じる
verified sliceである。一般Map API、record schema pattern semantic parity、full record pattern/import
target、ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後current-source gate、全公開
surface、`LEGACY-LANG-01` aggregateの完了を意味しない。`TODO.md` の `[~]` と Rust oracle /
bootstrap / host integration 境界を維持する。

### LEGACY-LANG-01 selfhost bounded record type lookup/equality slice (2026-07-31)

`Type.ls` の `type-record-field-type` と `type-record-fields-eq` に残っていた record field scanを、
64要素の bounded loopとchunk境界の rooted continuationへ移行した。field lookupは存在しない hashを
`0`として返す既存契約を保ち、record equalityは field hash順序と各 field typeの比較結果を維持する。
移行後の `Type.ls` は616行で、500〜800行のファイルサイズ境界内に収まる。次の record type-check
bounded scanは別 batchとして扱い、今回の変更で巨大ファイル化しない。

Evidence: `test_e2e_selfhost_type_record_ops_use_bounded_chunks` と
`test_e2e_selfhost_large_record_type_operations_preserve_results` の2 tests、既存
`selfhost_typeinfer_private_visibility` の1 test、`selfhost_typeinfer_records_computation` の10 tests、
`git diff --check`、Rust fixtureの `rustfmt --edition 2021 --check` が passした。

source commit `77f177ab` に対して、`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=77f177ab-type-record-ops`
の local Lima `lsharp-linux-x86` gateを一度だけ実行した。host-generated stage1 x86 payloadがLinux
x86_64 VM内でstage2/stage3 native self-regenerationを完走し、summaryは `status=pass`、stage2/stage3
code lengthは双方 `11168596`、stdout SHA256は双方
`dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。証跡は
`ci-artifacts/native-linux-x86-hostgen-vm/77f177ab-type-record-ops/actual-selfregen-summary.json`
へ summaryだけを残し、11 MiB級 stdout、stage debug、stage1中間物、local Cargo targetは回収後に削除した。
VMはgate後に停止し、root空き容量は約7.2 GiBを維持した。

これは record type lookup/equalityのbounded scanとLinux x86_64 stage2/stage3 fixed-pointだけを閉じる
verified sliceである。record type-check全体、一般Map API、record schema pattern semantic parity、full
record pattern/import target、ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後
current-source gate、全公開 surface、`LEGACY-LANG-01` aggregateの完了を意味しない。`TODO.md` の
`[~]` と Rust oracle / bootstrap / host integration 境界を維持する。

### LEGACY-COMP-01 / LEGACY-LANG-01 selfhost bounded block and record export scans (2026-07-31)

TypeInferBlock と TypeInferRecordDecl に残っていた大きな線形走査を、同じ native selfhost
検証単位としてまとめた。`TypeInferBlock.ls` の do/computation 子要素走査は 64 要素単位の
bounded/rooted continuationへ、`TypeInferRecordDecl.ls` の `:only` 判定と accessor filteringは
64 要素単位の bounded/rooted scanへ移行した。移行後のファイルはそれぞれ 660 行と 714 行で、
500〜800 行の運用境界内に収まる。

Evidence: `selfhost_typeinfer_block_loops` の2 tests、`selfhost_typeinfer_record_filtering` の2
tests、既存 `selfhost_typeinfer_records_computation` の10 tests、既存
`selfhost_typeinfer_private_visibility` の1 test、Rust fixtureの `rustfmt --edition 2021 --check`、
`git diff --check` が passした。source commit `f8780d62` に対する local Lima
`lsharp-linux-x86` gateは一度だけ実行し、host-generated stage1 x86 payloadが Linux x86_64 VM内で
stage2/stage3 native self-regenerationを完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/f8780d62-block-recorddecl/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11168596`、stdout SHA-256は双方
`dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。VMは停止し、
stage debug、stdout、stage1中間物、local Cargo targetは回収して summaryだけを残した。

これは block/computation と record export filteringの bounded scan、ならびに Linux x86_64
stage2/stage3 fixed-pointだけを閉じる verified sliceである。computation全体の意味論、record
schema pattern semantic parity、full record/import target、ftable/linear-memory/runtime ABI、
Mac Apple Siliconのこの変更後 current-source gate、full-program compiler closure、全公開
surface、`LEGACY-COMP-01` / `LEGACY-LANG-01` aggregateの完了を意味しない。既知の
`test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type` の `1` vs `0` baselineと
Rust oracle / bootstrap / host integration境界は維持する。

### LEGACY-LANG-01 selfhost bounded parser primitive scans (2026-07-31)

`Syntax.Parser.ls` に残っていた name hash、qualified symbol dot、整数桁、string literal hashの
直接再帰 scanを、各 64 要素単位の bounded loopと rooted continuationへ移行した。escape時の
2文字進行、負数の符号処理、record accessor hashの初期 accumulatorは既存契約を維持する。

Evidence: `selfhost_parser_primitive_scanners` の2 tests、`selfhost_parser_forms` の22 tests、
`selfhost_parser_metadata_forms` の29 tests、`selfhost_type_parser_parity` の22 tests、
`selfhost_private_record_compiler` の2 tests、`string_escape_sequences` の4 tests、Rust fixtureの
`rustfmt --edition 2021 --check`、`git diff --check` が passした。`selfhost_lexer_parser` は31 tests
が passし、1 testは変更前 `fd8bf7fc`でも同じ parser errorとなる baseline failureだった。

source commit `0fc4af92` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、
host-generated stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを
完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/0fc4af92-parser-primitive/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11193360`、stdout SHA-256は双方
`92afcfd91b4512af66722eca380e3fe20f805f669b917b146c39a6bf51239aad` で一致した。VMは停止し、
stage debug、stdout、stage1中間物、local Cargo targetは回収して summaryだけを残した。

これは4つの parser primitive bounded scanと Linux x86_64 stage2/stage3 fixed-pointだけを閉じる
verified sliceである。`selfhost_type_parser_parity` の既存 `nested_module_decl` failure、
`selfhost_lexer_parser` の既存 `negative_int` parse failure、parser recovery/metadata全体、
Parser.ls の大規模ファイル分割、Mac Apple Siliconのこの変更後 current-source gate、全公開
surface、`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle / bootstrap / host integration
境界と TODO の `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser collection scans (2026-08-01)

`Syntax.Parser.ls` に残っていた `vector-set-at` と
`defn-signature-param-present-v3` の直接再帰 collection scanを、64要素単位の bounded loopと
chunk境界の rooted continuationへ移行した。setterは不変 vector の結果を維持し、signature scanは
既存の presence 判定値と return-type 判定を維持する。fixtureは深い nested `vector-push` を
避け、`tokenize-with-spans` が生成した64要素超の flat vectorでchunk境界を検証した。

Evidence: `selfhost_parser_collection_scanners` の static/runtime 3 tests、
`selfhost_parser_forms` の22 tests、`selfhost_parser_metadata_forms` の29 tests、Rust fixtureの
`rustfmt --edition 2021 --check`、`git diff --check` が passした。source commit `3d20368c` に対する
local Lima `lsharp-linux-x86` gateは一度だけ実行し、host-generated stage1 x86 payloadが Linux
x86_64 VM内でstage2/stage3 native self-regenerationを完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/3d20368c-parser-collection/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11208875`、stdout SHA-256は双方
`6a58242abe669b9fbf8add62e36e68e8c7ba251b90b7f0d67c076d9641b1a603` で一致した。VMは停止し、
stage debug、stdout、stage1中間物、local Cargo targetは回収して summaryだけを残した。

これは parser collectionの bounded scanと Linux x86_64 stage2/stage3 fixed-pointだけを閉じる
verified sliceである。parser recovery/metadata全体、Parser.lsの大規模ファイル分割、record schema
pattern semantic parity、全 pattern/import target、ftable/linear-memory/runtime ABI、Mac Apple
Siliconのこの変更後 current-source gate、全公開 surface、`LEGACY-LANG-01` aggregateの完了を
意味しない。Rust oracle / bootstrap / host integration境界と TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser structural/recovery scans (2026-08-01)

`Syntax.Parser.ls` に残っていた `parse-skip-bracket-v3`、`parse-skip-brace-v3`、
`scan-defn-param-form-end-v3`、`parse-delimiter-balance-loop`、`recover-to-next` の直接再帰
token/span scanを、64要素単位の bounded loopとchunk境界の rooted continuationへ移行した。
bracket/brace depth、parameter formの終了index、最初のdelimiter diagnostic code、recovery cursorの
既存契約を状態ベクタへ分離して保持する。`collect-example-expression-spans` は result vectorの
ownershipが異なるため、このbatchには含めず次のREDへ残した。

Evidence: `selfhost_parser_structural_scanners` の2 tests、129 tokenのcross-chunk runtime、
`selfhost_parser_forms` の22 tests、`selfhost_parser_metadata_forms` の29 tests、
`selfhost_type_parser_parity` の22 tests、Rust fixtureの `rustfmt --edition 2021 --check` と
`git diff --check` が passした。`selfhost_type_parser_parity` の
`nested_module_decl` は `origin/main` の detached baseline `58122df8` でも `left "3" right "1"`
となる既存 failureであり、変更起因ではない。

source commit `14b9ad0e` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、
host-generated stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを
完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/14b9ad0e-parser-structural/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11253431`、stdout SHA-256は双方
`33c3d63945792d23d4026b0f052846ff20267aa283c5b98dbf7c87a7ee188085` で一致した。VMは停止し、
stage debug、stdout、stage1中間物、local Cargo targetは回収して summaryだけを残した。

これは parser structural/recovery bounded scanと Linux x86_64 stage2/stage3 fixed-pointだけを
閉じる verified sliceである。`collect-example-expression-spans`、parser recovery/metadata全体、
Parser.lsの大規模ファイル分割、record schema pattern semantic parity、全 pattern/import target、
ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後 current-source gate、全公開
surface、`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle / bootstrap / host integration
境界と TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser expression span scans (2026-08-01)

`Syntax.Parser.ls` の `collect-example-expression-spans-v3-loop` を、式一つを返す step、64要素
bounded loop、rooted continuation、public wrapperへ移行した。`:example` / `:assert` の bracket内
にある atom/list 式の `(start,end)` の順序、nested list の終端、RBracket 停止を既存の span contract
のまま保持し、`result` と `next-result` は vector allocation 前後で root 化した。

Evidence: `selfhost_parser_expression_spans` の static/runtime 2 tests、
`selfhost_parser_forms` の22 tests、`selfhost_parser_metadata_forms` の29 tests、fixtureの
`rustfmt --edition 2021 --check`、`git diff --check` が passした。parser 回帰の
`test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms` は変更前の detached
`origin/main` (`5e0b0b3a`) でも `left "7"` / `right "5"` となり、今回の変更起因ではない。

source commit `23af742e` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、
host-generated stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを
完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/23af742e-parser-expression-spans/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11260030`、stdout SHA-256は双方
`5467c937c0b33a7662896d1d211a581ecf3441550357fb83943fd9ad5378a0eb` で一致した。VMは停止し、
中間 stage、stdout、local Cargo targetは回収して481Bのsummaryだけを残した。

これは parser expression span collection の bounded scan と Linux x86_64 stage2/stage3 fixed-point
だけを閉じる verified sliceである。`:case` / `:assert` metadata list loop、parser recovery/metadata
全体、Parser.lsの大規模ファイル分割、record schema pattern semantic parity、全 pattern/import target、
ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後 current-source gate、全公開 surface、
`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle / bootstrap / host integration境界と
TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser metadata list scans (2026-08-01)

`Syntax.Parser.ls` の `parse-defn-meta-case-loop-v3` と `parse-defn-meta-assert-loop-v3` を、
metadata item 一つを返す step、64要素 bounded loop、rooted continuation、public wrapperへ移行した。
`:case` の expectation pair と個別 expression span、`:assert` の predicate vector、RBracket/EOF cursorを
既存 contract のまま保持する。case の不正 payloadは item が進行しない場合に done stateへ移し、metadata
loopが同じ cursorを再帰し続けないようにした。

Evidence: `selfhost_parser_metadata_scanners` の static/runtime 2 tests、
`selfhost_parser_metadata_forms` の29 tests、`selfhost_parser_forms` の22 tests、
`selfhost_assertion_spans` の4 tests、`selfhost_case_spans` の5 tests、fixtureの
`rustfmt --edition 2021 --check`、`git diff --check` が passした。既知の
`test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms` は current sourceでも
`left "7"` / `right "5"` で、expression-span batch前の baselineと同じだった。

source commit `5dad0498` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、
host-generated stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを
完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/5dad0498-parser-metadata-loops/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11274770`、stdout SHA-256は双方
`aba2a41334336c9490d04e4812dba0a947e6d45fdbd26207347a342d58e7cbb3` で一致した。VMは停止し、
中間 stage、stdout、local Cargo targetは回収して481Bのsummaryだけを残した。

これは parser metadata `:case` / `:assert` list の bounded scan と Linux x86_64 stage2/stage3
fixed-pointだけを閉じる verified sliceである。parser recovery/metadata全体、Parser.lsの大規模
ファイル分割、record schema pattern semantic parity、全 pattern/import target、ftable/linear-memory/
runtime ABI、Mac Apple Siliconのこの変更後 current-source gate、全公開 surface、
`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle / bootstrap / host integration境界と
TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser evidence sequence scans (2026-08-01)

`Syntax.Parser.ls` の `parse-source-evidence-shrinks-loop-v3` と
`parse-source-evidence-coverage-loop-v3` を、item一つを返す step、64要素 bounded loop、rooted
continuation、public wrapperへ移行した。`:evidence` payloadの shrinks integer list と coverage
bucket/count list の RBracket/EOF停止、順序、既存 vector payload は変更しない。

Evidence: `selfhost_parser_metadata_sequences` の static/runtime 2 tests、65 shrinks / 65 coverage
entriesのcross-chunk cursor/count、`selfhost_parser_` filterの114 tests、
`selfhost_evidence_registry` filterの59 tests、fixtureの `rustfmt --edition 2021 --check`、
`git diff --check` が passした。parser filterに含まれる
`test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms` は current sourceでも
`left "7"` / `right "5"` となる既知の detached baseline failureであり、今回の変更起因ではない。

source commit `30eaff32` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、
host-generated stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを
完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/30eaff32-parser-metadata-sequence/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11292130`、stdout SHA-256は双方
`9dce4fddbb6cd6473a2d85fd40368432592169688e566bcd9eaeec311371edae` で一致した。VMは
`4 CPU / 16 GiB RAM / 12 GiB disk` の既存設定を使用し、完了後に停止した。stage debug、stdout、
stage intermediate、local Cargo targetは回収して小さいsummaryだけを残す運用とする。

これは parser evidence の shrinks / coverage bounded scan と Linux x86_64 stage2/stage3 fixed-point
だけを閉じる verified sliceである。metadata outer directive loop、review/evidence field schema全体、
parser recovery/metadata全体、Parser.lsの大規模ファイル分割、record schema pattern semantic parity、
全 pattern/import target、ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後
current-source gate、全公開 surface、`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle /
bootstrap / host integration境界と TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser typed signature scans (2026-08-01)

`Syntax.Parser.ls` の `parse-defn-param-signature-loop-v3` と direct-recursive append pathを、
`[done, next-index, next-signature]` stateを返す item step、64要素 bounded loop、rooted continuation、
public wrapperへ移行した。typed parameterの raw type再読、untyped/invalid parameterの skip、return typeの
後置は既存 contractのまま保持し、signature vectorの順序と countを変えない。

Evidence: `selfhost_parser_signature_scanners` の static/runtime 2 tests、65 typed paramsの
cross-chunk signature count・先頭 Int・末尾 String・return Bool、`selfhost_parser_metadata_forms` の29 tests、
`selfhost_parser_forms` の22 tests、fixtureの `rustfmt --edition 2021 --check`、`git diff --check` が
passした。root lifetime checkerの bounded helperも passしている。

source commit `42819065` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、
host-generated stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを
完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/42819065-parser-signature/actual-selfregen-summary.json`
に保存し、`status=pass`、stage2/stage3 code lengthは双方 `11297018`、stdout SHA-256は双方
`3e86547744086a66579c8a381390d85bf51e370c27e566b945233dd4d1499de5` で一致した。VMは
`4 CPU / 16 GiB RAM / 12 GiB disk` の既存設定を使用し、完了後に停止した。stage debug、stdout、
stage intermediate、local Cargo targetは回収して小さいsummaryだけを残す運用とする。

これは parser typed parameter signature の bounded scan と Linux x86_64 stage2/stage3 fixed-pointだけを
閉じる verified sliceである。metadata outer directive loop、review/evidence field schema全体、parser
recovery/metadata全体、Parser.lsの大規模ファイル分割、record schema pattern semantic parity、全
pattern/import target、ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後 current-source
gate、全公開 surface、`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle / bootstrap /
host integration境界と TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser outer metadata scans (2026-08-01)

`Syntax.Parser.ls` の `parse-defn-metadata-loop-v3` を、directive一つを処理して更新済み metadataを返す
single-step、64 directive bounded loop、rooted continuation、public wrapperへ移行した。`:doc`、`:example`、
`:params`、`:returns`、`:invariant`、`:case`、`:assert`、`:property` と source/evidence/review metadataの
各 handlerから旧 outer loopへの直接再帰を除去し、未知 directiveの skip と cursor advancementも同じ一 step
の契約で保持した。outer stepの `[done, meta]` stateと `result`/`next-meta` の root ownershipを固定し、
handler側の個別テストを増やさず一つの cross-chunk fixtureで全体の境界を検証した。

Evidence: `selfhost_parser_metadata_outer_scanners` の static/runtime 2 tests、65個の `:property []` の
metadata form count `65` と EOF cursor `99`、`selfhost_parser_` grouped filter 68 tests、fixtureの
`rustfmt --edition 2021 --check`、`git diff --check` が passした。static contractは handlerからの旧
`parse-defn-metadata-loop-v3` 直接再帰が wrapper一箇所以外に残っていないことも確認する。

source commit `7b7a4f24` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、host-generated
stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/7b7a4f24-parser-metadata-outer/actual-selfregen-summary.json` に保存し、
`status=pass`、stage2/stage3 code lengthは双方 `11297622`、stdout SHA-256は双方
`6082b4494b46a2244b1d1e84c4faf3824f9b6ff664ca6859785acc8c73240a3c` で一致した。VMは `4 CPU / 16 GiB RAM /
12 GiB disk` の既存設定を使用し、完了後に停止した。stage debug、stdout、stage intermediate、local Cargo
targetは回収し、481Bのsummaryだけを残した。

これは parser outer metadata directive scan と Linux x86_64 stage2/stage3 fixed-pointだけを閉じる verified
sliceである。review/evidence field schema全体、parser recovery/metadata全体、Parser.lsの大規模ファイル分割、
record schema pattern semantic parity、全 pattern/import target、ftable/linear-memory/runtime ABI、Mac
Apple Siliconのこの変更後 current-source gate、全公開 surface、`LEGACY-LANG-01` aggregateの完了を意味しない。
Rust oracle / bootstrap / host integration境界と TODOの `[~]` は維持する。

### LEGACY-LANG-01 selfhost bounded parser evidence/review field schema scans (2026-08-01)

`Syntax.Parser.ls` の `parse-source-evidence-fields-loop-v3` と
`parse-source-review-attestation-fields-loop-v3`、および各 field value handlerを、field一つを処理する
step、64 field bounded loop、rooted continuation、public wrapperへ移行した。evidenceの integer/string/vector
field、reviewの string/sequence fieldは旧 field loopへ直接再帰せず、`[done, payload]` stateで duplicate marker、
payload更新、cursor進行を次のchunkへ渡す。65個の重複 `:subject` / `:subject-digest` fieldで evidence/review
payload length `82` / `75`、body cursor kind `10` を維持する cross-chunk contractを固定した。

Evidence: `selfhost_parser_metadata_fields` のstatic/runtime 2 tests、`selfhost_parser_` grouped filter 70
tests、fixtureの `rustfmt --edition 2021 --check`、`git diff --check` が passした。gate実行時の source commit `7f1e21d8` に
対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、host-generated stage1 x86 payloadが Linux x86_64
VM内でstage2/stage3 native self-regenerationを完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/7f1e21d8-parser-metadata-fields/actual-selfregen-summary.json` に保存し、
`status=pass`、stage2/stage3 code lengthは双方 `11314848`、stdout SHA-256は双方
`63466a74503a2e979f7bb805b8d99f91a848f883ea5c0f4124ac8ffdc0a288a7` で一致した。VMは `4 CPU / 16 GiB RAM /
12 GiB disk` の既存設定を使用し、空き容量 `7680847872` bytes が required `4294967296` bytesを満たした後に
停止した。stage debug、stdout、stage intermediate、local Cargo targetは回収し、481Bのsummaryだけを残した。
gate後に `origin/main` へ入った差分はMCP専用で、gate時の feature対象3ファイルと統合後の対象3ファイルは
byte-for-byte 一致したため、同じLinux replayは重複実行していない。

これは evidence/review field handlerとfield scanの bounded parity、ならびに Linux x86_64 stage2/stage3
fixed-pointだけを閉じる verified sliceである。`source-evidence-seen-new-v3-loop`、required-field check、
`source-review-attestation-seen-new-loop` の固定長初期化 scan、field schema全体の診断/semantic parity、parser
recovery/metadata全体、Parser.lsの大規模ファイル分割、record schema pattern semantic parity、全 pattern/import
target、ftable/linear-memory/runtime ABI、Mac Apple Siliconのこの変更後 current-source gate、全公開 surface、
`LEGACY-LANG-01` aggregateの完了を意味しない。Rust oracle / bootstrap / host integration境界と TODOの `[~]` は
維持する。

### LEGACY-LANG-01 selfhost bounded parser evidence/review metadata initializers (2026-08-01)

`Syntax.Parser.ls` に残っていた `source-evidence-seen-new-v3-loop`、
`source-evidence-required-fields-present-loop-v3`、`source-review-attestation-seen-new-loop-v3` を、要素一つを
処理する step、64要素 bounded loop、rooted continuation、public wrapperへ移行した。seen vectorの初期化は
`[done, next-index, seen]` state、required-field 判定は `[done, next-index, result]` stateで保持し、17要素の
evidence field table、12要素のreview field table、optional field 11/12のskip、missing requiredの `0` と
complete の `1` を従来契約のまま維持する。旧 direct-recursive loop名は sourceから除去した。

Evidence: `selfhost_parser_metadata_initializers` のstatic/runtime 2 testsで、evidence seen length `17`、
空seenの required 判定 `0`、required fieldを全て埋めた判定 `1`、review seen length `12` を確認した。
`selfhost_parser_` grouped filter 72 tests、fixtureの `rustfmt --edition 2021 --check`、`git diff --check` も
passした。source commit `0064a5bb` に対する local Lima `lsharp-linux-x86` gateは一度だけ実行し、host-generated
stage1 x86 payloadが Linux x86_64 VM内でstage2/stage3 native self-regenerationを完走した。summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/0064a5bb-parser-metadata-initializers/actual-selfregen-summary.json` に保存し、
`status=pass`、stage2/stage3 code lengthは双方 `11332908`、stdout SHA-256は双方
`aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad` で一致した。VMは `4 CPU / 16 GiB RAM /
12 GiB disk` の既存設定を使用し、空き容量 `7680364544` bytes が required `4294967296` bytesを満たした後に
停止した。stage debug、stdout、stage intermediate、local Cargo targetは回収し、481Bのsummaryだけを残した。

これは evidence/review metadata initializerの bounded scanと Linux x86_64 stage2/stage3 fixed-pointだけを閉じる
verified sliceである。field schema全体の診断/semantic parity、parser recovery/metadata全体、Parser.lsの大規模
ファイル分割、record schema pattern semantic parity、全 pattern/import target、ftable/linear-memory/runtime ABI、
Mac Apple Siliconのこの変更後 current-source gate、全公開 surface、`LEGACY-LANG-01` aggregateの完了を意味しない。
Rust oracle / bootstrap / host integration境界と TODOの `[~]` は維持する。

### LEGACY-LANG-01 record pattern schema visibility contract (2026-08-01)

`TypeInferPattern.ls` の canonical record patternは、全 program の schemaを保持する
`record-env`だけでなく、現在の moduleで可視な constructor schemeを必要とする。既存の低レベル
fixtureが value environmentを空にしていたため、private visibilityをfail-closedにする実装を
`result-failed=1`として観測していた。fixtureを可視 constructor scheme経由の成功ケースへ修正し、
同じ schema registryを持つ空の value environmentは拒否されることを同一テストで固定した。

Evidence: `test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type` の可視
field binding (`Int`, tag `1`, hash `100`) と非可視拒否、
`selfhost_typeinfer_quote_patterns` 13 tests、`selfhost_typeinfer_private_visibility` 1 testが
passした。selfhost source bytesは変更していないため、native stage0の再生成・Linux VM replayは
重複実行せず、直近の current-source fixed-point evidenceを再利用する。

これは record pattern schema visibilityとfield bindingの verified partial sliceであり、record
pattern全体のsemantic parity、import/parametric/deep cases、runtime/ftable/linear-memory ABI、
Mac Apple SiliconとLinux x86_64の変更後artifact matrix、`LEGACY-LANG-01` aggregateの完了を意味しない。
Rust oracle / bootstrap / host integration境界と TODOの `[~]` は維持する。

### V2-16e / LEGACY-BOOT-01 Linux current HEAD stage0 package/source-file smoke (2026-08-02)

current `HEAD=0b668bf500afbf03ffb55297abd381dfee845957` で
`test_e2e_native_linux_x86_host_generates_actual_selfregen_stage1_bundle_artifact` を一度だけ実行し、fresh
stage1は `1 passed` / `305.60s`、target `x86_64-unknown-linux-gnu`、code `4,393,234` bytes、data `2,757`
bytes、entrypoint `4,390,778`、function-start `3,409`、main function `3,418`、manifest source commit一致を
記録した。これを `package-native-linux-x86-actual-stage1-vm.sh` で VM materializeし、current source commitを
束縛した `lsharp-native-selfhost-stage0` packageを生成した。package manifestは target、relative compiler /
transport driver / materializer path、source commitを保持し、artifactは
`ci-artifacts/native-linux-x86-hostgen-vm/0b668bf5-stage0-package/` に保存した。

同じ packageを `native-linux-x86-native-stage0-source-file-smoke.sh` へ渡し、transport chunk `64`、chunk
timeout `900` 秒、VM free `7,682,572,288` bytes / required `4,294,967,296` bytesで、native stage0の
`parse`、`check`、`fmt`、通常/metadata/property `test`、EC-M3 validation positive/negative fixture、
`compile`、`build`を実行した。runは exit `0` で `Linux x86_64 native stage0 source-file smoke passed` となり、
positive core outputは `decls:1` / `first-body:int` / `diagnostics:0`、`Int` / `diagnostics:0`、
`examples:0` / `invariants:0` / `failures:0`、metadata `examples:2` / `invariants:1` / `failures:0`。
compile/buildはいずれも `wasm-size:2559`、Wasm size `2,559` bytes、SHA-256
`afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`で一致した。成功経路の PATHでは
`cargo`、`rustc`、host `lsharp`をblocklistし、Rust fallbackを使っていない。EC-M3 negative fixtureのstderrは
期待される診断として個別 artifactへ保存した。

evidence manifestは exit `0`、target、source commit、stage0 manifest SHA-256
`fe1bf90ff3e7542cd6df7b92f9ef5069620d93d49d693a8002b505e104ee9a19`、stage0 payload SHA-256
`0111da02752e0e9e3ffa20deaf7818e1aa167c2f20c2168132a340a220625c8e`を記録し、
`ci-artifacts/native-linux-x86-hostgen-vm/0b668bf5-source-smoke-evidence/`に保持した。これは current Linux
stage0 package/source-file smokeの verified sliceであり、公開 release asset acquisition、rollback実行、全公開
command、component sidecar、Mac/Linux packaged artifact parity、`LEGACY-BOOT-01` aggregateの完了を意味しない。
既存の source-smoke evidence export contractは `docs/adr/decisions-v0.3-native-source-smoke-evidence.md` に記録する。

### V2-16e / LEGACY-BOOT-01 Linux current-source stage2/stage3 fixed point (2026-08-02)

source commit `a41cf0655a12d88ac3ed2185492183049234cc7d` で生成した Linux x86_64 stage1 code/data/seed
payloadを再利用し、Lima `lsharp-linux-x86` の同一VM-side lock下で stage2/stage3 native self-regenerationを
重複起動なしに実行した。`ci-artifacts/native-linux-x86-hostgen-vm/a41cf065-stage23-reuse/actual-selfregen-summary.json`
は target `x86_64-unknown-linux-gnu`、host `Linux x86_64`、`status=pass`、stage2/stage3 code length各
`11,374,654`、stdout length各 `12,207,069`、stdout SHA-256一致
`7a837812e20e71378632bbe0a101d18c141e3304fb63562890e8ee4425a00930`、stderr空を記録した。stage1 manifestは
code `4,393,234` bytes、data `2,757` bytes、entrypoint `4,390,778`、function-start `3,409`、main
function `3,418` である。

後続の `665623857282359b32dce2bda0cc37d3e9d7424e` は stage1 replay debug payloadの seed source mappingを
補強した script/test-only changeであり、同一コンパイラ出力について重い stage2/stage3 replayを再実行しなかった。
この結果は current-source Linux stage2/stage3 fixed-pointの verified sliceであるが、current HEADの native
stage0 package/source-file smoke、release acquisition/rollback、Mac/Linux artifact parity、`LEGACY-BOOT-01`
aggregateの完了を意味しない。ADR: `docs/adr/decisions-v0.3-native-linux-read-file-bounded-heap.md`。

### V2-13a-5 / V2-16b Linux x86 read-file bounded heap (2026-08-02)

Linux x86 selfhost `emit-x86-selfhost-read-file-helper` の per-file `mmap` allocationを、materializer-owned
`r14` native heapのcursor/limitへ置き換えた。cursor fallbackは `8192`、予約上限は `0x200010`、read上限は
`0x200000`。read成功後だけcursorを更新し、open/read/close、String header、high-bit tag、callee-saved ABIを
維持する。helperは207→208 bytesとなり、後続helper offsetとrel32 call-siteを `+1` へ同期した。

REDは `test_native_codegen_x86_read_file_uses_bounded_heap_cursor` による旧 mmap syscall byte sequenceの検出。
GREENは bounded cursor、Linux syscall/close ABI、helper byte vector、call-site/trailer offset、2 MiB capの focused
test群と、host-side object smoke `read-file-object-summary.json` の target `x86_64-unknown-linux-gnu`、
expected/actual exit `7`、`status=pass`。artifactは
`ci-artifacts/native-linux-x86-hostgen-vm/0fb8a19b-read-file-bounded/` に保持した。

これは read-file bounded allocationと narrow ELF object runtimeの verified sliceに限定される。全 fd/error
semantics、current HEADの Linux native stage0 source-file smoke、Mac Apple Silicon parity、package acquisition/
release/rollback、Rust-free全体の完了証拠には拡張しない。ADR:
`docs/adr/decisions-v0.3-native-linux-read-file-bounded-heap.md`。

### V2-16b/c current Mac native read-stdin routing (2026-08-02)

`read-stdin` を native selfhost compiler の builtin/opcode `91` として固定し、x86_64 helper（104 bytes）と
AArch64 helper（156 bytes）、nullary stack effect、runtime call target、helper trailer offsetを追加した。
RED→GREENは `scripts/ci/test-native-selfhost-read-stdin-contract.sh` で確認し、source-level `Cli` の
`run-lsp-stdio-server` が同じ builtinで wire inputを読む接続も固定した。x86 helperの rel32分岐・保存先と
AArch64 helperの分岐を独立 disassemblyで確認した。

current source commit `1ee26eef38fe6f32ac1f6a1e7342bcf8cb1fec41` の Mac Apple Silicon actual `App.Cli`
release gateは `1 passed` / `877.69s`、package manifestは `target=aarch64-apple-darwin` と同じ source commit、
Mach-O arm64を記録した。同packageを current source-file smokeへ渡し、parse/check/fmt/test/metadata test/
compile/build と validation positive/negative を exit `0` で確認した。compile/build Wasmは各 `2,559` bytes、
SHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`、evidence manifestは stage0
manifest SHA-256 `fd1f47bd7a61e0f45bd2b8d086021fe0b919e3d75de172c82102b0e653518dd7`、payload SHA-256
`36f7cd4e27c58bb23b299eca5d5f0d1be266b821b1f1e9708d3a9b8fca70b5c9` を記録した。成功経路では `cargo`、
`rustc`、host `lsharp`、Rust fallbackを使用していない。

同packageの `lsp --stdio` は stdin wireを読み、stderr空、`Content-Length` frameを返すところまで実 native
確認した。ただし initialize resultの selfhost配列表現と Rust LSP object shapeの parity、hover等の semantic
projectionは未完である。また `valid/io-read-stdin` の生成 Preview1 Wasm compileは native CLIの明示的な
`unsupported standalone Preview1 runtime capability` 境界で拒否され、Wasm runtime証拠にはしていない。
Linux x86_64 current HEADの同opcode stage0/package/runtime、component sidecar、release asset acquisition/
rollback、Mac/Linux packaged parityは未検証のため、`V2-16b` / `V2-16c` / `V2-16e` と aggregateは `[~]` のまま残す。

### V2-16b/c current type Diagnostic projection and native built-in boundary (2026-08-03)

`lsp-type-diagnostic-to-lsp` は既存の legacy 6 fieldsを保持した10要素 projectionへ拡張し、end position、canonical
code、messageを追加した。generic rendererはこの projectionを標準 LSP Diagnostic objectの `range`、`severity`、
`code`、`source`、`message`へ変換する。selfhost sourceには undefined、condition/branch、infinite、argument mismatch
を `LS1001`〜`LS1004`へ対応させる狭い code mapを追加した。

Rust-hosted selfhost bundleの `(defn bad [] (+ 1 true))` は、標準 objectの `LS1004` / `function argument type mismatch`
を返す focused mapper testで確認した。renderer groupは6 tests、native LSP stdio contractは5 testsがpassし、commit
`7744350c4e36eb279861ba19007f732b72ff3e71`を`origin/main`へpushした。これは標準 wire shapeとRust-hosted selfhost
projectionの verified partialであり、type diagnostics全体の完了ではない。

同じ current source commitで Mac Apple Silicon actual `App.Cli` release gateを実行し、artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-7744350c-lsp-type/` の manifestは
`target=aarch64-apple-darwin`、`selfhost_fixed_point=true`、program SHA-256
`beea2a615b2a27156fb90a681a97aaa2d40af33bc7256330dd2d4efc68e9bb7f`を記録した。Linux x86_64 hostgen→Lima
VM gateも同じ sourceで `status=pass`、stage2/stage3 code length各 `11,408,204`、stdout SHA-256各
`b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr空で一致した。artifactは
`ci-artifacts/native-linux-x86-hostgen-vm/current-7744350c-lsp-type/`に保存し、VM workdirとlockを回収、Limaは停止した。

一方、current Mac native programへ直接 `lsp --stdio` と `check` を渡すと、標準 Diagnostic objectの形は得られるものの、
`(defn bad [] (+ 1 true))` と `(defn main [] (+ 1 2))` はともに `LS1001` / `undefined symbol`になった。Rust-hosted bundleの
`LS1004`との差分はprojection rendererではなく、native `App.Cli`の`TypeInferBuiltins`が構築する built-in type
environmentのretentionまたはlookup境界を示す。したがって次の REDは同じ fixtureで、nativeの正常な`+`解決と不正引数の
`LS1004`、標準 wire objectを固定する。原因確定前に広いroot、spill、offset、map helperの変更を再試行しない。

### V2-16b current-source native built-in retention fix (2026-08-03)

LLDBで `typeinfer-init-builtin-env` の戻り値を追跡したところ、built-in mapは live count `53` の populated mapとして
返っていた。一方、生成された `typeinfer-program-analysis-loop` の長い state accessor束では、env accessorの結果を
後続のsubst accessorが同じ一時slotへ上書きし、`infer-defn-predeclared`へ空のenvが渡っていた。map/root helperや
offset/state layoutを広く変更せず、`TypeInfer.ls` に env/substを短い束として保持する helperを追加し、defn branchの
入力だけをそこへ集約した。REDは `scripts/ci/test-native-selfhost-type-builtins.py` の3ケースで固定した。

current source commit `75038aa445e7c6c42443dd3b7ddfb83fd53c0d60` で Mac Apple Silicon actual release gateを実行し、
underlying ignored E2Eは `1 passed`（`836.95s`）。生成した stage0 manifestは target `aarch64-apple-darwin`、同じ
`source_commit`、manifest SHA-256 `2f59eb86f66fae2f4318238c068d867ca9c6ec7d2f4cdd0fd7316a2c4da27965` を記録した。
その stage0を `scripts/native-selfhost-dev.sh` に渡して native `App.Cli`を再生成し、回帰scriptは `3 tests`、
valid `+`、argument mismatch、標準 LSP wire objectの全てで passした。成功経路は current-source native stage0であり、
この回帰実行にRust fallbackは入っていない。

Linux x86_64の current-source gateは `bash scripts/ci/native-linux-x86-selfregen.sh` を
`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=75038aa4-current` で一度だけ実行した。
`ci-artifacts/native-linux-x86-hostgen-vm/75038aa4-current/actual-selfregen-summary.json` は target
`x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,408,204`、
stdout length各 `12,249,104`、stdout SHA-256各
`b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr空を記録する。stage1 manifestは
同じ source commit、code `4,408,352`、data `2,757`、entrypoint `4,405,792`、function-start `3,418`、main
function `3,427`を持ち、stage2-debug / stage3-debugも同じ source commit、code `11,408,204`、entrypoint
`11,403,596`、function-start `3,418`、main function `3,427`で一致した。summaryは expected/actual exit code `42`
の一致も記録する。VMは `4 CPU / 16GiB memory / 12GiB disk`、開始時のfree spaceは `7,679,692,800` bytes、
必要量は `4,294,967,296` bytesで、成功後にworkdir・lockを回収し停止した。

これは built-in `+` retention、型不一致診断の標準 projection、Mac/Linux current-source native fixed pointに限定した
verified sliceである。全 built-in/type diagnostic parity、他の言語機能、全公開command、component sidecar、stage0
acquisition/release/rollback、Mac/Linux packaged parity、Rust-free全体の完了を意味しない。TODOの `V2-16b` は `[~]` と
Rust oracle / bootstrap / host integration境界を維持する。

### V2-16b current-source native signed 64-bit immediate encoding (2026-08-03)

既存の unary builtin apply retention fix後に native builtin matrixを拡張したところ、`int-to-string`、`map-contains?`、
`file-exists?` のように parserの `name-hash` が負の signed 64-bit値になる builtinだけが native `undefined symbol` となった。
parserのhash計算、negative map keyの insert/lookup、Rust oracleは一致していたため、failure boundaryを native codegenの
64-bit即値生成へ限定した。直接生成された負の literalは parser hashと一致しなかったが、`(- 0 hash)` の計算結果は一致し、x86の
legacy/append emitterが下位32-bitの正規化を一度しか行わず、上位32-bitで truncating division を使っていたことを確認した。

REDは `scripts/ci/test-native-selfhost-type-builtins.py` の expanded valid/invalid matrixと、
`e2e::selfhost_native_stage_chain::test_native_codegen_x86_i64_const_direct_append_avoids_zero_byte_vector` の backend contractで固定した。
`60a02b4cba79560888942cc2401166d7207d6aa1` では `NativeCodegen.ls` に signed floor division と u32/u16 normalizationを追加し、
x86のlegacy/append emitterとAArch64の4 chunk全てへ適用した。fixture側の `file-exists` は実際の公開名 `file-exists?` に修正した。
Rust focused backend contract、Rust builtin environment parity、surface registrationの各testはpassした。

current source commit `60a02b4cba79560888942cc2401166d7207d6aa1` で Mac Apple Silicon actual `App.Cli` release gateを実行し、
underlying ignored E2Eは `1 passed`（`923.69s`）。stage0 manifestは
task-owned temporary packageの manifestに `target=aarch64-apple-darwin` と同じ `source_commit` を記録した。その stage0から生成した
native `program` の native builtin matrixは `5 tests` で全て passし、
成功経路にRust fallbackは入っていない。

同じ source commitの Linux x86_64 current-source gateは
`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=60a02b4c-signed-immediates bash scripts/ci/native-linux-x86-selfregen.sh` を一度だけ実行した。
`ci-artifacts/native-linux-x86-hostgen-vm/60a02b4c-signed-immediates/actual-selfregen-summary.json` は
target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,412,074`、
stdout SHA-256各 `d5c7adef8f4b164ef216205e5025fb571e63015aabdeff471377c20924f61e89`、stderr空を記録する。
stage1 manifest、stage2/stage3 debug artifact、VM内のruntime smokeとtransport/materialize/compareを同じ jobで確認した。
VMは `4 CPU / 16GiB memory / 12GiB disk`、free space gateは `7,679,373,312` bytes available /
`4,294,967,296` bytes requiredを満たし、成功後にVM workdirを削除してLimaを停止した。

これは負の builtin hashを含む native immediate encodingと、builtin retention matrixの Mac/Linux current-source fixed pointに限定した
verified sliceである。全 builtin/type diagnostic parity、他の言語機能、全公開command、component sidecar、stage0 acquisition/
release/rollback、Mac/Linux packaged parity、Rust-free全体の完了を意味しない。TODOの `V2-16b` は `[~]` とRust oracle /
bootstrap / host integration境界を維持する。

### V2-16b current-source native unary builtin apply retention (2026-08-03)

前項の built-in environment retention fixを current-source stage0で再利用して native CLI matrixを広げたところ、
numeric operators、string、vector、map、referenceの valid callのうち、`string-length`、`vector-length`、`map-size`、
`ref-get` が `function argument type mismatch` になった。built-in map lookup、`infer-apply-one-final` の direct
unify、Rust oracleは成功していたため、failure boundaryは builtin登録や root/offset/state layoutではなく、実際の
`infer-apply-legacy-raw` 1引数分岐に絞った。これは RED として
`scripts/ci/test-native-selfhost-type-builtins.py` の valid 10種・invalid 3種、CLI `+`、標準 LSP wire objectで固定した。

`50a2ad3c282806175d76054f2e118e76f7a7f924` では、1引数適用の引数後半と関数後半を既存の rooted helper patternへ分離し、
外側に残っていた未使用の `s1` / `func-ty` 束縛を除去した。広い append helper、spill floor、offset-depth ctx/state refactorは
再試行していない。修正後、同じ native matrixは `5 tests` で全て passし、Rust oracleの builtin environment parityも passした。

Mac Apple Siliconの current-source actual App.Cli release gateは underlying ignored E2E `1 passed`、生成 stage0 manifestは
`target=aarch64-apple-darwin` と source commit `50a2ad3c282806175d76054f2e118e76f7a7f924` を記録した。その stage0から
`scripts/native-selfhost-dev.sh` で生成した native programに対する valid/invalid builtin matrixは `5 tests` 全 passで、
成功経路に Rust fallbackは入っていない。

同じ source commitの Linux x86_64 current-source gateは
`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=50a2ad3c-builtin-unary bash scripts/ci/native-linux-x86-selfregen.sh` を一度だけ実行した。
`ci-artifacts/native-linux-x86-hostgen-vm/50a2ad3c-builtin-unary/actual-selfregen-summary.json` は
target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,408,204`、
stdout SHA-256各 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a` を記録する。stage1 manifestは同じ
source commit、code `4,408,352`、data `2,757`、entrypoint `4,405,792`、function-start `3,418`、main function `3,427`で、
stage2/stage3 stderrは空、stdout lengthは各 `12,249,104`だった。VMのfree space gateは `7,678,709,760` bytes available /
`4,294,967,296` bytes requiredを満たし、成功後にworkdirを回収してLimaを停止した。

零引数 builtin applyのRust oracle (`test_e2e_selfhost_typeinfer_zero_argument_builtin_call_applies_unit`)も passし、current
native App.Cliの `(defn main [] (command-line-args))` checkは `diagnostics:0` だった。ただしこれは零引数 family全体の
runtime/artifact parityを閉じた証拠ではない。全 built-in/type diagnostic parity、他の言語機能、全公開command、component
sidecar、stage0 acquisition/release/rollback、Mac/Linux packaged parity、Rust-free全体の完了は未検証であり、TODOの
`V2-16b` は `[~]` を維持する。

### I-09 / M3-05-N9 / EC-M3-05 package root `src/` symlink ownership (2026-08-03)

regular installed package `demo-1.0.0` の `src/` を package root外の directoryへ向け、外部 `Geometry.ls`だけを置き、
`docs/api.json`を欠く同一 fixtureを Rust MCP と native MCP に追加した。旧実装は `src` symlinkを辿って外部 moduleを
package APIへ投影し、native routeでは外部 sourceに対して `doc --json`を実行した。

native `_package_source_files` は root `src` symlinkを空の source setとして扱い、既存の missing `api.json` / source
diagnosticへ戻すようにした。Rust `mcp_context` は同じ missing-source boundaryを返し、`lsharp-tooling` の
`build_api_doc_for_package`も root `src` symlinkを `symlink_metadata`で拒否する。これにより外部 sourceの module/function/
metadataは responseへ投影されず、native fake programの logも作成されない。

REDは `test_package_api_rejects_symlinked_src_directory` と Rustの同名 MCP test、および API-doc builderの direct testで
外部 `Geometry`投影を確認した。GREENは native MCP 107 tests、Rust package API 9 tests、API-doc package 2 tests、
`rustfmt`、Python `py_compile`、`git diff --check`で確認した。code checkpointは
`f90a5f8963bf0df6cc4479963cbb29470798a40e`、commitは `origin/main`へ push済みである。

これは root `src/` directory symlinkの verified partialだけである。regular `src/`配下の child symlink、個別 source file、
`docs/` directory、installer/provider/auth、current-source Mac Apple Silicon / Linux x86_64 runtime、packaged/release/
rollback parityは未検証のため、TODOの `I-09` / `M3-05-N9` / `EC-M3-05` と Rust-free全体の完了を意味しない。ADR:
`docs/adr/decisions-v0.3-native-mcp-package-src-directory-ownership.md`。

### V2-16b current-source comparison and logic builtin type contract (2026-08-03)

native built-in type environmentの次の狭い semantic familyとして、比較 `>`, `<`, `<=`, `>=`, `==`, `!=` と論理 `not`,
`and`, `or` の valid/invalid fixtureを `scripts/ci/test-native-selfhost-type-builtins.py` に追加した。REDは built-in
argument mismatchを `diagnostics:1` / `function argument type mismatch` として固定し、valid fixtureは `diagnostics:0`
と stderr空を要求する。Rust oracleの `test_e2e_selfhost_typeinfer_builtin` focused groupは `6 tests` passし、比較と論理の
opcode/type registration contractを確認した。

`0459ad983ac973ac4df6eca91cd8bd0e833ba202` の current sourceで Mac Apple Silicon actual `App.Cli` release gateを一度だけ
実行し、underlying ignored E2Eは `1 passed`（`1514.94s`）となった。同じ stage0から生成した native programの expanded
builtin matrixは `5 tests` 全 passで、成功経路に Rust fallbackは入っていない。

同じ source commitの Linux x86_64 current-source gateは
`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=0459ad98-comparison-logic bash scripts/ci/native-linux-x86-selfregen.sh` を
一度だけ実行した。`ci-artifacts/native-linux-x86-hostgen-vm/0459ad98-comparison-logic/actual-selfregen-summary.json` は
target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,412,074`、
stdout SHA-256各 `d5c7adef8f4b164ef216205e5025fb571e63015aabdeff471377c20924f61e89` を記録する。runtime smokeは exit `42`、
stderrは空で、transport/materialize/compareも同じ jobで通過した。VMの free space gateは `7,678,877,696` bytes available /
`4,294,967,296` bytes requiredを満たし、成功後にworkdir・lock・一時cargo targetを回収してLimaを停止した。

これは比較/論理 built-inの type contractと current-source Mac/Linux self-regeneration fixed pointに限定した verified partial
である。比較/論理の全 lowering/runtime意味論、全 built-in/type diagnostic parity、他の言語機能、全公開command、component
sidecar、stage0 acquisition/release/rollback、Mac/Linux packaged parity、Rust-free全体の完了を意味しない。TODOの
`V2-16b` は `[~]` とRust oracle / bootstrap / host integration境界を維持する。

### V2-16b current-source write-file-bytes type contract (2026-08-03)

`write-file-bytes` の `String -> Vector -> Int` contractについて、valid callと first-argument mismatchを Rust oracleの
`test_e2e_selfhost_typeinfer_write_file_bytes_contract` と `scripts/ci/test-native-selfhost-type-builtins.py` の native fixtureへ追加した。
valid結果は `diagnostics:0`、invalid結果は exit `1` と `function argument type mismatch` を要求し、Rust focused oracleは `1 passed` である。

`496a8ff3a67da3d511d01d840a3d9e07fcf0acb4` の current sourceで Mac Apple Silicon actual `App.Cli` release gateを一度だけ実行し、
underlying ignored E2Eは `1 passed`（`821.97s`）となった。同じ stage0から生成した native programの expanded builtin matrixは
`5 tests` 全 passで、成功経路に Rust fallbackは入っていない。

この batchでは `0459ad983ac973ac4df6eca91cd8bd0e833ba202` 以降の `selfhost/src` 差分がないことを確認した。前項の Linux x86_64
current-source stage2/stage3 fixed-pointは同じ selfhost sourceを検証済みだが、manifestの source commitが一致しない旧 artifactを
再利用せず、今回の fixtureについて重い Linux replayは再実行していない。したがって Linux x86_64 current-source direct checkは
未検証として残す。これは write-file-bytes の型 contractだけを閉じる verified partialであり、opcode 90の実 native runtime/artifact
bytes、全 I/O semantics、全 built-in/type diagnostic parity、全公開command、component sidecar、stage0 acquisition/release/rollback、
Mac/Linux packaged parity、Rust-free全体の完了を意味しない。TODOの `V2-16b` は `[~]` を維持する。

### V2-16b / LEGACY-IO-01 standalone I/O runtime matrix (2026-08-03)

`proc-exit` を含む standalone Preview1 の native runtime contractを、同じ current-source native programで確認するため、
`scripts/ci/test-native-selfhost-io-runtime.py` に4ケースの matrixを追加した。REDでは `proc-exit` compileが
`unsupported standalone Preview1 runtime capability` で明示拒否された。実装は `wasi_snapshot_preview1.proc_exit` の importを
既存の6 importへ追加し、import index `6`、`(i32) -> ()` type、defined function/user call/wrapper exportの index shiftを整合させた。
IR opcode `88` は `i32.wrap_i64`、proc-exit call、`i64.const 0`を出力し、selfhost user functionの `() -> i64` result contractを
壊さない。広い append helper、spill floor、offset-depth ctx/state refactorは再導入していない。

Rust parityでは `test_e2e_selfhost_standalone_user_call_after_preview1_import` が `1 passed` / `316.38s`、既存の
`test_e2e_selfhost_standalone_write_file_bytes_retries_partial_fd_write` が `1 passed` / `315.96s` となった。`a5317c95`
の Mac Apple Silicon (`aarch64-apple-darwin`) current-source actual stage0 gateは `1 passed` / `830.50s`、同stage0から
生成した native programで次の4 observablesが全て passした。

- `print-string`: stdout `hello`、exit `0`
- `write-file`: task-owned directoryの `text.txt` が `payload`、stdout空、exit `0`
- `write-file-bytes`: `raw.bin` が bytes `00 61 73 6d`、stdout空、exit `0`
- `proc-exit`: Preview1 `proc_exit(7)` の exit `7`

成功経路の native programは Rust fallback、`cargo`、`rustc`、host `lsharp`を呼び出していない。この結果は standalone I/Oの
Mac runtime verified sliceであり、Linux x86_64 (`x86_64-unknown-linux-gnu`) current-source stage2/stage3 fixed pointと直接の
I/O runtime matrixはこの時点では未検証だった。

続いて clean `HEAD=5163a1d67ad2883ceaea08ea4310e99c201acfe3` の Linux x86_64 current-source gateを一度だけ実行した。
`ci-artifacts/native-linux-x86-hostgen-vm/a5317c95-standalone-io/actual-selfregen-summary.json` は target
`x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,420,772`、stdout SHA-256各
`86dfc089c2d56311a6e778b393fe4848f784e6c559196ce5a8d2ca659c9fd7b5`、runtime smoke exit `42`、stderr空を記録した。stage1、
stage2、stage3 manifestの target/source commitも一致し、VMの free-space gateは `7,678,283,776` bytes available /
`4,294,967,296` bytes requiredで passした。

raw stage1 payloadを public `Cli`として直接実行すると compileが no-opになったため、同じ actual stage1 artifactから
`package-native-linux-x86-actual-stage1-vm.sh` で Linux stage0 packageを作成し、VM内の `native-selfhost-dev.sh` に
`selfhost/src` とともに渡す正規経路へ切り替えた。この Linux x86_64 native `Cli`に対して
`scripts/ci/test-native-selfhost-io-runtime.py` を実行し、`print-string` stdout `hello`、`write-file` の `payload`、
`write-file-bytes` の bytes `00 61 73 6d`、`proc-exit(7)` exit `7` の4 cases全 passを確認した。成功経路では Rust fallback、
`cargo`、`rustc`、host `lsharp`を呼び出していない。Linux direct standalone I/Oの4ケースは Mac evidenceと合わせた verified partial
になったが、read-stdinのstandalone Wasm runtime、全 fd error semantics、argv/full command-line、4096 bytes超の動的 layout、
component sidecar、公開 release asset acquisition/rollback、Mac/Linux packaged artifact provenance parity、Rust-free全体の完了を
意味しない。`TODO.md` の `V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

### V2-16b / LEGACY-IO-01 standalone read-stdin runtime (2026-08-04)

`2d96d09e5160dce7a0707e4b690d68260fc28571` で standalone Preview1 `read-stdin` の狭い runtime familyを閉じた。REDは
`test_e2e_selfhost_standalone_read_stdin_runtime` で、旧 emitterが生成した空 stdoutを `payload` と比較して失敗した。実装は
IR opcode `91` を native Wasm opcodeへ追加し、WASI importを含む定義関数の index shiftを反映したうえで、`fd=0` の
`fd_read` を 4096-byte chunk loopで読み、tagged Stringへ累積する bounded bodyを追加した。`_start` exportは wrapperの新しい
function indexへ修正した。広い append helper、spill floor、offset-depth ctx/state refactorは導入していない。

Rust parityの focused E2Eは `1 passed` / `321.86s` で、生成artifact `/tmp/lsharp-read-stdin-v4.wasm` は `wasm-tools validate` を通過し、
Wasmtime 43.0.0で stdin `payload` を stdout `payload` として exit `0`で返した。これは Rust-host oracle/runtimeの証拠であり、native evidenceとは
別に記録する。native側では Mac Apple Silicon actual `App.Cli` gateが `1 passed` / `816.52s`、manifest targetが
`aarch64-apple-darwin`、program SHA-256が `2abf3480c7f237a5271cf14bef63a212c790ead7c0ebcb852b9854ad253eecff` となった。同じ
current-source native programで `scripts/ci/test-native-selfhost-io-runtime.py` を実行し、`print-string`、`read-stdin`、`write-file`、
`write-file-bytes`、`proc-exit(7)` の5 casesを全て passした。

Linux x86_64では同じ source commitから actual stage1 -> stage2 -> stage3を一度だけ実行した。
`ci-artifacts/native-linux-x86-hostgen-vm/2d96d09e-standalone-io/actual-selfregen-summary.json` は target
`x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,441,967`、stdout SHA-256各
`848677d2fcce2bbd47fe405ada0ba766fcfffced6899e6308bdc6938856e526f`、stderr各 `0`を記録した。VM free-space gateは
`7,676,780,544` bytes available / `4,294,967,296` bytes requiredで passし、workdirとlockは終了時に回収した。
この green stage2 artifactを再利用した target-only `App.Cli` materializeも status `pass`で、
`ci-artifacts/native-linux-x86-hostgen-vm/2d96d09e-standalone-io-target/manifest.json` は target `x86_64-unknown-linux-gnu`、
source commit `2d96d09e...`、`selfhost_fixed_point=true`、code `13,367,530` bytes、program SHA-256
`c4755b25f58d12cfc863ede0da18db79e24051bca13aed3bef74996766b9cb3e`を記録した。同VMへ Linux Wasmtime 43.0.0をtask-owned
一時配置して、同じ5-case matrixを実行し全て passした。native成功経路では Rust fallback、`cargo`、`rustc`、host `lsharp`を呼び出していない。

これは bounded UTF-8 stdinを standalone Wasm runtimeで両対応targetから読める verified partialである。`fd_read` errno/EOFの全 error semantics、
空入力とzero-byte I/Oの詳細 parity、4096 bytes超の動的 root/data/heap layout、argv/full command-line、全公開command、component sidecar、
release asset acquisition/rollback、Mac/Linux packaged artifact provenance parity、Rust-free全体の完了を意味しない。`TODO.md` の
`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

### V2-16b / LEGACY-IO-01 read-stdin boundary expansion (2026-08-04)

`e553df73e66971f763740587283f72414fdd1b2e` では `fd_read` が bytesとnon-zero errnoを同時に返す場合を REDへ追加した。旧 standalone
emitterはerrno分岐で stdoutを空にしたが、Rust `emit_read_stdin_func` は fd_read resultをdropして `nread` と EOFだけを処理するため、
Rust/native parityが崩れていた。Wasm emitterはこの分岐を削除し、fd_readの戻り値をdropして nread scratchを直接 String concatへ渡す
narrow fixにした。既存の 4096-byte chunk loop、allocator、tagged String、import/index、root/data/heap layoutは変更していない。

Rust focused E2E `test_e2e_selfhost_standalone_read_stdin_runtime` は `1 passed` / `316.15s` で、同じ standalone Wasmを再利用して
`payload`、empty stdin、4096 bytes (`a`*4095 + `b`)、4097 bytes (`a`*4096 + `b`)、partial bytes + errnoを確認した。errno caseは
`nread` bytesを保持し、3回の fd_read、stdinへの fd_closeなし、stdout `payload`を要求する。これは Rust-host oracle/runtime evidenceであり、
native evidenceとは別に扱う。

Mac Apple Silicon actual `App.Cli` gateは source commit `e553df73...`、target `aarch64-apple-darwin`、`selfhost_fixed_point=true`、
program SHA-256 `48720214af0e7c2f8ca3ab76cc46067f6f2d369a29de280b35b63f9043daf69a`で `1 passed` / `835.93s`となった。artifactは
`ci-artifacts/native-release/aarch64-apple-darwin/e553df73-standalone-io-errno/manifest.json` に保存し、
`scripts/ci/test-native-selfhost-io-runtime.py` の native I/O matrixは8 cases全 passだった。

Linux x86_64では同じ source commitから actual stage1 -> stage2 -> stage3 fixed pointを一度だけ実行した。
`ci-artifacts/native-linux-x86-hostgen-vm/e553df73-standalone-io-errno/actual-selfregen-summary.json` は target
`x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,440,809`、stdout SHA-256各
`c263840ae70301622e3e8d41ca911e3fa536a7ecd5ac87adb809196426169f38`、stderr各 `0`を記録した。VM free-space gateは
`7,679,078,400` bytes available / `4,294,967,296` bytes requiredで passし、成功後にVM workdirを回収した。
この green stage2 artifactを再利用した target-only `App.Cli` materializeも status `pass`で、
`ci-artifacts/native-linux-x86-hostgen-vm/e553df73-standalone-io-errno-target/manifest.json` は target `x86_64-unknown-linux-gnu`、
source commit `e553df73...`、`selfhost_fixed_point=true`、code `13,366,372` bytes、program SHA-256
`e0db7dcddd9f82d32daffc20ad8dea049aa834bd0b14df3158606b610bf5f894`、stderr `0`を記録した。同VMへ Wasmtime 43.0.0をtask-owned
一時配置して、同じ8-case matrixを実行し全て passした。native成功経路では Rust fallback、`cargo`、`rustc`、host `lsharp`を呼び出していない。

これは nread-bearing fd_read errno、empty stdin、4096/4097 bytesの bounded readを standalone Wasm runtimeで両対応targetから確認した
verified partialである。zero-byte I/Oやerrno/EOFの全組合せ、入力が大きく増えた場合の動的 root/data/heap layout、argv/full command-line、
全公開command、component sidecar、release asset acquisition/rollback、Mac/Linux packaged artifact provenance parity、Rust-free全体の完了を
意味しない。`TODO.md` の `V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

### V2-16b / LEGACY-IO-01 standalone command-line runtime (2026-08-04)

`d2dcea7e135ba839c41cb2f29e416b91b2993d72` で standalone command-line runtimeのREDを
`test_e2e_selfhost_standalone_command_line_runtime` に追加した。`alpha` / `beta` では `alphabeta2\n` を得たが、strictなargc=0では
旧Wasm emitterが整数ゼロを符号分岐内だけで処理し、改行だけを出力した。Rustのcanonical `print_i64` と差分を確認し、ゼロを
符号分岐の後で1桁出力する narrow fixを `selfhost/src/Backend/Wasm/WasmEmit.ls` に追加した。Rust focused E2Eは current-source
cache生成後 `1 passed` / `1.27s`（初回生成 `317.69s`）で、保存Wasmは `wasm-tools validate` と disassemblyを通過した。
広い append helper、spill floor、offset-depth ctx/state refactorは導入していない。

Mac Apple Silicon actual `App.Cli` release gateは source commit `d2dcea7e...`、target `aarch64-apple-darwin`、
`selfhost_fixed_point=true`、program SHA-256 `f6e63869a8ea69d3ff6177639454f229dec63a18ae51ef82b1f8c8fe1e80a9ec`、
`1 passed` / `825.50s` となった。同artifactの `scripts/ci/test-native-selfhost-io-runtime.py` matrixは `11 cases` 全 passである。

Linux x86_64では current `HEAD=eb8086a8d5bf3bb5893102a4d692ff8aa1a058ef` から actual stage1 -> stage2 -> stage3 fixed pointを
一度だけ実行し、target `x86_64-unknown-linux-gnu`、status `pass`、stage2/stage3 code length各 `11,442,429`、stdout SHA-256各
`2526caaefa9e86b934d5d08eb800847ac96e6b3989f3c3c37c7d2c933516086e` を確認した。VM free-space gateは
`7,678,435,328` / `4,294,967,296` bytesでpassし、成功後にVM workdir/lockを回収してVMを停止した。
stage2を再利用した Linux `App.Cli` target-only manifestは target `x86_64-unknown-linux-gnu`、source commit `eb8086a8...`、
source tree SHA-256 `b79ae7dca8f81b661355f5f4cae7f872dae4150012b1111a7a849dfd4fb514d1`、`selfhost_fixed_point=true`、
code `13,367,992` bytes、program SHA-256 `af9c7944db08acf1ccd966c2ac40fe1162cc0f56e0bfed1599dc3d0a6597d537`、stderr `0`を記録した。
同VMの native I/O matrixも `11 cases` 全 passで、成功経路に `cargo`、`rustc`、host `lsharp`、Rust fallbackは入っていない。

非空argvは Wasmtime CLIに `--argv0` を明示して Rust runnerの `alpha` / `beta` と一致させた。外部CLIはargc=0を表現できないため、
empty-argv0 caseは `argc=1` の境界（出力 `1\n`）として扱い、strict argc=0の証拠はRust standalone E2Eに残した。`print-zero` caseは
ゼロ整数出力を両native targetで確認する。これは standalone command-line/zero-printの両target runtime verified partialであり、fd error/EOFの全semantics、
より大きい入力でのdynamic root/data/heap layout、完全なargv/command-line semantics、全公開command、component sidecar、release asset
acquisition/rollback、Mac/Linux packaged artifact provenance parity、Rust-free全体の完了を意味しない。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

### V2-16b / LEGACY-IO-01 native read-file runtime matrix expansion (2026-08-04)

`9b7ac735b...` で `scripts/ci/test-native-selfhost-io-runtime.py` の同一 matrixへ `read-file` の通常入力、zero-byte file、4097-byte file、
missing-pathを追加し、11 casesから15 casesへ拡張した。Mac Apple Siliconの既存 `d2dcea7e...` App.Cli artifactと、Linux x86_64の既存
`eb8086a8...` target-only artifactを再利用して、両targetとも `15 cases` 全 passした。Linux側では Wasmtime `43.0.0` をVM内のtask-owned
一時領域へ配置し、`program.native` は target `x86_64-unknown-linux-gnu` の ELF、program SHA-256
`af9c7944db08acf1ccd966c2ac40fe1162cc0f56e0bfed1599dc3d0a6597d537` のままと確認した。runner-onlyのテスト拡張なので、stage1再生成や
stage2/stage3 replayは重複実行していない。成功経路に `cargo`、`rustc`、host `lsharp`、Rust fallbackは入っていない。

これは normal/empty/large/missing の read-file runtimeを両対応targetで確認する verified partialである。fd_read/fd_close/path_open errnoを
注入するnative matrix、EOF/errorの全組合せ、入力がさらに大きい場合のdynamic root/data/heap layout、完全なargv/command-line semantics、全公開command、
component sidecar、release asset acquisition/rollback、Mac/Linux packaged artifact provenance parity、Rust-free全体の完了を意味しない。
`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

### V2-16c / LEGACY-TOOL-01 native core CLI runtime matrix (2026-08-04)

`86534799` で、current-source native `App.Cli` artifactを直接起動する
`scripts/ci/test-native-selfhost-cli-core-runtime.py` を `22 cases` へ拡張した。`--help`、`--version`（`lsharp 0.1.0`、改行なし）、
`parse`、`check`、`fmt`、text/metadata `test`、`compile`、`build` に加えて、`review`/`review --json`、
`doc`/`doc --json`/`doc --format json`、`doc-ack`/`--trailer`、`doc-check`/`--strict`、`install`、`repl`、bare `lsp` summary、
実際の `lsp --stdio` initialize→didOpen→hover wire、`doc --format yaml` の明示拒否を同じ contractで実行した。Mac Apple Siliconの
既存 `d2dcea7e...` artifactとLinux x86_64の既存 `eb8086a8...` target-only artifactで各 `22 cases` 全 passした。Linux programは target
`x86_64-unknown-linux-gnu`、program SHA-256 `af9c7944db08acf1ccd966c2ac40fe1162cc0f56e0bfed1599dc3d0a6597d537` の ELFであり、
runner-only test contractのため stage1/stage2/stage3 replayは重複実行していない。成功経路に `cargo`、`rustc`、host `lsharp`、Rust fallbackは
入っていない。

これは public command routingと代表的な doc/REPL/LSP wireを両対応targetで確認した verified partialである。実 install/package registry、
対話 REPL、LSP の全 semantic projection、component helper、全公開 command、stage0 package acquisition/release/rollback、Mac/Linux
packaged provenance parity、`LEGACY-TOOL-01` aggregateの完了を意味しない。`V2-16c` / `LEGACY-TOOL-01` は `[~]` のまま維持する。

### I-09 / M3-05-N9 package src child symlink ownership (2026-08-04)

`ce8a4cb7` で package root `src/` の下にある child directory symlinkが外部 sourceをAPI-docやmodule-indexへ投影しない境界を閉じた。
REDでは Rust API-doc collectorの `Path::is_dir()` が `src/linked -> <external>/` を追跡し、外部 `Geometry.ls` をAPIへ出力した。
GREENではAPI-doc collectorとmodule-index collectorの双方が各 entryを`symlink_metadata`で検査し、
`package src tree must not contain symlinks`で外部sourceを読む前に拒否する。Rust MCP package APIも同じエラーを返す。

native MCP shimは同じfixtureで既存の除外動作を確認し、external `Geometry`をresponseへ投影せず、fake native programを起動しない。
検証は `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package_`（3 tests）、
`cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_package_api_tool`（10 tests）、
`cargo test -p lsharp-driver --bin lsharp tests::test_cmd_install`（24 tests）、
`PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py`（108 tests）で行った。
この verified partialはAPI-doc/module-indexのchild directory symlink boundaryに限定され、regular child source fileの直接fixture、
`docs/` directory、installer/provider、current-source両target runtime、packaged/release/rollback parityは未完了である。
ADR: [`decisions-v0.3-native-mcp-package-src-child-symlink-boundary.md`](../../adr/decisions-v0.3-native-mcp-package-src-child-symlink-boundary.md)。

### V2-16e / LEGACY-BOOT-01 package and release preflight contracts (2026-08-04)

現行 checkout の stage0 package/release boundaryを、重い native regenerationを起動せずに一括再検証した。
`test-native-stage0-package.sh`、`test-native-stage0-release-package.sh`、`test-fetch-stage0-archive-provenance.sh`、
`test-fetch-stage0-atomic-install.sh`、`test-fetch-stage0-provider-url.sh`、`test-release-smoke-provider-snapshots.sh`、
`test-native-official-release-replay-lock.sh`、`test-native-official-release-snapshots.sh` は全て passした。

この batch で確認した境界は、stage0 packageの file-setと host tool exclusion、archive round-trip の checksum/provenance、
未登録payloadの拒否、install move failure時の既存 package復元、unsafe provider URLと live Linux replay lockの fail-closed、
review/provider snapshotの all-or-none・digest・rollback payload検証である。全テストは task-owned temporary rootを使用し、
Linux VMは起動していない。

これは local/fake harness の契約 evidenceであり、実 provider release assetの取得、current-source Mac/Linux package生成、
rollback archiveの実行、両 targetの packaged provenance parity、Rust oracle/host integrationの分離を実行時に証明しない。
したがって `V2-16e` / `LEGACY-BOOT-01` と `V2-16c` / `LEGACY-TOOL-01` は `[~]` のまま維持する。

### I-09 / M3-05-N9 package src nested regular ownership (2026-08-04)

`f76dafb9` で package API ownershipの通常 nested source fixtureを追加した。`src/Geometry/Vec2.ls` を API-doc / Rust MCP / native MCPの
同一 package fixtureから収集し、nested module `Geometry.Vec2` の name/function projectionを確認した。`docs/guides/README.txt` は
package documentationとして配置しても source collectorの対象にならず、native MCPの `doc` invocationには渡らない。

この batchの focused evidenceは次のとおりである。

- `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package` — 4 tests passed。
- `cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_package_api_tool` — 11 tests passed。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py` — 109 tests passed。
- `git diff --check`、Python `py_compile` — passed。

前段の child symlink拒否と合わせ、package `src/` の regular directory/file再帰、`.ls` filtering、symlink fail-closed、`docs/` exclusionの
verified partial boundaryを構成する。native成功経路に Rust fallback、`cargo`、`rustc`、host `lsharp`は入っていない。この contractは
source ownershipに限定されるため、個別 source file symlink / 特殊 filesystem entryの explicit fixture、installer / registry / provider/auth、
current-source Mac Apple Silicon / Linux x86_64 runtime、packaged/release/rollback parityは残る。`I-09` / `M3-05-N9` / `EC-M3-05` と
Rust-free全体は `[~]` のまま維持する。ADR: [`decisions-v0.3-native-mcp-package-src-child-symlink-boundary.md`](../../adr/decisions-v0.3-native-mcp-package-src-child-symlink-boundary.md)。

### V2-16b / LEGACY-IO-01 substring three-argument type boundary (2026-08-04)

`33d5483f` で `substring` の3引数 applyに対する narrow type contractを追加した。valid
`(substring "abc" 0 1)` は `String` resultを返し、invalid `(substring "abc" true 1)` は途中引数の type mismatchとして失敗する。
これは generic applyの引数収集・curried function型・最終 unifyを I/O/runtime ABIから分離して検証するREDである。

Evidence:

- `cargo test -p lsharp-wasm --test e2e e2e::selfhost_typeinfer_builtin_parity:: -- --nocapture` — 6 tests passed。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-type-builtins.py --program ci-artifacts/native-release/aarch64-apple-darwin/d2dcea7e-standalone-command-line/program.native` — 5 tests passed。ただし保存済み artifactは source commitが現 checkoutと一致しない replay-onlyであり、current-source native evidenceには数えない。

current-source Mac Apple Silicon / Linux x86_64 native check、全 builtinの診断 code/span、runtime/codegen、公開 command、component、packaged/release
provenanceは残る。同じ production sourceを変更しない test-only batchのため、Linux stage2/stage3 replayは重複起動しない。`V2-16b` / `LEGACY-IO-01` は
`[~]` のまま維持する。ADR: [`decisions-v0.3-native-typeinfer-substring-arity-boundary.md`](../../adr/decisions-v0.3-native-typeinfer-substring-arity-boundary.md)。

### V2-16b / LEGACY-IO-01 current-source native provenance gate (2026-08-04)

`ee08f23b132d6146716c5c025cf9d543cfc4b88a` の clean checkoutで、Mac Apple Siliconの current-source App.Cli stage chainを
`bash scripts/ci/native-macos-aarch64-stage0-release.sh` から一度だけ実行した。Rust E2Eは `1 passed` / `832.82s`、stage0
manifestは target `aarch64-apple-darwin`、source commit `ee08f23b...` と一致し、stage0 packageを
`ci-artifacts/native-stage0/aarch64-apple-darwin/ee08f23b-substring-type-contract/` に保存した。保存packageを
`scripts/native-selfhost-dev.sh` へ渡して current-source App.Cliをmaterializeし、
`PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-type-builtins.py --program target/native-current-app-cli-ee08f23b/program.native`
は `5 tests` 全 passとなった。ここには `substring` の valid callと途中 `Bool` argument mismatchが含まれる。

同じ source commitから Linux x86_64 actual self-regenerationを
`NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=ee08f23b-substring-type-contract bash scripts/ci/native-linux-x86-selfregen.sh`
で一度だけ実行した。`ci-artifacts/native-linux-x86-hostgen-vm/ee08f23b-substring-type-contract/actual-selfregen-summary.json` は
target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,442,429`、stdout
SHA-256各 `2526caaefa9e86b934d5d08eb800847ac96e6b3989f3c3c37c7d2c933516086e` を記録した。VM free-space gateは
`7,683,088,384` bytes available / `4,294,967,296` bytes requiredで passし、成功後にVM workdirとreplay lockを削除し、VMを停止した。

この gateで current-source Mac/Linux stage provenanceとstage2/stage3 fixed point、Mac native substring type fixtureを確認した。
Linux側の全 builtin/type-diagnostic matrix、全公開 command、component、release asset acquisition/rollback、Mac/Linux packaged
artifact provenance parity、Rust-free全体の完了は示さない。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

### I-09 / M3-05-N9 direct file symlink and special entry ownership (2026-08-04)

package `src/` の direct source file symlinkと special filesystem entryを、既存の child directory symlink / regular nested source
fixtureへ追加した。`src/Linked.ls -> external-source.ls` は Rust API-doc collectorと Rust MCPで
`package src tree must not contain symlinks`として拒否し、外部 `Geometry` sourceを投影しない。Unix socketを `src/S.ls` に置いた
fixtureは regular `.ls` fileとして扱わず、Rust tooling/MCPでは空の source ownership、native MCPでは生成対象なしの明示エラーとなり、
fake native `doc` programは起動しない。

Evidence:

- `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package` — 6 tests passed。
- `cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_package_api_tool` — 13 tests passed。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py` — 111 tests passed。
- Python `py_compile`、`git diff --check`、docs audit — passed。

この batchで source ownershipの regular directory/file、`.ls` filtering、directory/direct-file symlink fail-closed、special entry exclusion、
docs exclusionを確認した。installer / registry / provider/auth、current-source Mac Apple Silicon / Linux x86_64 runtime、
packaged/release/rollback parity、Rust-free全体の完了は未検証であり、`I-09` / `M3-05-N9` / `EC-M3-05` は `[~]` のまま維持する。

### V2-16b / LEGACY-IO-01 special argv ownership (2026-08-04)

standalone command-line runtimeの同一 fixtureを区切り文字付き出力へ拡張し、argvの空要素を前後の要素と区別できる
observable contractを固定した。`prog name`、空要素、UTF-8の `雪 空`、空白を含む `tail value` を渡すと
`prog name||雪 空|4` となり、通常の `alpha` / `beta`、外部CLIの empty-argv0（argc=1）、Rust runnerの strict argc=0も
既存境界として維持する。

Evidence:

- `cargo test -p lsharp-wasm --test e2e selfhost_standalone_io::test_e2e_selfhost_standalone_command_line_runtime -- --nocapture` — 1 passed / 336.08s。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-io-runtime.py` — Mac Apple Silicon保存artifactで16 cases passed。
- 同じnative runnerを Linux x86_64 `eb8086a8` target-only保存artifactへ渡し、16 cases passed。これは replay-onlyであり、current-source Linux provenance evidenceには数えない。
- Python `py_compile`、`git diff --check` — passed。

この batchで argvの空要素、UTF-8、空白保持を verified partialとして追加した。current-source Linux native regeneration/runtime、
fd error/EOFの全組合せ、dynamic root/data/heap layout、全公開command、component sidecar、release asset acquisition/rollback、
Mac/Linux packaged provenance parity、Rust-free aggregateは残る。ADR: [`decisions-v0.3-native-standalone-command-line-argv-boundary.md`](../../adr/decisions-v0.3-native-standalone-command-line-argv-boundary.md)。

### V2-16b / LEGACY-IO-01 standalone allocator memory growth (2026-08-04)

`19b01384` で、262144-byte stdinを旧 standalone bump allocatorへ渡すと linear memory `0x1000000`（16 MiB）で trapする
REDを固定した。aligned heap endが `memory.size << 16`を超える場合に必要ページ数を算出し、`memory.grow`後に既存の bump allocationを
続ける narrow fixを追加した。GC、root table、data layout、容量失敗の診断はこの修正へ混ぜていない。

Evidence:

- Rust focused E2E `selfhost_standalone_io::test_e2e_selfhost_standalone_read_stdin_runtime` — 1 passed / 317.18s。
- Mac Apple Silicon current-source stage0は source commit `19b01384281a8efdcc9f0b9ecddb4faeed36b113` と一致し、1 passed / 827.33s、
  materialized native I/O matrixは256KiB stdinを含む17 cases全 pass。
- Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointは target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、
  `status=pass`、stage2/stage3 code length各 `11,445,101`、stdout SHA-256各
  `4ddaa27ed209bf8fce4305ea459a10ed99d308db7c1818222f5cfae38dbf44bc`、stderr空。
- 同じ Linux stage2 artifactからの current-source App.Cli target-only materializeは `selfhost_fixed_point=true`、code `13,370,664` bytes、
  program SHA-256 `25a3dd5c9ca786ac54c7f88ba1be7cccbf77589cee9cb65bf477817167af961d`、`--version` `lsharp 0.1.0`、native I/O matrix17 cases全 pass。
  stage0 package manifestは target `x86_64-unknown-linux-gnu`と同 source commitを保持した。
- Static read-stdin contract、Python `py_compile`、`git diff --check`を通過し、VM workdir/replay lock、Wasmtime tarball、matrix workdirを回収してVMを停止した。

これは standalone allocator growthと256KiB stdinの両対応target verified partialである。fd error/EOFの全 semantics、より大きい dynamic
root/data/heap layout、GC/容量失敗診断、全公開command、component sidecar、release asset acquisition/rollback、Mac/Linux packaged
provenance parity、Rust-free aggregateは未完了であり、`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。ADR:
[`decisions-v0.3-native-standalone-allocator-memory-growth.md`](../../adr/decisions-v0.3-native-standalone-allocator-memory-growth.md)。

### V2-16b / LEGACY-IO-01 standalone stdin geometric buffer growth (2026-08-04)

`8ab2dd589410fa668ffa5c01f596bdfa046d466c` で、`(4 * 1024 * 1024) - 1` bytesのstdin payloadを受ける standalone
`read-stdin` のREDを固定した。旧経路は読み込みごとにStringを再確保していたため、payloadサイズに比例してallocator負荷と
linear-memory growthを増やしていた。新経路は初期容量4096 bytesの単一String bufferを使い、capacity不足時だけ容量を倍増し、
新しいString objectへ既存bytesを`memory.copy`してからfd_readを続ける。fd_readのscratch/iovec、String length offset、
standalone WASI call indexを保存Wasmとruntimeで検証した。既存のallocator growth、GC、root/data layout、容量失敗診断はこの
batchへ混ぜていない。

Evidence:

- Rust focused E2E `selfhost_standalone_io::test_e2e_selfhost_standalone_read_stdin_runtime` — 1 passed / 315.43s。
- Mac Apple Silicon current-source stage0は source commit `8ab2dd589410fa668ffa5c01f596bdfa046d466c` と一致し、1 passed / 825.61s、
  materialized native I/O matrixは18 cases全 pass。
- Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointは
  `ci-artifacts/native-linux-x86-hostgen-vm/8ab2dd58-standalone-stdin-over-4m/actual-selfregen-summary.json` で
  target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,449,265`、
  stdout SHA-256各 `179821d0fddaceaac637b08a128beee7c31c4afdc4bd4a90e88b013755855f3d`、stderr空を記録した。
- 同じsource commitのLinux x86_64 App.Cli target-only materializeは `selfhost_fixed_point=true`、code `13,374,828` bytes、
  program SHA-256 `f21ebd22261a2dd392e5123293faac948dcf40c586069966cfcab46545960cc4`、stderr 0 bytesとなり、native I/O matrixも
  18 cases全 passした。これは current-source target-only runtime evidenceであり、全公開commandのcompletionではない。
- Static read-stdin contract、Wasm validation、Python `py_compile`、`git diff --check`を通過した。Linux VM workdir/replay lock、
  Wasmtime tarball、matrix workdirを回収し、VMを停止した。

このverified partialはstandalone stdin bufferの容量倍増と4MiB超payloadの両対応target executionを閉じる。fd error/EOFの全組合せ、
より大きい/同時実行のdynamic root/data/heap layout、GC/容量失敗診断、全公開command、component sidecar、release asset
acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了である。`V2-16b` / `LEGACY-IO-01` は
`[~]` のまま維持する。代表ADR: `docs/adr/decisions-v0.3-native-standalone-stdin-capacity-growth.md`。

### V2-16b / LEGACY-IO-01 standalone read-file 4MiB bounded chunk (2026-08-04)

`ad65eaffdd7b928ec5e2d226c6f4695236afd05c` で、正確に4MiBの file を standalone `read-file` へ渡す REDを
`test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4m` と native I/O matrixへ追加した。旧経路は4096-byte readごとに
`string-concat` を繰り返し、5分超実行しても assertionへ到達しなかった。実装は既存の open/read/close と fail-closed semanticsを
維持し、file objectとiovecを4MiBへ拡張して、bounded chunkからStringを生成する narrow fixに留めた。任意サイズのdynamic file
bufferと全fd error/EOF semanticsはこの batchの対象外である。

Evidence:

- Rust focused E2E `selfhost_standalone_io::test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4m` — 1 passed / 316.52s。
- Mac Apple Silicon current-source stage0は source commit `ad65eaffdd7b928ec5e2d226c6f4695236afd05c` と一致し、1 passed / 828.19s、
  native I/O matrixは19 cases全 pass。
- Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointは
  `ci-artifacts/native-linux-x86-hostgen-vm/ad65eaff-standalone-file-over-4m/actual-selfregen-summary.json` で target
  `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,448,943`、stdout SHA-256各
  `a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e`、stderr空を確認した。
- 同じ stage2 artifactを再利用した Linux x86_64 `App.Cli` target-only materializeは
  `ci-artifacts/native-linux-x86-hostgen-vm/ad65eaff-standalone-file-over-4m-linux-cli/actual-selfregen-summary.json` で
  `selfhost_fixed_point=true`、code `13,374,506` bytes、program SHA-256
  `a090cd8474c6115ac3a2bcf5570226cc912d7479d0285f6991ca02fb5a1d6469`、stderr `0` bytes、`--version` `lsharp 0.1.0`を確認した。
  同じ Linux target-only native programに公式 Wasmtime 43.0.0を渡した native I/O matrixは19 cases全 passだった。
- Static read-stdin contract、Python `py_compile`、`git diff --check`を通過した。Linux VM workdir、replay lock、matrix workdir、
  Wasmtime tarballを回収し、VMを停止した。

これは standalone `read-file` の4MiB bounded chunkと両対応target runtimeの verified partialである。任意サイズのdynamic file buffer、
fd_read/fd_close/path_openの全 error/EOF組合せ、同時実行時のdynamic root/data/heap layout、GC/容量失敗診断、全公開command、component
sidecar、release asset acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了であり、TODOの
`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。ADR: `docs/adr/decisions-v0.3-native-standalone-read-file-4m-chunk.md`。

### V2-16b native builtin source-level polymorphic reuse (2026-08-04)

`e962b6ef` で、native builtin matrixに Float `+.` の source-level valid/mismatch fixtureを追加した。続く `13185038` では、
同じ定義内で `(print 1)` と `(print "x")` を連続して使う polymorphic builtin contractを、Rust oracleとnative fixtureへ追加した。
さらに `3d163344` では、同じ定義内で vector の要素型と map のキー・値型を変えて `vector-push/get` と `map-insert/get` を再利用する
contractを追加した。続く `56f78589` では、Float `+.` / `-.` / `*.` / `/.` の4演算子すべてに valid/mismatch source contractを広げた。
どれも既存の selfhost implementationを変更せず、builtin scheme lookup後に型ごとの fresh instantiationを要求する narrow contractである。

Evidence:

- Rust focused oracleは Float、repeated `print`、collection reuse contractをそれぞれ `1 passed` で通過した。Float contractは4演算子の
  valid/mismatchを含む。
- Linux x86_64では `ad65eaff` の既存 App.Cli native programへ更新後の
  `scripts/ci/test-native-selfhost-type-builtins.py` を replayし、valid/invalid matrix、LSPを含む `5 tests` 全 passとなった。
- replay対象の program は production selfhost source commit `ad65eaff` から生成された保存artifactであり、test-only commitの source
  provenanceとは一致しない。従ってこの結果は fixture replay evidenceに限定し、current-source Mac/Linux stage gateへ拡大解釈しない。
  test-only変更のため同じ production sourceの重い stage regenerationは重複実行していない。
- Python `py_compile` と `git diff --check` は passし、Linux replay後にVM workdirを削除して `lsharp-linux-x86` を停止した。

これは builtin polymorphic reuseの source-level verified partialである。全 builtin family、全型診断 code/span、runtime/codegen parity、
全公開command、component sidecar、release asset acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは
未完了であり、TODOの `V2-16b` は `[~]` のまま維持する。

### V2-16c Linux native App.Cli core command matrix replay (2026-08-04)

`ad65eaff` の Linux x86_64 `App.Cli` target-only native programへ `scripts/ci/test-native-selfhost-cli-core-runtime.py` を渡し、
public core command matrixを replayした。`--help`、`--version`、parse/check/fmt、text/metadata test、Preview1 `compile` / `build`、
review、doc JSON、doc-ack、doc-check、install、repl、bare LSP、`lsp --stdio` の initialize/didOpen/hover、unsupported doc formatを
含む `22 cases` が全 passし、成功経路の stderrは空だった。続く `d05d2642` で `validate --source --format text` の
unknown trace-gap reportと exit `2` を追加し、同じ Linux replayは `23 cases` 全 passとなった。実行後はVM workdirを削除し、
`lsharp-linux-x86` を停止した。

この program は production selfhost source `ad65eaff` から生成された保存 artifactであり、現HEADの test/docs commitとは一致しない。
従ってこれは Linux x86_64の public command replay evidenceであって、current-source provenance gateではない。実 install/package registry、
対話 REPL、LSP全 semantic projection、component helper、stage0 acquisition/release/rollback、Mac/Linux packaged provenance parity、
`V2-16c` / `LEGACY-TOOL-01` aggregateは未完了のまま維持する。

### V2-16c Linux native MCP runtime boundary replay (2026-08-04)

`bb2725be` で、保存済み `ad65eaff` Linux x86_64 `App.Cli` target-only native programを
`scripts/ci/test-native-selfhost-mcp-runtime.py` から実行した。実際の
`scripts/native-selfhost-mcp.py` に対して JSON-RPC `initialize`、`tools/list`、
`lsharp_check`、`lsharp_format`、`lsharp_install`、`lsharp_validate` の6 requestsを送り、
`native MCP runtime contract passed: 6 requests`、exit `0`、stderr空を確認した。

selfhost CLIの legacy `check --format json` は clean sourceで
`command/type/diagnostics/migration/failureKinds` を返すため、shimは各 fieldを検証し、
diagnostics count `0`、migration空、failureKindsが空または全て `0` の場合だけ
`{ok:true, diagnostics:[], migrationDiagnostics:[]}`へ投影する。診断、migration row、
非ゼロ failure kindは構造化 MCP の owner/range semanticsを満たせないため、捨てずに明示
fail-closedとした。`lsharp_install`は既存の explicit external provider adapter boundaryを
返し、registry/network/provider helperを実行しない。

`lsharp_validate` は native processの exit `2`（source graphが unknown）をプロトコル失敗へ昇格せず、
`status:"unknown"` と zero counters の valid structured resultとして保持した。empty/ malformed JSONや
schema不一致は従来どおり tool error へ fail-closedする。

`cf3692e5` でこの6-request runtime contractを追加し、`1f0de6a7` で migration rowを含む legacy summaryの
negative contractを追加した。native MCP suiteは `113 tests` 全 passで、clean summary以外を structured MCP successへ
暗黙投影せず、migration・診断・非ゼロ failure kindを含む legacy outputを明示的に fail-closedすることを確認した。

これは production source `ad65eaff` から生成された保存 artifactへの replay-only evidenceであり、
現HEADの source provenance gateではない。native MCPの診断/migration structured parity、実
install/provider、LSP全 semantic projection、component helper、両target packaged release parity、
`LEGACY-TOOL-01` aggregateは未完了のため、`V2-16c`は`[~]`のまま維持する。

### V2-16c / V2-16e current-source Linux fixed point and MCP runtime (2026-08-04)

`cbbafe94` の current checkoutで、`LSHARP_NATIVE_LINUX_X86_SKIP_HOST_PROBES=1` と既存の VM-side lockを使い、Linux x86_64
actual stage1→stage2→stage3を一回だけ再生成した。stage1 manifestは target `x86_64-unknown-linux-gnu`、source commit
`cbbafe9423509030270a4f76ef909ff42ed23663`、code `4,434,755`、data `2,757`、entrypoint `4,432,195`、function-start `3,425`、
main function `3,434`。fixed-point summaryは `status=pass`、stage2/stage3 code length各 `11,448,943`、stdout SHA-256各
`a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e`、stderr空だった。host probeを省略したのは、今回の目的が
current source provenanceとstage fixed pointであり、個別 builtin probeを同じ仮説の証拠へ重複させないためである。

同じstage2 artifactを再利用した `App.Cli` target-only exportは target `x86_64-unknown-linux-gnu`、source tree SHA-256
`e4a69c95109f00bf3019572efe01c6dff0e7555d14be19308574d0cacadeeedd`、`selfhost_fixed_point=true`、program SHA-256
`a090cd8474c6115ac3a2bcf5570226cc912d7479d0285f6991ca02fb5a1d6469`、code `13,374,506`、`--version` `lsharp 0.1.0`、stderr空を
確認した。この実Linux programを `scripts/ci/test-native-selfhost-mcp-runtime.py` へ渡し、JSON-RPC 6 requests、`tools/list` の
`lsharp_validate` advertisement、check/format/install/validateのstructured/external boundaryを確認して
`native MCP runtime contract passed: 6 requests`となった。runtime作業領域とVMは終了後に回収・停止した。
同じ `App.Cli` programへ `scripts/ci/test-native-selfhost-cli-core-runtime.py` を渡した public core command matrixも
`native CLI core runtime matrix passed: 23 cases` となった。help/version、parse/check/fmt、text/metadata test、Preview1
compile/build、review/doc/doc-ack/doc-check、install/repl、bare LSP、`lsp --stdio` hover、validate unknown report、unsupported
doc formatを含み、成功経路の stderrは空だった。

これは current-source Linux native selfhostとMCP runtimeのverified partialであり、保存場所は
`ci-artifacts/native-linux-x86-hostgen-vm/cbbafe94-mcp-current-source-skip/` と `...-cli/` である。生成時点の source commitは
test-only commitの親であるため、後続の `0e0a6a6c`で追加したruntime assertionは同じprogramへ再実行して確認した。
release asset acquisition/rollback、Mac/Linux packaged provenance parity、全公開 command、native MCPのdiagnostic/migration
structured parity、実 install/provider、`LEGACY-TOOL-01` aggregateは未完了であり、`V2-16c` / `V2-16e`は`[~]`のまま維持する。

### V2-16c / V2-16e current-source Mac App.Cli release and runtime (2026-08-04)

`HEAD=a0845320584ba044c690404fd0249770a5118fff` が clean であることを確認した上で、
`scripts/ci/native-macos-aarch64-selfhost-release.sh` を一度だけ実行した。E2Eは
`e2e::selfhost_native_stage_chain::test_e2e_native_macos_aarch64_actual_app_cli_release_program` の
`1 passed` / `900.55s` で、current-source stage2→stage3 fixed-pointから `App.Cli` release programを生成した。

- artifact: `ci-artifacts/native-release/aarch64-apple-darwin/current-a0845320/`
- target: `aarch64-apple-darwin`
- manifest source commit: `a0845320584ba044c690404fd0249770a5118fff`
- `selfhost_fixed_point`: `true`
- program SHA-256: `281d178673c975d89fbb47b810d1fdac380935a173912e6fc46843107f5704c7`
- `file`: Mach-O 64-bit executable arm64
- artifact size: `4,748 KiB`
- `--version`: `lsharp 0.1.0`, stderr `0` bytes

同じ `program.native`を再利用して `scripts/ci/test-native-selfhost-cli-core-runtime.py` の public core CLI matrix
`23 cases` と `scripts/ci/test-native-selfhost-mcp-runtime.py` の JSON-RPC `6 requests` を実行し、いずれも passした。
MCPは `initialize`、`tools/list`、`lsharp_check`、`lsharp_format`、`lsharp_install`、`lsharp_validate` を含む。
生成後に cargo targetは自動削除され、Mac側の一時実行資源は残していない。

これは Mac current-source App.Cli/MCP runtimeのverified partialである。Linux x86_64とは生成時の source commitが異なるため、
同一commitの両target packaged provenance parity、公式 release asset acquisition、archive rollback、全公開 commandのsemantic
parity、native MCPのdiagnostic/migration structured parity、実 install/provider、component sidecar、Rust-free aggregateの
証拠には拡大解釈しない。`V2-16c` / `V2-16e` は `[~]` のまま維持する。

### V2-16b native map-remove receiver diagnostic contract (2026-08-04)

`cf9ac17f` で、`map-remove` の receiver mismatch を Rust oracle と native type-builtin matrixへ追加した。
fixtureは `(map-remove true 1)` で、Rust focused testは `1 passed`、
`e2e::selfhost_typeinfer_builtin_parity::` moduleは `10 passed`、current Mac App.Cli artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-a0845320/program.native` の native type-builtin matrixは
`5 tests` 全 passとなった。期待値は native CLI exit `1` と `function argument type mismatch` である。

同じ batchの初期REDとして key 型 mismatch `(map-remove (map-insert (map-new) 1 2) true)` も試したが、canonical `Map` は
key/value 型を保持しない消去型であり、`map-remove` の型は `forall k. (Map, k) -> Map` である。また既存の `Int` と heap handleの
runtime representation compatibilityにより `Int` receiverもこの型診断の境界にならないため、key mismatch/Int receiverを新しい
意味論として固定しなかった。selfhost implementationは変更していない。

変更は test-only なので、`a0845320` sourceから生成済みの Mac artifactを replayし、stage1→stage3 regenerationは重複実行していない。
これは Map receiver diagnosticの verified partialであり、Mapの key/value semantic typing、全 builtin family、全型診断、全公開
command、component/packaged release parity、Rust-free aggregateは未完了のまま `V2-16b` を `[~]` とする。

### V2-16b native ref-set source-level contract (2026-08-04)

`c97c01c0` で、`ref-new` と `ref-set` の let bindingを跨ぐ source-level contractを Rust oracle と native type-builtin matrixへ追加した。
valid fixtureは `(let [r (ref-new "x")] (ref-set r "y"))`、invalid fixtureは同じ `r` へ Intを書き込む
`(ref-set r 1)` である。Rust focused testは `1 passed`、`e2e::selfhost_typeinfer_builtin_parity::` moduleは `11 passed`、
current Mac App.Cli artifact `ci-artifacts/native-release/aarch64-apple-darwin/current-a0845320/program.native` の native matrixは
`5 tests` 全 passとなり、invalid pathは exit `1` と `function argument type mismatch` を返した。

既存の `Ref a -> a -> Unit` schemeとselfhost implementationは変更していない。test-only変更なので `a0845320` source由来の Mac artifactを
replayし、stage1→stage3 regenerationは重複実行していない。これは ref source-level lookup/applyの verified partialであり、全 builtin
family、全型診断、全公開 command、component/packaged release parity、Rust-free aggregateは未完了のまま `V2-16b` を `[~]` とする。

### V2-16c native didChange diagnostics refresh (2026-08-04)

`8cc58a4a` で、標準 LSP wireの `initialize -> didOpen(valid) -> didChange(invalid) -> publishDiagnostics` sequenceを、Rust oracleと
native CLI runtime matrixの同一 fixtureへ追加した。`file:///tmp/lsharp-lsp-didchange.ls` を didOpen、didChange、publishDiagnosticsの
全てで保持し、invalid source `(defn bad [] (+ 1 true))` に対して次の標準 Diagnostic objectを要求した。

```json
{
  "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
  "severity": 1,
  "code": "LS1004",
  "source": "lsharp",
  "message": "function argument type mismatch"
}
```

Evidence:

- Rust focused E2E `e2e::selfhost_cli_core::test_e2e_selfhost_cli_lsp_stdio_didchange_publishes_type_diagnostics_with_uri` は `1 passed` / `336.41s`。
- Mac Apple Siliconの current-source `a0845320` App.Cli artifact
  `ci-artifacts/native-release/aarch64-apple-darwin/current-a0845320/program.native` を再利用した
  `scripts/ci/test-native-selfhost-cli-core-runtime.py` は `native CLI core runtime matrix passed: 24 cases`、exit `0`、stderr空だった。
- native sequenceは4 frames（initialize response、didOpen projection、didChange projection、publishDiagnostics）を返し、didOpen/didChange
  の source byte数とwire URI、publishDiagnosticsの URI・severity・code・source・message・rangeを完全一致で検証した。
- `scripts/ci/test-native-selfhost-cli-core-runtime.py` の `py_compile` と `git diff --check` は passした。今回 `selfhost/src` は変更していないため、
  production source `a0845320` からの重い stage1 -> stage3 regenerationは重複実行していない。

これは didChange後の type diagnostics refreshとwire URI/標準 Diagnostic fieldsに限定した verified partialである。Linux current-source
App.Cliでの同じ direct LSP sequence、全 diagnostics/type/lint parity、複数診断の全文、definition/references/renameの全 semantic
projection、cross-document URI provenance、component sidecar、release asset acquisition/rollback、Mac/Linux packaged provenance parity、
Rust-free aggregateは未完了であり、`V2-16b` / `V2-16c` / `V2-16e` は `[~]` のまま維持する。

### V2-16c native stale diagnostics clear (2026-08-04)

`1512cac9` で、同じ string URIに対する `didChange(invalid) -> publishDiagnostics(LS1004) -> didChange(valid) -> publishDiagnostics([])`
sequenceを追加した。invalid sourceで公開した type diagnosticが、valid sourceへの更新後に同じ URIの空 diagnosticsへ置き換わり、
stale recordが残らないことをRust oracleとnative CLI runtimeで確認した。

Evidence:

- Rust focused E2E `e2e::selfhost_cli_core::test_e2e_selfhost_cli_lsp_stdio_didchange_clears_stale_type_diagnostics` は `1 passed` / `334.00s`。
- Mac Apple Siliconの current-source `a0845320` App.Cli artifact
  `ci-artifacts/native-release/aarch64-apple-darwin/current-a0845320/program.native` を再利用した native core CLI matrixは
  `native CLI core runtime matrix passed: 25 cases`、exit `0`、stderr空だった。
- Linux x86_64では保存済み `cbbafe94` App.Cli ELF
  `ci-artifacts/native-linux-x86-hostgen-vm/cbbafe94-mcp-current-source-cli/program.native` をLima VMへコピーして同じmatrixを実行し、
  `native CLI core runtime matrix passed: 25 cases`、exit `0`、stderr空を確認した。VM作業領域は削除し、`lsharp-linux-x86` は停止済みである。
- `cbbafe94` から現HEADまで `selfhost/src` の差分はないため、stage1 -> stage3 regenerationは重複実行していない。ただしLinux artifactの
  manifest source commitは現HEADと異なるため、このLinux結果は replay-only evidenceであり、current-source provenance gateへ拡大解釈しない。

これは stale diagnostics clearの verified partialである。current-source Linux packaged provenance、全 diagnostics/type/lint parity、複数診断の全文、
definition/references/renameの全 semantic projection、cross-document URI provenance、component sidecar、release asset acquisition/rollback、
Mac/Linux packaged provenance parity、Rust-free aggregateは未完了であり、`V2-16b` / `V2-16c` / `V2-16e` は `[~]` のまま維持する。

### V2-16b / V2-16c native standard lint Diagnostic projection (2026-08-05)

1409e18b / d3e852a6 で、review の unused-let lint diagnostic を legacy の source/rule/line/col/messageHash 配列から
標準 LSP Diagnostic objectへ投影した。REDでは current selfhost bundleが legacy payloadを返すことをRust actual E2Eで確認し、
GREENでは point range、severity 2、code L0001、source lsharp、message let binding unused is not used を固定した。

Rust focused E2E は RED 357.54s の後、GREEN 1 passed / 341.85s。Mac Apple Silicon current-source release gateは
artifact 4,748 KiB、manifest source_commitは current HEAD、selfhost_fixed_point=true、stderr 0、
native core runtime matrix 26 cases全 passだった。

Linux x86_64の actual self-regeneration summaryは target x86_64-unknown-linux-gnu、host Linux/x86_64、status pass、
stage2/stage3 code length各 11,448,943、stdout SHA-256各
a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e、stderr空の一致を記録した。
保存済みstage2をVM-side lock付きで再利用した target-only App.Cli artifactは source commit
d3e852a6572cbdf4ea705eee851d67230b20772e、code 13,375,178、program SHA-256
b155abe13cb16c71f6c34e02152b33b4f819c9a8cceb769386740317f3a6f988、--version smoke stderr 0だった。
同じLinux ELFを停止前のLima VM内で native core 26 cases、type builtins 5 tests、MCP 6 requests全 passした。

続く test-only batch では、同じ projection を `empty-do` rule へ拡張した。Rust actual bundle の
`test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_empty_do_diagnostic` は 1 passed / 430.65s で、
point range、severity 2、code L0002、source lsharp、message `do block has no expressions` を固定した。Mac Apple Silicon と
Linux x86_64 の保存済み current-source App.Cli artifactに同じ native runnerを適用し、unused-let と empty-do を含む native core
runtime matrix は各 27 cases 全 pass となった。Linux は x86_64 VM 内で実行し、VM内の一時 program/script は検証後に回収した。
production selfhost source は d3e852a6 から変更せず、後続 c50b7b3c は docs-only のため、既存の d3e artifact を replay-only で再利用した。

これは unused-let と empty-do の標準projectionに限定したverified partialであり、複数lint診断の順序/dedup、
正確なspan end、全rule code/message parity、全diagnostics/type/lint parity、definition/references/renameの全semantic projection、
component/packaged release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。
検証後はVM workdir、replay lockを削除し、lsharp-linux-x86を停止した。Evidence commits: 1409e18b, d3e852a6, 6e09ff86。

### V2-16b native remaining builtin argument diagnostics (2026-08-04)

`16871a7a` で、builtinの引数位置別 mismatchを一つのfixture familyへ追加した。対象は文字列の `string-concat`、`string-eq`、
`string-char-at`、`substring`、Vectorの `vector-new`、`vector-set`、`vector-push`、Mapの `map-get`、`map-contains?`、Refの `ref-get` で、
receiver mismatchはBool、index/size/argument mismatchはBoolまたはIntを使い、Rust/native双方で `function argument type mismatch` になることを固定した。

Evidence:

- Rust focused E2E `e2e::selfhost_typeinfer_builtin_parity::test_e2e_selfhost_typeinfer_remaining_builtin_argument_mismatches` は `1 passed`。
- Mac Apple Siliconの current-source `a0845320` App.Cli artifactを再利用した `scripts/ci/test-native-selfhost-type-builtins.py` は `5 tests` 全 pass。
- Linux x86_64では保存済み `cbbafe94` App.Cli ELFをLima VMへコピーして同じsuiteをreplayし、`5 tests` 全 pass、stderr空を確認した。VM作業領域は削除し、
  `lsharp-linux-x86` は停止済みである。
- `selfhost/src` は変更していないため、Mac/Linuxのstage1 -> stage3 regenerationは重複実行していない。Linux artifactのmanifest source commitは現HEADと
  異なるため、Linux結果はreplay-only evidenceとして扱う。

初期REDで試した Vector/Map/Ref receiverへのInt mismatchは、Rust oracleの既存 `Int`/heap handle compatibilityにより受理された。同じ理由で、
canonical Mapがkey/value型を保持しないkey mismatchと同様、この入力は新しい診断契約に採用していない。これはbuiltin argument diagnosticsの
verified partialであり、全builtin family、全型診断 code/span、runtime/codegen parity、全公開command、component sidecar、release asset
acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了のため、`V2-16b` / `V2-16c` / `V2-16e` は `[~]` のまま維持する。

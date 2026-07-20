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

Evidence: RED の `test_e2e_selfhost_test_runner_projects_and_runs_ordered_assertion_forms`、GREEN の同 test、`cargo run --bin lsharp -- check selfhost/src/Tools/Test/TestRunner.ls` / `Cli.ls` / `EmbeddedCli.ls`。full CLI bundle は Rust type inference の待ち時間が大きいため default E2E から分離し、`test_e2e_selfhost_cli_reports_canonical_assertions` を ignored manual gate として残す。

これは parser-owned predicate projection と限定 evaluator の verified slice であり、predicate source span、Rust checker/oracle との assertion diagnostic parity、undefined-variable の専用診断、全 AST/runtime の assertion evaluation、legacy migration、Mac/Linux current-source artifact/runtime gate は残件である。したがって `:assert` は selfhost runner の supported subset で実行可能になったが、EC-M1-03/04 または全機能 Rust-free 完了とは扱わない。

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

### record runtime 更新 (2026-07-14)

自己ホスト Wasm compiler は `CompilerMode` の file-compile 経路と legacy `compile-program-functions` / `compile-program-functions-with-base` で、`RecordLit`、`RecordUpdate`、direct `FieldAccess`、nonparametric record の `Point ...` constructor、`Point.field` static accessor を既存の `Map` runtime に lower する。record 本体を field 式の allocation 中も root に保持し、field hash を key に `map-insert` / `map-get` を使う。record update は更新 field だけを持つ patch Map に base Map を sentinel key `-1` で保持し、field lookup が patch chain を再帰的に辿るため、元の record は変更されない。record constructor と static accessor は user `defn` より前に prelude として function table / Wasm body へ登録し、Wasm entrypoint が最後の user function のままになる順序を保つ。actual compiler-mode E2E は `{Point label "record" x 42}` から `(. point label)` を `string-length` へ渡して `6`、`(. point x)` から `42` を出力し、`Point (inc 40) 2` の `Point.x` / `Point.y` が `41` / `2` を出力することを確認した。import された別 module の `Point` でも同じ `41` / `2` を generated Wasm で確認し、parametric `Box Int` / `Box Bool` の別具体化も `41` / `1` を出力する専用 E2E で確認した。さらに `p -> q -> r` の nested update を static / dynamic access で読み、`p` の値が保持されることを `test_e2e_selfhost_compiler_mode_record_update_runs` と ftable 経路の `test_e2e_selfhost_ftable_compiler_record_update_and_static_accessor_run` で確認した。normal compiler-mode の 11-import ABI に function table base `11` を揃え、direct source compile、imported file compile、`Cli` / `EmbeddedCli` の source-to-Wasm 入口で constructor/accessor call が runtime import と衝突しないことを確認した。record pattern は未完である。`App.Cli`、`EmbeddedCli`、`SmokeCli`、`PipelineSmoke` の source/full helper は full functions/data payload を `build-wasm-bytes-wasi` へ渡し、`PipelineSmoke` は Rust host compile と Wasm validate まで確認した。一方 `run-main-smoke` の単一 AST `lower` は診断用に残り、no-arg pipeline entrypoint の full-program runtime/native E2E と component sidecar の生成は未完了である。generated Wasm の opcode 87 (`print-string`) は通常の `CompilerMode` 出力で 11 番目の `env` runtime import への `call 10` として出力するところまで修正済みだが、外部 runtime の文字列 ABI 接続と standalone WASI Preview1 実行は別の output parity gap として残る。

source / ftable compiler-mode では record literal / static constructor が nominal marker `-3` を Map に保存し、canonical record pattern の type hash と照合する。同じ field layout を持つ別 record type の arm fallback、ftable nominal pattern の独立 E2E、`p -> q -> r` patch/base Map chain への marker 伝播を確認済みである。さらに source / ftable compiler-mode の nonparametric nested record pattern で親・子 field binder、nested literal child、record field 内の nested constructor child を actual Wasm 実行まで確認した。`map-contains?` / `map-remove` / `map-size` は integer key と string literal key の source / ftable actual Wasm slice まで確認済みだが、その先の一般 Map API parity は残課題である。

### legacy source compile boundary 更新 (2026-07-14)

`App.Cli`、`EmbeddedCli`、`SmokeCli`、`PipelineSmoke` の source/full helper は、`parse-program` の結果を `compile-program-functions-with-source` に渡し、先頭 IR だけを返す `lower` ではなく全 functions/data を `build-wasm-bytes-wasi` へ渡す。これにより helper 自体は複数 top-level function を落とさない。`PipelineSmoke` は Rust host compile と Wasm validate まで確認した。`EmbeddedCli` の component target は summary text を出力せず、外部 component packaging が必要な境界を明示的に返す。一方 `run-main-smoke` の単一 AST `lower` は診断用として残り、App.Cli / EmbeddedCli の component sidecar、no-arg pipeline entrypoint の full-program runtime/native E2E は別の未完了 surface である。

### 型・宣言意味論の更新: ordinary ADT (2026-07-14)

ordinary ADT は parser が variant 名と raw field TypeExpr を保持し、`TypeInferAdt.ls` の prepass が type parameter を束縛した constructor scheme を値環境へ登録する。`(type (Maybe a) (Just a) Nothing)` の constructor application と match pattern は同じ polymorphic scheme を使い、`Int` と `Bool` の別使用箇所で独立に instantiate される。さらに selfhost Wasm compiler の source / ftable 経路で Map-based constructor、variant tag、direct field binder、nested constructor pattern、constructor mismatch fallback を actual Wasm 実行で確認した（`test_e2e_selfhost_compiler_mode_adt_constructor_pattern_binds_and_falls_back`、`test_e2e_selfhost_compiler_mode_adt_nested_constructor_pattern_runs`、`test_e2e_selfhost_ftable_compiler_adt_constructor_pattern_runs`）。複数フィールドの内部 key は Map runtime の空スロット sentinel `0` を避ける `idx+1` 契約を constructor insertion / pattern check / binder で共有する。これは ordinary ADT の parser / 型検査と source / ftable runtime の Rust-free slice を示すが、full ftable/import target parity、Rust linear-memory ABI parity、nominal/exhaustiveness closure は未完了である。GADT の variant return type、pattern refinement、exhaustiveness も含まれない。

## 現在の事実

- `lsharp-native-selfhost-stage0` package は `compiler`、`transport_driver`、`materializer`、40 桁 lowercase hex の `source_commit` を持つ manifest で native bootstrap を開始する。`scripts/native-selfhost-dev.sh` は provenance のない manifest を拒否する。release 用の `App.Cli` archive は stage0 package ではない。
- Mac Apple Silicon では、current fixed-point stage3 compiler を stage0 package 化し、`scripts/native-selfhost-dev.sh` を通す source-file smoke が成功している。smoke は `cargo`、`rustc`、host `lsharp` を PATH 上で失敗させた状態で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を実行する。
- Linux x86_64 は、commit `4bd9ee9` から生成した fresh actual-stage1 を stage0 package 化し、Lima `lsharp-linux-x86` VM 内で source-file smoke を成功させた。続く current-source stage0 `/tmp/lsharp-native-linux-x86-stage0-7807089` の再確認でも、`LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=64`、timeout 1200 秒で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を完走した。実行中は `cargo`、`rustc`、host `lsharp` を blocklist にし、VM は 11 GiB disk 中 3.2 GiB 使用（30%）で終了、temporary workdir/lock は残していない。`7807089` の actual stage1 -> stage2 -> stage3 selfregen も別 gateで pass している。2026-07-14 の historical `8dd37ef-static-string-fixedpoint` replay における `parse stdout is missing decls:1` は、fresh stage0 で解消された過去の failure evidence として残す。
- 2026-07-19 の current HEAD `c5c9751d53a6d8845a24c61593a0364aecad09b1` では、Linux x86_64 actual stage1 の `source_commit` を検証してから、現行の data/heap frontier materializer を含む stage0 package を Lima `lsharp-linux-x86` で再作成した。この package から current `selfhost/` source を再生成し、`LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=64`、timeout 900 秒、`cargo` / `rustc` / host `lsharp` blocklist の source-file smoke を実行して `parse`、`check`、`fmt`、通常/metadata/property `test`、`compile -o`、`build -o` を完走した。actual stage1 -> stage2 -> stage3 selfregen も同じ source commit で pass し、stage2/stage3 の code length は各 10,744,009 bytes、stdout SHA-256 は `50111731985fe62d4107aaafa2a2afecfff035a1796caa6f74748e65404b5163b` で一致した。これは Linux x86_64 の current-source daily core boundary を閉じる evidence であり、EC-M1-04/05/06 の各 Linux evidence を補うが、各 milestone 全体、Mac/Linux の aggregate parity、公開 surface、未移行 semantics の完了を意味しない。
- native bootstrap の初回だけは source tree を再生成する。fingerprint が不変なら `scripts/native-selfhost-dev.sh` は生成済み `program.native` を再利用する。
- repo 内の旧 stage0 artifact に `source_commit` がない場合は、native runner の成功経路へ再利用せず、source commit と fixed-point evidence を付けた package を再生成する。
- `LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY` は macOS host policy 上、生成済み Mach-O の実行に署名が必要な環境でだけ指定する。成功時の codesign 出力は command stderr に漏らさず、失敗時だけ診断として返す。
- GitHub Actions の自動 build は使わない。検証と release は Mac と Lima VM の手動 local gate で行う。

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

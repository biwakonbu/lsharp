# ADR: selfhost の 0 引数 `defn` を `Unit -> body` として登録する (2026-08-22)

- **Status**: accepted
- **Date**: 2026-08-22
- **Scope**: `selfhost/src/Types/TypeInfer.ls` の `infer-defn-predeclared` (param-count 0 の分岐)
- **Related**: `ISSUES.md` `I-45` / `I-49` / `TODO.md` `CASE-ZERO-ARITY-01` /
  [`decisions-worktree-absorption-2026-08-20.md`](decisions-worktree-absorption-2026-08-20.md) の群 3

## Context

`:case [(expect (zero) 1)]` のように **引数を取らない `defn`** を canonical `:case` の
`expect` 内で呼ぶと、`cases:0` / `executed:0` のまま `status:"fail"` / exit 1 になっていた。
期待値が合っているか外れているかに関係なく同じ結果になるため、`:example` から `:case` への
移行が機械的にできない。

観測は selfhost lane に固有である。`lsharp test` は `--format json` を付けない限り
embedded selfhost component へ委譲され (`crates/lsharp-driver/src/main.rs:1080`)、
Rust 実装 (`--format json` / `LSHARP_DISABLE_EMBEDDED_COMPONENT=1`) は同じソースを
`status=pass cases=1 executed=1` / exit 0 で通す。

原因は evaluator ではなく型推論の内部不整合だった。

| 箇所 | 0 引数をどう扱っていたか |
|---|---|
| `TypeInfer.ls:466-486` `infer-defn-predeclared` | param-count 0 の `defn` を **body の型そのもの** (`zero : Int`) で env へ登録 |
| `TypeInferApply.ls:688-716` `infer-apply-legacy-raw` | argc 0 の apply に **`Unit -> a`** を要求 |
| `TypeInferApply.ls:33-45` `infer-lambda` | param-count 0 の lambda を **`Unit -> body`** で構築 |
| Rust `lsharp-types` | 0 引数 `defn` を `Fun([], Con("Int"))` で保持 |

食い違うのは `infer-defn-predeclared` の 1 箇所だけである。unify が落ちると
`check-case-expectation` (`TypeInferAssertions.ls:1481-1535`) が `infer-expr` の失敗を
一律 `canonical-case-type-error-code` = 1001 へ潰し、`EmbeddedCli.ls:1065-1078` の
preflight が suite 生成前に短絡するので `cases:0` になる。

## Decision

`infer-defn-predeclared` の param-count 0 分岐で `fun-ty = (mk-fun (mk-unit) body-ty)` を作り、
placeholder との unify と `typeinfer-finalize-defn-result-with-env-vars` の両方へ `fun-ty` を渡す。
param-count 1 以上の分岐 (`infer-defn-parameterized-predeclared`) が
`typeinfer-build-curried-fun` の結果を同じ 2 箇所へ渡しているのと形を揃えた。

`typeinfer-defn-return-annotation-subst` へは **unwrapped の `body-ty` のまま**渡す。
これは戻り値注釈 (`:returns` 相当) と body の型を突き合わせる処理であり、
関数型を渡すと注釈側と形が合わなくなる。

## 却下した選択肢

- **apply 側を緩める** (`infer-apply-legacy-raw` の argc 0 で非関数の callee を許す)。
  同じ selfhost の `infer-lambda` が 0 引数 lambda を `Unit -> body` にしているので、
  lambda と `defn` で 0 引数の意味が割れる。Rust 実装 (`Fun([], _)`) からも離れる。
  収束先を 2 つ作る修正であり、`:case` が緑になっても不整合は残る。
- **`check-case-expectation` の 1001 を握り潰す / preflight を `:case` で無効化する。**
  症状 (`cases:0`) は消えるが、型の食い違いはそのまま残り、
  今度は本物の型エラーが preflight をすり抜ける。安全側の壊れ方を危険側へ倒す変更になる。
- **Rust 側を selfhost に合わせる** (0 引数 `defn` を body の型で持つ)。
  Rust lane は既に正しく、`(expect zero 1)` に対して `actual=() -> Int, expected=Int` と
  正確に報告できている。正しい方を壊す向きなので採らない。

## Evidence

RED は `crates/lsharp-driver/tests/metadata_test_selfhost_case_arity.rs` (新規)。
受入条件どおり `lsharp test` の **exit code と `coverage.executed` の両方**を見て、
arity 1 の control を同じ fixture 群に置いた。

| test | 修正前 | 修正後 |
|---|---|---|
| `selfhost_case_zero_arity_actual_side_is_executed_and_passes` | FAIL (`executed=0 code=1001`) | ok |
| `selfhost_case_zero_arity_expected_side_is_executed_and_passes` | FAIL (`executed=0 code=1001`) | ok |
| `selfhost_case_zero_arity_mismatch_is_executed_and_fails` | FAIL (`executed=0`、実行されずに fail) | ok |
| `selfhost_case_arity_one_control_is_executed_and_passes` | ok | ok |
| `selfhost_case_arity_one_mismatch_control_is_executed_and_fails` | ok | ok |

修正前の 3 FAIL / 2 pass は「arity だけが変数」であることを示している。

### 影響範囲の計測

`I-48` の前例 (類似の修正で失敗 defn が 0 → 262 件になった) を踏まえ、着地前に計測した。

非 e2e の 6 crate を `--no-fail-fast` で全 target 走らせた (2026-08-22、修正後)。

```bash
cargo test --no-fail-fast -p lsharp-driver -p lsharp-types -p lsharp-ir \
  -p lsharp-tooling -p lsharp-syntax -p lsharp-lsp
```

**1592 passed / 15 failed。** 失敗 15 件は
`docs/development/validation/workspace-expected-failures.txt` が
この 6 crate について挙げている 15 件と **完全に一致** (`diff` で 0 行差)。
新規 FAIL は 0 件、pass へ転じた expected も 0 件である。

自己適用も確認した。修正後の selfhost から作った embedded component で
selfhost 自身の entry を全モジュールごとコンパイルできる。

```bash
./target/debug/lsharp compile selfhost/src/App/EmbeddedCli.ls -o /tmp/selfapp.wasm
# => コンパイル成功: ... (1211823 bytes) / exit 0 / real 0m55.6s
```

`I-48` の前例で問題になった「修正パッチ下で selfhost 自身が型検査を通らなくなる」形は
本修正では起きていない。

### 契約変更に追随させた e2e 5 本 (2026-08-23)

下の「計測していない範囲」に挙げた **workspace e2e lane** を後日 sweep したところ、
本 ADR の契約変更前の型を pin していた e2e が赤のまま残っていた (`I-60`)。
契約の正本は本 ADR なので、期待値の側を新契約へ張り直した。
**「実装に合わせて期待値を変える」禁止則の例外**であり、根拠は
「契約 ADR が先に変わっており、test が旧契約を pin していた」ことである。

| test | 旧期待 | 新期待 |
|---|---|---|
| `selfhost_lexer_parser::..._program_analysis_preserves_first_defn_type` | `["1","100"]` | `["3","1","100"]` |
| `selfhost_lexer_parser::..._gadt_constructor_registers_refined_return_type` | `["0","5","1","1","100"]` | `["0","3","5","1","1","100"]` |
| `selfhost_typeinfer_pipeline_bootstrap::..._pipeline_complete_stages` | `ty_tag == 1` | `ty_tag == 3` |
| `selfhost_main_module_determinism::..._pipeline_macroexpand_typeinfer_integration` | `ty_tag == 1` | `ty_tag == 3` |
| `strings_patterns_compiler_integration::..._selfhost_main_integration` | `lines[28] == "1"` | `lines[28] == "3"` |

前 2 本は harness が `.rs` 内の inline `.ls` なので、`ty-fr` / `type-fun-ret` で `Fun` を
剥がしてから戻り型を pin する形へ書き換えた。**tag 3 (Fun) 自体も pin に残した**ので、
新契約が壊れれば test が落ちる。

後 3 本は `PipelineSmoke.ls` の `compile-full-pipeline` が出す 5 要素 summary を読む。
ここは **summary の slot 1 の意味を変えた** — `(vector-get ty-result 1)` を素で出すと
`Fun` の param slot (pointer) が出てしまうため、`Fun` (tag 3) のときは戻り型の名前ハッシュを
出すようにした (`PipelineSmoke.ls:98-103`)。slot 0 は生の tag のままなので **新契約 (Fun=3) が
pin され、値の型 (Int=100) の pin も残る**。

**slot 数は 5 のまま変えていない。** print 回数を変えると `lines[30]` / `lines[31]` を読む
別 test と `lines.len() >= 32` ガードが全部ずれる。

```
# 旧実装での RED (2026-08-23)
/Users/biwakonbu/github/tmp/lint-span-01/base4.log   0 passed; 4 failed; 80.07s
/Users/biwakonbu/github/tmp/i60/base5.log            0 passed; 2 failed; 88.93s
# 張り直し後の GREEN
/Users/biwakonbu/github/tmp/i60/green5.log           5 passed; 0 failed; 112.24s
```

### 6 本目 (2026-08-23)

予告どおり、下限は下限だった。`ASSERT-DIAG-MESSAGE-01` の回帰 lane
(`cargo test -p lsharp-wasm --test e2e selfhost_cli_actual_main_args`) で 6 本目が出た。

| test | 旧期待 | 新期待 |
|---|---|---|
| `selfhost_cli_actual_main_args::test_e2e_selfhost_cli_main_check_json_aliases` (`EC-M1-03`) | `reports[0]["type"] == "Int"` | `== "Fn"` |

fixture は `(defn main [] 42)`。`render-type-text` (`Cli.ls:715` / `EmbeddedCli.ls:114`) は
ty-fun (tag 3) を `"Fn"` へ潰すので、`Unit -> Int` になった `main` は `"Fn"` を返す。
**この test は前 5 本と違い、型そのものではなく `check --json` の利用者向け出力を見ている。**
つまり本 ADR の契約変更は `lsharp check` の `type` フィールドという **user-visible な出力**まで
変えていた。前回の追随ではそこまで届いていなかった。

```
# RED (回帰 lane で観測、2026-08-23)
/Users/biwakonbu/github/tmp/assertdiag/reg2.log   FAILED. 17 passed; 1 failed; 25 ignored; 1182.20s
                                                  left: String("Fn") / right: "Int"
# 張り直し後の GREEN
/Users/biwakonbu/github/tmp/i60b/green.log        ok. 1 passed; 0 failed; 262.18s
```

`ASSERT-DIAG-MESSAGE-01` の回帰でないことは、当該 slice の編集を一切含まない凍結済み
`target/debug/lsharp` が既に `{"command":"check","type":"Fn",...}` を返すことで確認した。

**`"Fn"` は情報量が乏しい**が、これは 0 引数 defn に固有の問題ではない。
`render-type-text` は arity を問わず全ての関数型を `"Fn"` へ潰しており、
`I-45` はその bucket に 0 引数 defn を加えただけである。arrow 型を描画する話は
本 ADR の範囲外。

**6 本もまだ下限である。** sweep が覆ったのは e2e 約 3,075 本のうち 511 本 + 今回の
2 lane (`selfhost_cli_actual_main_args` 43 本 / `selfhost_cli_core` 442 本) で、
`914bd9f1` 以降 full lane は一度も回っていない。全数確定は次の full lane に委ねる。
`workspace-expected-failures.txt` へは追記していない — 同ファイルの正本は 2026-08-16/17 の
計測で、その時点では 5 本とも緑だったからである。

### 7〜11 本目 (2026-08-27、`--ignored` 全量 sweep で確定)

前節の「**6 本もまだ下限である**」は当たっていた。`I-64` の `--ignored` lane 全量 sweep
(2026-08-24、18 module / 1,431 件) が `914bd9f1` 以降はじめて full lane を回し、
**残り 5 本が確定した**。合計 11 本。

| test | 旧期待 | 実測 = 新期待 |
|---|---|---|
| `selfhost_cli_actual_main_args::..._main_with_args_check_file` | `"Int"` | `"Fn"` |
| `selfhost_cli_actual_main_args::..._main_with_args_check_json_file` | `report["type"] == "Int"` | `== "Fn"` |
| `selfhost_cli_core::..._check_file_handler` | `"Int"` | `"Fn"` |
| `selfhost_cli_core::..._check_source_core` | `"Int"` | `"Fn"` |
| `selfhost_cli_core::..._check_source_builtin_application_type_contract` | `"Bool"` | `"Fn"` |

5 本とも fixture は 0 引数 `defn` で、`run-check-program` (`Cli.ls:744-745`) が
`infer-program-analysis-type` の返す **program 全体の型**を `render-type-text` へ渡す経路である。
**11 本すべてが同一の機序**で、新しい失敗モードは出なかった。

#### 5 本目は別扱いを要した — 判別の記録

`..._check_source_builtin_application_type_contract` だけ旧期待が `"Int"` ではなく `"Bool"` で、
fixture も `(defn probe [] (not true))` と `(not x)` を含む。ここには
**`render-type-text` が適用結果を関数型へ潰している**という別の仮説が立ちうる。
それが正しければ本物の型付けバグであり、pin を `"Fn"` へ動かすのは**バグの塗り潰し**になる。

**pin を動かす前に判別した。結果は契約側**である。`run-check-program` が
`render-type-text` へ渡すのは program の型であって式の型ではない。`(not true)` の型は
`render-type-text` に一度も到達しない。program の末尾は `defn` なので、`I-45` 適用後は
arity を問わず常に関数型であり、`"Fn"` は正しい出力である。**実装は触っていない。**

**ただしこの pin は緑になっても検査を失っている。** `"Fn"` はどの `defn` に対しても返るので、
builtin `not` の戻り値型を 1 ビットも区別しない。test 名が主張する検査は消えた。
**緑になることと検査していることは別である。** 失われた coverage は `ISSUES.md` の `I-76` /
`TODO.md` の `CHECK-BUILTIN-RET-COV-01` が保持する。ここで代替経路を作らないのは、
「`check` に式の型を問う口を足すか、`TypeInferBuiltins` の unit test へ寄せるか」が
契約の話であり、pin 追随の slice で決めるべきでないからである。


```
# RED (--ignored 全量 sweep で観測、2026-08-24)
/Users/biwakonbu/github/tmp/i64/mod-selfhost_cli_core.log            left: "Fn" / right: "Bool"
/Users/biwakonbu/github/tmp/i64/mod-selfhost_cli_actual_main_args.log  left: "Fn" / right: "Int"
# 張り直し後の GREEN
/Users/biwakonbu/github/tmp/checkpin/pin.log        ok. 5 passed; 0 failed; 311.96s
```

### 計測していない範囲

- **selfhost の自己適用 (stage chain)**。`#[ignore]` lane
  (`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`) と
  `./stage0` を要する bootstrap は本 slice では回していない。
  修正後の selfhost が**自分自身を**コンパイルできるかは未検証である。
- workspace 全体の e2e lane (実測 5h38m)。

### `I-46` / `I-48` との関係

`lsharp compile` は同じ 0 引数呼び出しを含むプログラムを修正前から通していた。
compile 経路は pass-1 の生 placeholder を unify するため矛盾が顕在化せず、
確定した analysis env を見る `:case` preflight だけが露出させていたと見られる。
**本修正は placeholder の穴 (`I-46` / `I-48`、`TypeInfer.ls:485`) を閉じない。**
前方参照経由の呼び出しは従来どおり素通りするので、`INFER-FORWARD-GEN-01` の
`[BLOCKED: I-48]` は本修正では解けない。

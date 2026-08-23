# selfhost runner の `cases` / `coverage.executed` は「実行した contract 数」を数える

- **Status**: accepted
- **Date**: 2026-08-23
- **Scope**: `selfhost/src/Tools/Test/TestRunner.ls` の `run-examples-loop` が
  test result の `actual` slot へ何を入れるか。`assurance-result-actual-loop`
  (`selfhost/src/App/Cli.ls:899` / `selfhost/src/App/EmbeddedCli.ls:848`) の集計規則は変えない。
- **Related**:
  [`I-67`](../../ISSUES.md#i-67) (本 ADR が解く問題),
  [decisions-selfhost-example-fail-reason.md](decisions-selfhost-example-fail-reason.md)
  (同じ `run-examples-loop` を触った直前の slice),
  [decisions-selfhost-contract-quote-parity.md](decisions-selfhost-contract-quote-parity.md)

## 問題

`:example` が 1 件失敗する fixture で、selfhost runner の JSON が
`cases 0` / `coverage.executed 0` / `coverage.failed 1` を返す。
「1 件も実行していないのに 1 件失敗した」という自己矛盾した report になっている。

rust runner (oracle) は同じ形の fixture に対して `cases 1` / `executed 1` / `failed 1` を返す
(`crates/lsharp-driver/tests/metadata_test_cli.rs:96`, `:assert` 版)。

## 根本原因

集計側 `assurance-result-actual-loop` は test result の index 2 (`actual`) を総和する。
その `actual` に何を入れるかが kind ごとにバラバラで、`:example` だけが `passed` を入れている。

| kind | 構築箇所 | 診断なし時の `actual` | 意味 |
|---|---|---|---|
| `:assert` | `run-assertions-loop` (`:4165`) | `1` | 実行した contract 数 |
| `:case` | `run-cases-loop` (`:4287`) | `(value-int-or-bool actual)` | 式の**値** |
| `:invariant` | `materialize-invariant` (`:5178`) | `sample-count` | サンプル数 |
| property | `materialize-property-with-span` (`:4939`) | `actual-count` | 実行サンプル数 |
| `:example` | `run-examples-loop` (`:4161`) | **`passed`** | 真偽 |

診断が立った result はどの kind も `actual = 0` を入れる。これは rust の preflight が
`cases 0 / executed 0` を返す挙動 (`crates/lsharp-driver/src/main.rs:1510`) と一致しており、
**意図的な設計**である。

`:example` は「実行したか」ではなく「通ったか」を入れているため、失敗すると
実行数から消える。これが `I-67` の直接の原因。

## 決定 (案 A): `:example` の `actual` を「実行したら 1」に揃える

`run-examples-loop` の両分岐 (pass / fail) で `actual = 1` を入れる。
`:assert` と同じ形にする。集計側には一切触らない。

理由:

- oracle と一致する。rust の `MetadataTestRun::total()`
  (`crates/lsharp-tooling/src/metadata_test.rs:19`) は `results.len()` であり、
  pass / fail に関わらず contract 1 件を 1 と数える。
- **既存の pass 系 pin は数値的に不変**。通っている `:example` は `passed = 1` なので
  変更前後どちらも `actual = 1`。動き得るのは「失敗した `:example`」の値だけで、
  その値を固定した pin はリポジトリ内に 1 件も無い (下記 Evidence)。
- 外れ値 1 つを他の 4 kind に寄せる変更であり、slot の意味を全 kind ぶん再解釈しない。

## 却下した案

### 案 B: 集計を `vector-length` の総和に変える

`assurance-result-actual-loop` を捨て、result 数をそのまま数える。

**却下**。診断で落ちた result まで数えてしまう。non-Bool invariant fixture は現在
`cases 0 / executed 0` を返し、これは rust の preflight 値と一致していて
**live な e2e が固定している** (`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs:480`,
`assert_non_bool_invariant_json`)。案 B はこれを `1 / 1` に変え、いま取れている parity を壊す。
`I-67` を直すために別の parity を壊すのは差し引きで損。

### 案 C: 集計を「`actual > 0` なら 1 を足す」に変える

**却下**。二重に外す。第一に `:example` の失敗は `actual = 0` のままなので `I-67` が解けない。
第二に `:case` は式の**値**を入れるので、`(expect (f) 0)` のように値が 0 の正常な case が
実行数から消える。今より悪い。

### 案 D: `:case` / `:invariant` / property の `actual` も同時に 1 へ揃える

「slot の意味を全 kind で統一する」という点では最も筋が通る。

**却下** (今回は)。`:invariant` の `sample-count` と property の `actual-count` は
`cases` にサンプル数を載せる挙動を作っており、`cases 5` を固定した pin が実在する
(`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs:436`,
`crates/lsharp-wasm/tests/native_cli_output.rs:438`)。どちらも rust oracle は `1` を返すので
**これも parity 違反ではある**が、`I-67` の受入条件は `:example` に閉じており、
サンプル数を `cases` に載せる契約を畳むかどうかは独立の判断を要する。
`I-68` として台帳へ切り出し、本 slice では触らない。

## 実装

`selfhost/src/Tools/Test/TestRunner.ls` の `run-examples-loop` 系だけを変更した。
CLI 2 系統 (`Cli.ls` / `EmbeddedCli.ls`) と集計層は**無変更**。

| 箇所 | 変更前 | 変更後 |
|---|---|---|
| `run-examples-loop` の pass 分岐 | `(make-test-result name passed passed)` | `(make-test-result name passed 1)` |
| `make-example-failure-result` | `actual` 引数が `0` | `actual` 引数が `1` |

## Evidence

### 固定した test

`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs` の
`test_e2e_selfhost_embedded_cli_test_format_json_example_failure_message`
(`EXAMPLE-FAIL-REASON-01` で足した live test) に `cases` / `coverage.executed` の
assert を追加した。fixture ごとの期待値を tuple に持たせている。

| fixture | `:example` | 期待 `cases` / `executed` |
|---|---|---|
| `single` | 偽 1 件 | 1 / 1 |
| `second-of-two` | 真 1 件 + 偽 1 件 | 2 / 2 |

- RED (2026-08-23、実装前): `single` が `cases 0` で FAILED。189.88s。
  panic 出力の JSON は `"cases":0,...,"coverage":{"executed":0,"failed":1}` で、
  「実行 0 件なのに失敗 1 件」という自己矛盾がそのまま出ている
- GREEN (実装後): 2 passed / 0 failed。383.16s

同じ run に GC rooting の live pin
`test_selfhost_test_runner_example_case_roots_nested_ast_value` を巻き込んだ。
`run-examples-loop` の失敗分岐に span vector と message string の確保を足した直前の slice
(`EXAMPLE-FAIL-REASON-01`) がこの pin を通していなかったため、ここで一緒に通した。
RED / GREEN どちらでも ok。

### shipped binary での両 runner 実測

`cargo build` で embedded component を再生成したうえで、fixture と同じ dir から相対パスで実行
(2026-08-23、`./target/debug/lsharp test <file>`。rust lane は
`LSHARP_DISABLE_EMBEDDED_COMPONENT=1` + `--format json`)。

| fixture | runner | `cases` | `executed` / `failed` | exit |
|---|---|---|---|---|
| `:example [(= (abs 5) 5)]` | selfhost | 1 | 1 / 0 | 0 |
| 同上 | rust | 1 | 1 / 0 | 0 |
| `:example [(= (abs 5) 6)]` | selfhost | **1** | **1** / 1 | 1 |
| 同上 | rust | 1 | 1 / 1 | 2 |
| `:example [(= (abs 5) 5) (= (abs -3) 9)]` | selfhost | **2** | **2** / 1 | 1 |
| 同上 | rust | 2 | 2 / 1 | 2 |

太字が本 ADR で動いた値。変更前は失敗を含む 2 fixture がそれぞれ `0 / 0` と `1 / 1` だった。
**`cases` / `coverage.executed` / `coverage.failed` は 3 fixture すべてで 2 runner が一致する。**

### 波及確認 (実装前に取得)

- 既存の pin は `crates/lsharp-wasm/tests/` と `crates/lsharp-driver/tests/` に
  `implementation_conformance.cases` が 9 件、`coverage.executed` が 10 件 (本 slice の追加分を除く)。
  うち**失敗した `:example`** の値を固定しているものは 0 件
- 通っている `:example` の値を固定している pin は変更前後で同値 (`passed = 1 = 実行数 1`)
- `assert_non_bool_invariant_json` (`selfhost_cli_actual_main_args.rs:480`) の
  `cases 0` / `executed 0` は診断経路であり、`actual = 0` を入れる分岐は触っていないので不変
- index 2 (`actual`) を読む消費側は `assurance-result-actual-loop` の 2 箇所
  (`Cli.ls:906` / `EmbeddedCli.ls:855`) だけ。text lane は `vector-length` 系の別の数を使う

### 回帰

`cargo test -p lsharp-wasm --test e2e -- e2e::selfhost_cli_actual_main_args` (live のみ):
`20 passed; 0 failed; 25 ignored` / 1439.80s (2026-08-23)。
この module が `Cli.ls` / `EmbeddedCli.ls` の argv 経路をまとめて持つ。

`cargo clippy -p lsharp-wasm --tests`:
warning は残るが、いずれも `selfhost_parser_collection_scanners.rs` /
`native_cli_output.rs` / `selfhost_native_stage_chain.rs` / `support.rs` /
`wasi_tests/core.rs` の既存分 (`needless_borrow` 系)。
本 slice が触った `selfhost_cli_actual_main_args.rs` からの warning は 0 件。

## 満たしていないこと

- **exit code の 1 / 2 は揃っていない。** selfhost は 1、rust は 2 を返す。
  `EXAMPLE-FAIL-REASON-01` の時点から変わっておらず、本 ADR でも触っていない。
  意図的な使い分けなのか未確認で、**解決の引き取り先はまだ決まっていない**
  (`I-67` は実測として記録するだけ)。
- **`:invariant` / property の `cases` は依然サンプル数を載せる。** 案 D で述べた通り
  rust oracle は contract 数を返すので parity 違反が残る。`I-68` /
  `SAMPLE-COVERAGE-CONTRACT-01` へ切り出した。
- **`:case` の `actual` は式の値のまま。** `(expect (f) 3)` は `executed` へ 3 を寄与する。
  これも `I-68` の範囲。本 slice の受入条件は `:example` に閉じているため触っていない。
- `TestRunner.ls` は 5,283 行で、CLAUDE.md の 500〜800 行を大きく超えたまま。
  本 slice の増分は +5 行。分割は `RUNNER-SCANNER-01` の範囲。

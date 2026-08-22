# ADR: selfhost test lane へ `:assert` predicate の型検査を接続する

- Status: Accepted
- Date: 2026-08-22
- Scope: `selfhost/src/App/EmbeddedCli.ls`, `selfhost/src/App/Cli.ls`
- Related: [I-49](../../ISSUES.md#i-49), [I-45](../../ISSUES.md#i-45),
  [decisions-selfhost-zero-arity-defn-type.md](decisions-selfhost-zero-arity-defn-type.md)

## Context

selfhost の `test` lane は preflight で `check-canonical-cases-with-analysis` だけを呼び、
`:assert` の対応物である `check-canonical-assertions-with-analysis`
(`selfhost/src/Types/TypeInferAssertions.ls:2183`) を呼んでいなかった。
関数は存在し、`check` lane (`EmbeddedCli.ls:174` / `Cli.ls:774`) では実際に使われていたので、
**欠けていたのは実装ではなく接続**である。

結果として、未定義の変数を呼ぶ predicate が型診断 0 件のまま runtime 評価まで進み、
「述語が偽」として落ちていた。`:case` (`I-45`) が正しい contract を赤にする安全側の壊れ方
であるのに対し、こちらは**誤った contract を緑で通し得る**向きの穴である。

## Decision

`run-test-source-json` と `run-test-source-text` の両方で、property boundary の判定の直後・
case 検査の判定の前に assertion 検査を挟む。`check` lane の既存の優先順位
(base → assertion → case → property) をそのまま test lane へ写した。

- **診断の値は 4 要素をスカラーへ取り出してから使う。** `check-canonical-assertions-with-analysis`
  の戻り値は `[count, code, start, end]` の vector だが、この直後に走る
  `check-canonical-cases-with-analysis` が allocate するため、vector のまま持ち回ると
  `Cli.ls` 側の `root_push` / `root_pop` 収支に新しい slot を足す必要が出る。
  整数 4 個へ落とせば rooting を一切増やさずに済む
- **text lane は既存の `run-test-source-case-preflight` を再利用する。** 診断 vector は
  property boundary 分岐と同じ形 (`[count, code]`) を組んで渡す

## Alternatives considered

- **`case-preflight-diagnostics-summary` を assertion 用に複製する** — 却下。
  canonical assertion のコードは case と数値が一致しており
  (type error = 1001、non-bool = 1002、vacuous = 2005)、既存の写像がそのまま使える。
  複製すると同じ写像が 2 箇所になる。なお `canonical-assertion-empty-code` (2004) だけは
  `test-diagnostic-code-text` に対応する分岐が無く `LS0000` へ落ちるが、
  これは本 slice の前から `check` lane でも同じであり、ここで変えると診断コード体系の
  変更になるので触らない
- **`case-preflight-diagnostics-summary` を汎用名へ rename する** — 却下。
  `crates/lsharp-syntax/tests/selfhost_cli_validation_contract.rs:103` が名前を literal で
  固定しており、rename は本 slice の目的と無関係な契約変更になる
- **`Cli.ls` 側に `assertion-check` 用の root slot を足す** — 却下。
  `root_pop` は 4 箇所の分岐にそれぞれ 4 回並んでおり、slot を 1 つ足すと 4 箇所すべての
  収支を書き換えることになる。スカラー化すれば rooting は不変で済む

## Evidence

- RED: `test_e2e_selfhost_cli_test_source_json_typechecks_assert_predicate`
  (`crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs`) が
  `diagnostics.count` `left: Number(0)` / `right: 1` で失敗
  (2026-08-22、`cargo test -p lsharp-wasm --test e2e -- --ignored`、421.41s)
- control は RED の時点で既に緑:
  `test_e2e_selfhost_cli_test_source_json_keeps_running_well_typed_assert_predicate`。
  「壊れているのは型検査の接続だけで、健全な predicate の実行経路ではない」ことを
  同じ run で固定した
- GREEN: 同じ 2 test が 248.36s で `ok`。`EXIT=0`
- CLI 実測 (guest lane、fixture を cwd に置いて `lsharp test bad.ls`):
  変更前 `diagnostics.count=0 executed=1 failed=1`、
  変更後 `diagnostics.count=1 firstErrorCode=1001 firstErrorSpan=25..37 executed=0`、rc=1。
  Rust oracle (`--format json`) の `count=1 / firstErrorCode=1001` と同じ向き・同じコード
- 健全な fixture (`(defn incr [x] (+ x 1)) (defn caller [] :assert [(> (incr 1) 0)] 0)`) は
  変更前後とも `status=pass executed=1 diagnostics.count=0` / rc=0
- 最終確認: 契約を分けたあとに `assert` フィルタで再走し
  `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 3068 filtered out;
  finished in 494.16s` / `EXIT=0` (2026-08-22)。緑になった 6 本は
  `test_e2e_selfhost_cli_reports_canonical_assertions` /
  `..._test_source_json_reports_assertion_failure_coverage` /
  `..._test_source_json_keeps_running_well_typed_assert_predicate` /
  `test_e2e_selfhost_test_runner_projects_and_runs_ordered_assertion_forms` /
  `..._test_source_json_typechecks_assert_predicate` /
  `..._test_source_rejects_vacuous_assert_predicate`

### 接続によって露見した既存 test の契約衝突

`test_e2e_selfhost_cli_reports_canonical_assertions` (EC-M1-03) が
`failures:0` を期待して赤になった。実測は
`failures:2` / `diagnostics:2,LS2005` (vacuous assertion)。

**期待値を下げずに契約を分けた。** この test の意図は「canonical `:assert` の件数が結果へ
反映されること」で、vacuity とは無関係である。ところが fixture
`(defn positive [] :assert [(> 1 0) (= 1 1)] true)` は predicate が定数だけで構成されており、
**Rust oracle も同じ fixture を `LS2005` で落とす** (2026-08-22 実測、
`LSHARP_DISABLE_EMBEDDED_COMPONENT=1 lsharp test`)。つまり旧 `failures:0` は
Rust との乖離を固定していた期待値であって、守るべき契約ではなかった。

- 件数の test は fixture を非 vacuous な
  `(defn incr [x] (+ x 1)) (defn positive [] :assert [(> (incr 0) 0) (= (incr 0) 1)] true)` へ
  差し替えた。両 lane とも `assertions:2` / `failures:0`
- vacuity の契約は新しい test
  `test_e2e_selfhost_cli_test_source_rejects_vacuous_assert_predicate` が
  元の fixture ごと引き取り、`failures:2` / `diagnostics:2,LS2005` を固定する

### sweep で露見した範囲外の既存不具合

`assert` フィルタの緑を確認したあと、隣接する `--ignored` lane を確認するため
`lsp_stdio_completion` フィルタでも走らせた。結果は **1 passed / 9 failed**
(`finished in 839.52s`, 2026-08-22)。緑は `..._lsp_stdio_completion` のみ。

失敗 9 本は signature が 2 系統に分かれ、片方だけ直しても残りは緑にならない:

- **A (位置規約)** 8 本 — wire の `line`/`col` を 0-indexed とみなす `+1` 正規化
  (`9175c6e5`, 2026-08-03) に fixture が追随しておらず、prefix が `""` になって
  keyword 7 件が常に混ざる
- **B (snapshot 形式)** 6 本 — snapshot file が completion item を三要素配列で持つ
  2026-04-03 の縮約形のままで、object 形へ変わった実出力 (`5db1c2a4`, 2026-08-03) と合わない

うち 5 本は A と B の両方を持つ。`..._completion_schema_snapshot` は request が
`"params":0` で位置を送らないため **A では説明できず、B 単独**である。
「同じ family が同じ理由で落ちている」と丸めるとこの 1 本の原因が消えるので、分けて記録した。

**本 slice の diff は補完経路に一行も入っていない** — 触ったのは `Cli.ls` / `EmbeddedCli.ls` の
`run-test-source-json` / `run-test-source-text` だけで、原因 commit はいずれも本 slice の
19 日前 (2026-08-03) である。よって本 slice の回帰ではない。ただし HEAD での再現は
未実施 (working tree 復元が auto-mode classifier に阻まれたため)。

`ISSUES.md` `I-52` として起票し、`TODO.md` の `LSP-COL-CONV-01` (A) と
`LSP-SNAPSHOT-SHAPE-01` (B) が引き取った。**本 slice では直さない** — 位置規約は
0-indexed 側を正本にする決定が要り、snapshot 側は縮約器を入れるなら何を縮約するかが
契約になるため、どちらも別 ADR に値する。

### 触っていない既存の非対称

`run-test-source-json-preflight` は `assurance-report-json` へ件数を literal の `1` で渡すため、
JSON lane の `diagnostics.count` は診断が何件でも 1 になる (上の fixture では text lane が
`diagnostics:2` を返すのに JSON lane は `count:1`)。これは case / property preflight でも
同じで本 slice の前から存在する。assert だけ直すと preflight 間の形が割れるので触らない。

### 満たせなかった受入条件

- **診断 message は空のまま。** Rust oracle は
  `[LS1001] [error] caller: :assert predicate の型推論に失敗しました: ...` を返すが、
  selfhost の `run-test-source-json-preflight` は message に `""` を渡す設計で、
  これは case preflight でも同じである。`ASSERT-TYPECHECK-01` の受入条件は
  「`diagnostics.count >= 1`」と「Rust lane と同じ向きで落ちること」だったので条件自体は
  満たしているが、**message parity は満たしていない**。message を埋めるには preflight の
  診断本文生成を case / property と揃えて設計する必要があり、別 slice とする
- **`LS2004` (assert empty) の code text は `LS0000` のまま** — 上記 Alternatives のとおり

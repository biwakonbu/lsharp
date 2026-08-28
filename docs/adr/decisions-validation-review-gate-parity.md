# ADR: 独立 review gate の `outcome=pass` 条件を selfhost 3 経路へ揃える

- **Status**: doc-GREEN (focused 7 本まで / lane 未了 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED) / 2026-08-28 (RED-1) / 2026-08-28 (RED-2) / 2026-08-28 (GREEN)
- **Scope**: `selfhost/src/App/EmbeddedCli.ls` / `selfhost/src/Tools/Validation/ManifestInput.ls` の
  独立 review 計数と、それを固定する e2e 期待値 2 件
- **Related**: `ISSUES.md` の `I-96`、`TODO.md` の `VALIDATION-REVIEW-GATE-PARITY-01`。
  gate 意味論の正本は `decisions-v0.2-validation-independent-review-outcome.md` (2026-07-29)

## Context

独立 review の計数は selfhost 側に **3 つ**ある。`I-96` が実測したとおり、
`outcome=pass` の連言が入っているのは 1 つだけである。

| 定義 | gate | 由来 |
|---|---|---|
| `App/Cli.ls:238-250` | **あり** (method + independence + outcome) | `e37b9cd6` (2026-07-31) |
| `App/EmbeddedCli.ls:427-437` | **なし** (method + independence の 2 条件) | `793a5343` (2026-07-27、ADR より前) |
| `Tools/Validation/ManifestInput.ls:178-182` | **なし** (2 パターンの出現数の min) | `e37b9cd6` が同時に新設 |

`e37b9cd6` は `App/Cli.ls` に gate を入れたが、他 2 経路にも e2e 期待値にも手を入れなかった。
結果として **ADR 以前の挙動を緑の test が pin している**状態が 2026-07-31 から続いている。

**この parity 欠落は 2026-07-29 の ADR 自身が Boundary 節で follow-up に送っていた範囲**であり、
放置ではなく明示的な繰り延べである。

## Decision

### D1. gate の意味論は再裁定しない

`method=review` かつ `outcome=pass` かつ `independence=independent-review` の 3 条件連言、という
定義は 2026-07-29 に裁定済みである。**本 ADR はその契約を 3 経路へ揃えるだけで、内容には触れない。**

### D2. `EmbeddedCli.ls` に `outcome=pass` の連言を足す

`App/Cli.ls:238-250` と同一の形にする。L# の `and` は二項なので `(and a (and b c))` と書く
(`App/EmbeddedCli.ls:1446-1456` の既存 idiom)。
accessor は `source-evidence-record-outcome` (`Tools/Validation/Evidence.ls:76`) を使い、
`vector-get` で slot を再導出しない。

### D3. 期待値 2 件を `1` -> `0` へ是正する

- `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs:15794` (`#[ignore]` lane、現在赤)
- `crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs:1459` (既定 lane、**現在緑**)

**2 箇所を同時に動かす。** 後者は緑の test を一旦赤にする変更であり、D2 の実装で緑へ戻る。

これは `CLAUDE.md` が禁じる「実装に合わせて期待値を変える」ではなく、
`テストの設計ミスを除く` 側の例外にあたる。**根拠は 4 つあり、いずれも実装出力とは独立である。**

1. `decisions-v0.2-validation-independent-review-outcome.md` (2026-07-29, Accepted) が
   3 条件をすべて満たすものに限定すると定め、**`contradicted` を名指しで除外している**
2. `decisions-v0.3-native-validation-boundary-followups.md:17,33-34` が
   「the failed-review fixture counted a failed independent review as an independent review」を
   **欠陥として**記録している。`e37b9cd6` の gate 追加は意図的な是正である
3. Rust canonical の pin test `crates/lsharp-types/tests/intent_validation.rs:211`
   `failed_independent_review_does_not_satisfy_review_gate` が
   `EvidenceOutcome::Contradicted` で gate 不成立を固定している
4. `decisions-v0.2-native-validation-failed-independent-review.md` (2026-07-29) が
   native smoke 側に同じ契約 (`independent_reviews` は `0`) を敷いている

### D4. 対照は `selfhost_cli_actual_main_args.rs:1017` を動かさないことで取る

同 module の `..._validate_source_reports_pass` (`:975`) は **`:outcome "pass"` を除いて同形の
fixture** を使い、`:1017` で `independent_reviews == 1` を assert している。
`:1459` との差は `:outcome` (`pass` / `contradicted`) と edge (`:supports` / `:contradicts`) だけである。

**この 1 は修正後も 1 のままでなければならない。** ここが動いたら、D2 は gate を締めたのではなく
計数そのものを壊したことになる。**その場合は test ではなく実装が誤りである。**

### D5. `ManifestInput.ls` は record 窓走査へ寄せる (案 A を採用)

canonical manifest の wire 順序は `Evidence.ls:806-834` / `:656-699` で固定されている。

- top-level: `schema_version` -> `nodes` -> `evidence` -> (`reviews`) -> (`review_evidence_identity`) -> `edges`
- evidence record 内: `id...` -> `"method"` -> `"subject"` -> `"outcome"` -> `"execution"` ->
  `"provenance"` -> **`"independence"` (最終 field)**
- `"method"` と `"independence"` は **evidence record にしか現れない**
  (review record の field は namespace / key / provenance_digest / visibility / verification_state)

したがって次が成り立つ:

> `"method":"review"` の出現位置 `m` から、直後の `"independence":"` の出現位置 `e` までが
> **ちょうど 1 record の尾部**である。

**採用する計数**: `m` を順に走査し、各 `m` について

1. `e` = `m` 以降で最初の `"independence":"` の位置。無ければ **そこで打ち切る** (fail-closed)
2. `e` の位置が `"independence":"independent-review"` に一致するか
3. `"outcome":"pass"` が `[m, e)` の内側に現れるか

の 2 と 3 が**両方**成り立つものだけ数える。

**これは近似ではなく、上記 wire 順序の下では厳密である。**

### 却下した案

- **案 B: `"outcome":"pass"` の出現数を第 3 項に足して min を取る。**
  最も安い。fixture (contradicted 1 件のみ) では 0 になるので、D3 の test は緑になる。
  **却下する。** min は record 間の対応を一切見ないので、`method=test` の pass record と
  `method=review` の contradicted record が混ざった manifest では今と同じく誤る。
  **test が緑になるのに欠陥が残る形**であり、次に誰かが測ったときに
  「gate は揃っている」と誤読させる。安さと引き換えに診断可能性を失う。
- **案 C: `...-approx-count` へ改名し、近似であることを明示する。**
  却下する。呼び出し側 `App/Cli.ls:569` はこの値を report の `independent_reviews` として
  そのまま出す。**契約上の数値**を近似値で埋めるのは、直すか拒否するかのどちらよりも悪い。
  名前を変えても report を読む側には近似だと伝わらない。
- **案 D: JSON を typed manifest graph へ完全に parse する。**
  却下**しない**が、本 slice では採らない。`ManifestInput.ls` 冒頭のコメントが
  「selfhost の JSON decoder がまだ typed manifest graph へ接続されていない間も」と書いており、
  **案 D はこのファイルそのものを消す長期目標**である。案 A は案 D を妨げず、
  案 D が入った時点でまるごと削除できる。規模が 1 slice に収まらないので分ける。

### 案 A が厳密でなくなる条件 (**採用と同時に書いておく**)

案 A の厳密性は「canonical emitter が evidence record の最後に必ず `"independence"` を
出す」ことに依存している。したがって次の場合には厳密でなくなる:

- **`"independence"` を持たない手書き manifest。** 窓が次 record まで伸び、
  後続 record の `"independence":"independent-review"` を誤って拾いうる。
  `Tools/Validation/Evidence.ls:656-699` の emitter は `independence` を常に出すので、
  この形は selfhost が生成した manifest には現れない。**手書き入力にのみ残る過計数**である。
- **将来 `"independence"` より後ろにフィールドが増えたとき。** 窓が record の途中で
  閉じるので `"outcome":"pass"` を取りこぼしうる。**wire 順序の変更は本 ADR の前提を壊す**
  ので、`Evidence.ls` の emitter を触る際は本節を確認すること。

いずれも案 D (typed parse) で消える。**今の入力で厳密であることと、
どんな入力でも厳密であることは違う** ので、区別して記録しておく。

## 予測 (**実装前に書く。結果を見ていない**)

`TODO.md` の受入条件 (d) が要求する `contradicting_observations` の実測を含む。

| 対象 | 予測 | 根拠 |
|---|---|---|
| `selfhost_cli_actual_main_args.rs:1017` (`independent_reviews`) | **1 のまま** | fixture が `:outcome "pass"`。D2 の連言を通る |
| `selfhost_cli_actual_main_args.rs:1459` (`independent_reviews`) | **0** | fixture が `:outcome "contradicted"` |
| `selfhost_cli_core.rs:15794` (`independent_reviews`) | **0** | 2026-08-24 sweep の実測 `left: Number(0)` |
| `selfhost_cli_core.rs:15795` (`contradicting_observations`) | **1** | contradicted record 1 + `:contradicts` edge 1 が dedup されて 1 |
| `selfhost_cli_actual_main_args.rs:1460` (同) | **1 のまま** | 現在緑。EmbeddedCli 側の contradiction 計数は触らない |

**外れる可能性を書いておく。**

- 4 行目 (`contradicting_observations` = 1) は **`I-96` の時点で「未実測」と明記した唯一の数値**である。
  dedup が record 側と edge 側で別々に数えていれば **2** になる。
  `App/Cli.ls` の `validation-contradictory-records-loop` は id で dedup しているが、
  edge 由来の contradiction を同じ id 空間へ入れているかは確認していない。
  **2 が出たら、期待値ではなく dedup の契約を先に調べる。**
- 5 行目が動いたら、D2 の連言が `validation-contradictory-records-loop` に副作用を持ったことになる。
  別の loop なので起きないはずだが、起きたら実装が誤りである。
- `ManifestInput.ls` (D5) を通る e2e が何本あるかは**測っていない**。0 本なら D5 は
  「壊していないこと」しか示せない。その場合はそう書く。

### 予測の補強 (**結果が出る前に書く**)

上の 4 行目 (`contradicting_observations` = 1) を「未実測の導出」としたが、
**source を読んで根拠を上げられたので、結果を見る前に補強しておく。**

`App/Cli.ls:269-282` の `validation-evidence-metrics` は、record 由来 (`ids1`) と
edge 由来 (`ids2`) の contradiction を **同じ id 集合へ入れて** `(vector-length ids2)` を返す。
`validation-add-evidence-id` (`:234-237`) は既存 id を弾く。
fixture の record id と `:contradicts` edge の left はどちらも
`evidence:checkout/cancel-counterexample` なので **dedup されて 1** になる。

したがって「2 が出たら dedup の契約を先に調べる」は、より強い形で
「**2 が出たら `validation-add-evidence-id` の等値判定が効いていない**」に置き換わる。

### D5 の e2e 被覆 (**実装前に測った**)

`crates/lsharp-wasm/tests/e2e/` に **manifest JSON を `validate` へ食わせる test は 1 本も無い**
(`grep -rn '"--manifest"'` = 0 件、`"schema_version":1` を含む fixture はすべて
native stage0 / rollback archive 側のもの)。つまり D5 は **e2e 被覆 0 の経路**である。

これは「壊していないこと」しか示せない状態なので、**D5 の RED として pin test を 2 本足す**。

- `..._validate_manifest_review_gate_is_per_record` — `method=review`/`outcome=contradicted` の
  record と `method=test`/`outcome=pass` の**別 record** を持つ manifest。
  出現数の min を取る旧実装は 1 を返し、record 窓なら 0 になる。**判別する側**
- `..._validate_manifest_counts_passing_independent_review` — 3 条件を満たす record 1 件のみ。
  **新実装が「常に 0」ではないこと**を固定する対照。旧実装でも 1 なので判別はしない

**`selfhost_cli_core` の宣言数は 382 -> 384 になる。** `SWEEP-LANE-RERUN-01` の
完走判定はこの新しい分母で行う。

CI 側の `scripts/ci/native-selfhost-dev-source-file-smoke.sh:1552` は
`validate <manifest>` の出力が `validate --source <src>` の出力と **byte 一致**することを
要求している。当該 fixture は `independent_reviews` が 0 で、review evidence を持たないので
新旧どちらの実装でも 0 である。**この parity 制約は D5 で壊れない** (ただし本 slice では
CI script を実行しない。`I-96` の scope 外)。

### RED-2 の予測 (**RED-2 の結果を見る前に書く**)

manifest pin test 2 本を、**green patch を当てない状態で**先に回す。3 run 構成
(RED-1 / RED-2 / GREEN) を時間節約のために潰さない。潰すと D5 の判別証拠が消えるためである
-- pin test を green patch と同時に入れると「新実装で緑」しか示せず、
「旧実装では赤だった」を示せない。

| test | 予測 | 根拠 |
|---|---|---|
| `..._validate_manifest_review_gate_is_per_record` | **`independent_reviews` が `left: Number(1)` で FAILED** | 旧実装は `"method":"review"` の出現数と `"independence":"independent-review"` の出現数の min を取る。fixture はどちらも 1 出現なので 1 を返す。record をまたいでいることを見ていない |
| `..._validate_manifest_counts_passing_independent_review` | **`ok`** | 3 条件を満たす record 1 件だけなので旧実装でも新実装でも 1 |

**2 本目が緑であることが判別の一次元性を担保する。** 1 本目だけを見ると
「manifest 経路が丸ごと壊れている」と区別が付かない。

なお 1 本目は `status` / `contradicting_observations` も assert しているが、
そこは**新旧で同値**である (contradiction が支配するので `status="fail"` は動かない)。
割れるのは `independent_reviews` の 1 次元だけになるよう fixture を組んである。

## Evidence

### RED-1: `EmbeddedCli` 経路 (既定 lane、`selfhost_cli_actual_main_args`)

期待値を `1` -> `0` へ動かしただけの状態で 2 本を測った。

| 項目 | 値 |
|---|---|
| 実行 | `cargo test -p lsharp-wasm --test e2e -- --test-threads 1 --nocapture --exact <2 本>` |
| 起動 | `/Users/biwakonbu/github/tmp/i96/run_red.py` を `os.setsid()` で切り離し |
| ログ | `/Users/biwakonbu/github/tmp/i96/red.log` |

| 結果 | `test result: FAILED. 1 passed; 1 failed; 0 ignored; 3079 filtered out; finished in 396.20s` / `RUNEXIT=101` / `ELAPSED=396.69` |

| test | 実測 | 予測表の該当行 | 判定 |
|---|---|---|---|
| `..._validate_source_reports_fail` (`:1462`) | `left: Number(1)` / `right: 0` で FAILED | 「`..._main_args.rs:1459` -> **0**」 | **予測どおり赤**。実装が gate 前の `1` を返している |
| `..._validate_source_reports_pass` | `ok` | 「`..._main_args.rs:1017` (`independent_reviews`) -> **1 のまま**」 | **予測どおり緑**。D4 の対照が動いていない |

`3079 filtered out` + 2 = **3081**。`MODEXIT` に相当する `RUNEXIT=101` は libtest の通常 test 失敗
(SIGKILL の `-9` ではない) なので、測り直しではなく RED として読む。

**この 1 本で RED になったのは EmbeddedCli 経路だけである。** `selfhost_cli_core.rs:15794` は
`#[ignore]` 側なので本 run では走っていない (`filtered out` に入っている)。そちらの RED は
2026-08-24 sweep の実測 `left: Number(0)` を根拠に予測しており、**本 slice では測り直さない**
(lane に委ねる)。

### RED-2: manifest 経路 (`selfhost_cli_core`、新設 pin test 2 本)

green patch を**当てない状態で** pin test 2 本だけを回した。

| 項目 | 値 |
|---|---|
| 実行 | `cargo test -p lsharp-wasm --test e2e -- --test-threads 1 --nocapture --ignored --exact <2 本>` |
| 起動 | `/Users/biwakonbu/github/tmp/i96/run_red2.py` を `os.setsid()` で切り離し。pid 18788 |
| ログ | `/Users/biwakonbu/github/tmp/i96/red2.log` |
| 結果 | `test result: FAILED. 1 passed; 1 failed; 0 ignored; 3081 filtered out; finished in 209.69s` / `RUNEXIT=101` / `ELAPSED=219.01` |

| test | 実測 | 予測 | 判定 |
|---|---|---|---|
| `..._validate_manifest_review_gate_is_per_record` (`:15865`) | `left: Number(1)` / `right: 0` で FAILED | 「`left: Number(1)` で FAILED」 | **予測どおり赤** |
| `..._validate_manifest_counts_passing_independent_review` | `ok` | 「`ok`」 | **予測どおり緑** |

**旧 min 実装が record 境界を見ていないことが、実測で確定した。** fixture は
`method=review` / `outcome=contradicted` の record と `method=test` / `outcome=pass` の
別 record を持つ。両者を突き合わせられれば 0、出現数の min を取れば 1 -- 実測は 1 だった。

**対照 (2 本目) が緑であることも同時に確認できた。** 1 本目だけでは
「manifest 経路が丸ごと壊れている」と区別が付かないが、3 条件を満たす record 1 件だけの
manifest では旧実装も正しく 1 を返している。**割れているのは record 境界の 1 次元だけ**である。

`3081 filtered out` + 2 = **3083**。pin test 2 本を足す前の workspace 全体は 3081 だったので数が合う
(`selfhost_cli_core` module 単位では 382 -> **384**)。

### text format 側の全数調査 (**GREEN を回す前に測った**)

D2 は `and` に連言を足すだけなので計数は**減る方向にしか動かない**。したがって
`independent-reviews:` (hyphen) を期待する text format の test が非 pass fixture で `1` を
期待していないかを先に確認した。

```
grep -rn 'independent-reviews:' crates/lsharp-wasm/tests/e2e/ crates/lsharp-driver/src/
```

| 位置 | 所属 test | 期待 | fixture の review |
|---|---|---|---|
| `selfhost_cli_core.rs:15665` | `..._validate_source_text_reports_trace_gap` | 0 | -- (減っても 0) |
| `selfhost_cli_core.rs:15731` | `..._validate_source_text_reports_pass` | 1 | `:outcome "pass"` + `:independence "independent-review"` |
| `selfhost_cli_review_identity.rs:60` | `..._validate_text_projects_optional_identity_as_dash` | 0 | -- |
| `..._main_args.rs:697` | `..._main_with_args_validate_source_text_trace_gap` | 0 | -- |
| `..._main_args.rs:752` | `..._main_with_args_validate_source_text_pass` | 1 | `:outcome "pass"` + `:independence "independent-review"` |
| `..._main_args.rs:1353` | `..._main_validate_projects_explicit_review_evidence_identity` | 0 | -- |

**`1` を期待する 2 件はどちらも `outcome=pass` の fixture である。** D4 の対照が
json 側 (`..._main_args.rs:1017`) だけでなく text 側にも 2 件あることになり、
D2 が「gate を締めた」のか「計数を壊した」のかの判別材料が 3 件に増えた。
`0` を期待する 4 件は減少方向の変更では動かない。

### GREEN: D2 + D5 適用後の 7 本

RED-1 の 2 本、RED-2 の 2 本、対照 3 本を 1 プロセスで測った。

| 項目 | 値 |
|---|---|
| 実行 | `cargo test -p lsharp-wasm --test e2e -- --test-threads 1 --nocapture --include-ignored --exact <7 本>` |
| 起動 | `/Users/biwakonbu/github/tmp/i96/run_green.py` を `os.setsid()` で切り離し。pid 28050 |
| ログ | `/Users/biwakonbu/github/tmp/i96/green.log` |
| 結果 | `test result: ok. 7 passed; 0 failed; 0 ignored; 3076 filtered out; finished in 770.86s` / `RUNEXIT=0` / `ELAPSED=777.69` |

| test | 役割 | 予測 | 実測 |
|---|---|---|---|
| `..._embedded_cli_validate_source_reports_fail` | RED-1 の赤 | 緑になる | `ok` |
| `..._validate_manifest_review_gate_is_per_record` | RED-2 の赤 | 緑になる | `ok` |
| `..._validate_source_json_reports_contradicting_evidence` | D3 の `selfhost_cli_core` 側 | 緑になる | `ok` |
| `..._embedded_cli_validate_source_reports_pass` | **D4 の対照 (json)** | 1 のまま | `ok` |
| `..._validate_manifest_counts_passing_independent_review` | **D5 の対照** | 1 のまま | `ok` |
| `..._validate_source_text_reports_pass` | **対照 (text / `Cli.ls`)** | 1 のまま | `ok` |
| `..._main_with_args_validate_source_text_pass` | **対照 (text / `EmbeddedCli.ls`)** | 1 のまま | `ok` |

`3076 filtered out` + 7 = **3083**。RED-2 で確認した 3083 と一致する。

**D4 が要求した判別が成立した。** 対照 4 本が 1 本も動いていないので、D2 / D5 は
「gate を締めた」のであって「計数そのものを壊した」のではない。特に text 側の対照 2 本は
`Cli.ls` 経路と `EmbeddedCli.ls` 経路の両方を踏んでおり、**D2 が変えた側の経路でも
`outcome=pass` の record は依然として数えられている**ことを直接示している。

### 受入条件 (d): `contradicting_observations` の初実測

`selfhost_cli_core.rs` の `..._validate_source_json_reports_contradicting_evidence` は
2026-08-24 sweep では手前の `independent_reviews` で落ちていたため、
**`contradicting_observations` の assert に一度も到達していなかった**
(`decisions-v0.2-selfhost-evidence-parser-duplicate.md` の訂正節が明記していたとおり)。

本 GREEN で初めて到達し、**`assert_eq!(value["contradicting_observations"], 1)` が通った**。

予測の根拠は `App/Cli.ls:269-282` の `validation-evidence-metrics` が record 由来の id 集合
(`ids1`) と edge 由来の id 集合 (`ids2`) を**同一集合**に入れ、`validation-add-evidence-id`
(`:234-237`) が重複を弾くことだった。fixture の contradicted record 1 件と `:contradicts` edge
1 件は同じ id を指すので dedup されて 1 -- **予測どおりである。**
「2 が出たら `validation-add-evidence-id` の等値判定が効いていない」と結果より先に書いてあり、
そうはならなかった。

`..._main_args.rs:1460` の同 assert も緑のままである。

## 満たせなかったこと

- **`#[ignore]` lane を回していない。** 本 slice の証拠は focused 7 本であって、
  `selfhost_cli_core` / `selfhost_cli_actual_main_args` の完走ではない。
  台帳 `ignored-lane-expected-failures.txt:409` の削除も lane の後である。
  引き取り先は `TODO.md` の `SWEEP-LANE-RERUN-01`。
- **`selfhost_cli_core` の宣言数が 382 -> 384 へ増えた。** manifest pin test 2 本を
  足したためである。次の lane の完走判定はこの新しい分母で行う
  (`SWEEP-LANE-RERUN-01` の受入条件 (b) が正本)。
- **CI smoke script を回していない。** `scripts/ci/native-selfhost-dev-source-file-smoke.sh:1552`
  は `validate <manifest>` と `validate --source <src>` の出力が byte 単位で一致することを
  要求しており、D5 はこの経路に触れている。当該 fixture の `independent_reviews` は 0 で、
  D5 は計数を減らす方向にしか動かさないので破らないと読めるが、**読みであって実測ではない**。
  CI は本作業のスコープ外である。
- **MCP 経路の parity は見ていない。** 2026-07-29 ADR の Boundary が別に挙げている範囲で、
  本 slice の Scope 外。
- **案 A の厳密性は canonical emitter の wire 順序に依存している。** 手書き manifest で
  `"independence"` を省いた場合の過計数は残る (上の「案 A が厳密でなくなる条件」節)。
  案 D (typed parse) が入るまで消えない。

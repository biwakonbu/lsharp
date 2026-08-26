# ADR: e2e ハーネスが実行失敗を握り潰す形の是正

- **Status**: accepted
- **Date**: 2026-08-27
- **Scope**: `crates/lsharp-wasm/tests/e2e/` の test ハーネス。production コードは変更しない
- **Related**: `I-79` (本 ADR の起点) / `I-82` (本 ADR が範囲外とした形) / `I-83` (是正で表に出た実バグ) /
  `I-72` (発見経路) / `I-77` / `I-70` (同じ「緑だが検査していない」類型) /
  `TODO.md` の `PROBE-ASSERTS-NOTHING-01` / `COMPILER-MODE-STACK-01`

## 背景

`I-79` は「10-import helper の戻り値を握り潰す test 8 件が、緑のまま何も検査していない」と
起票された。この 8 件は `I-72` の呼び出し元調査の副産物であり、**網羅的な探索の結果ではない**。
`TODO.md` の `HARNESS-SWALLOWED-ERR-01` は受入条件として
「`Err` 腕を潰す前に、同じ形が他に何件あるかを数えること」を要求していた。

## 決定

### 1. 「握り潰し」を 4 つの形に分け、本 slice は形 (b) だけを直す

全数調査 (`crates/lsharp-wasm/tests/e2e/` 全 `*.rs`) の結果、`I-79` が 1 つの形として
書いていたものは実際には 4 つの異なる形だった。

| 形 | 定義 | 実測 | 本 slice |
|---|---|---|---|
| (b) | `if let Ok(..)` / `match` で **Ok 側に assertion があり**、`Err` では skip される | 5 箇所 / 3 test | **直す** |
| (c) | `Result` を束縛して `{:?}` で表示するだけ。assertion が最初から無い | 9 箇所 / 6 test | 範囲外 (`I-82`) |
| (a') | `match` の `Err` 腕が `eprintln!`、かつ **`Ok` 腕にも assertion が無い** | 6 箇所 / 6 test | 範囲外 (`I-82`) |
| (d) | `assert!` はあるが構造上**恒真** | 1 箇所 / 1 test | 範囲外 (`I-82`) |

**境界の根拠**: 形 (b) は「書かれた assertion が実行されない」ので、直し方に判断の余地が無い
(`Err` で落とせば、既に書かれている assertion が走る)。
形 (c)/(a')/(d) は **assertion がそもそも無い**ので、直すには「この probe は何を保証すべきか」を
新たに決めねばならない。合計 16 箇所 / 13 test で、うち 3 件は `#[ignore]` を持たない。
これは是正ではなく設計であり、`I-81`
(`VIOLATION-PROBE-STALE-01`) が別の probe について問うているのと同じ種類の裁定になる。
一つの slice に混ぜない。

### 2. `Err` は `panic!` で落とす。skip カウンタは導入しない

**却下した案 A: skip した回数を数えて上限を assert する。**
「n 件までは skip してよい」という閾値を置くと、閾値以下で恒常的に skip され続ける状態が
正常として固定される。`I-79` が問題にしたのはまさにその状態である。

**却下した案 B: `Err` 腕で `eprintln!` を残しつつ `panic!` も足す。**
`panic!` のメッセージに同じ内容を載せれば足りる。二重に出すと、失敗時のログで
どちらが本体か分からなくなる。

### 3. golden fixture の構文誤りは本 slice で直す

`tests/golden/types/type_errors.json` の case 3 は `(defn f [x : Int] ...)` と書かれていたが、
L# の型注釈構文は `(: x Int)` である (`examples/types.ls:11`)。
握り潰しを潰した結果この case はパースエラーで落ちた。

**却下した案: 台帳に expected failure として載せる。**
「構文が間違っている fixture」を expected failure に載せるのは、直せる欠陥を恒久化する。
fixture を正しい構文へ直すと test は緑になり、**E0004 の parity が初めて実際に検査された**。

### 4. 是正で表に出た赤は台帳へ載せる。直さない

`test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` は握り潰しを潰した結果、
selfhost compiler の codegen 不具合を表に出した (`I-83`)。
これは production 側の問題であり、本 slice (test ハーネスの是正) では直さない。

## Evidence

### RED: 壊れた入力を与えても緑のままであることの実証

`I-72` の fix 後、対象 3 件はいずれも**実際に走って緑**である。したがって
「走らせたら赤い」という形の RED は取れない。代わりに**入力を意図的に壊し、それでも
緑であること**を RED の証拠とした。

対象: `e2e::selfhost_native_differential::test_native_codegen_real_execution`
注入: `combined` の末尾に `(defn ((( broken` を追加

| 状態 | 結果 | 所要 | 出力 |
|---|---|---|---|
| 握り潰しあり + 正常入力 | `ok` | 19.68s (2 test 合計) | `✓ Native bytecode generation produced 8 bytes` |
| 握り潰しあり + **壊した入力** | **`ok`** | **0.06s** | `⚠ NativeCodegen execution result: "パースエラー: ..."` |
| `Err` を `panic!` へ + 壊した入力 | `FAILED` (EXIT 101) | 0.04s | `NATIVE-REAL-06: NativeCodegen.ls の実行に失敗した: "パースエラー: ..."` |
| `Err` を `panic!` へ + 正常入力 | `ok` | -- | `✓ Native bytecode generation produced 8 bytes` |

**所要時間の落差が最も雄弁である。** 19.68s → 0.06s は、この test が本来やるはずの仕事を
一切していないことを意味する。にもかかわらず結果は `ok` だった。
ログ: `/Users/biwakonbu/github/tmp/i79/red-*.log`

### GREEN: 3 test すべてで `Err` を落とす形へ是正した結果

`cargo test -p lsharp-wasm --test e2e -- --exact --nocapture --test-threads=1 --include-ignored`
(3 test 同時、88.93s)。

| test | 是正前 | 是正後 | 意味 |
|---|---|---|---|
| `selfhost_native_differential::test_native_codegen_real_execution` | ok | **ok** | 挙動不変。実行は元から成功していた |
| `selfhost_type_parser_parity::test_e2e_selfhost_type_error_parity` | ok | **FAILED** → fixture 修正後 ok | case 3 が一度も検証されていなかった |
| `selfhost_bootstrap_four_layer::test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` | ok | **FAILED** | `I-83`。台帳へ移送 |

是正前の 3 件はいずれも `ignored-lane-expected-failures.txt` にも
`workspace-expected-failures.txt` にも載っていなかった。
**載らないのは緑だったからであって、正しかったからではない。**

### 受入条件の判定

`TODO.md` の `HARNESS-SWALLOWED-ERR-01` に対して:

| 受入条件 | 判定 | 根拠 |
|---|---|---|
| 先に RED を立てる | 満たした | 入力注入による RED (上表)。走らせるだけでは RED が取れないことも記録した |
| `Err` 腕を潰す前に、同じ形が他に何件あるかを数える | 満たした | 全 `*.rs` を brace matching で走査。形 (b) 5 箇所 / 形 (c)(a')(d) 12 箇所 |
| 増えた赤は台帳へ正直に載せる | 満たした | `I-83` を `ignored-lane-expected-failures.txt` へ追加 |
| 「赤が増えないこと」を受入条件にしない | 満たした | 赤が 1 件増えた状態で slice を閉じている |

### 満たせなかったこと / 残渣

- **`I-79` の本文が事実として誤っていた。** 挙げられた 8 件のうち形 (b) は 1 件だけで、
  残り 7 件は「assertion が skip される」のではなく「assertion が最初から無い」形だった。
  また「8 件は全て `selfhost_bootstrap_four_layer` にある」も誤りで、形 (b) は
  `selfhost_native_differential` と `selfhost_type_parser_parity` にもあった。
  `ISSUES.md` の `I-79` 本文を実測で書き直した。
- **形 (c)/(a')/(d) の 16 箇所 / 13 test は手つかずである** (`I-82`)。うち 3 件
  (`test_i64_if_condition_validity` / `test_parse_compiler_ls` / `test_parse_caws_standalone`)
  は `#[ignore]` が無く、**通常 lane で毎回走りながら assertion を 1 つも持たない**。
- **件数を 2 度直した。** 最初は目視で 9 件と数えたが、走査を
  `scripts/sweep_unchecked_result.py` として書き直したとき `part_015` の 4 件が追加で出た。
  **手で数えた数は走査の網羅性ではなく目視の到達範囲を表す。**
  本 ADR が「定義を確かめずに数えるな」と書いた失敗を、同じ slice の中で自分でやっている。
- **走査には既知の偽陽性が 2 件残っている** (`part_007.rs:264` / `part_014.rs:651`)。
  タプル match と `?` 伝播はスクリプト側で除外済み。
  **走査結果をそのまま件数として使ってはならない。**
- `selfhost_bootstrap_four_layer` 以外の module については部分再測定を行っていない。
  形 (b) の是正 2 件はいずれも挙動不変または fixture 修正で緑化したため、
  台帳に新規行が要らないことを個別実行で確認したに留まる。

### 方法論として残すこと

**「同じ形が何件あるか」を数える前に、その形の定義を実物で確かめること。**
`I-79` は 1 つの形を仮定して 8 件を数えたが、実物は 4 つの形の混合だった。
定義を確かめずに数えると、**数は合っているのに中身が違う**という誤りが台帳に残る。
これは件数を過大でも過小でもなく、**分類として**間違える形の誤りであり、
数の検算では発見できない。

# 主題を検査していない probe test の裁定

- **Status**: doc-RED (裁定は確定、実装は未着手)
- **Date**: 2026-08-27
- **Scope**: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/` の 13 test と
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の 1 test
- **Related**: `I-82` (本 ADR の起点) / `I-79` と
  [`decisions-harness-swallowed-error-arms.md`](decisions-harness-swallowed-error-arms.md) (親。形 (b) だけが解決済み) /
  `I-83` (`test_i64_if.wasm` と同型の症状) / `I-81` (同種の probe 裁定だが対象は別 test)
- **引き取り先**: `TODO.md` の `PROBE-ASSERTS-NOTHING-01`

## 決めること

`I-79` の全数調査で、実行結果を `eprintln!` / `println!` するだけで終わる probe test が見つかった。
これらは入力が何であれ常に緑になる。**何を assertion にすべきかは test ごとに違う**ので、
一括置換では閉じられない。本 ADR は test ごとの裁定を確定する。

## 基準を先に置く

`I-82` はこの群を当初「**assertion を 1 つも持たない** probe test」と記述したが、これは誤りである。
13 件のうち 6 件は `.expect(...)` や `assert_valid_wasm(...)` で**中間結果を検査している**。
検査していないのは主題の方である。正しい基準はこうなる。

> 次の 2 つを両方満たす test を対象とする。
>
> 1. test 名またはコメントが**主題**を宣言している (何を確かめる test なのか)
> 2. その主題について、**結果を検査する assertion が無い** — 表示するだけ、または恒真な `assert!`
>
> 中間結果に assertion があっても、**主題が未検査なら対象**とする。
> 逆に主題を検査していれば、診断出力が併存していても対象外とする。

**基準を先に書き、membership をそこから導く。** `I-82` の件数はこれまでに 3 度動いており
(手作業 9 → 走査 13 → 基準の精緻化)、そのうち 2 回は「数え漏れ」だが、
3 回目は**数える対象の定義が間違っていた**ことによる。件数の検算では発見できない種類の誤りである。

### 基準の外にある隣接ケース

`part_015.rs` の `test_debug_stage2_save` は主題が「stage2 を保存すること」で、
`.expect("write failed")` がそれを検査している。**基準には当たらない。**
それでも本 ADR は削除対象に含める。理由は「検査していないから」ではなく、
「**そもそも test ではなく debug script だから**」である (裁定 1 を見よ)。
基準に当たらないものを同じ slice で処理する以上、その理由は別に書く。

`part_007.rs:264` にも #9 と同型の shape がある — `validate_wasm_detailed(stage2_self_compiler)` の
戻りを `match` で `eprintln!` へ捨て、その直後に弱い `assert_valid_wasm(...)` だけを assert する。
**強い検査を捨てて弱い検査を残す**という点で #9 と同じ間違いだが、**この test の主題は
4 層 bootstrap そのもの**で、主題を検査する assertion は別に持っている。**基準には当たらない。**

したがって **`I-82` の件数は動かさない。** 新規 issue も切らない。裁定 5 で #9 が
「stage2 wasm の妥当性」を主題として検査するようになれば、`part_007.rs:264` の側は
無害な診断出力へ格下げされ、独立に直す動機が消える。**「同じ形が何件あるか」を数える前に、
その形の定義を実物で確かめること** — 形が同じでも基準が同じとは限らない。

## 裁定

| # | test | 位置 | ignore | 裁定 |
|---|---|---|---|---|
| 1 | `..._stage2_reports_main_again_cache_pairs_progress` | `part_008.rs:344` | yes | assertion 追加 |
| 2 | `..._stage2_reports_main_again_progress` | `part_008.rs:455` | yes | assertion 追加 |
| 3 | `..._stage2_reports_module_resolver_progress` | `part_011.rs:390` | yes | assertion 追加 |
| 4 | `..._stage2_reports_string_length_if_progress` | `part_011.rs:434` | yes | assertion 追加 |
| 5 | `test_i64_if_condition_validity` | `part_015.rs:587` | **no** | assertion 追加 (**極性確定済み**) |
| 6 | `test_parse_compiler_ls` | `part_015.rs:620` | **no** | assertion 追加 (期待値は実測) |
| 7 | `test_parse_caws_standalone` | `part_015.rs:633` | **no** | assertion 追加 (期待値は実測) |
| 8 | `test_debug_stage2_output_minimal` | `part_015.rs:674` | yes | **削除** |
| 9 | `test_validate_stage2_wasm` | `part_015.rs:707` | yes | **assertion 追加** (裁定 1 を訂正。裁定 5 を見よ) |
| 10 | `test_debug_stage3_output_chars` | `part_016.rs:296` | yes | **削除** |
| 11 | `test_debug_stage3_main_again_output_chars` | `part_016.rs:382` | yes | **削除** |
| 12 | `..._stage2_classifies_chunked_lexer_failure_band` | `part_014.rs:596` | yes | 恒真 assert を実質化 |
| 13 | `..._representative_const_only_entrypoint_helper_offsets` | `stage_chain.rs:54963` | yes | assertion 追加 (**別 lane module**) |
| - | `test_debug_stage2_save` | `part_015.rs` | yes | **削除** (基準外・別理由) |

### 裁定 1: 削除 (#8 / #10 / #11 + `test_debug_stage2_save`)

いずれも名前が `test_debug_*` / `test_validate_*` で、**契約ではなく調査**を表明している。
#8 と `test_debug_stage2_save` は `stage2_debug.wasm` / `stage2_debug2.wasm` / `stage3_minimal.wasm` を
**カレントディレクトリへ書き出す**。test harness は成果物置き場ではない。

**診断出力の引き取り先** (`PROBE-ASSERTS-NOTHING-01` の受入条件):

| 削除する probe | 引き取り先 |
|---|---|
| #8 / #10 / #11 / `test_debug_stage2_save` | **`.wasm` の書き出しと生出力の目視。** 同じ chain を組む非 debug test が同 module に残るので、`cargo test -p lsharp-wasm --test e2e -- --ignored <name> --nocapture` で同じ生出力が読める。**この同値性は実装時に実測で確かめる** — 確かめずに削除しない |

### 裁定 2: assertion 追加 — 極性は実物から導く (#5)

`test_i64_if_condition_validity` は `tests/fixtures/selfhost-debug/test_i64_if.wasm` (32 bytes) を
wasmparser と wasmtime に食わせ、結果を `eprintln!` するだけである。
名前から「valid であることを assert する」と読みたくなるが、**実物は逆だった**。

```wat
(module
  (type (;0;) (func (result i64)))
  (func (;0;) (type 0) (result i64)
    i64.const 1        ;; ← if の条件に i64。仕様上 if 条件は i32
    if (result i64)    ;; ← else 節が無い。false 経路が i64 を残さない
      i64.const 2
    end))
```

`wasm2wat` は 2 つのエラーを出す。

```
000001c: error: type mismatch in if, expected [i32] but got [i64]
000001f: error: type mismatch in `if false` branch, expected [i64] but got []
```

したがって正しい契約は「**wasmparser と wasmtime の両方がこれを reject する**」である。
`is_ok()` を assert すると赤になり、赤を消そうとして fixture を書き換える、という悪い連鎖に入る。

**`I-83` との関係**: 2 つ目のエラー (`expected [i64] but got []`) は、`I-83` が記録した
`expected i64 but nothing on stack` と同じ形である。この fixture は
「`if (result i64)` に else が無い」という `I-83` の症状の最小再現形になっている可能性がある。
**参照に留める。統合しない。** 本 ADR が既に書いたとおり
**offset の集合は原因の集合ではなく**、症状の形が同じことは根が同じことを意味しない。

### 裁定 3: assertion 追加 — 期待値は実測で確定 (#1〜#4 / #6 / #7 / #13)

構造だけを決め、期待値は実装時の実測で埋める。

| test | 主題 | 足す assertion の構造 |
|---|---|---|
| #1〜#4 `..._reports_*_progress` | stage2 が `debug progress` モードで何を報告するか | 実行が成功すること + `progress_output` が期待 marker を含むこと。現状は `Result` を `{:?}` 表示するだけで、**実行失敗も握り潰している** |
| #6 `test_parse_compiler_ls` | `Compiler.ls` がパースできるか | パース結果を検査する。コメントは「構文エラーを検出する」と書きながら `eprintln!` で逃げている |
| #7 `test_parse_caws_standalone` | `test_caws.ls` がパースできるか | 同上 + decl 数 |
| #13 `..._const_only_entrypoint_helper_offsets` | selfhost 版と generic 版の entrypoint offset helper が一致するか | `selfhost_last == generic_last` / `selfhost_abs == generic_abs` の parity。**4 つを並べて print する構造自体が「一致するはず」という主張**であり、それが検査すべきことである |

**#6 / #7 の期待値を `is_ok()` と決め打ちしない。** これらが `eprintln!` で逃げているのは、
書かれた当時にパースが失敗していたからかもしれない (fixture は `selfhost-debug/` 配下にある)。
実測が失敗を示したら、`I-79` → `I-83` と同じ扱いにする — **新規 issue を切り、台帳へ載せ、
fixture や実装を赤を消す方向に触らない。**

**赤が出たときの引き取り先は test ごとに違う。**

| test | 引き取り先の台帳 |
|---|---|
| #5 / #6 / #7 (非 ignore) | `docs/development/validation/workspace-expected-failures.txt` (workspace baseline) |
| #1〜#4 / #12 / #13 (ignore) | `docs/development/validation/ignored-lane-expected-failures.txt` |

### 裁定 4: 恒真 assert の実質化 (#12)

現状はこう書かれている。

```rust
assert!(matches!(
    classification,
    "local-before-boundary" | "first-boundary-crossing" | "post-first-chunk"
        | "real-world-only" | "no-probe-failure"
));
```

`classification` は直前の if/else 連鎖がこの 5 文字列しか返さない構造で作られている。
**assertion があることと、検査していることは別である。**

裁定は「**現在の band を固定する**」。band が動いたら赤になるのが正しい。
動いてよいなら、そもそも test である必要がない — 分類が変わったことに気付くのがこの probe の目的だからである。
どの band かは実測で確定する。

### 裁定 5: assertion 追加 — 引き取り先が実在しなかったので削除を撤回する (#9)

**本 ADR は当初 #9 を削除と裁定したが、その根拠が偽だった。** 「主題は同 module の
`assert_valid_wasm(stage2_self_compiler)` が既に持つ」と書いたが、実物は逆で、
**捨てられている方が強い**。

| helper | 位置 | 実際に見るもの |
|---|---|---|
| `assert_valid_wasm` | `e2e/support.rs:693` | `wasm.len() > 8` と先頭 4 byte の `\0asm` **だけ** |
| `validate_wasm_detailed` | `four_layer/part_000.rs:139` | `wasmparser::Validator` を payload に流す。ただし `ValidPayload::Func` を捨てるので**関数本体は 1 つも検証しない** |
| `validate_wasm_function_bodies` | `e2e/support.rs:702` 付近 | 各関数本体を個別に検証する。**唯一の本物** |

`support.rs:698` の doc コメントはこの罠を逐語で記録しており、`part_017.rs` の I-71
回帰 test 群のヘッダコメントも同じ警告を繰り返している。**リポジトリは既にこれを知っていた。**
本 ADR の裁定 1 は、その記録を読まずに helper 名の見た目で同値と決めた。

**基準は変えない。基準に正しい事実を入れ直すと #9 は自然に assertion 追加へ落ちる** —
主題「stage2 wasm が valid か」を検査する assertion は、削除しても引き取る先が無いからである
(`validate_wasm_function_bodies` を実 selfhost stage2 に流す test は 1 つも無い。
`part_017.rs:86` の 2 箇所はどちらも temp dir の合成 Main.ls が主題である)。裁定の反転ではなく再導出である。

**設計判断 3 つ。**

1. **rename しない。** `test_validate_stage2_wasm` という名前は主題を既に正しく言っている。
   body が名前に追いつくだけである。これで下記の gate 行と script 行が無傷で済み、
   `AGENTS.md` の rename 再計測規約 (`d29cb5a1`) も発火しない
2. **`validate_wasm_function_bodies` を使う。`validate_wasm_detailed` ではない。**
   「detailed だと赤くなるかもしれないから弱い方を assert する」は、このリポジトリが
   数週間かけて文書化した anti-pattern そのものである (「緑になることと検査していることは別である」)
3. **実測が先。** 実装時に #9 を targeted で 1 回走らせ、両 validator の実際の戻りを見てから pin する。
   FAIL だった場合は **check を緩めない** — `ISSUES.md` に新規 issue (次番 `I-85`) を切り、
   ignored lane 台帳へ行を足す

**新規カバレッジという主張は過大なので、正直な効能を書いておく。** BOOT-04 の chain test 群は
stage2 を wasmtime で load して走らせており、wasmtime は load 時に関数本体まで検証する。
つまり chain が動いている限り実 stage2 は妥当なはずで、**実測は `Ok` の公算が高く、恒久赤の risk は低い**。
それでも #9 を残す理由は 3 つある。(a) 主題が validation そのものである唯一の test になる、
(b) chain test が別理由で赤いときにも生き残る、(c) load 失敗ではなく wasmparser の offset 付きで
局在化する。**`I-83` は「load 失敗に埋もれた validation error を偶然拾った」実例**なので、
(c) の価値には生きた証拠がある。

**内訳の移動。** 13 件は 13 件のまま。削除 4 → **3** (`test_debug_stage2_save` は基準外のまま別枠) /
assertion 追加 8 → **9**。件数の定義はこれまでに 3 度動いており、これ以上動かさないと決めた。
**動いたのは内訳であって母数ではない。**

## 却下した案

### 案 A: 13 件を一括削除

却下。#5 は極性を実物から確かめれば「toolchain が i64 条件の `if` を reject する」という
**本物の契約**を持つ。#13 も selfhost/generic parity という本物の契約を持つ。
一括削除はこれらを捨てる。**「検査していない」は「検査すべきものが無い」を意味しない。**

### 案 B: 13 件に一括で `.unwrap()` / `panic!` を入れる

却下。主題を検査しないまま「実行が成功する」だけを固定することになる。
#6 で `is_ok()` だけを assert すると、`I-79` と同型の
「test 名が主張することと、実際に検査していることのズレ」がそのまま残る。
`TODO.md` の `PROBE-ASSERTS-NOTHING-01` 受入条件が明示的に禁じてもいる。

### 案 C: 全部に `#[ignore]` を付けて通常 lane から外す

却下。非 ignore の 3 件を隠すだけで、検査していない事実は変わらない。
しかも lane の母集団を変えるので、**再計測コストは是正する場合と同じだけかかる**。
コストが同じなら、検査が増える方を選ぶ。

### 案 D: 極性・期待値を test 名から推定して assertion を書く

却下。**この slice の中で実際にやりかけた。** #5 について
「wasmparser / wasmtime 両方で valid を assert」と書きかけ、fixture を読んで初めて
実物が仕様上不正な wasm であり正しい契約が逆向きだと分かった。

> 実物を確かめずに期待値を固定するな。名前は主題を示すが、極性までは示さない。

これは本 ADR の親 (`decisions-harness-swallowed-error-arms.md`) が書いた
「定義を確かめずに数えるな」の一般形である。**同じ形の失敗を、続きの slice でまたやりかけた。**

## 実装順序の制約

**#1〜#12 と `test_debug_stage2_save` は全て `selfhost_bootstrap_four_layer` に属する。**
test の追加・削除は ignored lane の母集団を変えるので、`AGENTS.md` の partial-lane 規約
(commit `d29cb5a1`) により**実装後に同 module の再計測が 1 本要る** (前回実測 6748s ≈ 112 分)。

したがって:

- **four_layer に触る裁定は 1 つの slice に束ね、lane 1 本で覆う。**
  計測 → 変更 → 再計測を細切れに繰り返さない
- **#13 は `selfhost_native_stage_chain` に属する**ので、four_layer の slice には入れない。
  再計測の対象 module が違う

### #9 は module 外から名指しされている — 削除が高くつく理由であり、変換裁定の根拠でもある

**この節の前提は裁定 5 で反転した。** 当初は削除対象 5 件のうち
**`test_validate_stage2_wasm` (#9) だけが module 外から名指しされている**という事実から
「3 箇所を同一 slice で消す」という実行手順を導いていた。裁定 5 で #9 が変換になった以上、
**この実行手順は失効する** — 変換なら test 本体・`heavy_tests` の行・script の行の 3 箇所とも
byte 単位でそのまま残り、`selfhost_lsp_docs_ops` の単体確認も要らない。
**実測した結合の事実そのものは正しく、消さない。** 削除がなぜ高くついたかの証拠として、
変換裁定の根拠側へ読み替える。

実測で確かめた結合先は 2 つ:

| 参照元 | 位置 | 削除した場合 |
|---|---|---|
| `test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` の厳密名リスト `heavy_tests` | `selfhost_lsp_docs_ops.rs:3784-3787` | ループ末尾の `assert!(found, "{rel_path} (fragment を含む) に {test_name} が見つからない")` が発火し **`selfhost_lsp_docs_ops` が赤くなる** |
| phase11 の CI script | `scripts/ci/compile-phase11-inputs.sh:236` | nextest の `--exact` filter が 0 件一致になる |

**この赤は four_layer の再計測では検出できなかったはずである。** 落ちるのは別 module であり、上の
「実装順序の制約」が想定している lane の外側にある。**変換裁定ではこの赤は発生しない**が、
「lane の外に名指し参照がある」という構造は残るので、将来 #9 を rename / 削除するときは
同じ 3 箇所を同時に動かす必要がある。

ops03c 側の `phase11_script.contains(...)` 連鎖には `test_validate_stage2_wasm` は
**含まれていない** (同ファイル内の出現は `:3785` の 1 箇所のみ)。将来 script 行を消しても
ops03c の script 検査は壊れない、という予備知識として残す。

実際に削除する 4 件 (`test_debug_stage2_output_minimal` / `test_debug_stage3_output_chars` /
`test_debug_stage3_main_again_output_chars` / `test_debug_stage2_save`) は
`scripts/` にも `docs/` にも `selfhost_lsp_docs_ops.rs` にも参照が無い。
four_layer の prefix ルール 4 本 (`test_e2e_boot04_` / `test_e2e_bootstrap_` /
`test_v2_11_` / `test_v2_12_self_hosted_`) はいずれも `test_debug_` / `test_validate_` に
一致しないので、削除で **dead prefix (`TESTGATE-01`) が生じることも無い。**

> 削除した test が module 内で何も参照されていないことは、削除して安全であることを意味しない。
> gate は「どの test が存在するべきか」を別ファイルから名指しで持っている。

## Evidence

実装後に埋める。現時点で確定している実測は 1 つだけ。

| 対象 | 実測 | 取得条件 |
|---|---|---|
| `tests/fixtures/selfhost-debug/test_i64_if.wasm` の妥当性 | **不正**。`wasm2wat` が 2 件のエラーを出す (`expected [i32] but got [i64]` / `` `if false` branch, expected [i64] but got [] ``) | `wasm2wat tests/fixtures/selfhost-debug/test_i64_if.wasm`、2026-08-27、32 bytes |

## 満たせなかったこと

- **裁定だけで、実装は 1 件も入っていない。** 本 ADR は doc-RED である。
  緑にした test は 0 件で、`I-82` は open のままである
- **#6 / #7 / #12 / #13 の期待値は決まっていない。** 「構造を決めて実測で埋める」と書いたが、
  実測が「現状は失敗する」を示した場合に何件の新規 issue が出るかは分かっていない
- **`I-82` の枠組みを書き直した。** 「assertion を 1 つも持たない」は 13 件中 6 件で成り立たない。
  commit 済みの記述だが直した。**正確さが churn に勝つ**という判断であり、
  件数が 4 度目に動いても「基準が精緻化された」履歴として残るよう、基準文を先に置く構成にした

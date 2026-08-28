# 主題を検査していない probe test の裁定

- **Status**: doc-GREEN (完了 / 2026-08-28)
- **Date**: 2026-08-27 (doc-RED) / 2026-08-27 (実装) / 2026-08-28 (lane 完走確認)
- **Scope**: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/` の 13 test と
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の 1 test
- **Related**: `I-82` (本 ADR の起点) / `I-79` と
  [`decisions-harness-swallowed-error-arms.md`](decisions-harness-swallowed-error-arms.md) (親。形 (b) だけが解決済み) /
  `I-83` (`test_i64_if.wasm` と同型の症状) / `I-81` (同種の probe 裁定だが対象は別 test)
- **引き取り先**: `TODO.md` の `PROBE-ASSERTS-NOTHING-01`。2026-08-28 に完了・削除済み。
  覆えていない x86 側の parity は `I-92` / `NATIVE-X86-ENTRYPOINT-PARITY-01` が持つ

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

同じ確認の中で **3 番目の帯**が見つかった。`test_debug_boot04_*` **12 件**
(`part_009.rs` 4 / `part_010.rs` 7 / `part_011.rs` 1) は主題の assertion を持つが、
その中身が `assert!(!output.trim().is_empty())` 1 行だけである。probe の値そのものは
`eprintln!` へ流れる。**基準には当たらない** — assertion は有り、literal に恒真でもない
(出力が空なら落ちる)。**`I-82` の 13 件には含めず、件数も動かさない。**
別 issue `I-85` として起票し、引き取り先を `TODO.md` の `WEAK-SUBJECT-ASSERT-01` に置いた。

> 基準を後から広げて件数を合わせるのは、数え直しではなく基準の書き換えである。
> 基準の外にある問題は、基準の外にあるまま別の台帳へ載せる。

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
| #8 / #10 / #11 / `test_debug_stage2_save` | **生出力は同 module の非 debug test で読める (静的に確認済み。下表)。`.wasm` の書き出しは引き取り先が無く、意図して失う。** |

**引き取り先の静的確認 (2026-08-27、cargo 不使用)。**

| 削除する test | 主題 | 同 module に残る非 debug の同型 test |
|---|---|---|
| #8 `test_debug_stage2_output_minimal` | stage2 → stage3 (minimal.ls) | `part_007.rs` `test_e2e_boot04_stage2_compiler_to_stage3_minimal` |
| #10 `test_debug_stage3_output_chars` | 同上の出力文字列 | 同上 |
| #11 `test_debug_stage3_main_again_output_chars` | stage2 → stage3 (`src/App/Main.ls`) | `part_013.rs` `test_e2e_boot04_self_hosted_stage2_compiles_main_again` |
| `test_debug_stage2_save` | stage2 の生成 | stage2 chain は 12 fragment・数十 test が組む |

#8 / #9 / `test_debug_stage2_save` は **同一の stage2** を作る
(`compile_file_only(selfhost_main_path())` → `["compiler", "src/App/Main.ls"]` を selfhost root で実行)。
#9 が見ているのは**実 selfhost の stage2** であり、`part_017.rs:86` の 2 箇所が見ている
temp dir の合成 `Main.ls` とは主題が違う — 裁定 5 の根拠はここで裏が取れている。

**失うものを正直に書く。** `stage2_debug.wasm` / `stage2_debug2.wasm` / `stage3_minimal.wasm` の
**ファイル書き出しには引き取り先が無い。** 他のどの test もこれらを書かない。
再取得したければ一時的に 1 行足すか、単発 script を書くことになる。
**これは受け入れる損失であって、同値な代替があるという主張ではない。**
test harness を成果物置き場にしないことの対価である。

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

> **訂正 (2026-08-27、裁定 6 の実測による)。** 上の 1 文は誤りだった。実測では
> `validate_wasm_detailed` は **`Ok(())` を返す**。wasmparser が見逃したのではなく、
> この helper が `ValidPayload::Func` を捨てて関数本体を 1 つも検証しないためである
> (裁定 5 が指摘した弱さと同じもの)。**正しい契約は「両方が reject する」ではなく
> 3 者の強度差そのもの**であり、裁定 6 でそう固定した。
> `is_ok()` を assert すると赤になる、という予測も誤りで、実際には緑になる。
> **「実物から導く」と書いた裁定が、実物を見る前に極性を書いてしまっていた。**

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
   FAIL だった場合は **check を緩めない** — `ISSUES.md` に新規 issue を切り (採番は実装時。
   `I-85` は本 slice で別件に使った)、ignored lane 台帳へ行を足す

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

### 裁定 6: 非 ignore 3 件 (#5 / #6 / #7) の実質化 — 完了 (2026-08-27)

`#[ignore]` を持たない 3 件は lane を要さないので先に閉じた。**実測を先に取り、
その結果が裁定 2 の予測を 1 点で否定したので、裁定 2 を訂正したうえで契約を書き直した。**

#### 実測 (`cargo test -p lsharp-wasm --test e2e -- --nocapture --exact`)

| 対象 | 実測 |
|---|---|
| `validate_wasm_detailed(TEST_I64_IF_WASM)` | `Ok(())` |
| `validate_wasm_function_bodies(TEST_I64_IF_WASM)` | `Err("関数本体の検証に失敗: 1 件\nfunc[0] body=[23..32] err@26 (0x1a): type mismatch: expected i32, found i64")` |
| `wasmtime::Module::new(.., TEST_I64_IF_WASM)` | `Err("WebAssembly translation error … Invalid input WebAssembly code at offset 26: type mismatch: expected i32, found i64")` |
| `lsharp_syntax::parse(Compiler.ls)` | `Ok`、`decls = 312` |
| `lsharp_syntax::parse(test_caws.ls)` | `Err(Parse(Multiple([...])))` -- 末尾 `2945..2951` に 6 件 |

#### #5 は 裁定 5 の実行可能な証拠になった

裁定 5 は「3 つの wasm 検証 helper の強度が違う」をソース読解で導いた。**#5 の fixture は
その差が現れる最小の実例そのものだった** — 32 bytes の同じバイト列に対して、弱い helper は
`Ok`、強い helper と wasmtime は同じ型不一致 (`expected i32, found i64` @ offset 26) を返す。

そこで #5 の契約を「両方 reject」ではなく **3 者の強度差の pin** にした。
1 行目の `validate_wasm_detailed(...).is_ok()` は望ましさの表明ではなく、
**弱い helper が弱いままであることの確認**である。ここが赤くなったら 裁定 5 の前提
(弱い helper を検査の引き取り先にしてはならない) が変わったという合図なので、
失敗メッセージにそう書いた。

これで **「引き取り先の helper 選択は実測で裏付けられた」** — 裁定 5 は
「`validate_wasm_function_bodies` を使う」と決めたが、それが実際に本体の型不一致を捕捉することは
未実測だった。#5 がそれを恒常的に検査する。

#### #7 は fixture 側が壊れていた — 修復して pin した

`test_caws.ls` (2,952 bytes / 2 行) は括弧の釣り合いが取れている (文字列・コメントを
除いて数えても最終深さ 0) のに Rust parser が拒否する、という一見矛盾した実測から入った。
構造を機械的に走査して原因が 1 箇所に確定した。

- **offset 1795 の `(if (> arg-count 0) (do ...))` が else 節を欠く 2 引数 `if`。**
  L# の `if` は 3 引数である (`crates/lsharp-syntax/src/parser/expr.rs:194` の `parse_if` が
  cond / then / else を順に必須で読む)
- parser はこの `if` の閉じ括弧 (offset 2945) で else を求めて `)` に当たる。
  **報告された最初のエラー span 2945..2946 と一致する**。残り 5 件は回復の連鎖
- 走査した全 form のうち arity 異常は**この 1 箇所だけ**だった

**Rust parser の拒否が正しく、fixture が壊れていた。** そこで `0` を 1 つ補って修復し、
`Ok` / `decls == 2` を pin した。修復後、深くネストした 2.9KB の実コードが 1 ファイルとして
パースできることの回帰ガードになる。

**期待値を実測へ書き換えるのではなく、壊れた入力を直した。** この 2 つは形が似ているが別物である
(前者は本 ADR が繰り返し禁じてきたもの)。区別できたのは、fixture が壊れているという判定を
**parser の実装 (`parse_if` の arity) という独立した根拠**から取ったからである。
実測の値だけを根拠にしたなら、どちらの向きにも書けてしまう。

#### 副産物: selfhost parser は壊れた fixture を受理していた (`I-86`)

修復前の fixture を `lsharp parse` (native selfhost へ委譲される) に食わせると
`decls:2 diagnostics:0` を返した。最小例でも同じで、2 引数 `if` も top-level のゴミ atom
(`@@@ ###` → `decls:7`) も `diagnostics:0` で通る。**Rust reference より緩い。**
`diagnostics` チャネル自体は生きている (括弧不足は `P0001` を返す) ので、
報告経路が無いのではなく検査が無い。

**本 slice では直さない。** `ISSUES.md` の `I-86` / `TODO.md` の `SELFHOST-PARSE-LENIENT-01` へ切った。
なお fixture を修復したので、**この乖離を検出する test は現存しない**。修復と引き換えに
検出手段を失ったことは、`I-86` に明記した。

#### RED の取り方 (3 件とも入力を壊して取った)

| test | 壊した入力 | 結果 |
|---|---|---|
| #5 | `TEST_I64_IF_WASM[..20]` (切り詰め) | 1 つ目の assert で FAILED |
| #6 | `Compiler.ls` の先頭 2,000 bytes だけ | パース失敗で FAILED |
| #7 | fixture を修復前へ戻す | パース失敗で FAILED |

**期待値を一時的に狂わせる形 (弱い RED) は使っていない。** 3 件とも入力側を壊した。
`I-79` が定めた形 (b) と同じ取り方である。

#### 内訳は動かない

3 件はいずれも 裁定 2 / 裁定 3 の **assertion 追加** 帯にいたままである。母数 13 も内訳
(assertion 追加 9 / 削除 3 / 実質化 1) も動かない。**残 10 件は全部 `#[ignore]` 側**で、
`selfhost_bootstrap_four_layer` の再計測 1 本が要る。

### 裁定 7: pin の強さは「入力の由来」で決める (2026-08-27 追加)

裁定 3 は「期待値は実測で確定する」とだけ書いていた。実装してみると、**実測値をそのまま
`assert_eq!` に置くと壊れる test と、置かないと何も検査しない test の 2 種類がある**ことが分かった。
判断の分かれ目は入力がどこから来るかである。

| 入力の由来 | pin の強さ | 理由 |
|---|---|---|
| test 内の文字列リテラル / test が生成する fixture | **全値を `assert_eq!`** | 入力が 1 バイトも動かないので、出力が動いたら実装が変わったということ。下限で見ると検査していないのと同じ |
| 実在の `.ls` ファイル (`src/App/Main.ls` 等) | marker は exact、数値は**下限と関係式** | `.ls` を 1 行編集するだけで decl 数もバイト数も動く。exact に pin すると「ソースを触るたびに落ちる test」になり、主題 (progress の構造) から外れる |

**関係式が主役である。** 下限だけでは弱いが、下限に加えて
「冒頭で宣言した decl 数 == 末尾の decl 数」「import 数 == pair 数 - 1」
「出力長 == 21 + 10 * (decl 数 - 1)」のような**値どうしの拘束**を置くと、
個々の数が動いても構造の破れは捕まる。実装では
`part_018.rs` の `assert_debug_progress_shape` / `assert_build_compile_progress_shape` に
この拘束を集約した。

**却下した案。**

- **全部 exact。** `.ls` を触るたびに無関係な test が赤くなる。赤が日常になれば
  台帳が信用を失う — `I-84` が示した失敗形そのものである
- **全部下限。** `#4` (`..._string_length_if_progress`) の入力は test 内リテラルであり、
  下限で見ると「41 値のうち 1 値でも壊れて」も通る。実質化した意味が消える

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

### 13 件の決着 (2026-08-27)

| # | test | 裁定 | 状態 |
|---|---|---|---|
| 1 | `..._reports_main_again_cache_pairs_progress` | 3 | **実質化済** (`part_008.rs`) |
| 2 | `..._reports_main_again_progress` | 3 | **実質化済** (`part_008.rs`) |
| 3 | `..._reports_module_resolver_progress` | 3 | **実質化済** (`part_011.rs`) |
| 4 | `..._reports_string_length_if_progress` | 3 | **実質化済** (`part_011.rs`、全値 exact) |
| 5 | `test_i64_if_condition_validity` | 2 | 実質化済 (裁定 6) |
| 6 | `test_parse_compiler_ls` | 3 | 実質化済 (裁定 6) |
| 7 | `test_parse_caws_standalone` | 3 | 実質化済 (裁定 6。fixture 側の破損を修復) |
| 8 | `test_debug_stage2_output_minimal` | 1 | **削除済** |
| 9 | `test_validate_stage2_wasm` | 5 | **実質化済** (`part_015.rs`) |
| 10 | `test_debug_stage3_output_chars` | 1 | **削除済** |
| 11 | `test_debug_stage3_main_again_output_chars` | 1 | **削除済** |
| 12 | `..._stage2_classifies_chunked_lexer_failure_band` | 4 | **実質化済** (`part_014.rs`) |
| 13 | `..._const_only_entrypoint_helper_offsets` | 3 | **未着手** (`stage_chain.rs`。別 lane module) |
| - | `test_debug_stage2_save` | 1 | **削除済** |

削除 4 件は `grep -rn 'fn test_debug_stage2_output_minimal|...' crates/lsharp-wasm/tests/e2e`
が 0 hit であることで確認した (2026-08-27)。**残るは #13 の 1 件だけ**である。

### 実測値 (2026-08-27、`cargo test -p lsharp-wasm --test e2e -- --exact <t> --ignored --nocapture`)

| test | 実測 | pin の型 |
|---|---|---|
| #1 cache-pairs-progress | `[85, 32, 31]` | marker exact + pair 数下限 26 + `import == pair - 1` |
| #2 main-again progress | 42359 値。冒頭 `[1, 4, 2, 31, 3, 4146]` / 末尾 `[30, 31, 3688, 4, 4, 4146]` | marker exact + 冒頭と末尾の再掲一致 + 下限 |
| #3 module-resolver progress | 401 値。decl 39 / src 12043 bytes / import 0 | 同上 + `len == 21 + 10 * (decl - 1)` |
| #4 string-length-if progress | 41 値。decl 3 / src 203 bytes | **全 41 値 exact** (入力が test 内リテラル) |
| #9 stage2 wasm | `validate_wasm_function_bodies` = `Ok(())` / `wasmtime::Module::new` = `Ok(())` / 1575570 bytes | 2 経路の成功 + サイズ下限 1MB |
| #12 failure band | `no-probe-failure`。below=650 / cross=660 / multi=1090 / large=8975 / main=1575570 bytes | band 名 exact + バイト数の単調性 |

`validate_wasm_detailed` は **使わなかった**。裁定 5 の記述どおり `ValidPayload::Func` を捨てるため、
関数本体が壊れていても `Ok(())` を返す。実測でも 3 者とも `Ok` だったが、
**「壊れていないから 3 者とも Ok」なのか「見ていないから Ok」なのかを区別できない検査は使えない。**

### 実測が見つけた本物のバグ 2 件

**(1) probe 名の文字列は飾りだった。** `..._first_defn_ir_parity_on_minimal_demo_main_shape` は
`91..99` の marker を期待する test だが、実測は `150,1,151,1,152,4,153,4,154,0` —
`cache-compile-phase-probe` の marker 列だった。原因は
`selfhost/src/App/Main.ls` の dispatch が **「どの arg スロットが非空か」だけで probe を選ぶ**ことにある
(22 段の `(if (> (string-length (command-line-arg N)) 0) ...)` 連鎖)。probe 名の文字列自体は読まれない。
当該 test は名前を arg18 (= `cache-compile-phase`) に置いており、arg13 が正しかった。
**test 名が主張する probe に一度も到達していなかった。**
修正後の再実測で 17 値の `91..99` 列を得た。

同じ誤配置が他にないか、`part_008/009/010/011` の probe 名 17 箇所を dispatch 表と突き合わせた。
**他の 16 件は正しかった。** ただしこの照合は使い捨て script で行っており、常設の検査は無い。

**(2) fixture が 1 バイトも読まれていなかった (`I-87`)。** `..._first_defn_probe_on_minimal_make_type_constrained_shape` の
stage1 側 probe は `301,-1` (= defn が 1 つも無い) を返していた。
fixture を `std::env::temp_dir()` に置いて絶対パスで渡していたが、stage1 の WASI runner は
selfhost ルートだけを `"."` へ preopen する (`wasi_runner/preview1.rs:109-117`) ので**原理的に読めない**。
しかも `read-file` は失敗時にエラーではなく空文字列を返すため、**guest は空ファイルとして先へ進む**。
出力自体は空でないので `assert!(!output.trim().is_empty())` は通っていた。
fixture を `selfhost/target/test-artifacts/` 配下へ移し相対パスで渡したところ、
stage1 / stage2 とも `[301, 0, 302, 7]` となり、この test の主題 (stage1 と stage2 が同じ defn を同じ形で見る)
が初めて成立した。**空文字列に潰れる `read-file` 自体は直していない** — `I-87` として登録した。

### 隣接する `I-85` (12 件) も同じ slice で閉じた

`assert!(!output.trim().is_empty())` だけを持っていた `test_debug_boot04_*` 12 件
(`part_009.rs` 4 / `part_010.rs` 7 / `part_011.rs` 1) を裁定 7 の型で実質化した。
`grep -rn 'trim().is_empty()' crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/`
の hit は全て無関係な `.filter()` だけになった (2026-08-27)。

**後日の追記 (2026-08-28)**: この実質化で `part_010.rs` が 937 行になり、800 行の
file-size 契約を破った (`I-103`)。**`part_010.rs` (4 件) と `part_010b.rs` (3 件) へ
分割済み**なので、上の「`part_010.rs` 7」は分割前の配置を指す。test 名は変えていない。

### 検証 (2026-08-27)

- `cargo test -p lsharp-wasm --test e2e --no-run` — 警告 0
- `cargo clippy -p lsharp-wasm --tests` — 本 slice が触った fragment の警告 **0**
  (workspace 全体では別ファイル由来の既存 warning が 11 件残る)
- 個別実行 **19 件すべて `exit 0`** (`I-81` の改名後 test と #1〜#4 を含む)

### 13 という件数の検算 (2026-08-27、cargo 不使用)

`python3 scripts/sweep_unchecked_result.py` を再実行し、**18 hit** を得た。
hit は行単位なので、test 単位へ畳んでから基準を当てる。

| 段階 | 数 | 内訳 |
|---|---|---|
| 生 hit | 18 | `a':match` 8 / `b:if-let` 1 / `c:binding` 9 |
| test 単位へ畳む | 15 | `stage_chain.rs:54969/54974/54979/54984` の 4 hit が #13 の 1 test に畳まれる |
| 走査の偽陽性を落とす | 14 | `runtime_allocator_closures.rs:1604` — `LSHARP_GC_METRICS_OUT` の `if let` で probe ではない |
| 基準の外を落とす | **13** | `part_007.rs:264` — 主題を別に検査している (上の「基準の外にある隣接ケース」) |

**引用単位と test 単位を混ぜない。** 生 hit 18 をそのまま件数として使うと、
#13 が 4 件に膨らむ。これが「件数が 3 度動いた」うちの 1 回の正体である。

**走査は `I-85` の形を見つけられない。** `sweep_unchecked_result.py` の判定は
「結果を束縛したが assert していない」であり、`assert!(!output.trim().is_empty())` があれば hit しない。
走査の定義としては正しい。`I-85` の 12 件を見つけるには**別の走査が要る**
(「主題の assertion が存在するが、その中身が出力の有無しか見ていない」)。

| 対象 | 実測 | 取得条件 |
|---|---|---|
| `tests/fixtures/selfhost-debug/test_i64_if.wasm` の妥当性 | **不正**。`wasm2wat` が 2 件のエラーを出す (`expected [i32] but got [i64]` / `` `if false` branch, expected [i64] but got [] ``) | `wasm2wat tests/fixtures/selfhost-debug/test_i64_if.wasm`、2026-08-27、32 bytes |

### #13 の実装 — harness が emitter の列契約を破っていた (2026-08-27)

**この節は裁定 3 の #13 分の doc-GREEN である。**

#### RED — 主題の assertion が最初の 1 本目で落ちた

`println!` 4 本を parity assertion 3 本へ置き換えて測ると、**主題そのものが赤くなった**。

```
assertion `left == right` failed: 末尾関数版の entrypoint offset が selfhost emitter と
generic emitter で食い違う:
selfhost_last=[1, 10, -8] selfhost_abs=[1, 10, 0] generic_abs=[1, 10, 0] generic_last=[1, 10, 0]
  left: [1, 10, -8]
 right: [1, 10, 0]
```

`stage_chain.rs:54968`、2026-08-27、`--exact` 2 件で 1032.78s。
**Vec の要素 0 / 1 は harness の echo** (`(vector-length functions)` と `main-func-idx`) であり、
offset は要素 2 だけである。食い違っているのは **-8 と 0** の 1 箇所。

#### 原因は emitter ではなく harness である — cargo を使わずに確定した

`run_selfhost_override_entrypoint_offset_probe` は `offset_expr` から 4 つの束縛を見せる。
初版は **selfhost 側に `functions`、generic 側に `native-callables`** を渡していた。
この 2 つは長さが違う (1 と 11)。`native-callables` は先頭に import placeholder を 10 個持つ。

emitter が要求するのは **placeholder 込みの列**である。根拠は 3 つあり、いずれも実行を要さない。

| 根拠 | 位置 | 内容 |
|---|---|---|
| `collect-callable-function-starts-x86` | `NativeCodegen.ls:9264` | ループを `idx = import-count` から `(vector-length functions)` まで回す。aarch64 版 (`:18317`) も同じ |
| `native-last-callable-function-idx-with-import-count` | `NativeCodegen.ls:20785` | `len - 1` を返し、それが `callable-idx = entrypoint-func-idx - import-count` の被減数として使われる (`:20780`)。**大域 callable index を列長から導いている** |
| harness 自身 | `stage_chain.rs:21240` | `callables` を `push-import-placeholders 0 10` + `functions` として組み立てている。generic 側にはこれを渡していた |

長さ 1 の列に `import-count = 10` を渡すと、区間 `[10, 1)` は空になる。
`function-starts` は空 vector になり、`callable-user-total-size` は 0 になる。
**`-8` はこのゴミの帰結であって、selfhost emitter と generic emitter の意味論の差ではない。**

#### `selfhost_abs` の緑は、偽の前提の下での偶然だった

`selfhost_abs` は 0 を返して `generic_abs` と一致していた。**これは正しさの証拠ではない。**
`function-starts` が空なので `native-bundle-entrypoint-offset-for-function-with-import-count` の
`(< callable-idx len)` が偽になり、フォールバックの `0` が返っただけである。
`I-82` の 127 / 128 marker と同じく **binary 依存のゴミ**であり、
一致していたことに意味は無い。**赤い方だけを見て「片方は通っているのだから入力は正しい」と
読んではならない。**

#### 是正

`functions` を渡していた 4 箇所を `callables` へ揃えた。selfhost 版 emitter は内部で
`normalize-selfhost-native-function-metas-for-target` を掛けるので、正規化前の `callables` が
正しい引数である (generic 版は自分では正規化しないので `native-callables` のまま)。

| 位置 | test | #13 か |
|---|---|---|
| `stage_chain.rs:52679` / `:52685` | `..._helper_before_main_entrypoint_probe` | いいえ。`println!` のみの diagnostic probe |
| `stage_chain.rs:54925` / `:54930` | `..._const_only_entrypoint_helper_offsets` | **はい** |

**#13 の外の 2 箇所も同時に直した。** 同じ関数の同じ引数スロットに同じ誤りがあり、
片方だけ直すと「契約を知りながら残した」ことになる。ただし当該 test は `println!` しか
していないので、**判定は何も変わらない** (`I-85` の類型として別途残る)。

列契約は `run_selfhost_override_entrypoint_offset_probe` の doc コメントへ固定した。
**同じ誤りを次の人がやらない置き場は、test 本体ではなく harness である。**

#### 覆えていない範囲

`target` は `(host-target)` なので、本 test が通るのは **aarch64 の経路だけ**である。
`normalize-selfhost-native-function-metas-for-target` は `(= (target-arch target) 1)` のとき
恒等写像になるため、**x86 では selfhost 版と generic 版で正規化の有無が食い違う**。
本 test はその差を検出しない。x86 ホストで測るか target を注入できるようにするかは
本 slice の範囲外とし、`ISSUES.md` の `I-92` に載せた。

#### GREEN

`callables` へ揃えた binary で測り直すと **緑になった**。

| test | 結果 |
|---|---|
| `..._const_only_entrypoint_helper_offsets` (#13) | **ok** |
| `..._entrypoint_offset_resolves_to_app_main_main` (#5) | **ok** |

`e2e --ignored --nocapture --test-threads 1` に 3 filter を同時に渡して 1729.49s
(3 件目は `I-84` #2 の反転前実測)、2026-08-27、`target/debug/deps/e2e-aa343ded249bec81`。

**これで「emitter が食い違っている」という読みは否定された。** 4 つの emitter は
同じ入力に対して同じ entrypoint offset を出す。`-8` は harness が渡した列が
契約を満たしていなかったことの帰結だった。

**#5 は本 slice で追加した `start == entrypoint_offset` の assertion 込みで緑である。**
裁定 3 が「実測が否定したら落としてよい」と留保していた 2 つのうち、
こちらは**実測が支持したので残す**。もう一方 (#13 の `selfhost_last == selfhost_abs`) も
同じ実行で緑になったので残す。**どちらも落とさなかった。**

## 満たせなかったこと

- **#13 は 2026-08-27 に閉じた。** 実装と実測は上の
  「#13 の実装 — harness が emitter の列契約を破っていた」節。
  13 件すべてに主題の assertion が入った。
- **x86 の経路は測れていない** (`I-92` / `NATIVE-X86-ENTRYPOINT-PARITY-01`)。
  #13 の parity は aarch64 でしか成立を確認していない。
- **lane 再計測は 2026-08-28 に完了した (当初は未了)。** #13 を含む
  `selfhost_native_stage_chain` が 613 宣言 / 613 結果行 / FAIL 111 / `MODEXIT=101` /
  18,545.78s、`selfhost_bootstrap_four_layer` が 144 / 144 / FAIL 1 / 5,816.58s で完走し、
  どちらも `compare_ignored_lane.py` は `新規 FAIL 0 / 解消 0 / 未出現 0` を返した。
  個別実行 15 件の緑では代用できない (完走判定は `running N tests` == 一意な result 行数を
  module 単位で要求する) という当初の記述はそのとおりで、module 単位で測り直した。
  lane 実測は [ignored-lane-sweep-2026-08-23.md](../development/operations/ignored-lane-sweep-2026-08-23.md) の `結果 (2 回目 -- 3 module とも完走)`。
- **probe 名と arg スロットの対応を検査する常設の仕組みは無い。** 誤配置 1 件を見つけた照合は
  使い捨ての script で行った。同じ誤りは再発しうる
- **`I-87` (`read-file` が失敗を空文字列に潰す) は直していない。** test 側で fixture の置き場を
  変えて回避しただけである。**回避であって是正ではない**ことを `I-87` 本文にも書いた
- **`I-82` の枠組みを書き直した。** 「assertion を 1 つも持たない」は 13 件中 6 件で成り立たない。
  commit 済みの記述だが直した。**正確さが churn に勝つ**という判断であり、
  件数が 4 度目に動いても「基準が精緻化された」履歴として残るよう、基準文を先に置く構成にした

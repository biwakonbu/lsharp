# native/selfhost parity harness が型を比較する方法

- **Status**: doc-GREEN (focused 1 本まで / lane 未了 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED)
- **Scope**: `selfhost_native_stage_chain.rs:14633` の
  `test_e2e_selfhost_native_typeinfer_program_apply_matches_selfhost` が埋め込む L# harness と、
  そこで型を印字する方法。
- **含めない範囲**: `repl-session-eval` の tag 分岐 (`REPL-TYPE-TAG-01` / `I-69` が持つ)。
  `I-45` の 0 引数 `defn` 契約そのもの
  ([`decisions-selfhost-zero-arity-defn-type.md`](decisions-selfhost-zero-arity-defn-type.md) が正本)。
  `assert_representative_override_main_matches_selfhost` の実装。
- **Related**: `ISSUES.md` の `I-98` / `TODO.md` の `NATIVE-TYPEINFER-PARITY-PIN-01`

## 何が問題か

harness は 4 行を印字して native backend と Rust-hosted selfhost の stdout を突き合わせる。

```
(print (type-tag ty))
(print (type-name ty))
(print (infer-program-analysis-diagnostic-count analysis))
(print (infer-program-analysis-first-error-code analysis))
```

`ty` は `(defn p [] (not true))` の program 型で、`I-45` (`914bd9f1`, 2026-08-22) 以降
`Unit -> Bool` すなわち tag 3 (`type-fun`) である。tag 3 の layout は
`make-type-fun` (`Types/Type.ls:51-67`) が作る `[3, param-ty, ret-ty]` なので、
`type-name` (`Type.ls:249` = `(vector-get ty 1)`) が返すのは **名前ハッシュではなく
引数型 object の handle**、つまり heap address である。

`I-98` の実測は `-9223372036854106208` (native) vs `-9223372036853747400` (selfhost)。
どちらも bit 63 が立っており、offset は 669,600 / 1,028,408。
**backend が違えば heap 配置が違うので、この 2 値が一致することはない。**

**これは実装の欠陥ではなく、成立しない assertion である。** 実装を何一つ直さなくても永久に赤い。

## 判断

**harness を型の構造を印字する形へ書き換える。**

印字する 7 値は次のとおり。いずれも小整数であり、object handle を経由しない。

| 印字 | 期待される種類 |
|---|---|
| `(type-tag ty)` | 3 (`type-fun`) |
| `(type-tag (type-fun-param ty))` | param の tag |
| `(type-name-or-minus-one (type-fun-param ty))` | Con なら名前ハッシュ、Var なら id、それ以外 `-1` |
| `(type-tag (type-fun-ret ty))` | ret の tag |
| `(type-name-or-minus-one (type-fun-ret ty))` | 同上 |
| `(infer-program-analysis-diagnostic-count analysis)` | 0 |
| `(infer-program-analysis-first-error-code analysis)` | 0 |

`type-name-or-minus-one` は harness 内に置く helper で、tag が 1 (`type-con`) か
2 (`type-var`) のときだけ `type-name` を呼ぶ。**tag 3 / 4 / 5 へ `type-name` を当てる形は
harness から消える。** 名前ハッシュは `make-type-int` 100 / `make-type-bool` 200 /
`make-type-string` 300 / `make-type-float` 400 / `make-type-unit` 500 (`Type.ls:29-47`) の
リテラルであり、backend に依存しない。

### 検査は弱まらない。強くなる

`TODO.md` の受入条件 (b) が要求する説明である。

現状の harness は **型の中身を一度も比較していない**。比較できていたのは
tag (3) と diagnostics (0/0) の 3 値だけで、2 番目は両 backend で必ず違う値なので
情報を運んでいない。書き換え後は param と ret の tag と名前が両 backend で
一致することを要求するので、比較する意味のある値が 3 -> 7 へ増える。

test の目的 (native の parser -> TypeInfer apply 経路が Rust-hosted selfhost と
同じ結果を返すこと) は、**型の中身を見るようになる分だけ強く**なる。

## 却下した案

### 案 A: `(print (type-name ty))` の行を消すだけ

**却下。** 赤は消えるが残るのは tag と diagnostics の 3 値で、型の中身を一切見ない。
`I-99` の受入条件 (c) が `byte-at-or-zero` について却下したのと同じ形 --
「trap は消えるが主張が無内容になる」。

### 案 B: 期待値を固定値へ書き換える

**却下。** `assert_representative_override_main_matches_selfhost` は
native と selfhost の出力を互いに突き合わせる harness であり、固定期待値を持たない
(`selfhost_native_stage_chain.rs:48666-48711`)。固定値を持たせるには harness ごと
別種の assertion へ作り替えることになり、backend 間 parity という test の目的が消える。

### 案 C: `type-name` を tag 分岐する形へ `Types/Type.ls` 側で直す

**却下 (本 slice では)。** production の `type-name` は
「Con の名前ハッシュを取る」ものとして各所から呼ばれており、意味を変えると
呼び出し元全体に波及する。tag を確かめずに `type-name` を呼ぶ production の欠陥は
`I-69` / `REPL-TYPE-TAG-01` が別に持っている。**本 issue は harness の問題なので、
harness で閉じる。** `I-98` が「`REPL-TYPE-TAG-01` に束ねない」と書いた理由もこれである
(実装を直しても harness は address を印字したままになる)。

## 走査 (受入条件 (c))

`assert_representative_override_main_matches_selfhost` / `..._with_path_arg` の呼び出しは
`selfhost_native_stage_chain.rs` に **119 箇所**ある。同一の壊れ方が他に無いかを
2 通りで確かめた。

1. **pattern 走査**: 同ファイルの `(print (type-` は `:14647` / `:14648` の 2 行だけで、
   `type-name` の出現は **ファイル全体で 1 件** (`:14648`)。すなわち
   tag 3 の型へ `type-name` を当てる形は他に無い
2. **台帳照合**: `ignored-lane-expected-failures.txt` の
   `selfhost_native_stage_chain` 行のうち、native/selfhost の出力不一致という形の赤は
   `:412` (本件) の 1 行のみ。他は Linux x86 系 / artifact 生成系 / global window
   diagnostic 系で形が違う

**走査しきれていないもの**: `(print (vector-get ...))` が 347 箇所ある。
`vector-get` は object を返しうるので原理的には同じ壊れ方が起こせるが、
**347 件を個別に読んではいない**。上記 2 の台帳照合が間接証拠になる
(同型の赤なら既に台帳に載っているはず) が、直接の確認ではない。

## Evidence

### 書き換えた harness は緑になった

| 項目 | 値 |
|---|---|
| 起動 | `/Users/biwakonbu/github/tmp/i98/run_solo.py` を `os.setsid()` で切り離し |
| ログ | `/Users/biwakonbu/github/tmp/i98/solo.log` |
| 結果 | `test result: ok. 1 passed; 0 failed; 0 ignored; 3080 filtered out; finished in 119.20s` / `RUNEXIT=0` |

`3080 filtered out` + 1 = **3081**。宣言数と一致する。

旧 harness は `type-name` を Fn 型へ当てて heap address を印字していたので
backend 間で一致しようがなかった。構造化した 7 値はすべて小整数なので一致した。

### 緑だけでは足りなかった -- 非空検査で緩みが 1 件出た

`assert_representative_override_main_matches_selfhost` は
`parse_numeric_lines` の結果どうしを `assert_eq!` するだけである。
**両方が 0 行でも通る。** 上の緑が「7 個の意味ある値の一致」なのか
「0 個どうしの一致」なのかは、緑だけからは区別できない。
これは本 slice で `I-99` の 2 件について潰したのと同じ vacuous green の形である。

そこで helper を `Vec<i64>` 返しへ変え (呼び出し 114 箇所は戻り値を捨てるので影響しない)、
本 test だけで個数と中身を検査した。**期待値は測る前に selfhost source から導いて
事前登録した** (`/Users/biwakonbu/github/tmp/i98/prediction_values.md`)。

| # | 印字式 | 予測 | 実測 | 判定 |
|---|---|---|---|---|
| 1 | `(type-tag ty)` | 3 | 3 | 当たり |
| 2 | `(type-tag param-ty)` | 1 | 1 | 当たり |
| 3 | `(type-name param-ty)` | 500 | 500 | 当たり |
| 4 | `(type-tag ret-ty)` | 1 | **2** | **外れ** |
| 5 | `(type-name ret-ty)` | 200 | **1001** | **外れ** |
| 6 | `diagnostic-count` | 0 | 0 | 当たり |
| 7 | `first-error-code` | 0 | 0 | 当たり |

**`infer` は `Unit -> Bool` ではなく `Unit -> t1001` を返していた。**
`not : Bool -> Bool` は登録済み (`TypeInferBuiltins.ls:129` / `:182`)、
`infer-program-analysis-type` は返す前に `apply-subst` を通している
(`TypeInfer.ls:1473-1479`)。それでも型変数が残る。診断は 0 件なので落ちない。

**これは `ISSUES.md` の `I-101` として立てた。** 引き取り先は
`TODO.md` の `SELFHOST-INFER-RET-VAR-01`。

### 実測値を期待値に焼き込まなかった

`[2, 1001]` を assert に書けば緑になるが、それは実装の出力へ期待値を合わせることである
(`CLAUDE.md` が禁じる形)。**代わりに slot 3/4 は値を固定せず、
「Con か Var のどちらかである」ところまでに留めた。** これは
`not` の戻り型が関数型でもレコード型でもないことから導ける。
残る 5 slot は source から導けるので値まで固定した。

`I-101` が解けたら slot 3/4 を `[1, 200]` へ固定できる。**その予定は
`SELFHOST-INFER-RET-VAR-01` の受入条件に書いてある。**

緩めた pin で測り直して緑になった:
`test result: ok. 1 passed; 0 failed; 3080 filtered out; finished in 117.83s` / `RUNEXIT=0`
(`/Users/biwakonbu/github/tmp/i98/values.log`)。

**2 回の独立 run で `structural type values: [3, 1, 500, 2, 1001, 0, 0]` が一字一句同じだった。**
1 回目は pin に落ちた run (`RUNEXIT=101`)、2 回目は緩めた pin の run (`RUNEXIT=0`) で、
プロセスも build も別である。したがって型変数 id `1001` は run 間で安定している。
`I-101` の未確認事項のうち 1 件はこれで解けた。

### 測定中に起きた SIGKILL

4 本を 1 プロセスにまとめた run で本 test が `signal: 9` で殺された
(`RUNEXIT=101` / `ELAPSED=304.65`)。assertion 失敗ではない。
**結果を見る前に予測を書き** (`/Users/biwakonbu/github/tmp/i98/prediction.md`)、
単独で回して判別した。単独では 119.20s で完走し緑。原因は harness 変更ではなく
1 プロセスへの常駐量の蓄積である。kill の主体 (jetsam か否か) までは示せていない。
運用は `AGENTS.md` の `--ignored` lane 節へ書いた。

## 満たせなかったこと

- **lane を回していない。** focused 1 本の緑は lane 1 本の完走ではない。
  台帳 (`ignored-lane-expected-failures.txt:412`) の行はまだ落としていない。
  `SWEEP-LANE-RERUN-01` が 7 項目まとめて引き取る。
- **戻り型の値を pin できていない。** `I-101` が解けるまで slot 3/4 は
  tag の種別までしか固定していない。**この test は今のところ戻り型の中身を検査していない。**
- **`(print (vector-get ...))` 347 箇所は個別に読んでいない** (doc-RED 時点からの残り)。
  走査したのは `type-name` の当て方だけである。
- **`assert_representative_override_main_matches_selfhost` を使う残り 113 本には
  非空検査を入れていない。** 共有 helper へ一律に足すと、対照が 1 次元でなくなり
  lane で大量の赤が出たときに本 slice が原因かを判別できなくなるため見送った。
  必要なら個別に戻り値を受けて検査する形が使える。

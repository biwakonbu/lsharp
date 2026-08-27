# target-defn probe が AST の形を添字直打ちで辿っている

- **Status**: doc-GREEN (裁定 1〜3 とも実装済。lane 再計測待ち)
- **Date**: 2026-08-27 (doc-RED) / 2026-08-27 (doc-GREEN)
- **Scope**: `selfhost_bootstrap_four_layer` の target-defn parity probe 2 件と、
  その probe 本体 (`selfhost/src/App/CompilerMode.ls` の `target-defn` モード)
- **Related**: `ISSUES.md` `I-80` (本件) / `I-72` (これを隠していた) / `I-75` (移管元) /
  `I-82` (`decisions-probe-subject-unchecked.md`) / `I-84`
  (`decisions-always-failing-diagnostic-probes.md`)。
  引き取り先は `TODO.md` の `TARGET-DEFN-PARITY-01`。

## 何が起きているか

`I-80` は **compiler の regression ではなく、probe の陳腐化**である。

probe は `make-type-constrained` の AST を **添字直打ち**で辿る:

```
decl        = (vector-get decls target-idx)
body        = (vector-get decl (+ 3 (vector-get decl 2)))
outer-expr  = (vector-get body 3)
inner-call  = (vector-get (vector-get outer-expr 3) 4)
inner-func  = (vector-get inner-call 1)
```

この経路は `make-type-constrained` の body が

```
(let [v (vector-new 2)] (vector-push (vector-push v (ast-typeconstrained)) name-hash))
```

という **`let` + 二重 `vector-push`** の形をしていることを前提にしている。
現在の定義 (`selfhost/src/Syntax/AST.ls:260`) は

```
(defn make-type-constrained [name-hash]
  (vector-push-pair-rooted (vector-new 2) (ast-typeconstrained) name-hash))
```

で、**`let` が無く、呼び出し 1 つ**である。よって `body` の tag は `ast-let` (7) ではなく
`ast-apply` (5) になり、`outer-expr` 以下の添字は平坦化された AST の外へ出る。

これが 2 つの marker の実測値をそのまま説明する:

| marker | 中身 | 期待 | 実測 | 説明 |
|---|---|---|---|---|
| 126 | `body[0]` | 7 (`ast-let`) | 5 (`ast-apply`) | body が `let` ではなく apply |
| 127 | `inner-call[0]` | 5 (`ast-apply`) | 0 | `body[3][3][4]` が AST の外を指す |

## 証拠

1. **tag 値**: `selfhost/src/Syntax/AST.ls` で `ast-let` = 7、`ast-apply` = 5、`ast-defn` = 20、
   `ast-var` = 4。marker 126 の期待 7 / 実測 5 はちょうど「`let` を期待して apply を得た」形。
2. **現在の定義**: `AST.ls:260` は `vector-push-pair-rooted` 単一呼び出し。`let` は無い。
3. **旧 shape の残骸**: `part_009.rs:456`
   `test_debug_boot04_stage2_first_defn_probe_on_minimal_make_type_constrained_shape` は
   minimal fixture として **旧 shape の文字列をそのまま埋め込んでいる**。
   probe が何に対して書かれたかの直接証拠である。
4. **時系列**: probe 本体と test 名はどちらも `357f261d` (2026-04-11) で導入された。
   `AST.ls:260` が `vector-push-pair-rooted` へ書き換わったのは `901c10d8` (2026-04-22)。
   **refactor は probe より 11 日新しい。** probe は追随していない。
5. **名前が実在しない**: `ast-make-type-constrained` という名前はソースのどこにも無く、
   `TODO.md` / `ISSUES.md` にしか出てこない。実際の対象は `make-type-constrained`。

## 壊れていない部分 — 「別の defn を見ている」ではない

`I-80` は原因候補として「対象 defn が短く切れている」と「そもそも別の defn を見ている」を
挙げていた。**どちらでもない。**

同じ probe の marker 124 (`decl[0]` = 20 = `ast-defn`) と marker 125 (`decl[2]` = param 数 = 1)
は**通っている**。`make-type-constrained` は `[name-hash]` の 1 引数なので、
probe は**正しい defn を正しく見つけている**。壊れているのは
**その先の body 内ナビゲーションだけ**である。

## stage1 側と stage2 側は同一原因である

`I-80` は「marker が別なので 1 つの結論にまとめるな」と書いていた。妥当な慎重さだったが、
実物を読むと **2 つの test は同じ `target-defn` probe を、stage1 の binary と stage2 の binary で
走らせているだけ**である (`part_009.rs:270` / `:414` がどちらも `"target-defn"` を渡す)。
assertion の並び順が違うため落ちる marker が違って見えるにすぎない:

- stage1 側 (`..._lengths`) は 126 を `assert_eq!(.., 7)` で見るので **126 で落ちる**
- stage2 側 (`..._reaches_...`) は 126 を `> 0` でしか見ないので通過し、**127 で落ちる**

stage1 側も 126 を緩めれば 127 で同じように落ちる。**原因は 1 つ。**

## 未検証の層が残っている

**marker 129 以降の assertion は一度も走っていない。** どちらの test も 126 / 127 で
落ちるため、その先の

- `129 == 131` (use-site と def-site の hash 一致)
- `130 > 0` / `132 > 0` / `133 > 0` (lookup が空でない)

は評価されていない。加えて probe 本体は marker 131/132/134/136..139 のために
**`(vector-get decls 31)` を添字直打ち**している。decl の並び順が変われば同じ形で陳腐化する。

したがって **126/127 を直すと、その下から新しい赤が出る可能性がある**。
`I-72` を直したら `I-80` が出てきたのと同じ構造である。
**「126/127 が緑になった」を本項目の完了条件にしない。**

## 裁定

### 1. stage2 側 (`..._reaches_ast_make_type_constrained`) — リテラル pin をやめ parity 比較にする

test 名が主張しているのは **parity** である。それなら期待値リテラルではなく、
**stage1 の probe 出力と stage2 の probe 出力を比較**すればよい。
AST の形が変わっても壊れず、かつ「stage1 と stage2 が同じものを見ているか」という
主題はそのまま検査できる。形を pin することは主題ではない。

### 2. stage1 側 (`..._lengths`) — shape pin として残し、リテラルを実測へ更新する

こちらは比較相手が無いので parity 化できない。**shape pin であることを引き受ける**:

- `126` の期待を 7 から **5 (`ast-apply`)** へ更新する
- `127` の期待は現在の shape での実測に合わせる (盲目的に 5 を残さない)
- **何を pin しているのかをコメントに明示する** — 「`make-type-constrained` の body 形」。
  次に AST.ls を refactor した人が、赤が出た理由を 11 日ではなく即座に分かる状態にする

この test が将来の refactor で赤くなるのは**正しい挙動**である。今回の問題は
赤くなったことではなく、**4 ヶ月間ずっと `I-72` に隠れていて赤くならなかった**ことである。

### 3. minimal fixture (`part_009.rs:456`) — 旧 shape を現在の shape へ更新する

`(let [v (vector-new 2)] (vector-push (vector-push v ...) ...))` は AST.ls にもう存在しない。
「minimal な `make-type-constrained` の形」を名乗りながら、何も鏡写しにしていない。
現在の `vector-push-pair-rooted` 形へ差し替える。
**`vector-push-pair-rooted` は builtin ではない。** 本 ADR は当初「selfhost 内に `defn` が無く
builtin として解決されている」と書いたが、これは `selfhost/src/Compiler/` だけを検索した誤りである。
実際には `selfhost/src/Syntax/AST.ls:67` に 14 行の module-local `defn` がある
(`IR/IR.ls:96` と `Backend/Native/NativeCodegen.ls:46` にも同名の複製がある)。

```lisp
(defn vector-push-pair-rooted [base first second]
  (do
    (root_push first)
    (root_push second)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      ...
```

したがって **minimal fixture には定義を足さなければならない**。fixture は flat file を
CompilerMode へ食わせるものなので、未定義呼び出しのままでは shape を観測する前にコンパイルが落ちる。
足す定義は `root_push` / `root_set` / `root_pop` / `vector-push` / `vector-new` を引き込む。
これらが builtin であることは実装時に確かめる — **同じ誤りを二度しないため、検索範囲を
`selfhost/` 全体と `crates/` の両方に取ること。**

なお、この test の唯一の assertion は `assert!(!probe_output.trim().is_empty())` で、
**主題 (shape) を検査していない**。`I-82` の基準に照らすと境界事例だが、
恒真ではなく出力の存在は検査しているため、`I-82` の 13 件には**加えない**。
件数の定義を再び動かさないための判断である。裁定 3 の shape 更新と同時に
assertion を主題側へ寄せるかは実装時の裁量とする。

## 却下した案

### A. 期待値を現在の実測へ書き換えるだけ

126 を 5 に、127 を実測値にして終わり。**却下。** 2 件とも通るようにはなるが、
stage2 側は名前が主張する parity を依然として測らない。
そして次の AST refactor でまったく同じことが起きる。
今回 4 ヶ月気付かなかった理由 (`I-72` に隠れていた) は消えるが、
**壊れやすさそのものは残る**。

### B. probe 本体 (`CompilerMode.ls`) を shape 非依存の構造ハッシュへ作り替える

添字直打ちをやめ、decl を再帰的に走査して構造ハッシュを出す。**却下 (この slice では)。**
筋としては最も正しいが、selfhost 側の変更であり、stage0 の再生成と
native lane への波及を伴う。`I-80` は test 2 件の赤であって、
selfhost compiler の変更を正当化する規模ではない。
**裁定 1 (parity 比較) は test 側だけで同じ堅牢性を得られる。**
本案は `(vector-get decls 31)` の hardcode が実際に問題を起こした時点で再検討する。

### C. 2 件とも削除する

probe の主題 (stage1/stage2 の target-defn parity) は本物であり、
`I-72` の下から出てきたばかりで一度も緑を見ていない。
**検査の実績が無いものを、赤いという理由で消すのは、検査を諦めることと区別が付かない。** 却下。

### D. 「stage1 側と stage2 側は別原因かもしれない」を維持して 2 issue に割る

`I-80` の当初の慎重さはこれを示唆していた。**却下。**
実物を読んだ結果、同じ probe の同じナビゲーション経路であることが確定した。
**慎重さは実物を読むまでの態度であって、読んだ後まで持ち越すものではない。**

## 実装順序の制約

本件の test は `selfhost_bootstrap_four_layer` にある。同じ module には
`PROBE-ASSERTS-NOTHING-01` (12 件) と `VIOLATION-PROBE-STALE-01` (1 件) の実装対象もある。
**3 件の裁定は 1 つの slice に束ね、lane 1 本 (約 112 分) で覆う。**
module を跨ぐ `ALWAYS-RED-PROBE-01` の stage_chain 分と
`PROBE-ASSERTS-NOTHING-01` の #13 は**この束に入れない**。

## Evidence

### RED の記録 (2026-08-27)

`I-82` の部分再測定 (21 target / 2058.35s) が両方を赤で捉えている
(`/Users/biwakonbu/github/tmp/i82/measure.log:153-168`)。本 slice はこの赤を出発点にした。

| test | 落ちた位置 | left | right |
|---|---|---|---|
| `..._reaches_ast_make_type_constrained` | `part_009.rs:302` | 0 | 5 (marker 127) |
| `..._lengths` | `part_009.rs:411` | 5 | 7 (marker 126) |

同じ実行で **full dump が両 stage 分そろって取れた**。これが裁定 1 の設計を決めた。

```
stage1: 121:59 124:20 125:1 126:5 127:4294967296 128:72057594054705152 129:0 130:0
        131:-5490128408457682031 132:83 133:0 134:39 135:0 136:39 137:777 138:777 139:777
        140:32 141:778 142:555 143:444 144:333 145:222 146:1 147:1 123:21 122:21
stage2: 上と同じ。ただし 127:0 / 128:0
```

**27 ペア中、食い違うのは 127 と 128 の 2 つだけである。**

### 裁定 1 (stage2 側を parity 比較へ) — 実装済、ただし除外 2 件つき

期待値リテラルを全廃し、同じ probe を stage1 の binary でも走らせて突き合わせる形にした。
比較は 2 段:

1. **marker 列そのものの一致** — 除外した marker が消えても気付けるようにするため
2. **値の一致** — ただし 127 / 128 を除く

**127 / 128 を除外したのは、裁定を書いた時点では見えていなかった実測による。**
この 2 つは旧 `let` shape 前提のナビゲーションが AST の外を読んだ結果であり、
範囲外読み出しの値は binary 依存になる。stage1 は `4294967296` / `72057594054705152`、
stage2 は `0` / `0` を返す。**これは stage 間の意味論の差ではなく、ゴミを読んでいることの帰結である。**
全 marker を parity 対象にすると恒常赤になり、`I-84` (構造上必ず赤くなる probe) の類型に落ちる。
除外理由はコード内のコメントに書き、`TARGET_DEFN_OUT_OF_RANGE_MARKERS` として
`part_018.rs` に定数化した。

### 裁定 2 (stage1 側を shape pin として残す) — 実装済、ただし 127 の扱いは文言と違う

126 の期待を 7 → **5 (`ast-apply`)** へ更新した。ここは文言どおり。

**127 は「現在の shape での実測へ更新する」を実行しなかった。**
実測すると stage1 は `4294967296` で、これは値としての意味を持たないゴミである。
ゴミをリテラルで pin すると、次にここが赤くなったとき「何が変わったのか」を誰も読めない。
代わりに次を pin した:

- `129 / 130 / 133 / 135 == 0` — 壊れたナビゲーションの下流はどちらの stage でも 0 になる
  (hash 0 は ftable に無いので lookup が全部外れる)。**probe 本体を直すとここは非 0 になり
  赤くなる。それは正しい挙動である**とコメントに明記した
- `134 == 136` — chunked 登録と再帰登録が同じ関数へ同じ index を与えること
- `137 == 138 == 139 == 777` — 3 つの登録経路がどれも `decls[31]` を 777 へ写すこと
- `140 == 32` / `141 == 778` — `register-defns-step` の返す次 index と次関数 id
- `142..147` の sentinel 往復 (555 / 444 / 333 / 222 / 1 / 1)

つまり **pin の対象を「ゴミの値」から「登録経路の一致」へ移した**。
何を pin しているかは test 内のコメントブロックに 3 分類で書いてある。

### 裁定 3 (minimal fixture) — 実装済

fixture の body を `(let [v ...] (vector-push (vector-push ...) ...))` から
`(vector-push-pair-rooted (vector-new 2) (ast-typeconstrained) name-hash)` へ差し替え、
`AST.ls:67` の 14 行の `vector-push-pair-rooted` を定義ごと持ち込んだ。
`root_push` / `root_set` / `root_pop` は既存の minimal fixture
(`mini_vector_push_shape.ls`) と同じく stub を置いた。

**assertion は 302 が 7 → 5 へ変わる、という予測を立ててから測った。** 実測は
stage1 / stage2 とも `[301, 0, 302, 5]` で予測どおり。
この test の assertion は元から主題 (先頭 defn の形の parity) を見ていたので、そのまま活きた。

### marker 129 以降の初回評価 — 新しい赤は 0 件

`I-80` / `TODO.md` はどちらも「126/127 を直すと下から新しい赤が出る可能性がある」と書き、
「126/127 が緑になったことを完了条件にするな」と釘を刺していた。**実測では新しい赤は出なかった。**
ただし**これは「元の assertion がそのまま通った」という意味ではない**。元の

- `129 == 131` (use-site と def-site の hash 一致)
- `130 > 0` / `133 > 0` (lookup が空でない)

は**成立し得ない**。ナビゲーションが壊れている以上、`inner-func` から取った use-site hash は
0 であり、0 は ftable に無いので lookup も 0 になる。**前提が偽の assertion である。**
そこで assertion を「壊れている状態を pin する」側へ付け替えた
(`129 == 0` / `130 == 0` / `133 == 0` / `135 == 0`)。

**これは期待値を実測へ書き換えたのではなく、何を主題とするかを付け替えたものである。**
区別は次の一点にある — 付け替え後の assertion は、**probe 本体が直されたときに赤くなる**。
黙って緑にしたのではなく、直した人に気付かせる向きに置き直した。
元の 2 つの assertion を復元すべきことは `I-88` / `TARGET-DEFN-NAV-STALE-01` として台帳に載せた。

### 検証

| 対象 | 結果 |
|---|---|
| `cargo test -p lsharp-wasm --test e2e --no-run` | 警告 0 (10.01s) |
| `cargo clippy -p lsharp-wasm --tests` の `part_009` / `part_018` 分 | 警告 0 |
| `..._reaches_ast_make_type_constrained` 個別実行 | `===EXIT 0` (81.44s) |
| `..._lengths` 個別実行 | `===EXIT 0` (73.49s) |
| `..._first_defn_probe_on_minimal_make_type_constrained_shape` 個別実行 | `===EXIT 0` (76.76s) |

ログは `/Users/biwakonbu/github/tmp/i82/i80.log` と `i80b.log`。
`i80b.log` はコメント修正後の binary で 2 件を測り直したものである
(`i80.log` の 1 本目 / 2 本目は修正前の binary で走っていた)。

## 満たせなかったこと

- **`selfhost_bootstrap_four_layer` の lane 再計測をまだ回していない。** 個別実行 3 件が
  緑であることは lane 1 本の完走ではない。台帳 2 行の削除もこの再計測の後である。
  `I-80` / `I-81` / `I-82` / `I-85` の 4 項目が同じ 1 本を待っている。
- **裁定 2 の「127 を現在の shape での実測へ更新する」を文言どおりには実行しなかった。**
  上記のとおり、実測値がゴミであることが実行の前提を壊した。
  代わりに pin の対象を移し、その判断と根拠をここに書いた。**条件を静かに緩めてはいない。**
- **probe 本体 (`CompilerMode.ls`) は陳腐化したままである。** 却下案 B は却下のまま維持した。
  ただし本 slice で「壊れている状態を pin する」assertion を 4 つ増やしたので、
  案 B を実行するときに何を戻すべきかが具体化した。`I-88` に引き取らせた。
- **`(vector-get decls 31)` の hardcode は触っていない。** `TODO.md` の含めない範囲どおり。
  decl の並び順が変われば 131 以降が同じ形で陳腐化する。

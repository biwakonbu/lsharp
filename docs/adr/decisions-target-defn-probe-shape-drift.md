# target-defn probe が AST の形を添字直打ちで辿っている

- **Status**: doc-RED (裁定済み / 実装未着手)
- **Date**: 2026-08-27
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

(実装後に埋める。裁定 1〜3 の受入判定と、marker 129 以降で新しく出た赤の有無を書く。)

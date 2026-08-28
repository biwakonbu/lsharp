# 既定 compile mode における `command-line-args` の境界をどう扱うか

- **Status**: doc-RED (2026-08-28)
- **Date**: 2026-08-28
- **Scope**: selfhost compiler の `compile-file-mode` (既定 mode) が opcode 86
  (`command-line-args`) を含む target を emit しようとしたときの振る舞い。
- **Related**: `ISSUES.md` の `I-78` (診断の正本) / `TODO.md` の `CLI-SELFFEED-DIVZERO-01`
  (作業の正本) / `docs/development/operations/bootstrap-diff-artifacts.md`
  (backtrace と artifact の突き合わせ手順) /
  `docs/adr/decisions-selfhost-typeinfer-stub-override.md` 決定 4 (message-less trap の扱い)

## 何を決めるのか

`I-78` の診断は 2026-08-28 に確定した。stage1 compiler が `src/App/Cli.ls` を self-feed compile
するとき、`reject-native-only-wasm-opcode` (`Backend/Wasm/WasmEmit.ls:2212-2215`) が
opcode 86 を見つけて `(/ opcode 0)` で trap する。**算術バグではなく、意図的な拒否である。**

診断は終わったが、**どう直すかは決まっていない。** 本 ADR がそれを決める。

## 前提 (すべて cargo を起こさずに確認した。根拠は source 読解と artifact の逆アセンブル)

| # | 事実 | 根拠 |
|---|---|---|
| 1 | standalone builder (`build-wasm-bytes-wasi-standalone`) に到達する経路は `-o` 出力系だけである | `Cli.ls:869/871` と `EmbeddedCli.ls:1325/1326` の 4 defn のみが呼ぶ。さらにその唯一の呼び出し元は `run-compile-output` (`Cli.ls:879`) / `run-compile-source` (`EmbeddedCli.ls:1327`) |
| 2 | `App/Main.ls` のどの mode 分岐からも standalone builder には届かない | 上記 grep が `App/CompilerMode.ls` / `App/Main.ls` に 1 件も hit しない。**したがって「test の引数を変えれば直る」形ではない** |
| 3 | 既定 mode と standalone mode は「書き換えの有無」だけでなく **5 section すべてが別物**である | 下表 |
| 4 | 既定 mode の 11 import は runtime host 関数であって WASI ではない | `alloc` / `print` / `read-file` / `command-line-arg` / `string-concat` / `substring` / `file-exists` / `root-push` / `root-pop` / `root-set` / `print-string` (`WasmEmit.ls:2006`) |
| 5 | **出力 module の絶対 byte を pin している test は無い** | `..._stage_chain_match` 系は `assert_eq!(stage2_target, stage3_target)` という**相対**比較。`hash_fingerprint` の呼び出しはすべて `json!` の診断 payload 内で、assertion には 1 件も現れない |
| 6 | **出力 module は instantiate されない** | `extract_single_compiled_module` (`part_001.rs:715-740`) は `parse_emitted_wasm_modules` -> `assert_valid_wasm` までしかしない |
| 7 | 影響範囲は fixed-input-set 54 target のうち **1 件 (`Cli`) のみ** | `command-line-args` (複数形) を使う selfhost module は `App/Cli.ls` だけ |
| 8 | 空 vector を返す経路は**黙っていない** | `Cli.ls:882-883` が `standalone-preview1-capability-boundary-message` を stderr に出して `exit-compile-error` する |

### 2 つの builder の section 比較

| section | `build-wasm-bytes-wasi` (既定) | `build-wasm-bytes-wasi-standalone` |
|---|---|---|
| type | `emit-type-section-wasi-quad-functions` | `emit-type-section-wasi-standalone` |
| import | `...-alloc-print-read-arg-concat-sub-print-string` (**11 本 / runtime host**) | `emit-import-section-wasi-standalone` (**24 本 / WASI preview1**) |
| function | `emit-function-section-wasi-quad-functions` | `emit-function-section-wasi-standalone` |
| export | index base `(+ 11 func-count)` | index base `(+ 24 func-count)` |
| code | `emit-code-section-wasi-quad-functions-print-string` (**書き換え無し**) | `emit-code-section-wasi-standalone` (**書き換え有り**) |

出典は `App/CompilerMode.ls:6089-6120`。

## 前の commit の主張を 1 件訂正する

`789e26d8` の `ISSUES.md` `I-78` に、案 (i) の危険として

> 前者は 54 target 全ての出力 byte を変えるので fixed-point / differential の pin に広く波及する

と書いた。**これは誤りである。** 前提 5 / 6 のとおり、

- chain match は stage2 出力と stage3 出力の**相対**比較なので、両方が同じだけずれる案 (i) では壊れない
- fingerprint は診断 payload であって assertion ではない
- 出力 module は instantiate されないので、import 契約が 11 から 24 に変わっても実行時に露出しない
- differential の pin (`I-73`) は **native backend** の話で、この wasm 経路ではない

**帰結を数える前に書いた。** 案 (i) を却下するにしても、この理由では却下しない。
訂正は `ISSUES.md` 側にも反映する。

## 選択肢

### 案 (i) 既定 mode を standalone builder へ寄せる

`compile-file-mode` が `build-wasm-bytes-wasi` ではなく `build-wasm-bytes-wasi-standalone` を
呼ぶようにする。書き換え (`standalone-ir-instrs`) が効くので opcode 86 は 91 に化け、trap しない。

**却下する。** 理由は test の都合ではなく、契約の粒度である。

- 既定 mode の 11 import は **runtime host 契約**であり、standalone の 24 import は
  **WASI preview1 契約**である。両者は別の host に対する別の module である。
  `command-line-args` 1 命令のために module 全体の host 契約を差し替えるのは、
  必要な対処に対して桁が違う
- 既定 mode に `command-line-arg` (**単数形**、opcode 67) は既にある。
  欠けているのは複数形 1 個だけであり、「既定 mode は引数を扱えない」わけではない
- 書き換え `standalone-ir-instr` (`WasmEmit.ls:706-727`) は opcode の差し替えだけでなく
  `(make-instr 40 (+ operand 11))` という operand 調整も行う。これは 11 import レイアウトを
  24 import レイアウトへ寄せる補正であり、**既定 mode に持ち込むと二重補正になる**。
  案 (i) は「builder を差し替える」で済まず、書き換え側の前提も見直す必要がある

### 案 (ii) 境界を保ったまま、拒否を診断に変える

既定 mode 側にも pre-emit scan を置き、opcode 86 を見つけたら trap ではなく
診断メッセージ + 非 0 終了にする。**採用する。**

**リポジトリ内に先例がある。** standalone 側は既に

```
standalone-preview1-first-unsupported-opcode   Backend/Wasm/Compiler.ls:2366
  -> (vector-new 0)
  -> standalone-preview1-capability-boundary-message + exit-compile-error   Cli.ls:882-883
```

という「emit の**前**に走査し、境界で名前付きメッセージを出す」形をとっている
(`I-75` の予測がこの形をそのまま当てた。`/Users/biwakonbu/github/tmp/i75/prediction.md`)。
既定 mode 側にこれの対応物を置けば、3 本の書き換え無し builder すべてが同じ扱いになる。

**新しい emitter channel を作らない。** emitter は byte vector を返すだけで、
そこから診断を出す経路は無い。だから emit の途中で止めるのではなく、
emit の前に走査する既存パターンに寄せる。

### 案 (iii) 現状維持

**却下する。** message-less trap は **4 日間「未診断の算術バグ」として扱われた**
(`I-78` が `I-75` から分離されたのが 2026-08-27、診断確定が 2026-08-28)。
`decisions-selfhost-typeinfer-stub-override.md` 決定 4 が「message-less trap を新設しない」と
定めた根拠そのものが、この 1 件で実証された形になっている。放置は次の誤診を約束する。

同じ根から出る 2 件目の危険もある: opcode 91 (`read-stdin`) は既定 mode では
**trap されずに素通りする**ので、`command-line-args` に化けたまま無言で意味が変わる
(`ISSUES.md` `I-78` の「同じ根から出る、本件のスコープ外の危険 2 件」)。

## 決定

**案 (ii) を採る。**

1. 既定 mode 用の unsupported-opcode 走査を足す。standalone 側と同じく Compiler 層に置き、
   emit の前に走らせる
2. `reject-native-only-wasm-opcode` の `(/ opcode 0)` は**この slice では消さない**
   (下記「追補 1」で訂正した。当初は消すと書いていた)。backstop として残し、
   削除は「書き換えを持たない build entry がすべて走査を持つ」ことを満たした後の
   follow-up 受入にする
3. 診断メッセージは既存の `standalone-preview1-capability-boundary-message` とは別立てにする。
   「standalone では対応しているが既定 mode では未対応」という別の境界だからである

## 含めない範囲

- **既定 mode に `command-line-args` を実装すること。** 11 import に 12 本目を足す判断であり、
  Rust host 側 (`crates/lsharp-wasm`) の runtime 契約も動く。別 slice にする
- **opcode 40 の operand 調整 `(+ operand 11)` の妥当性。** 案 (i) の却下理由として触れたが、
  standalone 経路での正しさは検証していない
- **`Cli` を fixed-input-set 54 target から外すこと。** 本 ADR は境界の伝え方を決めるだけで、
  受入集合は動かさない
- **`read-stdin` (opcode 91) の素通り。** 同じ走査で拾える見込みだが、
  受入条件は本 ADR では立てない (`ISSUES.md` `I-78` に残す)

## 追補 (2026-08-28、決定を書いた直後の検算で判明した 2 件)

**決定を書いた時点では数えていなかったことが 2 つあり、いずれも決定 1〜3 の文言を変える。**
訂正の経緯を残すため、元の文言を消さずにここに書く。

### 追補 1: 書き換えを持たない build entry は 1 つではなく 4 つある

決定 2 は「走査が先に止めるので到達不能」と書いたが、**走査が守るのは既定 mode だけ**である。
`(/ opcode 0)` を持つ拒否末尾は 4 builder 共通なので、削除すると残り 3 経路が
raw 86 を**無言で emit する**。これは `ISSUES.md` `I-78` の「スコープ外の危険」に
自分で書いた raw 91 素通りと同じ形を、こちらの手で作ることになる。

| 呼び出し元 | builder | 決定 1 の走査が守るか |
|---|---|---|
| `App/CompilerMode.ls:6214` (`compile-file-mode` = 既定 mode) | `build-wasm-bytes-wasi` | **守る** |
| `App/CompilerMode.ls:6246` (`compile-file-mode-build-progress-debug`、`Main.ls:41` から到達) | `build-wasm-bytes-wasi-progress-debug` | 守らない |
| `App/PipelineSmoke.ls:42` | `build-wasm-bytes-wasi` | 守らない |
| `App/PipelineSmoke.ls:94` | `build-wasm-bytes-wasi` | 守らない |

したがって決定 2 を「消す」から「**残す**」へ変えた。削除を諦めたのではなく、
**削除の前提条件 (4 entry すべてが走査を持つ) を受入条件に格上げした**ということである。
backstop を残す限り、走査の取りこぼしは trap という気付ける形で出る。

### 追補 2: selfhost CLI に stderr 経路は無い。`cli-stderr` は stdout に書く

受入条件に「stderr に出て非 0 終了」と書いたが、**この 2 語はどちらも裏が取れていなかった**。

- `cli-stderr` (`App/Cli.ls:2291`) の実体は `(print-string (string-concat "error: " msg))` で、
  **stdout に書く**。名前が実態と食い違っている。
  実測でも裏付けがある -- `I-75` の harvest で `--target wasi-component` を渡した 3 件は
  `stdout="error: wasi-component output requires external component packaging\n"` を返した
- `cli-stderr` / `exit-compile-error` は `App/Cli.ls` / `App/EmbeddedCli.ls` / `App/SmokeCli.ls`
  にしか定義が無い。**`App/CompilerMode.ls` は `App.Cli` を import していない**
  (依存方向は Cli -> CompilerMode の一方向)。逆流はできないので、
  `compile-file-mode` から既存の診断 helper は呼べない
- `compile-file-mode` (`:6197-6224`) は exit を持たず、`print-wasm-module` の返り値を
  そのまま返すだけである。**非 0 終了がこの経路で成立するかは未確認**

よって決定 3 は「別立ての診断メッセージを用意する」だけでは足りず、
**それをどの層から、どの stream へ出すか**を実装 slice で決める必要がある。
既定 mode は wasm binary を stdout へ流す経路なので、診断も同じ stream に載る。
回収側 (`parse_emitted_wasm_modules`) が stdout を読む以上、
「wasm が 1 件も取れず、代わりに診断行が読める」が観測可能な形になる。

### 追補 3: gate 文書の記述

`docs/development/planning/completion-criteria.md:27,123` と
`docs/development/planning/phase11-implementation-plan.md:137` は赤 3 件に
`[赤: I-78]` 注記を持っており (`I-104` の是正済み)、本 ADR で追加の対応は要らない。
ただし `phase11-implementation-plan.md:142` の Acceptance 行だけは注記が無く、
6 本を並べて「pass する」と書いている。**本 ADR を倒しても 3 本は緑にならない**ので、
この行は実装 slice の doc-GREEN で注記を足す対象として引き継ぐ。

## Evidence

(実装後に埋める)

**この slice はまだ 1 行も実装していない。** cargo が要り、再計測 lane は
`selfhost_bootstrap_acceptance` である。**現在飛んでいる `SWEEP-LANE-RERUN-01` の 3 module には
含まれないので、あの lane の完走では本件は緑にならない。**

受入条件は次のとおり:

- RED: 既定 mode で `src/App/Cli.ls` を compile したとき、**trap ではなく**
  診断行が **stdout** に出て、wasm module が 1 件も回収できない形になること。
  **stderr ではない** -- 追補 2 のとおり selfhost CLI に stderr 経路は無い
- GREEN: `selfhost_bootstrap_acceptance` の赤 3 件
  (`..._stage_chain_match_cli_module` / `..._stage_chain_match` / `..._stage2_self_feed_fixed_input_set`)
  の失敗メッセージが `wasm trap` から診断文字列へ変わること。
  **緑になるとは限らない** -- `Cli` が compile できないこと自体は案 (ii) では解消しないためである。
  ここを緑にするには含めない範囲の 1 件目が要る

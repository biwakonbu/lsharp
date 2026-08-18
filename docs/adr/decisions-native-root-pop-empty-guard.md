# ADR: aarch64 の `root_pop` に空 stack ガードを inline で入れる

- Status: Accepted (verified slice)
- Date: 2026-08-18
- Scope: `NATIVE-ROOT-01` / `I-21` / `selfhost/src/Backend/Native/NativeCodegen.ls`
  (対象は aarch64 lane のみ。x86-64 lane の root API 実装そのものは含めない)
- Related: [`ISSUES.md` I-21](../../ISSUES.md#i-21)、
  [`decisions-runtime-spec-root-api-contract.md`](decisions-runtime-spec-root-api-contract.md)、
  [runtime spec](../language/runtime-spec.md)、[native backend spec](../language/native-backend-spec.md)

## Context

[root API 契約 ADR](decisions-runtime-spec-root-api-contract.md) の tier 1 項目 3 は
「**空の root stack に対する `root_pop` は trap せず、root stack を変更せずに `0` を返す**」を
全 backend 必須と定めた。wasm backend は `crates/lsharp-wasm/src/wasi/root.rs` の
`emit_root_pop_func` でこれを満たしており、e2e
`test_e2e_root_runtime_api_tracks_slots_and_values` が `0` を pin している。

aarch64 native backend は満たしていない。`emit-root-pop-aarch64` は空判定を持たず、
無条件に以下 3 命令 (12 byte) を出す。

```
mov x9, x0          ; 値窓のシフト
sub x27, x27, #8    ; root stack pointer を無条件に下げる
ldr x0, [x27]
```

`x27` = root stack pointer、`x28` = root stack base。空 (`x27 == x28`) のときに呼ぶと
`x27` が base を下回り、**root stack 領域の手前を読む**。さらに `x27` が下がったままになるので、
以降の `root_push` が返す slot index が負値になり、書き込み先も 1 slot ずれる。
契約の「trap しない」だけを満たし、「stack を変更しない」「`0` を返す」の 2 つを破っている。

native lane にはまだ GC が無いため実害が顕在化していない (`I-13`)。GC を載せた時点で
「root stack の手前にある値を live root として辿る」形で顕在化するので、GC より先に閉じる。

## Decision

**`emit-root-pop-aarch64` に空判定の分岐を inline で埋め込む。** 出力は 6 命令 24 byte になる。

```
mov  x9, x0         ; +0   値窓のシフト (従来どおり)
mov  x0, #0         ; +4   空だったときの返り値をあらかじめ置く
cmp  x27, x28       ; +8   x27 == x28 なら空
b.eq +12            ; +12  分岐命令自身から 12 byte 先 = +24 (末尾) へ
sub  x27, x27, #8   ; +16
ldr  x0, [x27]      ; +20
                    ; +24  末尾
```

空のときは `x27` に触れず `x0 = 0` のまま抜ける。非空のときは従来と同じ 2 命令を実行する。
`x9` は両経路で同じ値を持つので、`drop` (opcode 44) による値窓の復元は従来どおり動く。

### サイズ表を同時に更新する

`NativeCodegen.ls` は「バイトを出す前に offset を確定させる」ため、命令長を**独立した 2 つの
size 関数**が持っている。emitter だけ伸ばすと、以降の全 branch 変位が 12 byte ずれてモジュール全体が壊れる。
opcode 75 (`root_pop`) の定数を **12 → 24** へ両方同時に直す。

| 関数 | 現在 | 変更後 |
|---|---|---|
| `native-plain-instr-size-aarch64` (plain lane、offset 計算 2 箇所から呼ばれる) | `12` | `24` |
| `native-instr-size-aarch64-core` (bundle lane、`native-produce-one-size-aarch64` 経由) | `(native-produce-one-size-aarch64 12 current-depth)` | `(native-produce-one-size-aarch64 24 current-depth)` |

`direct-append-produce-one-bytes-aarch64` (opcode 75 の fast path) は `emit-root-pop-aarch64` を
そのまま呼ぶので、emitter を直せば追随する。サイズは上表の 2 関数から取るため専用の更新は要らない。

**この 24 という数字を ADR のコメントで担保しない。** test で
`(native-plain-instr-size-aarch64 75 0)` と `(vector-length (emit-root-pop-aarch64))` の一致を
検査し、算術ではなく実測で pin する。

### 新規に足す emitter

`cmp x27, x28` (= `SUBS xzr, x27, x28`) の emitter が無いので追加する。
エンコードは `0xEB1C037F` = `3944481663`。既存の `emit-aarch64-sub-x0-x27-x28`
(`SUB x0, x27, x28` = `0xCB1C0360`) と同じ shifted-register 形で、`op`/`S` bit と `Rd = xzr` だけが違う。
`mov x0, #0` は既存の `emit-aarch64-movz-x0-shift` に `imm=0 hw=0` を渡して得る。

## 却下した選択肢

**案 A — out-of-line の `lsharp_root_pop` helper を呼ぶ。却下。**
呼び出し 1 箇所あたり 4〜8 byte で済み、ガードの本体は 1 箇所に集まる。
しかし native backend には `lsharp_root_push` / `lsharp_root_pop` / `lsharp_root_set` の
シンボルが**存在しない**。opcode 74/75/76 は一貫して呼び出し側へ inline 展開される設計で、
契約 ADR も ABI シンボルを規定しないと明示している。helper を導入すると
「root API だけ ABI を持つ」例外を作ることになり、linker / stage0 パッケージング /
transport の各所に波及する。ガード 3 命令のために払う代償として大きすぎる。

**案 B — `csel` による branchless 版。却下。**
`sub x10, x27, #8` / `cmp x27, x28` / `csel x27, x10, x27, ne` / `ldr x0, [x27]` /
`csel x0, x0, xzr, ne` のような形。分岐が無いぶん綺麗に見えるが、
(1) `csel` の emitter が 2 種類必要で新規エンコードが 3 つ増える、
(2) **空のときも `[x28]` を読む**ので「読んではいけない領域を読まない」という当初の目的を
半分しか達成しない (base 自体は有効領域なので安全ではあるが、GC を載せた後に
「空なのに base slot を触る」経路が残るのは説明しにくい)、
(3) それでいて長さは同じ 24 byte。利点が無い。

**案 C — 12 byte を維持したまま既存 3 命令を組み替える。却下。**
比較と分岐で最低 2 命令、空経路の返り値 `0` で 1 命令が要る。`mov x9, x0` を落とせば
辻褄は合うが、`x9` は `drop` が値窓を復元するために読む。落とすと opcode 44 の意味が壊れる。
12 byte 維持は不可能である。

**案 D — x86-64 lane も同時に直す。却下 (今回のスコープ外)。**
x86-64 の `root_push` は常に `0` を返す stub、`root_pop` は emitter が無く、
`root_set` は store を出さない。3 つとも未実装であり、ガード 1 個を足す話ではなく
lane 全体の実装になる。`I-21` の本文と `TODO.md` の `NATIVE-ROOT-01` で
「含めない範囲」として明示済み。なお現状の x86-64 `root_pop` は空でも `0` を返すので、
tier 1 項目 3 に限れば偶然満たしている。

## 受入条件

1. aarch64 の `root_pop` が空 stack で `x27` を動かさず `0` を返すこと。
2. 上を **encoding** と **実行** の両方で検査する test を置くこと。
3. 命令長とサイズ表の一致を test が pin すること (算術での担保にしない)。
4. wasm backend の既存 pin (`test_e2e_root_runtime_api_tracks_slots_and_values`) が
   引き続き通り、backend を跨いで同じ観測結果になること。

## Evidence

計測日 2026-08-18、`macOS 25.5.0 / aarch64` (`host_native_exec_supported()` が true の環境)。
worktree `codex/native-root-01`、`main` `8475b00a` から分岐。

### 置いた test

| test | lane | 検査対象 |
|---|---|---|
| `e2e::selfhost_native_stage_chain::test_e2e_native_aarch64_root_pop_emits_empty_stack_guard` | wasm 上の selfhost harness | 実バイト列 (encoding) と size 表の一致 |
| `e2e::selfhost_native_stage_chain::test_e2e_native_host_binary_selfhost_root_pop_on_empty_stack_keeps_stack_pointer` | host aarch64 binary をリンクして実行 | 空 pop 後の `root_push` が slot index `0` を返すこと |

どちらも `#[ignore]` で、既存の prefix `test_e2e_native_aarch64_` /
`test_e2e_native_host_binary_` に属するため
`scripts/ci/compile-phase11-inputs.sh` の `--ignored` lane から自動的に走る。
`test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` の prefix ルールも新規登録なしで満たす。

### RED (実装前)

encoding test:

```
left:  [233, 3, 0, 170, 123, 35, 0, 209, 96, 3, 64, 249]                       (12 byte)
right: [233, 3, 0, 170, 0, 0, 128, 210, 127, 3, 28, 235, 96, 0, 0, 84,
        123, 35, 0, 209, 96, 3, 64, 249]                                       (24 byte)
```

behavioral test: `exit code 7 を期待したが -1 を得た` (56 byte の host binary が異常終了)。
**`I-21` が机上の非適合ではなく実際にプロセスを落とすことを、ここで初めて実測で示した。**
`ISSUES.md` の `I-21` は「bss 領域の手前を読む」までしか書いておらず、
落ちるところまでは確認されていなかった。

### GREEN (実装後)

両 test とも `ok`。behavioral test は exit code `7` を返す
(= 空 pop が `x27` を動かさず、続く `root_push` の slot index が `0`)。

### 受入条件の判定

| # | 条件 | 判定 |
|---|---|---|
| 1 | 空 stack で `x27` を動かさず `0` を返す | 満たす (encoding + 実行の両方) |
| 2 | encoding と実行の両方を検査する test | 満たす (上表 2 件) |
| 3 | 命令長と size 表の一致を test が pin | 満たす (encoding test が `native-plain-instr-size-aarch64` と実バイト長を比較) |
| 4 | wasm 側の既存 pin が通り、backend を跨いで同じ観測結果 | 満たす (下記 3 本を実測) |

条件 3 について補足する。当初 24 byte という数字を ADR のコメントだけで担保しかけたが、
**5 命令 20 byte と数え違えていた**。size 表に 20 を入れていれば `root_pop` 1 個につき
4 byte ずつ以降の branch 変位がずれ、モジュール全体が壊れていた。
test で pin する方針にしたのはこの数え違いが理由である。

条件 4 は 3 本を実測した (いずれも 2026-08-18、本 worktree)。

| 検査 | 結果 |
|---|---|
| `e2e::runtime_allocator_closures::test_e2e_root_runtime_api_tracks_slots_and_values` (wasm backend の pin) | `ok` |
| `scripts/ci/test-selfhost-rooting-guards.sh` (rooting parity 13 件) | 13/13 `ok`、exit 0 |
| `e2e::selfhost_lsp_docs_ops::test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` | FAIL。ただし offender は **164 件で本変更前と同数**、うち本変更由来は **0 件** |

ops03c の FAIL は `TESTGATE-03` / `I-22` として既に台帳にある既存の非適合であり、
本変更が増やしたものではない。判定は offender リストを実測して行った
(`grep -c 'root_pop\|empty_stack'` が 0)。ここを「pass しないから未検証」と
書かずに済ませたのは、**新規 offender 0 という条件のほうが検査したい内容だから**である。

wasm backend 側の実装 (`crates/lsharp-wasm/src/wasi/root.rs`) は本変更で無変更。

### 回帰確認 — 満たせなかった点

`selfhost_native_stage_chain` の `--ignored` lane 全体を実行中。
**分母は 614 test** (runner の `running 614 tests` 行で実測)。当初 534 と書いたのは
13 個の test 名 prefix を grep で足し上げた暫定値で、`test_e2e_linux_x86_actual_*` など
prefix 表に無い test を取りこぼしていた。**完走後の全件結果は本節へ追記する。**

この lane には **本変更と無関係な恒常 FAIL が既に存在する**ことが判明した。

| 分類 | 件数 | 扱い |
|---|---|---|
| `test_e2e_linux_x86_actual_*` / `test_e2e_native_linux_x86_*` / Lima VM 依存 | 環境要因 | 本変更と無関係 (下記) |
| `LSHARP_NATIVE_*` env 依存 (`test_e2e_saved_native_stage3_*` / `test_e2e_native_macos_aarch64_*`) | 環境要因 | 本変更と無関係 (下記) |
| `test_e2e_native_aarch64_bundle_initial_capacity_includes_full_helper_trailer` | 1 | **2026-08-03 `1ee26eef` から陳腐化した pin**。`I-23` として新規登録 |

環境要因 2 種の内訳を実測で確定させた (2026-08-18)。

- **Lima 依存**: `limactl list` は VM `lsharp-linux-x86` (x86_64 / 4 CPU / 16 GiB) が
  **`Stopped`** であることを示す。すなわち「macOS host では原理的に実行できない」のではなく
  「VM を起動していない」が正しい失敗理由である。当初この行に
  「macOS host では実行できない」と書いたのは不正確だったので訂正した。
  なお sweep 中に VM を起動すると 4 CPU を奪って計測を歪めるため、本 sweep では起動していない。
  `test_e2e_native_host_binary_bundle_if_result_preserves_outer_for_add` は名前が
  `host_binary` だが本体は `link_and_run_linux_x86_native_binary_via_lima` (`:44252`) を呼ぶので
  この分類に入る。
- **env 依存**: `test_e2e_saved_native_stage3_*` は
  `LSHARP_NATIVE_MACOS_AARCH64_STAGE3_COMPILER` を `.expect()` で要求し (`:56242-56243`)、
  `test_e2e_native_macos_aarch64_actual_app_cli_release_program` は
  `LSHARP_NATIVE_MACOS_AARCH64_APP_CLI_ARTIFACT_DIR` (`:56136`)、
  `..._fixedpoint_compiler_exports_linux_x86_*` は
  `LSHARP_NATIVE_MACOS_AARCH64_CROSS_LINUX_X86_APP_CLI_ARTIFACT_DIR` (`:56367`) を要求する。
  いずれも未設定なら panic するので、**assertion ではなく前提の欠落**である。
  これは `I-11` が「正しい環境前提は `./stage0` 不在ではなく `LSHARP_NATIVE_*` が全て未設定」と
  訂正した内容と同じ構図である。

**分類規則**: 以上により、本 sweep の FAIL は機械的に 3 分類できる。
(a) Lima VM 依存、(b) `LSHARP_NATIVE_*` env 依存、(c) それ以外。
回帰の候補になり得るのは (c) だけである。

**分類は test 名の prefix ではなく関数本体で行う。** 途中経過を prefix で数えたとき
`test_e2e_native_linux_x86_*` を一括で (a) に入れたが、これは誤りだった。この prefix の
大半は Lima ではなく `LSHARP_NATIVE_*` env を要求する (b) であり、同じ prefix の中に
ok と FAIL が混在する事実と整合しない。`fn <name>(` から次の `fn` 直前までを本体として
切り出し、本体中の `lima` / `LSHARP_NATIVE_` の有無で判定し直した。

その結果 (2026-08-18、途中経過 65 件時点):

| 分類 | 件数 | test |
|---|---|---|
| (a) Lima 依存 | 1 | `test_e2e_native_host_binary_bundle_if_result_preserves_outer_for_add` |
| (b) env 依存 | 61 | `test_e2e_native_linux_x86_host_generates_*` ほか |
| (c) それ以外 | 3 | 下記 |

(c) の 3 件は `bundle_initial_capacity` (`I-23`) に加え、
`test_e2e_selfhost_main_linux_x86_actual_seed_entry_call_offsets_diagnostic` と
`test_e2e_selfhost_main_linux_x86_actual_seed_function_size_matches_generated_length_diagnostic`。
**当初「(c) は `bundle_initial_capacity` のみ」と書いたのは prefix 分類による誤りで、訂正した。**

後者 2 件が本変更と無関係であることは差分の到達範囲で言える。本変更が触った size table は
`native-plain-instr-size-aarch64` (`:12373`) と `native-instr-size-aarch64-core` (`:18017`) の
2 つだけで、どちらも aarch64 専用である。この 2 件が print するのは
`native-instr-size-x86` / `native-function-size-x86` / `collect-native-bundle-offsets-x86` の値で、
x86 側の size 経路には本変更の差分が 1 byte も入っていない。
**ただしこれは到達不能性の議論であって、失敗理由そのものではない。**
panic message は libtest が run 完了時にまとめて出すため、完走後に実測で確定させる。

`bundle_initial_capacity` が本変更と無関係であることは**実測と算術の両方**で確定している。

- 実測: 単体で走らせると panic は `assert_eq!` 側で、`left: [3492, 4492]` /
  `right: [2520, 3520]`。`.expect(...)` (= 環境要因) ではない。
- 算術: 期待値 `2520` に対し `read-stdin helper offset` が既に `3296`、
  `helper trailer size = base + 3296 + 156` なので、opcode 75 の命令長に関わらず
  `2520` は再現できない。

**この 2 つを分けて書くのは、`I-23` が「実測していない数字を台帳に残すな」という
指摘そのものだからである。** 算術だけで済ませると同じ穴を作る。

### lane ごとの witness

サイズ表と実生成長の一致は、lane を跨いで名前のついた test で担保されている。

| lane | witness | 結果 |
|---|---|---|
| plain (emitter 単体) | `test_e2e_native_aarch64_root_pop_emits_empty_stack_guard` | `ok` |
| direct-append (深い depth) | `test_e2e_native_aarch64_deep_direct_append_matches_static_size` | `ok` |
| bundle (浅い depth・実バイナリ実行) | `test_e2e_native_host_binary_selfhost_root_pop_drop_restores_previous_value` / `test_e2e_native_host_binary_selfhost_root_pop_on_empty_stack_keeps_stack_pointer` / `test_e2e_native_host_binary_selfhost_root_set_drop_restores_bottom_value` | 3/3 `ok` |

当初この節は deep_direct_append が「direct-append lane と bundle lane の両方を通る」と
書いていたが、**誤り**。depth が閾値を超えた時点で direct-append 側に入るので、
あの test は bundle lane の witness にならない。bundle lane を通るのは上表 3 件である。

### stage0 の再生成が要らない理由

pin 済みの stage0 バイナリは本変更で触れていない。それでよい理由を残す。

- stage0 (旧・ガード無し) が**新しい**ソースをコンパイルすると、出力される stage1 には
  ガードが入る。ガードはソース側に書かれており、コンパイラのバージョンではなく
  入力に従うため。
- stage2 / stage3 はどちらも「ガード入りコンパイラ」の出力なので、
  self-regeneration の zero-diff 条件は保たれる。

つまり pin を更新しないことが、ガードが行き渡らない原因にはならない。

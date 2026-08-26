# ADR: selfhost compiler の空 `do` を unit 値 (`i64.const 0`) として emit する (2026-08-27)

- **Status**: accepted
- **Date**: 2026-08-27
- **Scope**: `selfhost/src/Backend/Wasm/Compiler.ls` の do 三経路
  (tag 9 dispatch / `compile-do-with-source` / `compile-do-with-source-normal-setup-diagnostic`)
- **Related**: `ISSUES.md` `I-71` / `I-77` / `TODO.md` `STAGE-WASM-TRANSLATE-01` /
  `crates/lsharp-ir/src/lower/expr/do_expr.rs` (Rust host の参照実装)

## Context

`--ignored` 全量 sweep (2026-08-24) の最大収量として、stage-N 生成 Wasm が
`Invalid input WebAssembly code at offset N: type mismatch: expected i64 but nothing on stack`
で wasmtime に蹴られる赤が 72 件挙がった (`I-71`)。distinct な offset は
329391 / 457947 / 310805 の 3 つだけである。

`I-71` の起票時点の見立ては「ftable / function index の誤解決」だった。
同一メッセージの先行事例が 2 件あり、どちらも index の誤解決だったためである
(`selfhost_standalone_io` offset 2456、`EC-M1-01` offset 2929)。
**この見立ては誤りだった。**

原因は do の空ブロックにある。selfhost compiler は `(do)` (expr-count = 0) を
「何も emit しない」で扱う。`(if cond (do ...) (do))` は blockty = i64 の `if` を作るので、
else 腕が空だと値が積まれず、`end` の時点で型検査に落ちる。

Rust host の参照実装は同じ入力に対して unit を積む:

```rust
// crates/lsharp-ir/src/lower/expr/do_expr.rs
if exprs.is_empty() {
    ctx.emit(Instruction::I64Const(0)); // unit
```

つまり **selfhost 側だけが host の契約から外れていた**。

do の compile 経路は `Compiler.ls` に 3 つある。tag 9 dispatch (`:1249`) は
expr-count の guard を**そもそも持たない**。`compile-do-with-source` (`:1353`) と
`compile-do-with-source-normal-setup-diagnostic` (`:1436`) は
`(if (= expr-count 0) instrs ...)` と書いており、instrs を素通しする。
3 つとも同じ欠落である。

## Decision

3 経路とも空 do で `(emit-to instrs 1 0)` (opcode 1 = `(op-i64-const)`、
`CompilerBase.ls:14`) を emit する。Rust host の `do_expr.rs` と同じ契約に揃える。

## 却下した選択肢

- **selfhost source から `(do)` を消す。** `selfhost/src/**` の空 do を書き換えれば
  sweep は緑になる。**却下**: compiler のバグは残ったまま、L# のユーザーが書いた
  `(do)` で同じ不正 Wasm が出る。緑になることと直っていることは別である。
- **空 do をパースエラーにする。** 契約変更であり `docs/language/` の更新が要る。
  **却下**: Rust host は空 do を受理して unit を返す。selfhost 側だけ拒否すると
  host/selfhost の差分が増える。差分を減らすのが本 slice の目的である。
- **`compile-do-exprs-step` の `(>= idx expr-count)` 分岐で emit する。**
  1 箇所で済むので魅力的に見える。**却下**: この分岐は空でない do の
  ループ脱出でも通るため、全ての do に余分な定数が積まれる。
- **`validate_wasm_detailed` を直してから fix する。** 検証の穴 (`I-77`) は実在するが、
  付け替えると `#[ignore]` 下の多数の test の verdict が一斉に変わる。
  **却下**: slice が混ざる。本 slice では新規 test 用に
  `support.rs` へ `validate_wasm_function_bodies` を足すに留め、既存呼び出し箇所は触らない。

## Evidence

計測は throwaway harness `/Users/biwakonbu/github/tmp/i71/` (stage1 を cargo で 1 度だけ生成して
キャッシュし、stage1 を wasmtime で走らせて stage2 を吐かせる) で取得した。
関数本体の検証は `src/bin/valid.rs` — `ValidPayload::Func` を捨てずに
`into_validator().validate(&body)` まで駆動する専用 validator を使う
(`I-77` のとおり、e2e 既存の `validate_wasm_detailed` はこれをしないので何も見えない)。

### 修正前 (`before.log`)

| 対象 | サイズ | code section 内の関数本体 | 壊れている関数 |
|---|---|---|---|
| `src/App/Main.ls` stage2 | 1,575,442 B | 4,147 | **2** |
| `src/App/CompilerMode.ls` stage2 | 573,217 B | 1,943 | **2** |

```
=== validate Main.ls stage2 ===
NG func[1231] body=[329285..329750] err@329391 (0x506af): type mismatch: expected i64 but nothing on stack
NG func[1623] body=[457475..458325] err@457947 (0x6fcdb): type mismatch: expected i64 but nothing on stack
=== validate CompilerMode.ls stage2 ===
NG func[1231] body=[310699..311164] err@310805 (0x4be15): type mismatch: expected i64 but nothing on stack
NG func[1623] body=[438889..439739] err@439361 (0x6b441): type mismatch: expected i64 but nothing on stack
```

**これが 3 offset の正体である。** sweep が挙げた 329391 / 457947 は `Main.ls` stage2 の
`func[1231]` / `func[1623]`、310805 は `CompilerMode.ls` stage2 の `func[1231]`。
**同じ 2 関数が、大きさの違う 2 モジュールの別の絶対位置に現れていただけ**で、
offset の数だけ独立した機構があったわけではない。

- sweep が挙げなかった 4 つ目 (439361 = `CompilerMode.ls` の `func[1623]`) が存在する。
  offset の集合は原因の集合ではないという反証がここにある。
- 起票時の「ftable / function index の誤解決」という見立ては**誤りだった**と確定した。
  この行は `I-71` 本文にも残す。訂正を消すと同じ見立てが再発する。

どの経路が実際に踏まれているかは marker 実験で特定した。3 箇所へ区別可能な定数
(111 / 222 / 333) を仮に置いて stage2 を逆アセンブルすると `else i64.const 222` が出る:

```wat
(func (;11;) (type 5) (param i64) (result i64)
  local.get 0
  i64.const 0
  i64.gt_s
  ...
  if (result i64)
    i64.const 1
  else
    i64.const 222   ;; = compile-do-with-source (:1353)
  end)
```

compiler モードは `compile-do-with-source` を通る。tag 9 dispatch (`:1249`) は
stage1 harness 経路で踏まれる。**2 経路それぞれに独立した test を立てた**のはこのためで、
1 本の緑では他方の経路を検査したことにならない。

### 修正後 (`after.log`)

| 対象 | サイズ | 関数本体 | 壊れている関数 | `wasm-tools validate` |
|---|---|---|---|---|
| `src/App/Main.ls` stage2 | 1,575,570 B (+128) | 4,147 | **0** | OK |
| `src/App/CompilerMode.ls` stage2 | 573,345 B (+128) | 1,943 | **0** | OK |

+128 B は追加した `i64.const 0` の分に一致する。

### test

`crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/part_017.rs` (新規、`#[ignore]` 2 件):

- `test_e2e_bootstrap_stage1_emits_valid_stage2_wasm_for_empty_do_block` — tag 9 dispatch 経路
- `test_e2e_bootstrap_compiler_mode_emits_valid_stage2_wasm_for_empty_do_block` — `compile-do-with-source` 経路

RED (fix 前) はどちらも本体検証で落ちる:

```
func[10] body=[247..262] err@260 (0x104): type mismatch: expected i64 but nothing on stack
func[11] body=[275..290] err@288 (0x120): type mismatch: expected i64 but nothing on stack
test result: FAILED. 0 passed; 2 failed; 3083 filtered out; finished in 230.97s
```

GREEN (fix 後):

```
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3083 filtered out; finished in 221.64s
```

検証には新設の `support::validate_wasm_function_bodies` を使う。
既存の `assert_valid_wasm` (magic byte と長さしか見ない) と
`validate_wasm_detailed` (`ValidPayload::Func` を捨てる) は、
**どちらもこの不正 Wasm を緑と報告する**。RED が立たない helper で書いた test は
fix の証拠にならないので、helper の追加自体が本 slice の必須部分である。

### 受入条件の判定

`TODO.md` `STAGE-WASM-TRANSLATE-01` の受入条件との対応:

- 「先に RED を立てる」— 満たす (`red.log`)
- 「3 offset それぞれについて、どの命令列がどの関数を呼ぼうとして stack を空にしているか特定」—
  **文言どおりには満たしていない**。特定の結果、**3 offset は独立した 3 事象ではなく
  2 関数 x 2 モジュールの重複だった**ため、「offset ごとの原因」は成立しない。
  代わりに (a) 各 offset がどのモジュールのどの func index かを表で確定させ、
  (b) sweep 未検出の 4 つ目を挙げ、(c) 経路が 2 つあることを marker 実験で示し、
  (d) 経路ごとに独立した test を立てた。意図 (単一サンプルで閉じない) は満たしている。
- 「1 本だけ検証して閉じない」— 満たす。2 経路 x 2 モジュールで検証した。
- 「GREEN 後 `ignored-lane-expected-failures.txt` の該当 72 行を削除」— **満たしていない。
  削除できる行が 1 行も無かった。** 下の再測定節が根拠。条件を緩めたのではなく、
  条件が立っていた前提 (「1 つの原因が 1 つの赤に対応する」) が誤りだった。

### fix 後の再測定 (2026-08-27) -- 赤は 1 件も減らなかった

`I-71` の 72 行を含む 3 module を同条件で測り直した。

| 項目 | 値 |
|---|---|
| 対象 | `runtime_allocator_closures` / `selfhost_bootstrap_acceptance` / `selfhost_bootstrap_four_layer` |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` (本 slice の fix 込み。lane 中は再ビルドしない) |
| 起動 | `python3 /Users/biwakonbu/github/tmp/i71/run_lane_i71.py` を `os.setsid()` で切り離し |
| ログ | `/Users/biwakonbu/github/tmp/i71/lane/mod-<module>.log` |
| 併走 | 無し (lane 中は `cargo` を一切起動しない) |

| module | 宣言 | 結果行 | 完走 | 赤 | 所要 |
|---|---:|---:|---|---:|---:|
| `runtime_allocator_closures` | 4 | 4 | OK | 4 | 233.41s |
| `selfhost_bootstrap_acceptance` | 28 | 28 | OK | 7 | 1,818.75s |
| `selfhost_bootstrap_four_layer` | 148 | 148 | OK | 77 | 5,582.32s |
| **計** | **180** | **180** | OK | **88** | **7,634s** |

`scripts/compare_ignored_lane.py <3 logs> --ledger <3 module 分の抜粋>` は
**新規 FAIL 0 / 解消 0 / 未出現 0 / exit 0**。
**部分 lane は必ず抜粋台帳と比べる。** 全量台帳で比べると、測っていない 7 module の
行が全て「未出現」になって非 0 になる。

症状を fix 前後で test 単位に突き合わせた結果:

| 旧症状 (2026-08-24) | 新症状 (2026-08-27) | 件数 |
|---|---|---:|
| `expected i64 but nothing on stack` | `expected 11 imports, found 10` | 74 |
| `expected 11 imports, found 10` | 同左 | 8 |
| その他 | `expected 11 imports, found 10` | 2 |
| `integer divide by zero` | 同左 | 1 |
| `expected i64` + `divide by zero` | `imports` + `divide by zero` | 1 |
| その他 | 同左 | 2 |

- **本 ADR が直した症状の出現は 3 module で 0 件**になった。fix は効いている。
- **赤の集合は 88 件のまま 1 件も動かなかった。** 症状が下の層へ移っただけである。
- **`I-71` は `I-72` を隠していた。** 72 件は `I-71` の帰結ではなく、
  `I-71` が先に当たる壁だっただけだった。`I-72` の赤は 8 件 → 82 件になった。
- 台帳の 72 行は**削除せず**、引き取り先を `I-71` → `I-72` へ付け替えた。
  実測が赤の行を消すのは台帳を壊す操作である
  (`compare_ignored_lane.py` が新規 FAIL 72 件で非 0 になる)。
- `I-75` の未分類 3 行に原因が付いたので、2 行を `I-72`、1 行を新設の `I-78`
  (`src/App/Cli.ls` の self-feed で `integer divide by zero` trap) へ移管した。
  この trap は 2026-08-24 の sweep log にも同数 (2) 出ており、本 fix の regression ではない。

**ここから引ける一般則**: 症状が消えたことは、その症状が塞いでいた test が
通るようになったことを意味しない。**「何件の赤が消えたか」は fix の効果の指標にならない。**
効果は「その症状の出現数が 0 になったか」で測り、台帳は「実測の赤と一致するか」で保つ。
2 つを混ぜると、直っているのに直っていないように見えるか、逆に台帳を壊すかのどちらかになる。


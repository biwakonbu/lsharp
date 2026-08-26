# ADR: selfhost compiler の 11-import ABI を正とし、harness の 10-import 経路を廃止する (2026-08-27)

- **Status**: accepted
- **Date**: 2026-08-27
- **Scope**: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/part_002.rs` の
  `run_wasm_with*_compiler_mode*` helper 群と、その全呼び出し箇所
- **Related**: `ISSUES.md` `I-72` / `I-79` / `TODO.md` `STAGE-WASM-IMPORT-COUNT-01` /
  `docs/adr/decisions-selfhost-empty-do-unit-value.md` (`I-71`。本件を隠していた層) /
  commit `b73938ea` (11-import ABI を導入した変更)

## Context

`I-71` (空 `do` が値を積まない) を直したところ、そこで止まっていた 74 件が次の壁まで進み、
`インスタンス化に失敗: expected 11 imports, found 10` の赤が 8 件から **82 件**になった。
数値は全件 `11` / `10` で完全に一致する。

`I-71` の教訓 —— 「**offset の集合は原因の集合ではない**」「**症状が消えたことは、その症状が
塞いでいた test が通るようになったことを意味しない**」 —— を踏まえ、本件は
**数を合わせる前に、どちらの側が正しいかを根拠付きで決める**ことから始めた。

## 両側の import 名を列挙した差分 (受入条件)

**compiler 側** (`selfhost/src/Backend/Wasm/WasmEmit.ls:2004` / `:2006`)。
`append-import-*-entry` を読んで名前とシグネチャ型を復元した:

| idx | name | type | 10-import emitter | 11-import emitter |
|---|---|---|---|---|
| 0 | `env.__alloc` | 0 | ある | ある |
| 1 | `env.print` | 1 | ある | ある |
| 2 | `env.read-file` | 0 | ある | ある |
| 3 | `env.command-line-arg` | 0 | ある | ある |
| 4 | `env.string-concat` | 2 | ある | ある |
| 5 | `env.substring` | 3 | ある | ある |
| 6 | `env.file-exists?` | 0 | ある | ある |
| 7 | `env.root_push` | 0 | ある | ある |
| 8 | `env.root_pop` | 4 | ある | ある |
| 9 | `env.root_set` | 2 | ある | ある |
| 10 | `env.print-string` | 1 | **無い** | ある |

**host 側** (`part_002.rs:191-211`) は同じ順序の `Vec<Extern>` を組み、
`include_print_string` が真のときだけ末尾に `print_string` を push する。

**差分は 1 本、末尾 1 箇所だけである。** 0..9 の prefix は完全に同一で、
11-import レイアウトは 10-import レイアウトの**厳密な superset** になっている。
だから `emit-print-string-instr` (`WasmEmit.ls:2018`) は `call 10` を**定数で埋め込める**。
逆に言えば、index 10 が `print-string` であることは compiler の code section が
依存している前提であり、動かせない。

## どちらが正しいか

**11 側が正しい。** 根拠は 3 つで、いずれも実測または一次資料である。

1. **compiler の production 経路は 11 側しか使っていない。**
   `selfhost/src/App/CompilerMode.ls:6093` / `:6140` はどちらも
   `emit-import-section-alloc-print-read-arg-concat-sub-print-string` (11) を呼ぶ。
   10 側を呼ぶのは alias `emit-import-section-runtime` (`WasmEmit.ls:2008`) だけで、
   その output を**インスタンス化する経路は存在しない**
   (唯一の参照 `test_v2_11_emit_import_section_runtime_produces_10_imports` は
   バイト列を parse して import 数を数えるだけで、instantiate しない)。

2. **11-import 側の呼び出し元は全数が緑である。** `run_wasm_with_eleven_imports_compiler_mode*`
   を呼ぶ test は **48 件**あり、そのうち `ignored-lane-expected-failures.txt` に
   載っているものは **0 件**。48/48 が緑である。

3. **10-import 側の呼び出し元には、正当な利用者が 1 件も無い。**
   直接呼び出す test は **90 件**で、内訳は完全に帰属が付いている:

   | 分類 | 件数 | 状態 |
   |---|---|---|
   | 台帳 `I-72` 行 (直接呼び出し) | 79 | 落ちる |
   | 台帳 `[d]` 診断用足場 | 3 | 落ちる |
   | 台帳に無い (`Result` を握り潰している) | 8 | **緑だが何も検査していない** → `I-79` |

   これに間接経路の 3 件 (`compile_fixed_input_target_with_stage2` 経由。
   `selfhost_bootstrap_acceptance/part_001.rs:699`) を足して台帳 82 行に一致する。
   **`10` を要求する module を渡している呼び出し元は 1 件も無い。**
   全員が CompilerMode の吐いた 11-import module を 10-import の host に食わせている。

`b73938ea` (2026-07-14 「fix(selfhost): emit print-string through runtime import」) の
commit message は「通常の CompilerMode build と diagnostic build の import/export/_start index を
11-import ABI に揃え、**旧 10-import bootstrap helper は互換用に維持した**」と書いている。
**この「互換用」の前提が、実測によって否定された。** 互換の相手は存在しなかった。
残されたのは互換経路ではなく、利用者のいない死んだ分岐である。

## 決定

1. `include_print_string = false` の分岐を**削除する**。`run_wasm_with_six_imports_compiler_mode` /
   `..._fs` / `..._fs_printed_first` の 3 helper を廃止し、呼び出しを eleven 側へ寄せる。
2. `run_wasm_with_eleven_imports_compiler_mode_fs_printed_first` を**新設する**。
   `printed_first = true` と `include_print_string = false` の組み合わせは
   `..._fs_printed_first` にしか無く、eleven 側に対応物が無いため。
3. `_inner` の `include_print_string` 引数を落とす。**"six" を名乗る helper を
   生かしたまま残さない** —— 名前と実態が食い違う helper が、本 slice が潰そうとしている
   drift そのものを再び呼び込む。

## 却下した選択肢

- **`print-string` を使うときだけ emit する (ABI を program 依存にする)。**
  却下。`emit-print-string-instr` が `call 10` を定数で埋めているので、
  import 数が program ごとに変わると index 10 の指す先が変わる。
  ABI を可変にすると、この定数を全て動的解決へ書き換える必要が出る。
  **払う価値のあるコストではない** —— 解こうとしている問題は
  「host 側が古い」であって「compiler 側が過剰」ではない。

- **host 側を name-based の `Linker` instantiation へ寄せる。**
  却下 (ただし将来の選択肢としては残す)。位置ずれのクラス全体を消せるのは事実で、
  実際 `part_010.rs` の 6 箇所はこの形を採っているため本件の影響を受けていない。
  しかし `Instance::new` の位置一致要求は、**import の順序が compiler と host で
  一致していることの暗黙の検査**でもある。`call 10` を定数で埋めている以上、
  順序ずれは silent な誤 dispatch になる。ここで検査を捨てると、
  次に順序が動いたとき**誰も気付かない**。本 slice では捨てない。

- **10-import helper を互換用に残す。**
  却下。これは `b73938ea` が採った選択であり、**その前提が本 ADR の計測で否定された**。
  互換の相手が実在しないまま 1 ヶ月半放置され、その間に 82 件の赤と
  8 件の「緑だが何も見ていない」test を産んだ。残す判断自体が問題の原因である。

## 台帳の扱い

- 台帳 `[d]` 3 行 (`runtime_allocator_closures` の診断用足場) は
  「10-import を意図的に供給している」ものだと**当初は読んだが、これは誤読だった**。
  `test_v2_12_stage2_six_import_debug_probe` の本体は helper の戻り値を
  `.expect("V2-12 debug: stage2 probe1 (cache-pairs-probe) 実行失敗")` で開き、
  `probe1_values[0] == 81` などを assert する。**成功を期待している。**
  注記の「落ちること自体が意図された出力」は足場の性格を述べたものであって、
  10-import 契約を pin したものではない。よって 3 件も一緒に移行する。
  行を消すかどうかは doc-GREEN で**実測が緑になったものだけ**を対象に判断する。
- `..._cli_module` の行は `I-78` (stage1 の `integer divide by zero` trap) のままにする。
  同じ `compile_fixed_input_target_with_stage2` を通るが、そこへ**到達する前に**
  stage1 側で落ちるため、本 fix の前後で挙動が変わらないはずである。
- 8 件の握り潰しは本 slice では**移行だけ**を行い、`if let Ok` の強化はしない (`I-79` の仕事)。
  移行後に赤が出たら、それは新規 FAIL として正直に台帳へ載せる。

## Evidence

(doc-GREEN で埋める。RED の実測 / GREEN 後の部分再測定 / 台帳差分をここに置く。)

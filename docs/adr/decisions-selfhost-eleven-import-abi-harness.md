# ADR: selfhost compiler の 11-import ABI を正とし、harness の 10-import 経路を廃止する (2026-08-27)

- **Status**: accepted
- **Date**: 2026-08-27
- **Scope**: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/part_002.rs` の
  `run_wasm_with*_compiler_mode*` helper 群と、その全呼び出し箇所
- **Related**: `ISSUES.md` `I-72` (resolved) / `I-79` / `I-80`
  (裁定は [`decisions-target-defn-probe-shape-drift.md`](decisions-target-defn-probe-shape-drift.md)) / `I-81` /
  `TODO.md` `TARGET-DEFN-PARITY-01` / `VIOLATION-PROBE-STALE-01` /
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
  `test_v2_12_stage2_eleven_import_debug_probe` の本体は helper の戻り値を
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

### RED (fix 前 / main `6a0caebc`)

focused に 1 件だけ回した。

```
cargo test -p lsharp-wasm --test e2e -- --ignored --exact \
  e2e::selfhost_bootstrap_four_layer::test_e2e_boot04_read_file_compiler_mode
```

| 項目 | 実測 |
|---|---|
| 結果 | FAILED |
| panic | `インスタンス化に失敗: expected 11 imports, found 10` |
| 位置 | `selfhost_bootstrap_four_layer/part_007.rs:129:5` |
| 所要 | 59.20s |
| EXIT | 101 |

### GREEN (fix 後 / main `12c41d58`)

同じ 1 件。

| 項目 | 実測 |
|---|---|
| 結果 | ok |
| stage2 Wasm | 288 bytes |
| 所要 | 62.77s |
| EXIT | 0 |

`cargo build -p lsharp-wasm --tests` / `cargo clippy -p lsharp-wasm --tests` はいずれも警告 0 で通る。

### 部分再測定 (3 module 全量 / 2026-08-27)

`target/debug/deps/e2e-aa343ded249bec81` を module ごとに `--ignored` で直列実行した。
取得条件と手順は
[`ignored-lane-sweep-2026-08-23.md`](../development/operations/ignored-lane-sweep-2026-08-23.md) が正本。

| module | 宣言 | 結果行ユニーク | 完走判定 | FAIL | 所要 |
|---|---|---|---|---|---|
| `runtime_allocator_closures` | 4 | 4 | OK | 2 | 488.67s |
| `selfhost_bootstrap_acceptance` | 28 | 28 | OK | 3 | 3,206.23s |
| `selfhost_bootstrap_four_layer` | 148 | 148 | OK | 3 | 6,748.02s |

- `expected 11 imports, found 10` — **3 ログとも 0 件**
- 逆向きの `expected 10 imports, found 11` — **3 ログとも 0 件**
- 台帳外の新規 FAIL — **0 件**

`runtime_allocator_closures` は probe test の rename 後に測り直した値である。
初回 lane (495.48s) は rename 前の binary で走っており、ログの test 名が台帳と一致しない。
**台帳が tree に存在しない名前を持つ状態を残さない**ため、同 module だけ再実行した
(赤 2 件は初回と同一)。

逆向きを併せて数えたのは、tree 全体を 11-import へ寄せたことで
「10-import の stage2 を生成する側」が壊れる可能性を潰すためである。
実際には `selfhost_bootstrap_acceptance/part_000.rs:638-660` と
`selfhost_bootstrap_four_layer/part_000.rs` が**独自のローカル `Instance::new` で 10 import を
供給しており**、共通 helper を通らない。自分が生成した 10-import stage2 と自己整合しているので、
本 fix の影響を受けない。実測の 0 件はこの読みと一致した。

### 台帳差分

対象 3 module の台帳行は 88 行あり、うち **80 行が緑に転じたので削除**した。残った 8 行:

| test | 引き取り先 | 症状 |
|---|---|---|
| `runtime_allocator_closures::test_e2e_alloc_metrics_ci_artifact_payload` | `REPL-TYPE-TAG-01` | 最後の推論型が `Int=100` でなく `-9223372036718940184` (`:313:5`) |
| `runtime_allocator_closures::test_v2_12_stage2_eleven_import_debug_probe` | `[d]` | probe1 (marker 81) は通るようになった。probe2 の marker が 67 でない (`:2975:5`) |
| `acceptance::test_e2e_bootstrap_fixed_input_set_stage_chain_match_cli_module` | `I-78` | `integer divide by zero` (`part_002.rs:318:18`) |
| `acceptance::test_e2e_bootstrap_fixed_input_set_stage_chain_match` | `I-78` (`I-72` から移管) | 同 trap (`part_002.rs:515:9`) |
| `acceptance::test_e2e_bootstrap_stage2_self_feed_fixed_input_set` | `I-78` (`I-72` から移管) | stage2 側の trap。trap kind 不明 (`part_002.rs:295:9`) |
| `four_layer::test_e2e_boot04_self_hosted_stage2_target_defn_parity_reaches_ast_make_type_constrained` | `I-80` (`I-72` から移管) | marker 127 が 0、期待 5 (`part_009.rs:302:5`) |
| `four_layer::test_e2e_boot04_stage1_target_defn_parity_reports_ast_make_type_constrained_lengths` | `I-80` (`I-75` から移管) | marker 126 が 5、期待 7 (`part_009.rs:411:5`) |
| `four_layer::test_v2_12_self_hosted_stage2_reports_compiler_mode_first_violation_body_diff` | `I-81` (`I-72` から移管) | `local_bound_violation_indices` が空 (`part_014.rs:205:10`) |

> **追記 (2026-08-27)**: 最後の 1 行は `I-81` の裁定で決着した。
> この test は violation が在っても無くても赤くなる構造 (末尾が無条件 `panic!`) で、
> 「空になったこと」は欠陥ではなく良い状態だった。極性を反転して
> `..._compiler_mode_has_no_local_bound_violation` へ改名し、台帳行は削除した。
> 詳細は [`decisions-always-failing-diagnostic-probes.md`](decisions-always-failing-diagnostic-probes.md)。

### 受入条件の判定

| 受入条件 | 判定 |
|---|---|
| 先に RED を立てる | 満たした (上記 RED) |
| 両側の import 名を列挙して差分を取り、どちらが正しいかを根拠付きで決める | 満たした (本 ADR の「両側の import 名を列挙した差分」節) |
| 数を合わせるだけの修正はしない | 満たした。10-import 分岐の**削除**として実装した |
| 実測で緑になった行**だけ**を台帳から削除する | 満たした。88 行中 80 行のみ削除 |
| 部分再測定は 3 module (`[d]` 3 行が動きうるので 3 つ目も必須) | 満たした。3 つ目は実際に動いた (`[d]` 2 行が緑・1 行が残存) |

### 予測と実測の照合

- ADR は「`..._cli_module` の行は本 fix の前後で挙動が変わらないはず」と予測した。
  **実測は一致した** — 同 test は fix 後も同じ位置で同じ trap のまま残っている。
- ADR は「`[d]` 3 行は 10-import 契約を pin したものではない」と当初の読みを訂正した。
  **実測はこの訂正を支持した** — 3 行のうち 2 行が緑に転じた
  (`test_v2_12_diagnose_s15_proof_fields` / `test_v2_12_stage2_production_output_size`)。
  10-import に pin されていたなら、tree を 11-import へ寄せた時点で 3 行とも赤のままだったはずである。

### 満たせなかったこと / 残渣

- **`docs/development/validation/workspace-expected-failures.txt:106` を更新していない。**
  非 ignored lane の `strings_patterns_compiler_integration` の注記に
  `(six-import alloc: end address が不正)` という記述が残っている。
  本 slice はこの lane を再測定していないので、**測っていない主張を書き換えない**。
  次にこの lane を回した人が判断すべき残渣である。
- **`I-72` の解決は「症状が消えたこと」で判定しており、赤が 80 行減ったことでは判定していない。**
  `I-71` で「症状が消えたのに赤が減らなかった」前例がある以上、
  赤の増減は fix の効果の指標として使えない。逆向きも同様に使えない。
- **`I-79` の 8 件は本 fix 後も緑のままである。** helper が 11-import になって実際に走り出し、
  `Ok` 腕の assertion を通ったということだが、`Err` を握り潰す構造は何も変わっていない。
  「実害が無かった」ことの証拠として読まないこと。

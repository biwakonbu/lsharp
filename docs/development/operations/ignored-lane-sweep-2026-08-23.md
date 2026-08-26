# `--ignored` lane 全量 sweep の実測 (2026-08-23)

`IGNORED-STALE-PIN-01` (`ISSUES.md` `I-64`) の受入条件「`#[ignore]` 付き e2e を全量実行して
赤を列挙する」に対する実測記録。判断の正本は
[decisions-ignored-lane-ledger-scope.md](../../adr/decisions-ignored-lane-ledger-scope.md)。

## 取得条件

| 項目 | 値 |
|---|---|
| revision | `85a4714a` (main、未 push) |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` (`cargo build` 済み、sweep 中は再ビルドしない) |
| 起動 | `python3 /Users/biwakonbu/github/tmp/i64/run_lane.py` を `nohup` + `os.setsid()` で切り離し |
| filter | module ごとに `<bin> --ignored 'e2e::<module>::'` |
| 並列度 | libtest 既定 (`--test-threads` 無指定 = 論理コア数) |
| 機種 | Mac17,2 / 10 core / macOS 26.5.1 |
| 併走 | **無し。** sweep 中は同一マシンで `cargo` を一切起動しない (binary が差し替わると revision が混ざる) |
| ログ | `/Users/biwakonbu/github/tmp/i64/mod-<module>.log` (末尾に `MODEXIT=` / `ELAPSED=`) |
| 進捗 | 同ディレクトリ `progress.txt` の `DONE <module> rc= elapsed=` 行 |

`selfhost_native_stage_chain` は先行 sweep の数値 (2026-08-19 `35ea7c32`) を繰り越さず、
他 module の完走後に**同条件で測り直す** (`chain_stage_chain.py` が `LANE-COMPLETE` を待って起動)。
併走させないのは所要が CPU 競合で歪むため。

## 対象の分母

`<bin> --list --ignored` で数えた module 別の `#[ignore]` 件数 (計 **1,431**)。

| module | 件数 | | module | 件数 |
|---|---:|---|---|---:|
| `selfhost_native_stage_chain` | 615 | | `selfhost_cli_actual_main_args` | 25 |
| `selfhost_cli_core` | 381 | | `bootstrap_selfhost_lsp_integration` | 12 |
| `selfhost_bootstrap_four_layer` | 146 | | `selfhost_gc_stateful_soak` | 8 |
| `selfhost_native_differential` | 104 | | `selfhost_typeinfer_pipeline_bootstrap` | 5 |
| `selfhost_lsp_docs_ops` | 54 | | `runtime_allocator_closures` | 4 |
| `selfhost_doctools_cli_diagnostics` | 38 | | `selfhost_native_stage23_gap` | 3 |
| `selfhost_bootstrap_acceptance` | 28 | | `selfhost_rooting_parity` / `selfhost_main_module_determinism` / `selfhost_macro_compiler` | 各 2 |
| | | | `selfhost_bootstrap_contracts` / `incremental_benchmark` | 各 1 |

**従来の台帳が覆っていたのは `selfhost_native_stage_chain` の 615 件だけで、残り 816 件は
一度も測られていない。** `I-64` が見つけた 1 本は `selfhost_cli_core` にあり、この未測定域に属する。

**完走判定の分母はこの 1,431 に対して行う。** 全ログの `running N tests` の和が 1,431 に
一致しない場合、module 名の書き落としか filter の綴り違いを疑う (`I-11` で確立した規律と同じで、
grep 由来の暫定値ではなく `--list` の実数に対して判定する)。

## 結果

**完走した (2026-08-24T02:37、`SWEEP-COMPLETE`)。18 module / 1,431 件 / 通算約 11.4 時間。**
下表は 18 module 全部の実測である (11 module 時点の暫定表を置き換えた)。

| module | 分母 | passed | failed | 所要 (s) | 完了時刻 |
|---|---:|---:|---:|---:|---|
| `incremental_benchmark` | 1 | 1 | 0 | 127.14 | 15:15:50 |
| `selfhost_bootstrap_contracts` | 1 | 1 | 0 | 53.85 | 15:16:44 |
| `selfhost_macro_compiler` | 2 | 2 | 0 | 22.58 | 15:17:06 |
| `selfhost_main_module_determinism` | 2 | 2 | 0 | 146.38 | 15:19:33 |
| `selfhost_rooting_parity` | 2 | 2 | 0 | 19.46 | 15:19:52 |
| `selfhost_native_stage23_gap` | 3 | 1 | **2** | 114.90 | 15:21:47 |
| `runtime_allocator_closures` | 4 | 0 | **4** | 228.39 | 15:25:35 |
| `selfhost_typeinfer_pipeline_bootstrap` | 5 | 5 | 0 | 276.80 | 15:30:12 |
| `selfhost_gc_stateful_soak` | 8 | 0 | **8** | 399.82 | 15:36:52 |
| `bootstrap_selfhost_lsp_integration` | 12 | 12 | 0 | 281.50 | 15:41:34 |
| `selfhost_cli_actual_main_args` | 25 | 22 | **3** | 1279.36 | 16:02:53 |
| `selfhost_bootstrap_acceptance` | 28 | 21 | **7** | 1453.20 | -- |
| `selfhost_doctools_cli_diagnostics` | 38 | 38 | 0 | 778.00 | -- |
| `selfhost_lsp_docs_ops` | 54 | 53 | **1** | 130.86 | -- |
| `selfhost_native_differential` | 104 | 71 | **33** | 718.96 | -- |
| `selfhost_bootstrap_four_layer` | 146 | 69 | **77** | 4109.35 | -- |
| `selfhost_cli_core` | 381 | 356 | **25** | 14740.93 | -- |
| `selfhost_native_stage_chain` | 615 | 501 | **114** | 16112.99 | -- |
| **合計** | **1431** | **1157** | **274** | **41043.53** | 2026-08-24T02:37 |

分母の和 1,431 は「対象の分母」表の宣言数と**過不足なく一致する**。
`compare_ignored_lane.py` の完走判定も OK (宣言 1431 / 結果行ユニーク 1431 / ログ間重複 0)。
module 名の書き落としも filter の綴り違いも無かった。

**所要 41,043s のうち 2 module で 75%** を占める (`selfhost_native_stage_chain` 16,113s /
`selfhost_cli_core` 14,741s)。wall clock が約 11.4 時間で通算とほぼ等しいのは
**module を直列で回したから**である (cargo の並列実行は測定を歪めるので使っていない)。

**1 件あたりの所要は module によって桁が違う。** `selfhost_rooting_parity` は 2 件 19.46s
(9.7s/件)、`selfhost_cli_actual_main_args` は 25 件 1279.36s (51.2s/件)。
後者は CLI bundle を毎回組み立てる test。**この外挿は実測で当たった** —
`selfhost_cli_core` 381 件は 14,740.93s (38.7s/件) で約 4.1 時間、
完走見込み「12 時間以上」に対し実測 11.4 時間だった。
「module 分割の利点は時間短縮ではなく、途中で殺されても部分成果が残ること」という
`I-11` 時点の判断がここでもそのまま当てはまる。

## 振り分け

### 台帳へ載せる規則 (doc-RED / 2026-08-23 確定)

**振り分けの結果がどちらであっても、赤は 1 本残らず
`docs/development/validation/ignored-lane-expected-failures.txt` へ行を足す。**
「修正する」に分類したものを台帳から外す運用にはしない。

理由は `compare_ignored_lane.py` の契約にある。同 script は台帳に無い FAIL を
**新規 FAIL として exit 1** にする。修正待ちの赤を台帳から外すと、fix が全部入るまで
sweep 完走の検証そのものが通らず、しかも「台帳にある既知」と「本当に新しく出た赤」を
exit code で区別できなくなる。台帳は *修正しないと決めたもの* の一覧ではなく、
**この revision で赤いと分かっているもの**の一覧である。

判定は行ではなく**注記**が持つ。書式:

| 振り分け | 注記の形 |
|---|---|
| 修正する | `# 引き取り先: <TODO-ID>` (例: `# 引き取り先: NATIVE-I32SUB-01`) |
| expected-failure | `# 陳腐化 pin: <理由>` / `# [a] Lima VM 依存` など既存の分類記号 |

先行する stage_chain の 113 行がこの形になっており (`# diagnostic:` 注記を持つ赤を
含む)、新 module も同じ形へ揃える。fix が入って緑になった行はその時点で削除する
— 削除の根拠は「振り分けたから」ではなく「実測が緑になったから」である。

### 結果 (11 module 分 / 赤 17 件)

| 引き取り先 / 分類 | 件数 | 内訳 |
|---|---:|---|
| `NATIVE-I32SUB-01` | 2 | `selfhost_native_stage23_gap` の 2 件。`NativeCodegen.ls` に `i32.sub` が無い**真の gap**。**表を直して閉じない** |
| `REPL-TYPE-TAG-01` | 9 | `runtime_allocator_closures` 1 + `selfhost_gc_stateful_soak` 5 + `selfhost_cli_actual_main_args` 3。`I-69` の L1/L2 |
| `[d] 診断用足場` | 3 | `runtime_allocator_closures` の `test_v2_12_*` 3 件。`2214bd49` が既知 gap の可視化として追加したもの |
| **保留** | 3 | `selfhost_gc_stateful_soak` の LSP stdio frame 系。`selfhost_lsp_docs_ops` のログ待ち |

17 件すべてに `ignored-lane-expected-failures.txt` の行を足した。
`compare_ignored_lane.py` を該当 4 ログへ流し、**新規 FAIL 0 / 解消 0** を確認済み
(残る `判定: NG` は未実行 module 113 件の「未出現」によるもので、sweep 完走で解消する)。

**`selfhost_cli_actual_main_args` の 3 件は `EMBEDDED-CLI-OPTION-SPACE-01` ではない。**
`..._check_json_file` は `--json` を明示する argc 3 の経路で、argc 2 の fallthrough を通らない。
この module が `I-69` の未確定点を埋めた経緯は `ISSUES.md` の `I-69` 本文にある。

### 結果 (18 module 全量 / 赤 274 件、うち新規 145 件)

台帳は 130 行 − 解消 1 + 新規 145 = **274 行**になった。
`compare_ignored_lane.py` を 18 ログ全部へ流し、
**新規 FAIL 0 / 解消 0 / 未出現 0、完走判定 OK、exit 0** を確認済み。

**解消 1 件**: `selfhost_native_stage_chain::test_e2e_selfhost_main_representative_failing_chunk_text_is_plain_bytes`
は台帳にあったが full sweep で緑に転じたので行を削除した。
**赤を消さない規則は赤にだけ効く。** 緑になったものを残すと compare が「解消」で非 0 になる。

新規 145 件の引き取り先:

| 引き取り先 | 件数 | 症状 |
|---|---:|---|
| `STAGE-WASM-TRANSLATE-01` (`I-71`) | 72 | 生成 Wasm の translation error。distinct な offset は **329391 / 457947 / 310805 の 3 つだけ**、メッセージは 3 つとも `type mismatch: expected i64 but nothing on stack` で完全一致 |
| `NATIVE-DIFF-PIN-01` (`I-73`) | 33 | native の exact-byte pin が一律ずれ (frame displacement -8 / epilogue 0x5C vs 0x5D / 長さ系は payload でなく bundle 全体) |
| `SWEEP-UNCLASSIFIED-01` (`I-75`) | 19 | どの cluster にも収まらなかった。症状が 1 件ずつ違う |
| `ROOT-IMBALANCED-HELPER-01` (`I-74`) | 9 | `ImbalancedExit depth:1` (`compile-file-state` 6 / `compile-pair-state` 3)。`I-14` 案 E の射程外 |
| `STAGE-WASM-IMPORT-COUNT-01` (`I-72`) | 8 | `expected 11 imports, found 10`。8 件とも数値が完全一致 |
| `CHECK-TYPE-PIN-01` | 3 | `selfhost_cli_core` の型名 pin。`I-45` の更新漏れが**さらに 3 本**見つかった |
| `REPL-TYPE-TAG-01` | 1 | `selfhost_cli_core::test_e2e_selfhost_cli_repl_core` のアドレス印字 |

**cluster が大きいことは同一原因の証拠ではない。** 72 件が 3 offset に収束するのは
強い信号だが、原因が 1 つだと決めた瞬間に「1 本直して全部緑」を期待して
検証を 1 本で打ち切る誘惑が生まれる。各 TODO 項目の受入条件でこれを禁じてある。

### 保留していた 3 件の判定 (2026-08-23 → 確定)

`selfhost_gc_stateful_soak` の LSP stdio frame 3 件は `selfhost_lsp_docs_ops` のログ待ちだった。
同 module 完走で **hover 4 件は全て緑**、さらに `selfhost_gc_stateful_soak` の実測出力自身が
`"contents":"type-info:2:22"` を返していた。
→ **`type-info:L:C` が現行の contents 形式であることが確定し、陳腐化 pin 仮説は否定された。**
残るのは range が `{-1,-1}` に潰れている点で、これは**実測出力側**に現れており
形式変更では説明できない。`I-75` が保持する。

## この sweep が実際に捕まえたもの (2026-08-23)

`I-64` の前提は「`#[ignore]` の下では回帰が観測されないので陳腐化 pin が溜まる」だった。
sweep はそれを 1 件、**混入から 1 日で**実証した。

`914bd9f1` (2026-08-22、`decisions-selfhost-zero-arity-defn-type.md` / `I-45`) が 0 引数 `defn` を
`Unit -> body` として登録するようにした。`selfhost_cli_actual_main_args.rs` には
この変更に晒される型名 pin が 4 本あるが、**同 commit はその 4 本を全部取り残した**。

翌 2026-08-23、陳腐化 pin の修復を目的とした commit `13a505b2` (`I-60`) が
`..._check_format_json` の pin を `"Int"` → `"Fn"` へ直し、`I-45` 由来である旨のコメントを
残した。**しかし直したのは 4 本のうち 1 本だけ**である。

| pin | `914bd9f1` 時点 | `13a505b2` 時点 | sweep 判定 |
|---|---|---|---|
| `..._check_format_json` | 取り残し | **修復** | ok |
| `..._check_file` | 取り残し | 取り残し | **FAIL** |
| `..._check_json_file` | 取り残し | 取り残し | **FAIL** |
| `..._repl_summary` (`.rs:1786`) | 取り残し | 取り残し | **FAIL** (別要因と重畳) |

**当初この節は「同じ人が同じ日に、見えている方だけを直して見えない方を残した」と書いていたが
誤りである (2026-08-23 訂正)。** 修復した `13a505b2` は `914bd9f1` の翌日の別 commit であり、
しかも修復された `..._check_format_json` 自身も `#[ignore]` 下にある (本 sweep のログに
`... ok` として現れる)。「赤で気付いて直した」のではない。

訂正で所見は**弱まるのではなく強まる**。陳腐化 pin の修復を目的として明示的に走らせた
パスですら、同一ファイル・同一 fixture (`(defn main [] 42)` = 17 byte) の兄弟 3 本を
取りこぼしている。つまりこの壊れ方は不注意ではなく、**`#[ignore]` lane を既定で回さない運用の
構造的な帰結**である。網羅は「気を付ける」では達成されず、lane を回すことでしか達成されない。
sweep を回さなければ、次に誰かがこれらを `#[ignore]` から外す日まで観測されなかった。

引き取り先は `TODO.md` の `CHECK-TYPE-PIN-01` (`check` 系 2 本) と
`REPL-TYPE-TAG-01` (`..._repl_summary`)。

## 再実行の手順

```bash
python3 scripts/compare_ignored_lane.py <log> [<log> ...]   # 台帳との 4 種差分。exit 0 が一致
bash scripts/ci/test-compare-ignored-lane.sh                # 上記の契約テスト (cargo 非依存)
```

lane 自体の回し方は `AGENTS.md` の「`--ignored` lane の実測と台帳突合」節を見よ。

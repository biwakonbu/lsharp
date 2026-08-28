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
| `check` の型名 pin | 3 | `selfhost_cli_core` の型名 pin。`I-45` の更新漏れが**さらに 3 本**見つかった。**2026-08-27 に 5 本まとめて解決** (`decisions-selfhost-zero-arity-defn-type.md` の「7〜11 本目」節)。台帳からも 5 行削除済み |
| `REPL-TYPE-TAG-01` | 1 | `selfhost_cli_core::test_e2e_selfhost_cli_repl_core` のアドレス印字 |

**`SWEEP-UNCLASSIFIED-01` (`I-75`) の 19 件は 2026-08-28 に全数の引き取り先が決まった。**
移管先は `I-72` / `I-76` / `I-78` / `I-80` / `I-84` / `I-90` / `I-93`〜`I-100`。
`I-75` は `resolved`、`SWEEP-UNCLASSIFIED-01` は `TODO.md` から削除し、
lane 再計測だけを `SWEEP-LANE-RERUN-01` が引き継いだ。
**19 件が直ったという意味ではない** -- 移管先はいずれも open である。

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

引き取り先は `check` 型名 pin の追随 slice (`check` 系 2 本、2026-08-27 解決) と
`REPL-TYPE-TAG-01` (`..._repl_summary`)。

## 再実行の手順

```bash
python3 scripts/compare_ignored_lane.py <log> [<log> ...]   # 台帳との 4 種差分。exit 0 が一致
bash scripts/ci/test-compare-ignored-lane.sh                # 上記の契約テスト (cargo 非依存)
```

lane 自体の回し方は `AGENTS.md` の「`--ignored` lane の実測と台帳突合」節を見よ。

## 副次成果: ADR Evidence の突き合わせ (2026-08-24)

sweep の verdict を `docs/adr/*.md` の `## Evidence` 節と突き合わせた (`I-70`)。
cargo を使わない後処理なので、sweep のログだけで実施できる。

取得手順:

1. `docs/adr/*.md` の `## Evidence` 節から `test_e2e_\w+` を抽出 → **427 件の citation**
2. `crates/**/*.rs` の `#[ignore]` 直後 (属性が続く形も追う) の `fn` 名を集める
3. 交差を取る → **43 件 / 15 ADR**。`#[cfg_attr(.., ignore)]` 形は 0 件なのでこれが全量
4. `mod-*.log` の `test <module>::<name> ... (ok|FAILED)` と突き合わせる

結果:

| verdict | 件数 |
|---|---|
| ok | 26 |
| FAILED | 16 |
| 同名 2 module で ok / FAILED が割れる | 1 |
| **計** | **43** |

**FAILED 17 件 (citation 単位) のうち訂正を要したのは 3 件 (一意 test 2 本) だけだった。**
内訳は `I-70` の解決節にある。11 件は ADR 自身が「赤である」と主張する分類表の一部で、
3 件は env / Lima VM の前提が sweep で未充足だったものである。

**件数は citation 単位で数える。** 同じ test を 2 ADR が引く例があるので、test 単位とは
一致しない。基数を混ぜると差分がどこかの分類へ吸われる (初版はこれで矛盾を 4 と書いた)。

**この後処理は sweep のたびに繰り返す価値がある。** ただし
**verdict の色で一括判定してはならない** — 上の 10 件を機械的に訂正していれば、
正しい Evidence を壊していた。判定は ADR の主張文を読んで行う。

## 部分再測定: `I-71` fix 後の 3 module (2026-08-27)

`I-71` (空 do が unit を積まない) の fix が台帳へ与える影響を測るため、
該当行を持つ 3 module だけを同条件で回した。判断の正本は
[decisions-selfhost-empty-do-unit-value.md](../../adr/decisions-selfhost-empty-do-unit-value.md)。

### 取得条件

| 項目 | 値 |
|---|---|
| 対象 | `runtime_allocator_closures` / `selfhost_bootstrap_acceptance` / `selfhost_bootstrap_four_layer` |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` (`Compiler.ls` の fix 込み。lane 中は再ビルドしない) |
| 起動 | `python3 /Users/biwakonbu/github/tmp/i71/run_lane_i71.py` を `os.setsid()` で切り離し |
| filter | module ごとに `<bin> --ignored 'e2e::<module>::'` |
| 並列度 | libtest 既定 |
| 機種 | Mac17,2 / 10 core / macOS 26.5.1 |
| 併走 | **無し。** lane 中は `cargo` を一切起動しない |
| ログ | `/Users/biwakonbu/github/tmp/i71/lane/mod-<module>.log` (末尾に `MODEXIT=` / `ELAPSED=`) |
| 完走マーカ | 同ディレクトリ `progress.txt` の `LANE-COMPLETE` 行 (`LANE-DONE` ではない) |

### 結果

| module | 宣言 | 結果行 | 完走 | 赤 | 所要 |
|---|---:|---:|---|---:|---:|
| `runtime_allocator_closures` | 4 | 4 | OK | 4 | 233.41s |
| `selfhost_bootstrap_acceptance` | 28 | 28 | OK | 7 | 1,818.75s |
| `selfhost_bootstrap_four_layer` | 148 | 148 | OK | 77 | 5,582.32s |
| **計** | **180** | **180** | OK | **88** | **7,634s (2.1 h)** |

`compare_ignored_lane.py` は **新規 FAIL 0 / 解消 0 / 未出現 0 / exit 0**。
**赤の集合は fix 前 (2026-08-24) と 1 件も違わない。**

### 部分 lane の比較は抜粋台帳に対して行う

```bash
# 測った module だけを台帳から抜き出す
grep -E "^lsharp-wasm::e2e (runtime_allocator_closures|selfhost_bootstrap_acceptance|selfhost_bootstrap_four_layer)::" \
  docs/development/validation/ignored-lane-expected-failures.txt > /tmp/subset.txt
python3 scripts/compare_ignored_lane.py /path/to/lane/mod-*.log --ledger /tmp/subset.txt
```

**全量台帳に対して回してはならない。** 同 script は「台帳エントリがあるのに、その module を
覆うログが無い」行を「未出現」として非 0 にするので、測っていない 7 module の行が
全て未出現になる。台帳を編集したあとに検証し直すときは、**編集後の台帳から抜粋を作り直す**
(編集前の抜粋を使い回すと、付け替えた行が差分として出る)。

### この再測定が示したこと

fix 前後で症状を test 単位に突き合わせると、`expected i64 but nothing on stack` の
出現は 3 module で **0 件**になった一方、**赤は 1 件も減らなかった**。
74 test が `expected 11 imports, found 10` (`I-72`) へ移っただけである。

**「何件の赤が消えたか」は fix の効果の指標にならない。** 同じ test に複数の層
(translation / instantiation / 実行時 trap) が積み重なっている場合、上の層を直しても
下の層で落ちるので、赤の数は動かない。効果は**その症状の出現数が 0 になったか**で測る。
台帳は**実測の赤と一致するか**で保つ。2 つを混ぜると、直っているのに直っていないように
見えるか、実測が赤の行を削除して台帳を壊すかのどちらかになる。

なお `integer divide by zero` (`I-78`) は fix 前後とも 2 件で変化しない。
本 fix が持ち込んだ regression ではないことの確認としてここに記録する。

## 部分再測定: `I-72` fix 後の 3 module (2026-08-27)

`I-72` (harness の import 集合が 10 本で古い) の fix が台帳へ与える影響を測るため、
`I-71` のときと同じ 3 module を同条件で回した。判断の正本は
[decisions-selfhost-eleven-import-abi-harness.md](../../adr/decisions-selfhost-eleven-import-abi-harness.md)。

### 取得条件

| 項目 | 値 |
|---|---|
| 対象 | `runtime_allocator_closures` / `selfhost_bootstrap_acceptance` / `selfhost_bootstrap_four_layer` |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` (main `12c41d58` で `cargo build -p lsharp-wasm --tests`。lane 中は再ビルドしない) |
| 起動 | `python3 /Users/biwakonbu/github/tmp/i72/run_lane_i72.py` を `os.setsid()` で切り離し |
| filter | module ごとに `<bin> --ignored 'e2e::<module>::'` |
| 並列度 | libtest 既定 |
| 機種 | Mac17,2 / 10 core / macOS 26.5.1 |
| 環境 | Lima VM `lsharp-linux-x86` は Stopped、`LSHARP_NATIVE_*` は全て未設定 |
| 併走 | **無し。** lane 中は `cargo` を一切起動しない |
| ログ | `/Users/biwakonbu/github/tmp/i72/lane/mod-<module>.log` (末尾に `MODEXIT=` / `ELAPSED=`) |
| 完走マーカ | 同ディレクトリ `progress.txt` の `LANE-COMPLETE` 行 |

### 結果

| module | 宣言 | 結果行 | 完走 | 赤 | 所要 |
|---|---:|---:|---|---:|---:|
| `runtime_allocator_closures` | 4 | 4 | OK | 2 | 488.67s |
| `selfhost_bootstrap_acceptance` | 28 | 28 | OK | 3 | 3,206.23s |
| `selfhost_bootstrap_four_layer` | 148 | 148 | OK | 3 | 6,748.02s |
| **計** | **180** | **180** | OK | **8** | **10,443s (2.9 h)** |

- `expected 11 imports, found 10` — **3 ログとも 0 件** (`I-71` fix 後は 74 件だった)
- 逆向きの `expected 10 imports, found 11` — **3 ログとも 0 件**
- 台帳外の新規 FAIL — **0 件**

台帳 88 行のうち 80 行が緑に転じたので削除し、8 行が残った。

### test を rename したら、その module は測り直す

`runtime_allocator_closures` の値は再測定である。初回 lane のあとで
`test_v2_12_stage2_six_import_debug_probe` を `..._eleven_import_debug_probe` へ rename したため、
初回ログの test 名が編集後の台帳と一致しなくなった。`compare_ignored_lane.py` は
これを **新規 FAIL 1 件 + 未出現 1 件**として非 0 で報告する (実際にそうなった)。

ここで台帳を旧名へ戻すのは誤りである。**台帳は tree に存在する名前を持たねばならない。**
かといって旧名のログを新名の台帳と突合させて手で「同じものだ」と読むのも誤りで、
それは新名が実際に赤いことを一度も測っていない状態を放置することになる。
正しいのは**その module だけ測り直す**ことで、実際 8 分で済んだ (488.67s / 赤 2 件で初回と同一)。
初回ログは `pre-rename-mod-<module>.log.bak` として残す。

### 所要は fix の前後で 37% 伸びた

同じ 3 module で 7,634s → 10,450s。原因は単純で、**instantiation で早期に落ちていた 80 件が
実際に走り切るようになった**ためである。赤が減ると lane は速くなる、という直感は逆である。
次に同じ 3 module を回す人は 3 時間を見込むこと。

### 判定に使ったのは症状数であって行数ではない

`I-71` の記録が示したとおり「何件の赤が消えたか」は fix の効果の指標にならない。
本件では逆に 80 行が消えたが、**それも判定の根拠にしていない**。
根拠は `expected 11 imports, found 10` の出現が 0 になったことの一点である。

新しく足したのは**逆向きの症状も数えること**である。tree 全体を 11-import へ寄せる変更なので、
「10-import の stage2 を生成して自分で instantiate する側」を壊しうる。
`expected 10 imports, found 11` を併せて grep して 0 件を確認した。
**片方向だけ数えると、直したつもりで別の場所を壊した状態を緑と読む。**

### 残った 8 行の引き取り先

| test | 引き取り先 |
|---|---|
| `runtime_allocator_closures::test_e2e_alloc_metrics_ci_artifact_payload` | `REPL-TYPE-TAG-01` |
| `runtime_allocator_closures::test_v2_12_stage2_eleven_import_debug_probe` | `[d]` (診断足場) |
| `acceptance::..._stage_chain_match_cli_module` | `I-78` |
| `acceptance::..._stage_chain_match` | `I-78` (`I-72` から移管) |
| `acceptance::..._stage2_self_feed_fixed_input_set` | `I-78` (`I-72` から移管) |
| `four_layer::..._stage2_target_defn_parity_reaches_ast_make_type_constrained` | `I-80` |
| `four_layer::..._stage1_target_defn_parity_reports_ast_make_type_constrained_lengths` | `I-80` (`I-75` から移管) |
| `four_layer::..._reports_compiler_mode_first_violation_body_diff` | `I-81` |


## `selfhost_bootstrap_four_layer` の再計測 (2026-08-27)

### 取得条件

| 項目 | 値 |
|---|---|
| runner | `/Users/biwakonbu/github/tmp/i79/run_lane_i79.py` (pid 43934、`os.setsid()` で切り離し) |
| 開始 | 2026-08-27 07:39:20 (`LANE-START`) |
| 所要 | **6517.18s** (108.6 分)。前回同 module 6748s に対し **3.4% 短縮** |
| ログ | `/Users/biwakonbu/github/tmp/i79/lane/mod-selfhost_bootstrap_four_layer.log` |
| 台帳 | subset `/Users/biwakonbu/github/tmp/i79/lane/subset-four_layer.txt` (4 行。module 名で抽出) |
| 突合 | `python3 scripts/compare_ignored_lane.py --ledger <subset> <log>` |

### 結果

```
宣言 148 / 結果行 148 / 重複 0  [144 passed / 4 failed / 6517.13s]
完走判定 : OK
新規 FAIL : 0 件 / 解消 : 0 件 / 未出現 : 0 件
判定: OK -- 完走し、台帳と一致した   (exit 0)
```

赤 4 件は subset 台帳の 4 行と過不足なく一致した。`I-83` の
`Invalid input WebAssembly code at offset 270: type mismatch: expected i64 but nothing on stack` と、
`I-81` の `part_014.rs:205:10` `V2-12 CompilerMode diff: stage3 output に violation があること` は
**どちらも既存記述どおりに再現した**。台帳への追記は不要である。

### 再現ではなく新しく分かったこと — 範囲外読み出しは 0 を返すとは限らない

`I-80` の 2 件について full marker dump が取れた。本 sweep の記録は
「marker 127 は AST の外を指して **0 になる**」と書いていたが、それは stage2 側だけだった。
stage1 側は marker 126 で落ちるので 127 を assert しておらず、値が見えていなかった。

| marker | stage1 | stage2 |
|---|---|---|
| 127 (`inner-call[0]`) | **4294967296** (= 2^32) | 0 |
| 128 (`inner-func[0]`) | **72057594054705152** (= 2^56 + 2^24) | 0 |

同じ probe を同じ入力で走らせても、Rust の stage1 と self-hosted の stage2 で
**範囲外読み出しの結果が違う**。詳細は `ISSUES.md` の `I-80` が正本。

> 片方の binary だけで観測した値を「こういう値が返る」と一般化しない。
> 範囲外読み出しの挙動は実装ごとに違い、それ自体が情報である。

### 緑の 144 件が運んだ情報

`test_validate_stage2_wasm` は **ok** で終わった。これは検査が通ったことを意味しない —
この test は結果を `eprintln!` に流すだけで、`Ok` でも `Err` でも緑になる
(`ISSUES.md` `I-82` の #9)。nextest は緑の test の出力を捨てるので、
**実際の validator の戻り値はこのログから読めない。**
`I-82` の実装では targeted 実行で `--nocapture` を取る必要がある。

同様に `test_debug_boot04_*` 12 件もすべて緑だが、主題の assertion は
`assert!(!output.trim().is_empty())` 1 行だけである (`I-85`)。
**「144 passed」は 144 件の契約が守られていることを意味しない。**

## 部分再測定: `I-80` / `I-81` / `I-82` / `I-85` の実装後 (2026-08-27)

4 項目とも `selfhost_bootstrap_four_layer` の実装 slice なので **lane 1 本で覆う**。
判断の正本は [decisions-probe-subject-unchecked.md](../../adr/decisions-probe-subject-unchecked.md) /
[decisions-always-failing-diagnostic-probes.md](../../adr/decisions-always-failing-diagnostic-probes.md) /
[decisions-target-defn-probe-shape-drift.md](../../adr/decisions-target-defn-probe-shape-drift.md)。

### 取得条件

| 項目 | 値 |
|---|---|
| 対象 | `selfhost_bootstrap_four_layer` のみ |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` (main `29fc24d1` で `cargo build -p lsharp-wasm --tests`。lane 中は再ビルドしない) |
| 起動 | `python3 /Users/biwakonbu/github/tmp/i80/run_lane_i80.py` を `os.setsid()` で切り離し |
| filter | `<bin> --ignored e2e::selfhost_bootstrap_four_layer::` |
| 並列度 | libtest 既定 |
| 機種 | Mac17,2 / 10 core / macOS 26.5.1 |
| 併走 | **無し。** lane 中は `cargo` を一切起動しない |
| ログ | `/Users/biwakonbu/github/tmp/i80/lane/mod-selfhost_bootstrap_four_layer.log` |
| 完走マーカ | 同ディレクトリ `progress.txt` の `LANE-COMPLETE` 行 |
| 抜粋台帳 | `/Users/biwakonbu/github/tmp/i80/subset.txt` (**台帳 3 行の削除後に取り直した**。残り 1 行) |
| rename 前ログ | `/Users/biwakonbu/github/tmp/i80/lane/pre-rename-mod-selfhost_bootstrap_four_layer.log.bak` (`I-79` 時点の測定) |

### 測る前に立てた予測

- **赤は 1 件だけになるはず。** `I-79` 時点の実測は 148 宣言 / 148 結果 / **FAIL 4** で、
  内訳は `I-80` の 2 件 + `I-81` の 1 件 + `I-83` の 1 件である。前 3 者は実装済で個別実行は緑、
  `I-83` は未着手なので赤のまま残る。台帳に残る four_layer の行も `I-83` の
  `test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` 1 件のみで、数が合う。
  - **訂正 (lane 完走前に記録)**: この行は当初「FAIL 3 / その 3 件は `I-80` 2 件と `I-81` 1 件」と
    書いていたが誤りである。`FAIL 3` は `I-72` 時点の測定値 (6,748.02s) であり、
    `I-79` 時点 (6,517.13s) は `I-83` が加わって 4 件だった。**結論 (赤は 1 件) は変わらないが、
    結果が出た後で基礎を直すと予測を後付けしたように見えるので、出る前に直しておく。**
- **`I-82` / `I-85` の 12 件は元から緑だったので、赤の増減には現れない。**
  ここで見たいのは「assertion を実質化したせいで新しく落ちる test が無いか」である。
  個別実行 19 件は緑を確認済みだが、**lane では他の test と同じプロセス空間で走る**ので
  順序依存が出るならここに出る。
- **宣言数は 148 から動かないはず。** 本 slice は test の追加も削除もしていない
  (`I-81` の rename 1 件のみ)。動いたら数え漏れを疑うこと。

### 結果 (1 回目 — SIGKILL で完走せず)

**この lane は完走していない。** `MODEXIT=-9` (SIGKILL) で 6,420.99s / 108 件の時点で落ちた。

| 項目 | 実測 |
|---|---|
| 宣言数 | **144** (予測は 148) |
| 結果行 (ユニーク) | 108 |
| ログ間重複 | 0 |
| FAILED | 1 (`test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` = `I-83`) |
| `MODEXIT` | **-9 (SIGKILL)** |
| `ELAPSED` | 6,420.99s |
| `compare_ignored_lane.py` exit code | **1** (`判定: NG -- 完走していない`) |

`progress.txt` には `LANE-COMPLETE 2026-08-27T13:29:48` が書かれているが、
**このマーカは完走を意味しない。** `run_lane_i80.py` は `subprocess.call()` が返った時点で
書くので、子プロセスが signal で殺されても書かれる。
**完走判定に使えるのは `MODEXIT` と `compare_ignored_lane.py` だけである。**

#### 外した予測 1: 「宣言数は 148 から動かないはず」

**外れた。実測 144。** 予測の根拠に書いた「本 slice は test の追加も削除もしていない
(`I-81` の rename 1 件のみ)」が**事実として誤りだった**。
`32565f9a` (`I-79` 測定時点) と `29fc24d1` (本 lane の binary) の間で four_layer の
`fn test_` は 152 → 149 に減っており、内訳は次のとおり:

| 変化 | test |
|---|---|
| 削除 | `test_debug_stage2_output_minimal` |
| 削除 | `test_debug_stage2_save` |
| 削除 | `test_debug_stage3_main_again_output_chars` |
| 削除 | `test_debug_stage3_output_chars` |
| 削除 | `test_v2_12_self_hosted_stage2_reports_compiler_mode_first_violation_body_diff` |
| 追加 | `test_local_bound_violation_indices_detects_out_of_range_local` |
| 追加 | `test_v2_12_self_hosted_stage2_compiler_mode_has_no_local_bound_violation` |

`--ignored` 側の差 -4 は、追加 2 件のうち 1 件が `#[ignore]` を持たないことで説明が付く。
**つまり数え漏れは lane 側ではなく、予測を書いた自分の側にあった。**
「動いたら数え漏れを疑うこと」と書いた釘は正しく効いたが、疑うべき対象を取り違えていた。

#### 外した予測 2 ではないもの: 「赤は 1 件だけ」

108 件までの範囲では**赤は `I-83` の 1 件だけ**で、予測どおりに見える。
**ただし完走していないので判定は保留する。** 残り 36 件の中に赤があるかは測っていない。
`compare_ignored_lane.py` も `新規 FAIL 0 / 解消 0 / 未出現 0` を返しているが、
これは 108 件分の情報であって 144 件分ではない。

#### SIGKILL の原因

**特定できていない。** 分かっていることだけ書く:

- 同じ binary 構成・同じ並列度 (libtest 既定 / 10 core) の `I-79` lane は
  148 宣言で `MODEXIT=101` (通常の test 失敗終了) / 6,517.18s で**完走している**。
  設定そのものが完走不能ということはない
- 落ちた時刻 (6,420.99s) は過去 2 本の完走時刻 (6,517s / 6,748s) の直前であり、
  **終盤で落ちている**。残っていたのは stage1→stage2 の重い 36 件で、
  この帯は同時に多数が 60s 超で走る (`has been running for over 60 seconds` が
  ログ末尾に密集している)
- 本 lane 中、`cargo` は起動していない。ただし**同一ホストで待機用の
  background job が走っていた**。これが直接の原因である証拠は無い
- macOS の jetsam は SIGKILL を使うので、メモリ逼迫は候補として残る。**未検証である**

#### 次の lane の条件をどうするか (**結果が出る前に決めておく**)

一度は「次は並列度を絞る」と書いたが、**撤回する。** 理由:

- **原因が未検証のまま条件を変えると、次に完走したときそれが「絞ったおかげ」なのか
  「たまたま」なのかを区別できない。** SIGKILL の原因は特定できていない
- 既定並列度は `I-79` で完走の実績がある。**壊れていると分かっていない条件を先に変えない**
- 残る lane は four_layer (約 1.8h) / stage_chain (約 4.5h) / cli_core (約 4.1h) の
  計 10.4h ある。10 core → 6 core に絞ると総計が 16h 規模になる。
  **未検証の仮説に 6 時間を払う判断にはならない**

したがって**次も既定並列度で回し、ホスト側を静かに保つ**ことだけを変える
(待機用 background job を置かない)。**同じ帯で再び SIGKILL されたら、そこで初めて
系統的な原因として並列度を絞る。** その順序をここに先に書いておく。

### 2 回目の取得条件 (**結果が出る前に記録する**)

| 項目 | 値 |
|---|---|
| 対象 | `selfhost_bootstrap_four_layer` -> `selfhost_native_stage_chain` -> `selfhost_cli_core` の **3 module を直列** |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` (1 回目と同一。`2caac21b` の probe 修正を含む。lane 中は再ビルドしない) |
| 起動 | `python3 /Users/biwakonbu/github/tmp/lane3/run_lane3.py` を `os.setsid()` で切り離し。pid 92253 |
| 並列度 | libtest 既定 (**1 回目から変えない**) |
| 併走 | `cargo` は一切起動しない |
| ログ | `/Users/biwakonbu/github/tmp/lane3/lane/mod-<module>.log` |
| 抜粋台帳 | `/Users/biwakonbu/github/tmp/lane3/subset-<module>.txt` (台帳 commit `9b4633a4` から module 名で再抽出。1 / 111 / 21 行) |
| 完走判定 | `MODEXIT` と `compare_ignored_lane.py` のみ。`LANE-COMPLETE` は使わない |

#### 満たせなかった条件: 「待機用 background job を置かない」

上の `#### 次の lane の条件をどうするか` で **「ホスト側を静かに保つことだけを変える
(待機用 background job を置かない)」** と決めたが、**これは満たしていない。**
lane の完了をエージェント側が検知する手段が待機プロセス以外に無いためである。

1 回目との差は polling 間隔だけにした (120s -> 900s)。**条件を静かに緩めたのではなく、
満たせないことをここに書く。** したがって 2 回目が完走した場合、
**「待機 job を消したから完走した」とは読めない。** 待機 job は 1 回目と同様に存在している。
逆に再び SIGKILL された場合も、待機 job を原因から除外することはできない。

`compare_ignored_lane.py` は lane 中に走らせても差し支えない (python の短時間実行) が、
判定は各 module の `MODEXIT` を読んでから行う。

### 結果 (2 回目 — 3 module とも完走)

| module | 宣言数 | 結果行 (ユニーク) | passed | failed | `MODEXIT` | `ELAPSED` | comparer |
|---|---|---|---|---|---|---|---|
| `selfhost_bootstrap_four_layer` | 144 | 144 | 143 | 1 | 101 | 5,816.58s | exit 0 |
| `selfhost_native_stage_chain` | 613 | 613 | 502 | 111 | 101 | 18,545.78s | exit 0 |
| `selfhost_cli_core` | 381 | 381 | 360 | 21 | 101 | 16,217.67s | exit 0 |

3 本とも `新規 FAIL 0 / 解消 0 / 未出現 0 / ログ間重複 0`、判定は
`OK -- 完走し、台帳と一致した`。lane 全体は 20:23:57 開始 / 翌 07:40:17 終了 (11h16m)。

`MODEXIT=101` は libtest が test 失敗で返す通常の終了コードである。
**1 回目の `-9` (SIGKILL) は再現しなかった。**

#### 予測との突き合わせ

- **「赤は 1 件だけ」(four_layer) — 当たった。** 実測 FAIL 1 件は `I-83` の
  `test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` で、台帳に残る four_layer の
  行と一致する。`I-82` / `I-85` で実質化した assertion が lane の順序依存で
  新しく落ちる、ということも起きていない
- **「宣言数 144」 — 1 回目で訂正した基礎のとおり。** 148 ではない。
  訂正は 1 回目の結果が出た後に書いたものだが、2 回目の予測としては結果より前に確定していた
- **SIGKILL の原因は依然として特定できていない。** 2 回目は完走したが、
  1 回目との差は「待機 job の polling 間隔 (120s -> 900s)」と
  「3 module 直列にしたこと」だけで、どちらも原因を説明する仮説ではない。
  **完走したことを「条件を改善したから」と読んではならない。**
  並列度は 1 回目と同じ libtest 既定である

#### 台帳の増減 (2 回目で確定)

| module | 台帳行数 | 内訳 |
|---|---|---|
| `selfhost_bootstrap_four_layer` | 1 | `I-83` のみ |
| `selfhost_native_stage_chain` | 111 | 実測 FAIL 111 と一致 |
| `selfhost_cli_core` | 21 | 実測 FAIL 21 と一致 |

`9b4633a4` で落とした 4 行 (削除済 probe 2 件 + 改名して緑になった 2 件) は、
comparer が `未出現` / `解消` を 0 と返したことで**過不足なく落ちていた**ことが確認できた。

#### 宣言数が `I-64` 時点から動いた分

| module | `I-64` 時点 | 本 lane | 差 | 説明 |
|---|---|---|---|---|
| `selfhost_native_stage_chain` | 615 | 613 | -2 | `I-84` #3 / #4 の probe 2 件を削除した |
| `selfhost_cli_core` | 381 | 381 | 0 | `I-84` #2 は削除ではなく極性の反転なので数は動かない |

## `SWEEP-LANE-RERUN-01` の共有 lane (2026-08-28)

`I-74` / `I-90` / `I-93` / `I-94` / `I-96` / `I-97` / `I-98` / `I-99` / `I-100` / `I-101` の
10 項目が同じ 1 本を待っている。項目ごとに lane を回さないと決めてあるので、まとめて 1 本で測る。
受入条件の正本は `TODO.md` の `SWEEP-LANE-RERUN-01`。

### 取得条件 (**結果が出る前に記録する**)

| 項目 | 値 |
|---|---|
| 対象 | `selfhost_cli_actual_main_args` -> `selfhost_cli_core` -> `selfhost_native_stage_chain` の **3 module を直列** |
| 順序の理由 | 所要の短い順。先に出た module から台帳突合を進められる (前回 lane3 は長い順で、最初の突合まで 1.6h 待った) |
| test binary | `target/debug/deps/e2e-aa343ded249bec81` / sha256 `cff50dd5...` / 14:24:45 build |
| binary を建て直した理由 | 前の binary は 13:44 build で、`cbafe700` (rustfmt) と `a0e30b38` より前だった。**整形だけでも HEAD を含まない binary の測定値は完走証拠にしない** |
| 起動 | `python3 /Users/biwakonbu/github/tmp/lane4/run_lane4.py` を `os.setsid()` で切り離し。pid 48035 (`PPID=1` を確認) |
| 並列度 | libtest 既定 (lane3 から変えない) |
| 併走 | `cargo` は一切起動しない (`scripts/audit_docs.sh` も cargo を呼ぶので lane 中は回さない) |
| ログ | `/Users/biwakonbu/github/tmp/lane4/lane/mod-<module>.log` |
| 抜粋台帳 | `/Users/biwakonbu/github/tmp/lane4/subset.txt` (台帳 sha256 `1d145bdd...` から module 名で再抽出。**133 行** = 1 / 21 / 111) |
| 完走判定 | `MODEXIT` と `compare_ignored_lane.py` のみ。`LANE-COMPLETE` は使わない |
| 分母 | `--list --ignored` 実測で 25 / **384** / 613 |

### 分母の取り違えを 1 件片付けた

`SWEEP-LANE-RERUN-01` の受入条件 (b) は `selfhost_cli_core` を **384** で判定すると定めているが、
ソースを `grep -cE '^\s*(async )?fn test_e2e_'` すると **439** が返る。この 2 つは食い違っていない。

| 数え方 | 値 | 何を数えているか |
|---|---|---|
| `grep 'fn test_e2e_'` | 439 | `test_e2e_` 接頭辞の宣言だけ |
| 宣言の全量 | 445 | 上に `test_selfhost_*` 名の 6 本を足したもの |
| `--list --ignored` | **384** | `#[ignore]` 付き = lane が実際に回す本数 |

lane は `--ignored` で回すので、分母は 384 が正しい。`AGENTS.md` に取り方を明記した。

### 予測 (**結果を見る前に書く**)

- `selfhost_cli_core`: 21 行のうち **7 行が緑に転じる** (`:402-403` の `I-90` 2 行、
  `:404-408` の `I-93`/`I-94` 5 行)。`:409` (`I-96`) も緑の見込みだが、
  focused run で測っていないのは `:409` だけなので、ここは実測で初めて決まる。残り 13〜14 行は赤のまま
- `selfhost_native_stage_chain`: 111 行のうち **2 行が緑に転じる** (`:411` の `I-99`、
  `:412` の `I-98`+`I-101`)。どちらも focused run では緑
- `selfhost_cli_actual_main_args`: 1 行は赤のまま (引き取り先の実装に手を入れていない)
- **`MODEXIT` は 3 本とも `101` を予想する。** `0` になったら台帳行が全部緑という意味なので、
  上の予測と矛盾する。`-9` は SIGKILL で測り直し
- 所要は lane3 と同程度 (**11 時間規模**)。`selfhost_cli_core` は 381 -> 384 で 3 本増えている

### 結果

(lane 完走後に埋める)

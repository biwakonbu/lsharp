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

**sweep 実行中 (18 module 中 11 完了、2026-08-23 16:03 時点)。** 完了分を先に記録する。
残り 6 module (`selfhost_bootstrap_acceptance` 28 / `selfhost_doctools_cli_diagnostics` 38 /
`selfhost_lsp_docs_ops` 54 / `selfhost_native_differential` 104 /
`selfhost_bootstrap_four_layer` 146 / `selfhost_cli_core` 381) と、
別プロセスが待機している `selfhost_native_stage_chain` 615。

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
| **小計** | **65** | **48** | **17** | 2950.22 | -- |

分母の和 65 は「対象の分母」表の同 11 module の宣言数と**過不足なく一致する**。
module 名の書き落としも filter の綴り違いも今のところ無い。

**1 件あたりの所要は module によって桁が違う。** `selfhost_rooting_parity` は 2 件 19.46s
(9.7s/件)、`selfhost_cli_actual_main_args` は 25 件 1279.36s (51.2s/件)。
後者は CLI bundle を毎回組み立てる test で、残る `selfhost_cli_core` 381 件が同水準なら
それだけで 5 時間規模になる。**全体の完走見込みは 12 時間以上**であり、
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

<!-- doc-GREEN: 残り 7 module 分をここへ追記する -->

## 再実行の手順

```bash
python3 scripts/compare_ignored_lane.py <log> [<log> ...]   # 台帳との 4 種差分。exit 0 が一致
bash scripts/ci/test-compare-ignored-lane.sh                # 上記の契約テスト (cargo 非依存)
```

lane 自体の回し方は `AGENTS.md` の「`--ignored` lane の実測と台帳突合」節を見よ。

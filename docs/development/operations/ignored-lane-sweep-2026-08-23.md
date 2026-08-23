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

<!-- doc-GREEN: sweep 完走後に module 別の所要・passed・failed と、
     compare_ignored_lane.py の 4 種差分の判定を埋める -->

未取得 (sweep 実行中)。

## 振り分け

<!-- doc-GREEN: 赤 1 本ずつを「修正する (別項目へ切る)」/「expected-failure として
     理由付きで台帳へ記録する」のどちらかへ割り当てた結果を書く -->

未取得。

## 再実行の手順

```bash
python3 scripts/compare_ignored_lane.py <log> [<log> ...]   # 台帳との 4 種差分。exit 0 が一致
bash scripts/ci/test-compare-ignored-lane.sh                # 上記の契約テスト (cargo 非依存)
```

lane 自体の回し方は `AGENTS.md` の「`--ignored` lane の実測と台帳突合」節を見よ。

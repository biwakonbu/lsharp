# ADR: heavy e2e 164 件の `#[ignore]` 契約をどちらへ寄せるか (TESTGATE-03)

- Status: 実装済み (2026-08-19)
- Date: 2026-08-18 (裁定) / 2026-08-19 (実装)
- Scope: `TESTGATE-03` / `I-22` /
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` /
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs` /
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` /
  `docs/development/validation/workspace-expected-failures.txt`
- Related: [`ISSUES.md` I-22](../../ISSUES.md#i-22)、
  [test gate 是正 ADR (TESTGATE-01/02)](decisions-test-gate-staleness-repair.md)、
  [lint dedup identity ADR](decisions-lint-diagnostic-dedup-identity.md)

## Context

`TESTGATE-01` で `test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` の構造的破損を
直したところ、**本物の違反 164 件**が現れた (`selfhost_cli_core.rs` 158 /
`selfhost_cli_actual_main_args.rs` 5 / `selfhost_native_stage_chain.rs` 1)。

`I-22` はこれを「規約 (prefix ルール) と実態 (`#[ignore]` の無い 164 件) のどちらが陳腐化して
いるか未決」として open のまま残し、`TODO.md` の `TESTGATE-03` に

> **判断材料**: 案 A は現在 run されて pass している 158 件を「どこでも走らない」状態にする
> (CI は停止中)。案 B は run set を 1,799 のまま保つので `I-11` の測定 anchor が全て有効に残る。

と書いていた。本 ADR はこの判断材料の**前半が事実誤認であること**を実測で示したうえで裁定する。

### 以前ユーザー判断待ちとしたことについて

本件は一度「規約側の意図の判断なので裁定を待つ」としてエスカレーションした。それを引き戻して
ここで裁定するのは、**提示した判断材料そのものに事実誤認があったから**である。誤った前提で
立てた問いの答えを待ち続ける理由は無い。以下の実測がその訂正にあたる。

## 実測 (2026-08-18)

### 訂正 1: 164 件は「どこでも走らない」状態にならない

`scripts/ci/compile-phase11-inputs.sh` は test を**厳密名ではなく prefix** で `--ignored`
付き起動する。164 件を prefix で照合すると **164/164 が既存の起動対象に入る**。

| 件数 | prefix | 起動箇所 |
|---:|---|---|
| 85 | `test_e2e_selfhost_test_runner_` | `compile-phase11-inputs.sh:242` |
| 73 | `test_e2e_selfhost_cli_` | `compile-phase11-inputs.sh:240` / `:244` |
| 5 | `test_e2e_selfhost_cli_main_with_args_` | `compile-phase11-inputs.sh:238` |
| 1 | `test_e2e_selfhost_pipeline_smoke_` | `compile-phase11-inputs.sh:296` |
| **0** | (未カバー) | -- |

`compile-phase11-inputs.sh` 自体は `.github/workflows/ci.yml:85` /
`scripts/release-playbook.sh:61` / `scripts/ci/test-rust-free-command-boundaries.sh:60`
から呼ばれる。したがって案 A を採っても 164 件は **phase11 lane で走り続ける**。
最初にこれを 0/164 と測ったのは厳密名 grep によるもので、script 側が prefix 起動である
ことを見落としていた。

**ただし緩めずに書く**: CI 自動実行は 2026-07-12 から停止したままである (`I-19`)。
案 A を採った直後の実行経路は「phase11 script の手動実行」と「release playbook」の 2 つだけで、
自動的に回るものは無い。「どこでも走らない」は誤りだが、「default の workspace run からは
確実に抜ける」は正しい。

### 訂正 2: 案 B の「prefix ルールが過広」という見立ては支持されない

prefix ルールは phase11 script の起動 prefix と**正確に鏡写し**の関係にある。ルールを絞れば
「gate が要求する集合」と「script が実際に回す集合」が非同期になるか、除外した分が default run に
残って `I-11` の 5h38m 問題が続くかのどちらかになる。164 件は 2026-07-17 以降のドリフトであり
(`I-22` の履歴表)、ルールが最初から広すぎたわけではない。

### 案 A の実測コスト 3 点

1. **default run set が 1,799 → 1,635 になる**。`I-11` の直近計測の分母が歴史値になる。
   ただし **baseline の FAIL 集合は不変** — 164 件のうち 161 件は現在 pass しており、
   `check-workspace-baseline.sh` の 5 条件はどれも発火しない。
2. **例外が 3 件ある**。164 件のうち以下の 3 件は `workspace-expected-failures.txt:54-56`
   に expected FAIL として載っている。`#[ignore]` を付けると junit から消えるため、
   checker の「expected が消えた」条件が発火する。**同じ変更でこの 3 行も外す必要がある**。

   ```
   e2e::selfhost_cli_core::test_e2e_selfhost_cli_check_file_resolves_imported_definition
   e2e::selfhost_cli_core::test_e2e_selfhost_cli_check_reports_invalid_canonical_case
   e2e::selfhost_cli_core::test_e2e_selfhost_cli_validate_source_json_reports_contradicting_evidence
   ```

3. **`DIAG-DEDUP-01` の pin 7 本が 164 件に含まれる**
   (`selfhost_cli_core.rs:17886` / `:17913` / `:17940` / `:18049` / `:18077` / `:18105` / `:18227`、
   いずれも `test_e2e_selfhost_cli_lsp_stdio_didopen_*`)。案 A を採ると
   [lint dedup identity ADR](decisions-lint-diagnostic-dedup-identity.md) の受入条件
   「7 件が pass」は default run では検証されず、phase11 lane での検証になる。
   同 ADR と `LINT-SPAN-01` / `LSP-DEDUP-MERGE-01` の受入条件は、この lane 移動を前提に読む。

### heaviness の根拠

`DIAG-DEDUP-01` の調査で実測したのは **7 件サンプルの ~186s/件**であり、164 件全部を計測した
わけではない。残り 157 件が同じ selfhost fixture 経路を通ることからの推論である。
「164 件が heavy だと実測した」とは書かない。

## Decision

**案 A を採る。** 164 件に `#[ignore]` を付け、phase11 script を唯一の実行 lane とする。
prefix ルール (規約) は変えない。

決め手は `DIAG-DEDUP-01` (`I-24`) と同じ立て方をした天秤である。あの件では
「規約どおりに直すと利用者から見える指摘が消える」という**具体的な損失**が決め手だった。
本件で案 A の側に想定していた損失 (「158 件がどこでも走らなくなる」) は実測で消えた。
残るのは分母の変化と expected FAIL 3 行の付け替えという**機械的なコスト**だけであり、
案 B が招く「gate と runner の非同期」より軽い。

## 却下した選択肢

- **案 B: prefix ルールを絞る** — 却下。上記「訂正 2」のとおり、ルールは phase11 script の
  起動 prefix と鏡写しなので、絞ると契約と runner が食い違う。除外分を default run に残せば
  `I-11` の 5h38m 問題がそのまま続く。「ルールが過広」という当初の見立ては、増加が
  2026-07-17 以降に集中しているという履歴実測に反する。
- **CI を再開してから決める** — 却下。`I-19` / `SMOKE-GATE-03` は本項目のスコープ外であり
  (`TODO.md` の `TESTGATE-03` が明示)、CI 再開を待つと gate が expected FAIL のまま滞留する。
  案 A は CI が止まっていても release playbook 経由で lane が生きているので、再開を待つ必要が無い。
- **164 件を高速化・統合して default run に残す** — 却下。`TESTGATE-03` の
  「含めない範囲」に明記済み。test 自体の設計変更であり、契約の裁定とは別の作業。

## 実装をこのブランチに載せない理由 (裁定時点の判断。2026-08-19 に別 slice で実装した)

3 つある。(1) `ops03c` を GREEN にする検証は test run を要し、現在 sweep が CPU を占有している。
(2) merge 対象の slice (`NATIVE-ROOT-01` / `DIAG-DEDUP-01`) に無関係な 164 ファイル変更が混ざる。
(3) run set の変更は `I-11` の baseline 直後の値と絡むので、merge 後に単独の slice で扱うほうが
revert 単位として正しい。

## Evidence

2026-08-19、worktree `lsharp-diag-dedup` / branch `codex/testgate-03` (`main` `6680e991` 起点) で実装した。

### 変更内容

| 対象 | 変更 |
|---|---|
| `selfhost_cli_core.rs` | `#[ignore]` 158 行追加 |
| `selfhost_cli_actual_main_args.rs` | `#[ignore]` 5 行追加 |
| `selfhost_native_stage_chain.rs` | `#[ignore]` 1 行追加 |
| `workspace-expected-failures.txt` | 4 行削除 + コメント 2 ブロック書き換え |

test 本体には一切手を入れていない。`git diff --stat` は test 3 ファイルで
**166 insertions / 0 deletions** (`#[ignore]` 164 行と `workspace-expected-failures.txt` の
コメント 2 行)、削除は expected-failures の 6 行のみ。

offender の特定は `test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` の panic 出力から
`file:line` を抽出して行った。panic header 自身の行 (`selfhost_lsp_docs_ops.rs:3891`) が
素朴な grep では 165 件目として混ざるので、**ヘッダ以降の行だけを取る**必要がある。
除外後は 164 件ちょうどで、内訳も doc-RED 時点の記録 (158 / 5 / 1) と一致した。

挿入位置は `fn` 行の直前である。`has_ignore_attribute`
(`selfhost_lsp_docs_ops.rs:3339-3351`) は `fn` 行から遡って**最初の非空行 1 行だけ**を見て
`#[ignore` で始まるかを判定するので、`#[test]` と `fn` の間に置かないと通らない。
`#[cfg_attr(..., ignore)]` 形も通らない。

### 受入判定

| 受入条件 | 結果 |
|---|---|
| `ops03c` が GREEN | **達成**。`test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted ... ok` |
| 同じ検査系を巻き添えにしていない | **達成**。`ops03` / `ops03b` / `ops03c` / `ops03d` の 4 件が ok, 0 failed |
| baseline checker が壊れない | **達成**。`scripts/ci/test-check-workspace-baseline.sh` exit 0 (PASS) |

### run set の実測 — 予測とのズレ 1 件

`--list` で前後を実測した (変更 3 ファイルを `git stash` して測り、`stash pop` で戻した)。

| | 合計 | ignored | default で走る |
|---|---|---|---|
| 変更前 | 3,062 | 1,262 | 1,800 |
| 変更後 | 3,062 | 1,426 | 1,636 |

**移動量は 164 でちょうど一致**し、合計は動いていない。
doc-RED 時点の予測は「1,799 → 1,635」だったので**両側とも 1 件多い**が、
差は分母 (合計 test 数) 側にあり、移動量 164 の側ではない。
予測を書いた 2026-08-18 以降に merge した slice が default test を 1 件増やしたためと見られるが、
**どの test かは特定していない**。ここは「予測が 1 件ずれていた」以上のことは書かない。

### 削除した expected FAIL 4 行について

doc-RED の「実測コスト 2」が挙げた 3 行 (`selfhost_cli_core` の
`check_file_resolves_imported_definition` / `check_reports_invalid_canonical_case` /
`validate_source_json_reports_contradicting_evidence`) は、164 件の**名前で交差を取り直して**
実測で確認した。行番号 `:54-56` は merge でファイルが動いた後も一致していた。

**doc-RED に無かった 4 行目がある。** `ops03c` 自身
(`e2e::selfhost_lsp_docs_ops::test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted`) が
expected FAIL に載っていたので、GREEN になった以上これも同じ変更で外さないと
checker の「expected が pass に転じた」条件が発火する。順序は
**「ops03c を実走して GREEN を確認 → 行を削除」**とした。

コメントブロックも実態へ合わせた。`[selfhost_cli_core]` の「4 要因」は残存 1 行分へ、
`[selfhost_lsp_docs_ops]` の「3 要因」は解消後の 2 要因へ書き直し、
どちらも**削除した要因が「直った」のではなく「lane が移った」だけ**である旨を残した。

### 満たせなかった条件 — 緩めずに書く

1. **`--ignored` lane の再走をしていない。** 164 件が phase11 lane で実際に走ることは、
   doc-RED 時点で `compile-phase11-inputs.sh` の prefix 被覆を読んで確認した推論である。
   今回それを**実行では確認していない** (該当 lane は 1 件 ~186s サンプルで、全件 5 時間規模)。
   なお 164 件は今回はじめて「default では走らず ignored lane でのみ走る」状態になったので、
   phase11 lane の実効カバレッジが変わる。この検証は次の lane 実走時に行う。
2. **削除した expected FAIL 3 行が「もう FAIL しない」ことを実測していない。**
   根拠は「`#[ignore]` は junit に載らない」という semantics からの帰結であって、
   前後の junit 比較ではない。3 件の未実装挙動そのものは直っていない
   (`[selfhost_cli_core]` のコメントにその旨を明記した)。
3. **164 件が heavy であることは依然として推論のまま。** doc-RED の「heaviness の根拠」節から
   変わっていない。実測したのは 7 件サンプルの ~186s/件だけである。

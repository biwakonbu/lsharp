# `completion-criteria.md` が赤い test を達成根拠に名指しする問題の裁定

- **Status**: doc-GREEN (2026-08-28)
- **Date**: 2026-08-28 (doc-RED) / 2026-08-28 (実装)
- **Scope**: `docs/development/planning/completion-criteria.md` が名指しする test 名のうち、
  `docs/development/validation/ignored-lane-expected-failures.txt` に期待 FAIL として
  載っている 3 件の扱い。および再発検出を `scripts/audit_docs.sh` へ足すかの決定。
- **含めない範囲**: 3 件の赤そのものの修正 (`I-78` / `REPL-TYPE-TAG-01` が持つ)。
  `docs/development/validation/workspace-expected-failures.txt` 側との照合
  (`I-11` の baseline が固まってから)。CI workflow の変更 (`SMOKE-GATE-03`)。
- **Related**: `ISSUES.md` の `I-104` / `TODO.md` の `COMPLETION-CRITERIA-RED-CITE-01` /
  `I-78` (bootstrap self-feed trap) / `I-69` (REPL 型名破損) / `DOC-08` (同型の陳腐化)

## 照合の実測

両 file から test 名を抽出して積を取った (cargo 不要、文字列処理のみ)。

- `completion-criteria.md` の test 名: 25 件 (`` `test_[a-z0-9_]+` `` 形)
- 台帳の test 名: 183 件 (`::test_[a-z0-9_]+` 形)
- **積: 3 件**

`I-104` が記録した 4 件のうち 1 件 (`..._compile_target_and_output_path`) は
`I-94` の裁定で既に根拠から外してあるので、残りが 3 件で一致する。

## 判断

**3 件は 2 通りに分かれる。「赤いから一律に外す」ではない。**

### 1. Gate 1 (Wasm bootstrap fixed-point) -- **`[done]` を取り消す**

名指しは `test_e2e_bootstrap_stage2_self_feed_fixed_input_set` と
`test_e2e_bootstrap_fixed_input_set_stage_chain_match` の 2 件。どちらも
**実装が未達であることを理由に赤い**。

- 台帳の引き取り先は `I-78` で、状態は `open`。
  「`src/App/Cli.ls` の self-feed compile が `integer divide by zero` で trap する」
- `src/App/Cli.ls` は Gate 1 が数える fixed input set 54 件 (selfhost 40 / stdlib 11 /
  examples 3) の**構成員**である。台帳の注記がその path を名指ししていることが証拠になる
- したがって Gate 1 の条件文
  「full input set (selfhost/stdlib/examples) に対する `stage1 -> stage2 -> stage3` の
  実体生成・比較」は**満たされていない**

`scripts/ci/compile-phase11-inputs.sh:131-148` は `RUN_BOOTSTRAP_FIXED_POINT=1` のときだけ
この 3 本を `--exact --ignored` で回す。既定は `0` (`:8`) なので、**既定の CI 経路はこの gate を
一度も実行していない。** 「ローカル再実行では script 全体が exit 0」という現況記述は
既定モード (`=0`) の観測としてなら正しいが、gate の根拠にはならない。

**よって Gate 1 は `[done]` -> `[in-progress]` へ戻す。** 数字を静かに直さず、
訂正の経緯と根拠を現況欄に残す。

### 2. Gate 2 (GC 有効 runtime stability) -- **`[done]` は維持し、名指しだけを外す**

名指しは `test_e2e_alloc_metrics_ci_artifact_payload` 1 件。こちらは
**そもそも gate の主張を支えていない誤引用**である。

- 当該 test (`runtime_allocator_closures.rs:1350`) が見ているのは
  `__alloc` を 5 回叩いてアドレス差を print することと、50 回ループで
  アドレスが単調増加すること (**bump allocator の性質**) である
- Gate 2 の主張は「collector-backed `summary.json` / `collector-proof.json` を
  required job から保存できる」こと。この JSON を作るのは
  `scripts/ci/collect-gc-metrics.sh` であって当該 test ではない
- すなわち test が緑であっても赤であっても、Gate 2 の根拠にはならない

台帳の引き取り先は `REPL-TYPE-TAG-01` (`I-69`) だが、**当該 test は REPL 経路を通らない**ので
この帰属が正しいかは本 ADR では判定しない (`I-104` の隣接所見として残す)。
赤の原因を確定させなくても、**誤引用であることは原因と独立に言える**。

## 却下した案

- **案 A: 3 件とも根拠から外して gate は `[done]` のまま**
  却下。Gate 1 の 2 件は誤引用ではなく実装未達の証拠そのものである。
  外すと「full input set の compare が通っている」という誤った状態が残る。
  **これは数字を静かに直す形にあたる。**
- **案 B: 3 件とも gate を `[in-progress]` へ戻す**
  却下。Gate 2 の 1 件は gate の主張と無関係なので、これを理由に gate を戻すのは
  根拠のない後退になる。`collect-gc-metrics.sh` と required job という別の根拠は
  本 slice で否定されていない。
- **案 C: 赤い test の名指しを一律禁止する**
  却下。「なぜ未達か」を書くには赤い test を名指しできる必要がある。
  Gate 1 の訂正文が現にそれを要求する。禁止ではなく**注記の強制**にする。
- **案 D: 照合を `audit_docs.sh` へ足さない**
  却下。`I-104` が記録したとおり、**台帳に載っている名前が達成根拠に出ていないかを
  見る経路が今どこにも無い。** 人手の照合は同じ見落としを繰り返す。
  照合は文字列処理だけで cargo を呼ばないので、`audit_docs.sh` の既存 check と同じ費用で入る。

## 採用する再発防止

`scripts/audit_docs.sh` に照合 check を足す。

- `completion-criteria.md` の `` `test_...` `` と台帳の `::test_...` の積を取る
- 積が空でなければ **ERROR**
- **例外は同一行の `[赤: <引き取り先>]` 注記でのみ認める。**
  引き取り先を書かせることで「赤いと分かったうえで名指ししている」ことを明示させる。
  Gate 1 の訂正文はこの形を使う

`workspace-expected-failures.txt` 側は含めない。`I-11` の baseline が
プレースホルダのままなので、今足すと意味のない差分を拾う。

## Evidence

### RED

照合 check を `scripts/audit_docs.sh` へ足した直後に回して、赤の名指しが検出されることを先に見た。

    --- [I-104] completion-criteria が赤い test を達成根拠に名指ししていないか ---
      ERROR: 期待 FAIL の test を達成根拠に名指ししている箇所が 5 件
        completion-criteria.md:27: test_e2e_bootstrap_stage2_self_feed_fixed_input_set
        completion-criteria.md:27: test_e2e_bootstrap_fixed_input_set_stage_chain_match
        completion-criteria.md:123: test_e2e_bootstrap_stage2_self_feed_fixed_input_set
        completion-criteria.md:123: test_e2e_bootstrap_fixed_input_set_stage_chain_match
        completion-criteria.md:129: test_e2e_alloc_metrics_ci_artifact_payload
    === 監査完了: エラー 5 件, 警告 0 件 ===

test 名は 3 件、出現は 3 行 5 箇所である。

### GREEN

上の判断どおりに直したあと、`bash scripts/audit_docs.sh` は **エラー 0 件 / 警告 0 件**。

    --- [I-104] completion-criteria が赤い test を達成根拠に名指ししていないか ---
      OK: 赤い test を無注記で名指ししている箇所なし

### check が本当に効くことの確認 (negative test)

**GREEN だけでは「check が常に OK を返すだけ」の可能性を排除できない**ので、
偽の名指しを 1 行足して非 0 になることを確かめた。

    - NEGATIVE TEST: `test_e2e_bootstrap_stage2_self_feed_fixed_input_set` を無注記で名指し

    ERROR: 期待 FAIL の test を達成根拠に名指ししている箇所が 1 件
      completion-criteria.md:177: test_e2e_bootstrap_stage2_self_feed_fixed_input_set

確認後に当該行を除去し、再び エラー 0 件へ戻ることも確かめた。

### 実際に直したもの

| 場所 | 変更 |
|---|---|
| `completion-criteria.md` 「監査整理 / bootstrap」 | 「BOOT-04 完了証跡は full input set compare まで到達した」を撤回。2 件に `[赤: I-78]` を付与 |
| `completion-criteria.md` ゲート 1 見出し | `[done]` -> `[in-progress]` |
| `completion-criteria.md` ゲート 1 現況 | 2 件に `[赤: I-78]` を付与。既定が `RUN_BOOTSTRAP_FIXED_POINT=0` である事実を追記 |
| `completion-criteria.md` ゲート 1 達成 | 「達成」を「未達 (訂正 2026-08-28)」へ。訂正前の文言と原因を明記 |
| `completion-criteria.md` ゲート 2 現況 | `test_e2e_alloc_metrics_ci_artifact_payload` の名指しを削除 |
| `completion-criteria.md` ゲート 2 | 訂正の経緯を追記。**gate の状態は戻していない** |
| `phase11-implementation-plan.md` BOOT-04 Current state | 「full-set 実体比較まで到達した」を撤回 (下記) |
| `scripts/audit_docs.sh` | 照合 check を追加 |

### スコープを広げた 1 件

`completion-criteria.md` を直す過程で、**`phase11-implementation-plan.md` の BOOT-04
「Current state」が同じ誤った到達主張を持っている**ことが分かった。同一の欠陥・同一の根拠なので
本 slice で直した。

**ただし照合 check の対象は `completion-criteria.md` だけに留めた。**
`phase11-implementation-plan.md` の「Acceptance」節は**通るべき test を列挙する場所**であり、
そこに赤い test 名が出るのは矛盾ではない。同じ check を当てると正当な記述を大量に
false positive で叩くことになる。2 つの file は test 名の意味が違う。

## 満たせなかったこと

- **赤 3 件そのものは直していない。** `I-78` は `open` のままで、
  `TODO.md` の `CLI-SELFFEED-DIVZERO-01` が引き取る。本 ADR は台帳の記述を正しくしただけである。
- **`test_e2e_alloc_metrics_ci_artifact_payload` が赤い原因は確定していない。**
  台帳の引き取り先は `REPL-TYPE-TAG-01` (`I-69`) だが、当該 test は REPL 経路を通らない。
  帰属が正しいかは `I-104` の隣接所見として残す。誤引用の判定はこの原因と独立に成り立つ。
- **Gate 2 の残る根拠 (`collect-gc-metrics.sh` と required job `gc-metrics-artifact`) を
  再実行していない。** CI はスコープ外である (`SMOKE-GATE-03`)。gate を `[done]` のままに
  したのは「本 slice で否定されなかった」という意味であって、再検証した結果ではない。
- **`workspace-expected-failures.txt` 側との照合は入れていない。** `I-11` の baseline が
  プレースホルダのままなので、今足しても意味のある差分にならない。
- **`[赤: ...]` の例外判定は行単位である。** 1 行に赤い test 名が 2 つ以上あり、
  そのうち 1 つだけに注記が付いている場合、**残りも黙って例外になる。**
  現状の `completion-criteria.md` は注記付きの行の赤 test 名がすべて注記対象なので
  実害は出ていないが、これは check の false negative である。
  名前ごとの判定にするには注記を `[赤: <test 名> = <引き取り先>]` の形にする必要があり、
  記述が冗長になるので今回は採らなかった。**1 行に赤を 2 つ並べないこと**で運用する。

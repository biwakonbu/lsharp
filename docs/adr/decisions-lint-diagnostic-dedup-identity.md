# ADR: 診断の「重複」定義に lint の rule identity を含める (AC-209 の文言是正)

- Status: Accepted (doc-RED)
- Date: 2026-08-18
- Scope: `DIAG-DEDUP-01` / `I-24` / `docs/development/planning/toolchain-parity-spec.md` AC-209 /
  `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` TEST-LSP-15c / 15e
  (対象は「何を重複とみなすか」の定義のみ。lint の span 精度そのものは含めない)
- Related: [`ISSUES.md` I-24](../../ISSUES.md#i-24)、
  [toolchain-parity-spec T4b-3](../development/planning/toolchain-parity-spec.md)

## Context

診断の重複除去は 3 つの正本が互いに食い違っている。**2 者間の drift ではなく 3 者間の衝突**である。

| # | 正本 | 主張 | 現状 |
|---|---|---|---|
| 1 | spec AC-209 (`toolchain-parity-spec.md:169`) | 「同一 span に対する重複診断は severity の高い方のみ残す」。rule に関する例外は**書かれていない** | 文言のみ |
| 2 | `test_e2e_selfhost_lsp_render_sorted_deduped_diagnostics_json` (TEST-LSP-15c) と `..._snapshot` (15e) | AC-209 の文言どおり。同一 start span の lint 2 件 (rule 201 / 202) を 1 件へ潰すことを要求 | **FAIL** (workspace baseline に 2 件として登録済み) |
| 3 | `LspServerNav.ls` の `dedup-diag-same-lint-identity` (`:1225-1245`) と、それを pin する 2 test | lint 同士は **rule と start/end span がすべて一致した場合だけ**重複。rule が違えば両方残す | pass |

3 の側の pin は 2026-08-11 `c00368ad` 「fix(native): preserve distinct lint diagnostics」で入った
LSP wire レベルの e2e 2 本である。

- `test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_multiple_diagnostics_in_stable_order` (TEST-CLI-02-AN32i)
- `test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_distinct_same_start_diagnostics` (TEST-CLI-02-AN32j)

AN32j は `(defn main [] (let [unused (do)] 0))` という実ソースに対し、`L0001`
「let binding unused is not used」と `L0002`「do block has no expressions」が**どちらも**
`publishDiagnostics` に載ることを要求する。両者は range `0:0..0:0`、severity 2 で
start も end も完全に一致する。AC-209 の文言をそのまま実装すると、この 2 件のうち片方は
利用者から**黙って消える**。

`c00368ad` の commit message は 1 行で、判断の根拠を残していない。したがって設計意図は
test の側から再構成するほかない。

## Decision

**AC-209 の「重複」の定義を是正する。** lint 診断については rule identity を重複判定に含める。
すなわち採用するのは実装側 (3) の意味論であり、spec の文言 (1) と、その文言に忠実な
test (2) の期待値を書き換える。

新しい AC-209 の文言:

> AC-209: 同一 span に対する重複診断は severity の高い方のみ残す。ただし lint 診断
> (`source = lint`) は rule ID が異なれば別の指摘であり、span が一致しても重複とはみなさない

## 却下した選択肢

### 案 B: 実装を AC-209 の文言へ寄せる (同一 span の lint は rule を問わず潰す)

**却下。利用者から見える指摘が消えるため。** AN32j の実ソースがそのまま反例になる。
`L0001` と `L0002` は原因も対処も別で、片方だけ表示されると残りは次の編集まで発見できない。
「spec の文言が正しいのだから実装を直す」という手順の正しさが、成果物を劣化させる。

なお案 B を採ると `c00368ad` の 2 test を落とすことになる。**pass している test を落として
FAIL している test を通す**取引であり、収支としても成立しない。

### 案 C: lint の span を精密化し、重複が自然に起きないようにする

**却下 (代替案としては)。前提が成り立たない。** 案 C を完遂しても、同一の識別子に対して
異なる 2 つの lint rule が正当に発火するケースは残る (例: 未使用かつ shadowing)。
span が精密になっても span は一致しうるので、**rule identity による判別は依然として必要**である。

つまり案 C は AC-209 の文言問題を解かない。**恒久的に正しい「重複」の定義は rule identity を
含むもの**であり、本 ADR はそれを「案 C までの暫定免除」としては書かない。

ただし span の精密化そのものは独立した品質課題として価値がある (現状 `L0001` / `L0002` が
ともに `0:0..0:0` に落ちるのは lint span の投影が未実装だからである)。**補完的な follow-up
として `LINT-SPAN-01` を採番して登録する**。本 ADR の判断は `LINT-SPAN-01` の完了後も変わらない。

### 案 D: 判断を保留し、baseline に 2 件を残したままにする

**却下。** 「どちらが陳腐化しているか未決」という状態が、AC-209 に当たれば決められることを
2026-08-16 から先送りしていた。`ISSUES.md:1201` は次の「規約 vs 実態」issue も
**本件の裁定に倣う**と書いており、保留は他 issue の解決も止める。

## 実装範囲

コード変更は**無い**。`dedup-diag-same-lint-identity` は既に採用する意味論を実装している。
変更するのは以下 5 箇所。

1. `docs/development/planning/toolchain-parity-spec.md` の AC-209 文言
2. TEST-LSP-15c の期待 JSON (2 要素 → 3 要素)
3. `tests/snapshots/lsp/diagnostics/sorted-deduped-diagnostics.json` (15e の期待値。
   insta ではなく `read_to_string` で読む手書き snapshot なので、`cargo insta accept` の
   対象外である)
4. `docs/development/validation/workspace-expected-failures.txt` の
   `selfhost_lsp_docs_ops` ブロック (5 件 / 4 要因 → 3 件 / 3 要因)
5. `ISSUES.md` の `I-11` 内 3 箇所の件数と、新規 `I-24`

**test の期待値を実装に合わせて書き換える**のは `CLAUDE.md` が原則として禁じている操作であり、
本 ADR がその例外の記録である。例外を認める根拠は「実装が正しいから」ではなく
「**test が写している spec の文言のほうが、実運用の要求を取りこぼしている**」ことにある。

## 派生して見つかった問題 (本 slice では直さない)

`merge-duplicate-diagnostics` (`LspServerNav.ls:1169`) は同一 start span を **rule を問わず**
潰す。`dedup-diagnostics` と逆の意味論を持つ重複実装だが、呼び出し元は
`LspServer.ls:144` の検証用 `main` と parity test 3 本だけで、**実運用の publish 経路には
入っていない** (実運用は `Cli.ls:1440` / `:1687` / `:1702` の
`(dedup-diagnostics (sort-diagnostics ...))`)。

死んでいるとまでは言えない (`main` 経由で pin されている) が、同じ概念の実装が 2 つあり
片方だけが正しい状態なので、単一正本化の対象として `I-24` に併記する。

## Evidence

すべて 2026-08-18、worktree `/Users/biwakonbu/github/tmp/lsharp-diag-dedup`
(branch `codex/diag-dedup-01`、`codex/native-root-01` の tip `8a20cfe2` から分岐) で実測。

### RED (期待値を書き換える前の実測)

TEST-LSP-15c / 15e はどちらも同じ差分で落ちていた。**期待値は実測出力から貼るのではなく、
sort key (`source*100000000 + sev*1000000 + line*10000 + col`) から先に導出し、
実測と一致することを確認する**手順を採った。

- 導出: diag-c `101020004` < diag-b `301050009` < diag-a `302050009` → `c, b, a` の 3 件
- 実測 (`left`): `[..rule 203.., ..rule 202.., ..rule 201..]` の 3 件

両者は一致した。

### GREEN

| 検査 | 結果 |
|---|---|
| `e2e::selfhost_lsp_docs_ops::test_e2e_selfhost_lsp_render_*` 6 件 (15c / 15e を含む) | 6 passed / 0 failed、20.95s |
| `lsharp-wasm --test lsp_diagnostic_parity` 15 件 | 15 passed / 0 failed、97.16s |
| `e2e::selfhost_lsp_docs_ops::test_e2e_selfhost_lsp_*` 27 件 | 26 passed / **1 failed**、600.46s |
| `e2e::selfhost_cli_core::test_e2e_selfhost_cli_lsp_stdio_didopen_*` 7 件 (AN32j を含む) | 7 passed / 0 failed、1300.48s |

4 行目は本 ADR の決め手にした AN32j
(`test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_distinct_same_start_diagnostics`) を含む
LSP wire 経路の pin であり、rule identity を残す挙動が publish 経路でも保たれていることを示す。
この 7 件は `dedup-diagnostics` (production 経路) を通るため、`merge-duplicate-diagnostics`
(`LSP-DEDUP-MERGE-01`) の意味論とは独立に成立する。

**満たせなかったもの**: 上記 3 行目の 1 FAIL は
`test_e2e_selfhost_lsp_type_diagnostics_use_standard_projection` で、
type diagnostic の range が `0:13..0:23` を期待されるのに `0:0..0:0` になる。
これは baseline の `selfhost_lsp_docs_ops` ブロックが要因 (2) として持つ
**既存の未実装** (標準 LSP Diagnostic 配列投影) であり、本変更由来ではない。
本 slice ではこの行を baseline から外さない。

なお**この FAIL は `LINT-SPAN-01` と同じ根**を持つ (診断の span 投影が全体に未実装で
`0:0` へ落ちる)。lint 側だけでなく type 側にも同じ穴があることを示している。

### baseline の更新

`workspace-expected-failures.txt` の e2e 件数を **52 → 50**、`selfhost_lsp_docs_ops`
ブロックを **5 件 / 4 要因 → 3 件 / 3 要因** へ更新した。
`check-workspace-baseline.sh` は「expected が pass に転じても非 0」を含む 5 方向で落ちるため、
**この更新を同じ commit に含めないと baseline gate が壊れる**。
実 test 行数は e2e 50 + 非 e2e 31 = 81 で、ヘッダの申告値と一致することを確認した。

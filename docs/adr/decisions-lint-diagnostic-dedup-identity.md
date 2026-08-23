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

---

## 追記 (2026-08-23): pin の再建方法 — `LINT-DEDUP-PIN-01` / `I-58`

本 ADR の裁定を pin していた AN32j
(`test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_distinct_same_start_diagnostics`) は、
`LINT-SPAN-01` で real span が入った結果、fixture の 2 診断が同一開始位置ではなくなり
(`L0001` = 20..26 / `L0002` = 28..30)、**pass したまま裁定に触れなくなった**。
再建にあたって、`TODO.md` の `LINT-DEDUP-PIN-01` が当初書いていた受入条件

> 開始位置が実際に一致する 2 診断を含む fixture を作り、rule が異なれば両方 publish される
> ことを検査する test を 1 本置くこと

は、**そのままでは満たせない**ことが実装読解で判明した。

### 受入条件が満たせない理由 (実装の非対称性)

`dedup-diag-same-span` は 2 引数が対称ではない。`dedup-find-span`
(`LspServerNav.ls:1241-1246`) が `(dedup-diag-same-span (vector-get result idx) diag)` と
呼ぶので、第 1 引数 `a` は**既に result に入っている診断**、第 2 引数 `b` は**これから
入れる診断**である。同一 start に対する分岐は次のとおり。

| result 側 `a` | 入力側 `b` | 判定 | 結果 |
|---|---|---|---|
| lint (`source=3`) | lint | `dedup-diag-same-lint-identity` — rule と end が全一致した時だけ 1 | rule が違えば**両方残る** |
| lint (`source=3`) | type / parse | 0 | 両方残る |
| type / parse | 何でも | **1 (無条件)** | severity 高い方だけ残る |

3 行目は AC-209 の元の意味論 (「parse/type の既存 same-start precedence は維持し」の
コメントが実装に付いている) をそのまま保存したものである。

そして実運用経路は `(dedup-diagnostics (sort-diagnostics diagnostics))`
(`Cli.ls:1474` / `:1739` / `:1754`) であり、`sort-diagnostics` の order key は
`source*100000000 + ...` なので **type (`source=2`) は lint (`source=3`) より必ず先に並ぶ**。
つまり同一開始位置の type + lint ペアを作れたとしても、type が先に result へ入り、
3 行目の分岐が効いて **lint のほうが黙って消える**。

したがって「type 診断と lint 診断を突き合わせる」という当初の攻め筋で書いた test は、
本 ADR の裁定 (rule が違えば両方残す) **とは逆の契約**を pin することになる。
lint 同士で同一 span を作れない (`let` / `do` が互いに素な kind) ことは `I-58` が既に
記録しており、e2e ソース fixture 経由でこの裁定を pin する手段は**存在しない**。

### 決定: `dedup-diagnostics` を直接呼ぶ関数レベル pin にする

`selfhost_cli_runtime_bundle()` に harness を連結して `dedup-diagnostics` を直接呼び、
診断ベクタを合成して契約を pin する。既存の
`test_e2e_selfhost_lsp_position_from_offset_covers_line_boundaries`
(`selfhost_cli_core.rs:18773`) が同じ形式で `lsp-position-from-offset` を pin しており、
新しい仕組みは要らない。

pin する分岐は 3 つで、片側だけでは通らないようにする。

1. lint + lint / 同一 start / **rule 相違** → 2 件残る (本 ADR の裁定そのもの)
2. lint + lint / 同一 start / 同一 rule / 同一 end → 1 件へ潰れる (dedup が実際に効く)
3. lint + lint / 同一 start / 同一 rule / **end 相違** → 2 件残る (end が判定に参加する)

1 だけだと「dedup が常に何もしない」実装でも通る。2 を足すことで失敗力を持たせる。

### 却下した選択肢

#### 案 B: 同一開始位置の type + lint fixture を作り、両方 publish を要求する

**却下。本 ADR の裁定と逆の契約を pin することになる。** 上表 3 行目のとおり、
実装は同一 start の type を lint より優先して lint を落とす。これは AC-209 の
既存 precedence であって bug ではない。この test を書けば FAIL し、通すには
precedence を壊す実装変更が要る — `I-24` が扱っていない範囲への越境である。
`LS1002` の span 決定規則を調べる作業も、この時点で不要になる。

#### 案 C: 3 つ目の lint rule が入るまで待つ

**却下。期限が無い。** `review-*-diagnostic` は 2 rule しかなく、追加の予定も無い。
本 ADR は「未使用かつ shadowing」のような将来の同一 span 発火を裁定の根拠に挙げており、
**その将来が来る前に契約が守られていることを検査できなければ pin の意味が無い**。

#### 案 D: `merge-duplicate-diagnostics` 側に寄せる

**却下。** そちらは rule を問わず潰す逆の意味論で、実運用の publish 経路に入っていない
(本 ADR「派生して見つかった問題」節)。pin 対象として誤り。

### 受入条件との差 (doc-sync の明示)

**文言どおりには満たしていない。** 当初の受入条件は「fixture を作り」「publish される」
を要求しており、本決定はソース fixture も publish 経路も通らない。
**意図は満たしていると判断する**。意図は「`I-24` の裁定 — rule が違えば別の指摘として
両方残す — に失敗力のある検査を置くこと」であり、それは `dedup-diagnostics` の
契約そのものである。publish 経路が `dedup-diagnostics` を通ることは
`test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_multiple_diagnostics_in_stable_order`
が引き続き覆っており、経路と契約の両方に検査がある状態になる。

AN32j 自身は削除しない。real span 導入後は「開始位置が異なる 2 lint が順序どおりに
publish される」ことの検査として残り、これは依然として意味がある。
ただし「same-start の pin である」という役割は失ったので、doc コメントで明示する
(実施済み)。

### Evidence (`LINT-DEDUP-PIN-01`)

2026-08-23、`/Users/biwakonbu/github/lsharp` (branch `main`、`310eb6a6` の上) で実測。

```
cargo test -p lsharp-wasm --test e2e -- --ignored lsp_dedup_diagnostics_keeps_distinct_lint_rules
```

`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3075 filtered out; finished in 255.30s`

**RED は取っていない。** 本 slice は pin の再建であり、実装は既に採用する意味論を
実装している (本 ADR「実装範囲」節のとおりコード変更は無い)。初回から緑になるのが正しい。
偽の RED を作るために実装を一時的に壊すことはしなかった。

失敗力は分岐の作り方で確保している。3 分岐のうち 2 番目
(同一 rule / 同一 end → 1 件) は dedup が実際に働くことを要求するので、
「常に何もしない」実装では落ちる。1 番目と 3 番目は逆向きで、
「常に潰す」実装で落ちる。したがってこの test は両方向に失敗する。

同 slice で `I-57` の回帰確認も取った
(`cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_rename lsp_stdio_hover`
→ `19 passed; 0 failed`、1037.03s)。座標変換が rename / hover を乱していないことの確認である。

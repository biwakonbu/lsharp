# selfhost の contract metadata に quote 契約を載せる位置

- **Status**: Accepted
- **Date**: 2026-08-23
- **Scope**: `selfhost/src/Tools/Test/TestRunner.ls`, `selfhost/src/App/EmbeddedCli.ls`, `selfhost/src/App/Cli.ls`
- **Related**: `I-65`, `I-59`, `I-62`, `I-66`, [decisions-invariant-quote-handling.md](decisions-invariant-quote-handling.md),
  [decisions-example-quote-handling.md](decisions-example-quote-handling.md),
  [decisions-selfhost-preflight-diagnostic-message.md](decisions-selfhost-preflight-diagnostic-message.md)

## Context

既定の `lsharp test` は selfhost runner へ委譲される。`I-65` の実測 (2026-08-23) では
`(defn caller [x] :invariant (= 'sym 'sym) x)` が **`status pass` / `executed 5, failed 0` / exit 0** を返す。
`I-59` / `I-62` が rust 側へ入れた quote 診断は既定経路から一切見えない。

### rust 側は 3 層で弾いている

| 層 | 位置 | 効き方 |
|---|---|---|
| 一般の型推論 | `crates/lsharp-types/src/infer/expr.rs:396` | マクロ展開後に残る quote を位置によらず拒否 |
| lowering | `crates/lsharp-ir/src/lower/expr/quote_expr.rs:9` | 同上 (二重の網) |
| contract 固有 | `metadata_check/legacy.rs:53` (`:invariant`) / `metadata_check/diagnostics.rs:154` (`:example`) | `find_quote_span` で式全体を走査し、原因と噛み合う見出しを付ける |

contract 固有の層がある理由は既存 ADR に書かれている — 型推論へ渡すと「未定義の変数」という
**原因と噛み合わない見出し**になり、`:example` では span が生成ソースを指してしまう。

### selfhost 側で quote が通ってしまう経路

1. `selfhost/src/Types/TypeInfer.ls:199-207` の `quote-like-tag?` は
   quote / unquote / unquote-splice を**すべて inner expr へ委譲する** (`:306`)。
   `'sym` は中身の型で通る。
2. `MacroExpand.ls` は **CLI パイプラインに接続されていない**。
   `expand-macros` (`MacroExpand.ls:483`) の呼び出しは `App/PipelineSmoke.ls:72` の
   1 箇所だけで、`EmbeddedCli.ls` / `Cli.ls` / `TestRunner.ls` は
   `Syntax.MacroExpand` を import すらしていない (grep 実測)。
   `Derive.ls:8` のコメントが書く想定順序 `Parser -> Derive -> MacroExpand -> TypeInfer` は
   **CLI の実際の経路ではない**。つまり selfhost では quote は展開されて消えることが無く、
   かつ誰も拒否しない。
3. `TestRunner.ls:1480` の `invariant-static-bool-kind` は quote 系を kind 2 (非 Bool) と
   判定**できている**。しかし `(= 'sym 'sym)` は根が compare の apply なので kind 1 で確定し、
   引数の中は覗かれない。**判定器はあるが、走査が根で止まる**のが直接の原因である。

### contract form の payload は 5 種類で形が揃っていない (実測)

parser が `defn-metadata` slot 5 へ積む ordered form は
`[kind, payload, directive-start, directive-end, (extra)]` である
(`Parser.ls:1168` / `:1191`)。kind の番号は `AST.ls:45-49`。**payload の型は kind ごとに違う。**

| kind | directive | payload | 出所 |
|---|---|---|---|
| 1 | `:example` | **ソース文字列** (`substring src content-start content-end`) | `Parser.ls:1364` |
| 2 | `:invariant` | 述語 AST | `Parser.ls:1522` |
| 3 | `:assert` | 述語 AST の vector | `Parser.ls:1810` |
| 4 | `:case` | `[actual, expected, ...]` の 8 slot vector の vector | `Parser.ls:1572-1588` |
| 5 | `:property` | **ソース文字列** | `Parser.ls:1846` |

**`:example` と `:property` は AST を持たない。** 当初想定した「payload を AST として再帰走査する」
実装は、この 2 kind に原理的に届かない (却下案 E)。

一方 `directive-start` / `directive-end` は **全 kind が持つ**。
`metadata-directive-start-v3` は `:` トークンの start、`metadata-directive-end-v3` は
payload を読み切った直後のトークンの end なので、両者は directive 全体を過不足なく囲む
(`Parser.ls:1160-1166`)。

### 既存の preflight は形が揃っている

`run-test-source-json` (`EmbeddedCli.ls:1114`) は
`property boundary → assertion → case → suite` の順に preflight を試し、
どれかが非 0 なら `run-test-source-json-preflight` へ落ちる。
assertion / case の検査はいずれも `(count, first-code, first-start, first-end)` の
4-vector を返す (`TypeInferAssertions.ls:2183` / `:2500`)。
`ASSERT-DIAG-MESSAGE-01` で入れた `preflight-diagnostic-message` は
この (code, span) から message を組み立てる。**新しい検査を同じ形で足せば、
message 生成も exit code も既存経路がそのまま面倒を見る。**

## Decision

### D1: contract の directive 範囲を **トークン列**で走査する (採用)

`TestRunner` 層に `check-contract-quote [program src]` を新設する。

1. program の decl を `defn` / `private` / `module-decl` の 3 分岐で辿り
   (`has-unsupported-property-in-decl` と同じ骨格)、各 defn の ordered form
   (`defn-metadata` slot 5) を順に見る。
2. ordered form の kind (slot 0) が **1..5** のものだけを対象にする。同じ列には v0.3 の
   evidence (kind 15) / review attestation (kind 20) / source-pair / source-triple も載るが、
   これらは contract ではないので走査しない。
3. 対象 form の `(directive-start, directive-end)` で `src` を `substring` し、
   **その断片だけを `tokenize-with-spans` へ通す** (`Lexer.ls:735`)。
   全体を 1 度 lex して span の包含判定をする形にしないのは、包含判定を書かずに済み、
   directive をまたぐ誤検出も起きないため。
4. 断片のトークン列に kind **54 / 55 / 56** (`tok-quote` / `tok-unquote` /
   `tok-splice-unquote`、`Token.ls:55-57`) が 1 つでもあれば hit とし、最初の 1 件を採る。
5. 戻り値は既存と同じ 4-vector。**span は quote トークンではなく directive 範囲を返す。**

`run-test-source-json` の分岐へ **property boundary の次、assertion より前**で差し込む。
**text lane (`run-test-source-text`) にも同じ位置で差し込む。**
text lane の preflight (`run-test-source-case-preflight`) は `(count, code)` の 2-vector しか
受け取らず出力に message field が無いため、span 付きの断片は載らない。
`test-diagnostic-code-text` (`TestRunner.ls:826`) に `LS2008` を足し、
`diagnostics:1,LS2008` を出すところまでとする。

**どちらが「既定 lane」かは 2 系統で違う (2026-08-23 の実測で判明、`I-66`)。**
`main` は `argc` が 2 のとき option 解析を通さず `(default-compile-target)` を opts として渡す。
その値は `EmbeddedCli.ls:44` が **1** (`compile-target-component`)、`Cli.ls:46` が **0**
(`compile-target-preview1`) で、`(test-option-json)` は両方 **1**。つまり
`lsharp test input.ls` は **EmbeddedCli では JSON lane、Cli では text lane** に入る。
両方に差し込む本決定はこの食い違いに依存しないので、判断は変えない。
番号空間の重なり自体は本 slice の範囲外として `I-66` へ切り出した。

理由:

- **payload の型に依存しない。** `:example` / `:property` がソース文字列でしか payload を
  持たない (上表) 以上、AST 走査では `I-65` の 2 fixture 目に届かない。
  directive 範囲は 5 kind すべてが持つ唯一の共通項である。
- **文字列走査ではなくトークン走査にする。** ソース本文から `'` を探すと
  `"it's"` のような文字列リテラル内の `'` を誤検出する。lexer に通せば kind で区別できる。
- 契約は「contract metadata に quote を書けない」であって「quote が型付かない」ではない。
  **契約のある場所で契約を検査する**のが素直で、rust の contract 固有層と同じ位置になる。
- 走査が根で止まる問題 (`invariant-static-bool-kind` の kind 1 打ち切り) を、
  kind 判定を直さずに回避できる。
- 既存 preflight の形に載るので、message / span / exit code / JSON schema の
  どれも新規に作らなくてよい。`ASSERT-DIAG-MESSAGE-01` の成果をそのまま使う。

**span に directive 範囲を選ぶ理由**: `preflight-diagnostic-message` は span で
ソース本文を切り出して message へ載せる (`EmbeddedCli.ls:991`)。quote トークンだけを
返すと切り出しが `'` の 1 文字になり、どの contract の話か読めない。
directive 範囲なら `:invariant (= 'sym 'sym)` が丸ごと載り、**その中に問題の `'` も含まれる**。
LS1001 / LS1002 が述語式の span を返しているのとも整合する。

### D2: 新しい診断コード `2008` を割り当てる

既存コードは 1001 / 1002 / 2004 / 2005 / 2006 / 2007 / 3002 が埋まっている
(`grep 'defn canonical-' selfhost/src` 実測)。quote は「型エラー」でも「非 Bool」でも
「空」でもないので、既存の再利用は見出しが嘘になる。`canonical-contract-quote-code` = **2008**、
見出しは 「contract に quote/unquote は書けません」。
`preflight-diagnostic-code-text` / `preflight-diagnostic-headline` の両方に足す。

### D3: 差し込み順序を property boundary の後、assertion の前にする

順序は 2 つの既存 code の出方を変えるので、意図的な選択であることを明記する。

| ケース | 本 slice の後に出る code | 理由 |
|---|---|---|
| `:property` 内の quote | **LS3002 のまま** (2008 にしない) | property runner は丸ごと未接続であり、「quote が書けない」より「property が未接続」の方が上位の事実。既存 property fixture の pin を動かさない |
| `:assert` 内の quote かつ型エラー | **LS2008** (LS1001 ではない) | 型エラーは quote の帰結であって原因ではない。原因と噛み合う見出しを出す (rust の contract 固有層と同じ判断) |

後者に該当する既存 fixture は無い。`.ls` / e2e の `.rs` を実測したところ、
contract directive を含む行で quote トークンを使っているものは **0 件**だった
(`.ls` 中の `'` は全て文字列リテラルかコメント)。それが `I-65` の穴そのものである。

### D4: `Cli.ls` と `EmbeddedCli.ls` の両方に載せる

意図的な重複であり、片方だけ直すと配布 CLI が置き去りになる。
`selfhost_bootstrap_contracts.rs` に contract test を足して両系統を pin する
(`ASSERT-DIAG-MESSAGE-01` の D3 と同じ)。

## 却下した選択肢

### A: `TypeInfer.ls` の quote-like 委譲を一般のエラーにする

rust の `infer/expr.rs:396` と同じ層で弾く案。**将来的にはこちらが正しい** —
selfhost では MacroExpand が接続されていないので、残った quote は原理的に実行できない。

却下理由: 影響範囲が本 slice の受入条件を大きく越える。`TypeInfer` は
`lsharp check` / compile / LSP の全経路で共有されており、selfhost 自身の bootstrap も
この checker を通る。`INFER-FORWARD-GEN-01` が `I-48` で止まっている前例のとおり、
selfhost の checker を一般に厳しくすると 262 defn 規模の巻き添えが出うる。
計測せずに当てる変更ではない。**D1 を入れてから、別 slice で A を計測する**余地は残す。

### B: rust の診断を委譲経路で運ぶ

既定経路で rust の `metadata_check` を走らせ、その診断を selfhost の JSON へ載せる案。

却下理由: 既定経路が selfhost であることの意味が消える。両方走らせれば
parse と型検査が二重になり、selfhost runner の存在理由 (rust を呼ばずに動く) を壊す。
`decisions-selfhost-preflight-diagnostic-message.md` の却下案 C と同じ理由。

### C: `invariant-static-bool-kind` を quote に対して再帰させる

kind 判定器を直し、apply の引数まで潜って quote を見つける案。

却下理由: この判定器は「静的に Bool と分かるか」を答えるもので、
契約違反の検出器ではない。quote を見つけたときに返せるのは kind 2 (非 Bool) だけで、
`canonical-assertion-non-bool-code` (LS1002) の見出し「contract の述語が Bool になりません」が
出る。**原因と噛み合わない見出しを出すことは rust 側が明示的に避けた失敗**であり
(`legacy.rs:48-50` のコメント)、それを selfhost で再現することになる。
また `:example` は Bool を要求しないので、この経路では拾えない。

### D: rust との逐語一致

`decisions-selfhost-preflight-diagnostic-message.md` の却下案 D と同じ。
selfhost は識別子を name-hash でしか持たず、function 名を message に載せられない。
逐語一致は本 slice の目的 (穴に気付けること) に必要ない。

### E: ordered form の payload を AST として再帰走査する

当初案。`:invariant` / `:assert` / `:case` の payload は AST なので、
`ast-tag` を見て quote タグ (16 / 17 / 18) を探せばよい、という発想だった。

却下理由は 2 つある。

1. **`:example` と `:property` の payload は AST ではなくソース文字列である**
   (Context の表)。受入条件 2 の fixture (`:example [(caller 'sym)]`) に原理的に届かない。
   ここだけ別実装を足すと、同じ契約に 2 つの検出器が並ぶ。
2. selfhost の AST ノードは int slot と node slot が同じ vector に混在する
   (例: `make-fieldaccess` は slot 1 が node、slot 2 が field-name-hash という int)。
   タグ分岐なしに `vector-get` で潜る汎用再帰は、int を node と誤読して
   偽陽性を出す。タグ分岐を全 tag ぶん書くなら
   `invariant-static-bool-kind` (`TestRunner.ls:1480`) の焼き直しになり、
   D1 のトークン走査より大幅に長い。

## 受入条件

1. `(defn caller [x] :invariant (= 'sym 'sym) x)` が既定経路 (`--format json` なし) で
   **非緑**になること — exit 2 かつ `diagnostics:1,LS2008`
   (現状 `status pass` / `executed 5, failed 0` / exit 0)。
2. `(defn caller [x] :example [(caller 'sym)] x)` が既定経路で**非空の
   `diagnostics.message`** を返すこと (現状 `message` 空 / `count 0`)。
3. 両 fixture の message に診断コード `LS2008` と、quote を含む **directive 範囲**の
   ソース断片が載ること。
4. 既存の preflight 3 経路 (assertion / case / property) の code / span / count / exit code が
   動かないこと。
5. `Cli.ls` と `EmbeddedCli.ls` が同じ検査を持ち、`selfhost_bootstrap_contracts.rs` が
   両方を pin すること。

## 含めない範囲

- selfhost に quote の実行時表現を入れること。
- `TypeInfer` を一般に厳しくすること (却下案 A。別 slice)。
- `:property` 内 quote を LS2008 にすること (D3 のとおり LS3002 のまま)。
- rust 側の診断文言の変更。
- `run-test-source-text` lane に span 付きの message を載せること。
  この lane の出力形式は `diagnostics:<count>,<code>` で message field を持たない。
  **検査そのものと `LS2008` の表示は入れる** (D1)。
- `:example` の quote **以外**の失敗理由 (`EXAMPLE-FAIL-REASON-01`)。

## Evidence

実測は 2026-08-23、`main` の working tree (macOS aarch64)。ログは
`/Users/biwakonbu/github/tmp/quoteparity/` に置いた (repo 外)。

### RED

| lane | log | 結果 |
|---|---|---|
| `selfhost_bootstrap_contracts` の新規 pin | `red-contracts.log` | `FAILED. 0 passed; 1 failed`。panic は `TestRunner.ls に contract quote の走査本体が必要` |
| `selfhost_cli_actual_main_args` の新規 e2e | `red-e2e.log` | `FAILED. 0 passed; 1 failed; finished in 197.06s` |

e2e の RED が観測した出力は `I-65` の症状そのものである:

```json
{"implementation_conformance":{"status":"pass","coverage":{"executed":5,"failed":0},
 "diagnostics":{"count":0,"firstErrorCode":0,"message":""},...}}
```

### GREEN

| lane | log | 結果 |
|---|---|---|
| `selfhost_bootstrap_contracts::` 全体 (21 本) | `green2.log` | `ok. 21 passed; 0 failed; 1 ignored`、7.12s |
| `e2e::selfhost_cli_actual_main_args::` の live 全体 (19 本) | `green2.log` | `ok. 19 passed; 0 failed; 25 ignored`、1352.83s |

`selfhost_cli_actual_main_args` の live 19 本を丸ごと回したのは、この lane が
`Cli.ls` / `EmbeddedCli.ls` を bundle として compile する唯一の live lane だからである
(受入条件 4 の回帰確認を兼ねる)。新規 test
`test_e2e_selfhost_embedded_cli_test_format_json_contract_quote_preflight` は
JSON lane 2 fixture + 既定 lane 2 fixture の計 4 回 bundle を走らせる。

GREEN の e2e が観測した出力 (`:invariant` fixture、既定 lane):

```json
{"implementation_conformance":{"status":"fail","coverage":{"executed":0,"failed":1},
 "diagnostics":{"count":1,"firstErrorCode":2008,"firstErrorSpan":{"start":17,"end":41},
 "message":"[LS2008] contract に quote/unquote は書けません: :invariant (= 'sym 'sym) (17..41)"},...}}
```

### 受入条件の判定

| # | 判定 | 根拠 |
|---|---|---|
| 1 | **文言どおりには満たしていない。意図は満たしている** | 下記 |
| 2 | 満たした | `:example` fixture が `count 1` / 非空 message を返す |
| 3 | 満たした | message に `LS2008` と directive 範囲 (`:invariant (= 'sym 'sym)` / `:example [(caller 'sym)]`) が載る |
| 4 | 満たした | `selfhost_cli_actual_main_args` の live 19 本が全 GREEN。preflight 既存経路の pin (`..._test_format_json_preflight_diagnostic_message`、`..._non_bool_invariant`、`..._property_precondition_span`) を含む |
| 5 | 満たした | `test_e2e_selfhost_contract_quote_preflight_is_present_in_both_cli_sources` |

**受入条件 1 の文言「exit 2 かつ `diagnostics:1,LS2008`」は満たしていない。**
実測した既定 lane の出力は `diagnostics:1,LS2008` という text 形式ではなく、
`firstErrorCode: 2008` を持つ assurance JSON である。原因は `I-66` —
EmbeddedCli では `(default-compile-target)` = 1 が `(test-option-json)` = 1 と同値で、
`lsharp test input.ls` が JSON lane に入るため。doc-RED の時点でこの重なりを知らず、
「既定 lane = text lane」と書いたのが誤りだった。

**意図は満たしていると判断する。** 受入条件 1 の目的は「既定経路で緑を返さないこと」であり、
実測は `status fail` / exit 2 / `firstErrorCode 2008` を返す。text 形式でないことは
検出の有無ではなく出力形式の話なので、条件を緩めるのではなく**前提の誤りとして記録する**。
`run-test-source-text` への差し込み自体は D1 のとおり両 CLI へ入れてあり、
`Cli.ls` (既定 target 0) ではこちらが既定 lane になる。

### 検証できていない範囲

- **`Cli.ls` の quote 分岐は実行されていない。** 新規 e2e は
  `selfhost_embedded_cli_runtime_bundle()` だけを使う。`Cli.ls` を argv 経由で `test`
  へ通す e2e は `test_e2e_selfhost_cli_main_with_args_test_file` /
  `..._test_format_json_file` の 2 本だけで、**どちらも `#[ignore]`** である (`I-64`)。
  したがって `Cli.ls` 側は `selfhost_bootstrap_contracts.rs` の文字列 pin と
  括弧・root slot の目視だけで、root slot 収支は実行で確かめていない。
  この gap は `IGNORED-STALE-PIN-01` の範囲であり、本 slice では埋めない。
- **`run-test-source-text` lane そのものを実行した test は無い。** 上記の理由で
  EmbeddedCli では到達せず、Cli では live な test が無い。

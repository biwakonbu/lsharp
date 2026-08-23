# `:example` の失敗理由を selfhost suite 経路へ載せる

- **Status**: accepted
- **Date**: 2026-08-23
- **Scope**: `selfhost/src/Tools/Test/TestRunner.ls` の `:example` 実行経路と、
  そこから `diagnostics.message` を組み立てる assurance JSON の集約層。
- **Related**: [`I-62`](../../ISSUES.md#i-62) (本件の親。`:example` + quote の残渣として登録)、
  [`I-65`](../../ISSUES.md#i-65) / [contract metadata の quote 契約](decisions-selfhost-contract-quote-parity.md)、
  [preflight の診断 message](decisions-selfhost-preflight-diagnostic-message.md) (`ASSERT-DIAG-MESSAGE-01`)、
  [`I-67`](../../ISSUES.md#i-67) (本件の計測中に見つけた `coverage` 系の食い違い)

## 問題

`lsharp test <file>` の selfhost runner は、`:example` が偽を返して落ちたとき
**理由をどこにも出さない**。fixture:

```lisp
(defn abs [x] :example [(= (abs 5) 6)] (if (< x 0) (- 0 x) x))
```

2026-08-23 実測 (`./target/debug/lsharp test ex_fail.ls`、fixture と同じ dir から相対パスで):

| runner | 呼び出し | 結果 |
|---|---|---|
| selfhost (既定) | `lsharp test ex_fail.ls` | `status fail` / `cases 0` / `executed 0, failed 1` / `count 0` / **`message ""`** / exit 1 |
| rust | `lsharp test ex_fail.ls --format json` | `status fail` / `cases 1` / `executed 1, failed 1` / `count 0` / **`message ""`** / exit 2 |

`failed 1` は出るので「落ちたこと」は見えるが、**どの式が落ちたのかが出ない**。
preflight 経路は `ASSERT-DIAG-MESSAGE-01` で埋めたが、suite 経路は空のままだった。

## 原因 (実測で確定)

`TestRunner.ls:4000` の `run-examples-loop` が結果を
`(make-test-result name passed passed)` で作っている。この constructor は
`[name-id, passed, actual, diagnostic-code]` の 4 要素で **`diagnostic-code` を 0 に固定**し、
message の slot (index 6) を持たない。

集約側の `first-diagnostic-message-loop` (`:884`) は `(> code 0)` を通過条件にするので、
code が常に 0 の example 結果は 1 件も拾われず、
`assurance-suite-diagnostic-message` (`EmbeddedCli.ls:1066` / `Cli.ls:1117`) は `""` を返す。
`run-test-source-json-suite` はそれをそのまま JSON へ流すだけなので、CLI 側に欠陥は無い。

## 決定

### 採用: 案 A — `message` だけを埋める。`diagnostics.count` は 0 のまま

失敗した `:example` の結果に message を持たせ、集約側に
「診断が無くても失敗結果の message は拾う」フォールバックを足す。
`diagnostic-code` は 0 のまま据え置く。

message の文言は rust runner の per-case `error` を oracle にする
(`crates/lsharp-wasm/src/test_runner.rs:335`):

```
:example 式が偽を返しました: <式のソース断片>
```

式のソースは AST から復元できない (識別子は `name-hash` にしか残らない。
`ASSERT-DIAG-MESSAGE-01` と同じ制約) ので、`find-invariant-source-span` と同型の
token 走査で `:example` payload 内の n 番目の form の span を取り、`src` を slice する。

**採用理由**:

- `TODO.md` の受入条件は「非空の `diagnostics.message`」だけを要求している。
  `count` も `firstErrorCode` も条件に入っていない。
- rust oracle も同じ fixture で `count 0` を返す。診断 (静的な契約違反) と
  `coverage.failed` (実行時の不一致) は別チャネルであり、`:example` の偽は後者である。
  case を診断として数え直すと**参照実装との新しい食い違いを作る**。
- 変更の波及は `message` フィールドが `""` → 非空になる 1 点だけ。

**波及の事前確認** (2026-08-23):
`crates/` 全体で `diagnostics.message` の空を pin している assertion は 0 件
(`grep -rn "message" crates/lsharp-wasm/tests/e2e/ --include="*.rs" | grep -i is_empty` は
preflight の**非空**要求 `selfhost_cli_actual_main_args.rs:1887` の 1 件のみ)。
`"failed":1` を含む snapshot は
`crates/lsharp-tooling/tests/snapshots/metadata_runner_semantics_inventory__rust_runner_metadata_semantics.snap`
の 1 件だが、これは rust runner の per-case `error` を見ており selfhost 経路を通らない。
**案 A で壊れる pin は無い。**

### 却下: 案 B — 新しい診断コード `LS2009` を割り当て `count 1` にする

`:example` の失敗を診断として扱い、`count` と `firstErrorCode` を埋める案。**却下**。

- **rust oracle が `count 0` を返す**。案 B は selfhost だけ `count 1` / `code 2009` になり、
  parity を主眼にしたリポジトリで**新しい構造的な食い違いを作る**。
  しかも本 slice の「含めない範囲」には既に「Rust との逐語一致」が入っており、
  parity を広げる方向の変更を持ち込む場所ではない。
- 診断コードの採番は契約レベルの行為である。参照実装が意図的に診断として分類していない
  概念に番号を与えるのは、番号空間の意味を弱める。
- `test` 経路で `count` / `firstErrorCode` を pin している assertion の全体像が未確認で、
  波及範囲が案 A より広い。

### 却下: 案 C — CLI 側 (`run-test-source-json-suite`) で message を合成する

`failed > 0` かつ message が空なら CLI が文言を作る案。**却下**。
`Cli.ls` と `EmbeddedCli.ls` は意図的に重複しており
(`selfhost_bootstrap_contracts.rs` が両方に逐語 assertion を張っている)、
CLI 側に置くと同じロジックが 2 箇所に増える。
さらに CLI は `src` を持つが**どの式が落ちたかを知らない**ので、
出せるのは「どれかが落ちた」までで、原因に辿り着けないという当初の問題が残る。
message を作れるのは test-case と結果の対応を持つ `TestRunner` だけである。

## 案 A の残るコスト

`count 0` のまま `message` が非空になるため、
**「`count > 0` のときだけ `message` を読む」消費側があると message が見えない**。
現時点でそう書かれた消費側はリポジトリ内に無い (上記の波及確認) が、
JSON schema 上は `diagnostics.message` が `diagnostics.count` と独立に埋まりうる、
という契約になる。これは案 A を採る代償であり、案 B へ寄せる理由にはしない
(理由は上の却下欄)。

## 含めない範囲

- contract metadata の quote 検査。`I-65` で解決済みで、正本は
  [contract metadata の quote 契約](decisions-selfhost-contract-quote-parity.md)。
- rust runner との**逐語一致**。文言は oracle に寄せるが、
  span の粒度や `cases` / `executed` / exit code は揃えない。
- `run-test-source-text` lane。`I-66` の option 番号空間の裁定が先に要る。
- `:example` 以外 (`:invariant` / `:assert` / `:case` / property) の message。
  これらは既に診断経路を持っている。

## 実装

`selfhost/src/Tools/Test/TestRunner.ls` だけを変更した。CLI 2 系統
(`Cli.ls` / `EmbeddedCli.ls`) は `assurance-suite-diagnostic-message` を
そのまま呼ぶだけなので**無変更**である (案 C を却下した通り)。

| 追加した関数 | 役割 |
|---|---|
| `find-example-nth-span-loop` | `:example` payload (vector) 内の n 番目の form の span |
| `find-example-source-span-loop` | defn の token 範囲から `:example` directive を探す |
| `find-example-source-span-loop-by-defn` / `find-example-source-span` | fn-hash で defn を特定して上へ入る |
| `example-ordinal-loop` | test-cases は defn をまたいで一列に積まれるので、同一 fn-hash の出現回数で payload 内の位置を決める |
| `example-failure-message` | `":example 式が偽を返しました: " + ソース断片`。span が取れなければ断片抜き |
| `make-example-failure-result` | 上を束ねて 7 要素の結果 vector を作る (code は 0 のまま) |
| `first-failure-message-loop` / `first-test-failure-message-with-properties` | `passed = 0` かつ message 非空の結果を拾う |

既存の `first-test-diagnostic-message-with-properties` は本体を
`first-test-diagnostic-code-message-with-properties` へ改名し、
同名の関数を「診断由来を優先し、無ければ失敗由来へ落ちる」2 段に置き換えた。
`run-examples-loop` は `src` を受け取る形へ変え、`run-examples` (2 引数) は
`src = ""` で委譲する互換 wrapper として残した
(`selfhost_cli_core.rs:12672` の L# snippet が 2 引数で呼ぶため)。
`generate-tests` は新設の `run-examples-from-source` を使う。

## Evidence

### 受入条件

`TODO.md` の受入条件は「当該 fixture の selfhost JSON が非空の `diagnostics.message` を
返すこと」。**満たした。**

### e2e pin

`test_e2e_selfhost_embedded_cli_test_format_json_example_failure_message`
(`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs`)。`#[ignore]` ではない。
2 fixture を 1 test にまとめている (bundle の compile + run が 1 fixture あたり約 190〜250s のため)。

- RED (2026-08-23、実装前): `message` は空。189.37s で FAILED
- GREEN (2026-08-23、実装後): 386.45s で 1 passed

pin している契約は `status = fail` / `coverage.failed = 1` /
**`diagnostics.count = 0`** / `message` 非空 / `message` が落ちた式のソース断片を含む、の 5 点。
`count = 0` を明示的に pin しているのは、案 A の判断そのものを固定するためである。

exit code は harness (`run_main_with_input_file_capture`) では **2** に写る。
preflight の pin も同じ 2 を見ている。driver 経由の shipped binary は 1 を返す
(`exit-runtime-error`)。この 1 / 2 の差は本 slice の対象外で、`I-67` が引き取る。

### shipped binary の実測 (2026-08-23、`cargo build` 後の `./target/debug/lsharp`)

fixture と同じ dir から相対パスで `lsharp test probe.ls`:

| fixture | `cases` | `executed` / `failed` | `diagnostics.message` |
|---|---|---|---|
| `:example [(= (abs 5) 5)]` (緑) | 1 | 1 / 0 | `""` |
| `:example [(= (abs 5) 6)]` | 0 | 0 / 1 | `:example 式が偽を返しました: (= (abs 5) 6)` |
| `:example [(= (abs 5) 5) (= (abs -3) 9)]` | 1 | 1 / 1 | `:example 式が偽を返しました: (= (abs -3) 9)` |

3 件目が **2 番目の式**を正しく指しているので、`example-ordinal-loop` による
payload 内の位置決定が効いている。緑の fixture の message は空のままで、
成功経路には影響していない。

### 満たしていないこと (明示)

- **`firstErrorSpan` は `0..0` のまま**である。span は結果 vector の index 4/5 に載せたが、
  集約側の `first-diagnostic-span-loop` も `code > 0` を通過条件にするため surface しない。
  案 A で `code = 0` を選んだことの直接の帰結であり、message 内にソース断片が入るので
  受入条件は満たす。span まで出すには診断コードの採番 (案 B) が要る。
- `cases` / `coverage.executed` は失敗した `:example` を数えないままである
  (上表の 2 件目が `cases 0`)。これは本 slice のスコープ外で、`I-67` /
  `EXAMPLE-COVERAGE-COUNT-01` が引き取る。
- `TestRunner.ls` は 5,280 行になり、`CLAUDE.md` のファイルサイズ上限 (500〜800 行) を
  従来どおり超えている。本 slice で +147 行増やした。分割は `RUNNER-SCANNER-01` の範囲。

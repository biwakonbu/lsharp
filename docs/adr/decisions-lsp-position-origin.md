# selfhost LSP の Position origin 契約

- **Status**: doc-GREEN (focused 3 本まで / lane 未了 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED) / 2026-08-28 (測定) / 2026-08-28 (実装)
- **Scope**: selfhost 側 LSP (`selfhost/src/Tools/Lsp/*.ls`, `selfhost/src/App/Cli.ls`) の
  `Position` origin 契約と、それに依存する e2e 期待値。
- **含めない範囲**: Rust 側 LSP (`crates/lsharp-lsp`) の origin。
- **Related**: `ISSUES.md` の `I-90` / `TODO.md` の `LSP-POSITION-ORIGIN-01`

## 何が問題か

`selfhost_cli_core` の赤 2 件 (`..._lsp_transport_hover_frame` / `..._lsp_transport_formatting_frame`) が、
`line` も `character` も一律 +1 ずれた期待値を持っている。`I-90` は response 側の origin を
兄弟 test の実測から 0 origin と確定させたが、**request 側の origin は未確定のまま残していた**。

`I-90` が未確定としたのは正しい。references は symbol の全出現を返すので、cursor が `square` へ
解決さえすれば期待値が当たる。「`line=2` が 2 行の文書で通っている」だけでは、
request が 1 origin なのか、範囲外を clamp しているのかを区別できない。

## 測定より先に記録する: source 読解で判ったこと

**以下は実行による測定ではなく、コードを読んで判ったことである。** 測定はこの後に行う。

`selfhost` の LSP には **2 つの層**があり、origin が違う。両方の境界に変換が入っている。

| 層 | origin | 変換箇所 |
|---|---|---|
| wire (stdio JSON-RPC の `"line"` / `"character"`) | **0** | -- |
| 内部 params vector (`[uri, line, col, source]`) | **1** | `lsp-stdio-nav-params` (`App/Cli.ls:2121-2130`) が wire から `(+ ... 1)` |
| 内部解析位置 | **1** | `lsp-offset-from-line-col` (`LspServerNav.ls:220-221`) が `line=1 col=1` から歩く |
| 内部 range -> wire | **0** | `lsp-render-wire-range-json` (`LspServerCore.ls:517-528`) が `(- ... 1)` |

`lsp-render-wire-range-json` には既にコメントで契約が書いてある:

```
;; LSP wire の Position は zero-based。内部解析位置 (1-based) から境界で変換する。
```

**つまり契約はコード中に既に存在し、両境界とも変換済みである。** 欠けているのは
「内部 params vector も 1 origin 側に属する」という一点が、どの正本にも書かれていないことだけである。

`run-lsp-transport-request` は **内部 params vector を直接受ける helper** であって wire 入口ではない。
したがって hover / formatting / references / rename の transport frame test 群が渡している
`(99, 2, 17, source)` は **1 origin として正しい入力**であり、返る frame は wire なので 0 origin になる。

### 傍証 (すでに lane で緑であることが確認済みのもの)

いずれも 2026-08-28 の `selfhost_cli_core` lane 完走 (381/381、failed 21、comparer exit 0) で緑であり、
`ignored-lane-expected-failures.txt` に載っていない。

| test | 何を示すか |
|---|---|
| `..._lsp_stdio_zero_based_position_contract` | wire 入口が **0 origin** (`"line":0,"character":6` で `helper` に当たる) |
| `..._lsp_stdio_standard_uri_navigation_contract` | wire 入口が 0 origin (`"line":1,"character":15`) |
| `..._lsp_transport_references_frame` | 内部 params `(99, 2, 17, ...)` で `square` に解決し、wire は 0 origin |
| `..._lsp_transport_rename_frame` | 同上。期待値がすべて 0 origin |

### 履歴

wire 変換 `lsp-render-wire-range-json` は `9175c6e5` "fix: normalize native lsp wire positions"
(2026-08-03) が入れた。同 commit は `..._lsp_stdio_zero_based_position_contract` を新設したが、
**先行して存在した hover / formatting の transport frame 期待値を更新していない。**
赤 2 件はこの取りこぼしである。

## 予測 (測定より先に記録する)

`I-90` が定めた判別測定 -- hover を `(99, 1, 8, source)` で撃つ -- の結果を、**測る前に**書く。

- request は 1 origin なので `line=1` は 1 行目 `(defn square [x] x)`、`col=8` は 0 origin の index 7
  (`square` は index 6..12 に載る) に当たる
- したがって解決する symbol は **`square`**、contents は `"defn square"`
- range は内部 1 origin で `line 1 / col 7` - `line 1 / col 13`、wire 変換後は
  **`{"line":0,"character":6}` - `{"line":0,"character":12}`**

**0 origin だった場合はここが `main` / `{"line":1,"character":6}`-`{"line":1,"character":10}` になる。**
symbol 名まで変わるので取り違えようがない。

## 裁定

### 採用: 入口 1 origin を契約として明文化する

内部 params vector は 1 origin であると正本へ書き、その契約を pin する test を足す。
期待値 2 件は wire (0 origin) へ直す。

### 却下: request も 0 origin へ揃える

却下理由は 2 つある。

1. **契約はすでに一貫している。** wire は 0、内部は 1、境界で両方向とも変換している。
   「入口と出口で origin を混ぜている」という `I-90` が懸念した状態ではない。
2. **揃えると変換を 3 箇所同時に外すことになる。** `lsp-stdio-nav-params` の `+1`、
   `lsp-offset-from-line-col` の `(1, 1)` 起点、`lsp-position-from-offset` の `(1, 1)` 起点。
   `lsp-stdio-rename-params` の `+1` も含めれば 4 箇所。**現契約で緑の test が 4 本ある**以上、
   この書き換えが買うものが無い。

## Evidence

### (a) 判別測定 -- 予測どおり

`I-90` が定めた判別測定を pin test として実装した:
`selfhost_cli_core::test_e2e_selfhost_cli_lsp_transport_request_params_are_one_origin`
(`selfhost_cli_core.rs:5076`)。hover を `(99, 1, 8, source)` で撃ち、
wire response が `{"line":0,"character":6}` - `{"line":0,"character":12}` /
`"contents":"defn square"` になることを固定する。

| 項目 | 値 |
|---|---|
| 実行 | `target/debug/deps/e2e-aa343ded249bec81 --ignored --test-threads 1 <3 本>` |
| 起動 | `/Users/biwakonbu/github/tmp/i90/run_probe.py` を `os.setsid()` で切り離し。pid 33491 |
| ログ | `/Users/biwakonbu/github/tmp/i90/probe.log` (**下記 verify 実行が同名で上書きした**。値は本 ADR に写し取ってある) |
| 結果 | `..._request_params_are_one_origin ... ok` / `ELAPSED=673.35` |

**上の「予測」節に書いたとおり `square` が返った。** したがって
`run-lsp-transport-request` が受ける内部 params の `line` / `col` は **1 origin** である。
`0 origin だった場合はここが main になる` と予測より先に書いてあり、そうはならなかった。

**この 1 本で決まったのは request 側だけである。** wire 入口が 0 origin であることは
別の 2 本 (`..._lsp_stdio_zero_based_position_contract` / `..._lsp_stdio_standard_uri_navigation_contract`)
が 2026-08-28 の lane で緑であることに依存しており、本測定はそこを測り直していない。

### 赤 2 件の実測 (同じ実行で取得)

| test | 実装が返した値 (left) | 修正前の期待 (right) |
|---|---|---|
| `..._lsp_transport_hover_frame` | `{"line":1,"character":15}` - `{"line":1,"character":21}` | `{"line":2,"character":16}` - `{"line":2,"character":22}` |
| `..._lsp_transport_formatting_frame` | `{"line":0,"character":0}` - `{"line":1,"character":3}` | `{"line":1,"character":1}` - `{"line":2,"character":4}` |

`ISSUES.md` の `I-90` が sweep ログから書き取った値と 4 数値すべて一致した。
`Content-Length` は 136 / 143 のまま変わらない (どの数値も桁数が変わらないため)。

### (c) 期待値の修正

**これは「テストの期待値を実装に合わせて変更した」形になるが、根拠は実装の出力ではない。**
根拠は次の 3 つで、いずれも実装出力とは独立である:

1. `lsp-render-wire-range-json` のコメントが wire = zero-based を契約として明記している
2. `lsp-stdio-nav-params` が wire -> 内部で `+1` している (対称な変換が両境界に揃っている)
3. 同じ helper・同じ文書・同じ params で 0 origin を期待する兄弟 test 2 本が lane で緑である

すなわち **`CLAUDE.md` が禁じる「実装に合わせて期待値を変える」ではなく、
`テストの設計ミスを除く` 側の例外**にあたる。設計ミスの出所は `9175c6e5` (2026-08-03) が
wire 変換を入れたときに先行 test 2 件の期待値を更新しなかったことである。

### (c) 修正後の再測定 -- 3 本とも緑

同じ binary・同じ 3 本を修正後に測り直した。

| 項目 | 値 |
|---|---|
| 起動 | `run_probe.py` を `os.setsid()` で切り離し。pid 58011 |
| ログ | `/Users/biwakonbu/github/tmp/i90/probe.log` (08:17:35 開始) |
| 結果 | `test result: ok. 3 passed; 0 failed` / `RUNEXIT=0` / `ELAPSED=648.61` |

`3078 filtered out` + 3 = **3081**。pin test 1 本を足す前は 3080 だったので数が合う。

**ログ経路の取り違えを 1 件起こした。** verify 用に出力先を `verify.log` へ変えるつもりで
`sed` の置換パターンを外し、`probe.log` を上書きしてしまった。監視していた Monitor は
存在しない `verify.log` を見ていたので `PROCESS-GONE without RUNEXIT` を出した。
**測定値そのものは失われていない** ((a) の値は上書き前に本 ADR へ写してある) が、
「ログが残っているはず」という前提は成り立たなかったので記録しておく。

## 満たせなかったこと

- **`selfhost_cli_core` の lane 再計測をまだ回していない。** focused 3 本の結果は lane 1 本の
  完走ではない。台帳 2 行 (`ignored-lane-expected-failures.txt:402-403`) の削除もその後である。
  `LSP-POSITION-ORIGIN-01` / `ROOT-IMBALANCED-HELPER-01` / `SWEEP-UNCLASSIFIED-01` の
  3 項目が同じ 1 本を待つ形にして、項目ごとに lane を回すことはしない。
  **追記 (2026-08-28)**: `I-75` の移管が全数終わって `SWEEP-UNCLASSIFIED-01` を削除したため、
  束ね役は `SWEEP-LANE-RERUN-01` が引き継いだ。待ち合わせる項目は 7 件に増えている
  (`I-74` / `I-90` / `I-96` / `I-97` / `I-98` / `I-99` / `I-100`)。
  **`ROOT-IMBALANCED-HELPER-01` (`I-74`) は当初の 3 項目束ねの一員なので落としていない。**
  **方針は変えていない。**
- **`selfhost_cli_core` の宣言数が 381 -> 382 へ増えた。** pin test を 1 本足したためである。
  次の lane の完走判定はこの新しい分母で行う。
- **wire 入口 (stdio) の origin は本 slice では測り直していない。** 上記のとおり、
  既に緑である 2 本に依存している。
- **Rust 側 LSP (`crates/lsharp-lsp`) の origin は見ていない。** Scope 外である。

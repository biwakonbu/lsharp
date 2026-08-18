# ADR: 陳腐化した検査 test の是正 (TESTGATE-01 / TESTGATE-02)

- Status: Accepted (TESTGATE-02 は verified / TESTGATE-01 は構造的破損の是正のみ verified、
  露出した 164 件の違反は `I-22` / `TESTGATE-03` へ切り出し)
- Date: 2026-08-18
- Scope: `TESTGATE-01` / `TESTGATE-02` / `I-11` /
  `crates/lsharp-wasm/tests/e2e/support.rs` /
  `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs`
- Related: [`ISSUES.md` I-11](../../ISSUES.md#i-11)、
  [`workspace-expected-failures.txt`](../development/validation/workspace-expected-failures.txt)

## Context

`I-11` の baseline 固定作業で、workspace 恒常 FAIL 108 件のうち **7 件が「production の
不具合ではなく、検査側が上流の変更に追随できていない」**ものだと判明した。
さらに、その調査の副産物として**落ちてすらいない**無言無効化を 1 件検出した。

本 ADR はこの 2 件 (`TESTGATE-01` / `TESTGATE-02`) の是正方針を定める。
どちらも検査 (test) 側を直す変更であり、**production コードの挙動は変えない**。

### TESTGATE-02 — verbatim 包含 assertion が bundle 正規化と食い違う

`support.rs` の `cached_selfhost_bundle` は selfhost ソースを結合する際に
`.replace("(import Types.TypeInfer)\n", "")` で当該 import 行を落とす。
一方 `test_support_selfhost_typeinfer_runtime_bundle_cached` は
`selfhost_module()` が返す **生テキスト**の verbatim 包含を要求する。
`(import Types.TypeInfer)` を持つモジュールは原理的に一致しない。

- assert の初出: 2026-03-27 `7f9bdbb4`
- `replace()` と import 行の同時追加: 2026-07-20 `2b0c54b1`

`mod support;` を 6 binary が共有するため、同一原因で 6 件が落ちる。

### TESTGATE-01 — `#[ignore]` ゲート検査が mod 分割に追随していない

`selfhost_lsp_docs_ops.rs` の `test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` は
heavy test に `#[ignore]` が付いているかを検査する。親ファイルが `include!` だけになった
分割後、検査は実体を見失っている。壊れ方が 2 種類ある。

| モード | 一致 0 件のときの挙動 | 気付けるか |
|---|---|---|
| 厳密名 | `panic!` | 落ちるので気付ける (baseline に載っている) |
| prefix | **何もしない** | **気付けない。baseline にも載らない** |

**後者の方が重い。** 検査は pass したまま何も見ていない。

## Decision

### 決定 1: どちらも「検査側が陳腐化している」と判定し、test の期待値を変える

このリポジトリは 「テストの期待値を実装に合わせて変更しない」 を規律として持つ
([`tdd-workflow.md`](../../.claude/rules/tdd-workflow.md))。本件はその例外
**「テストの設計ミスを除く」** に当たる。根拠は `I-11` 本文で既に採決済みで、
`workspace-expected-failures.txt:111` にも
「2026-07-20 の `2b0c54b1` で検査が陳腐化した」と記録されている。
production 側の挙動 (import 行の除去、mod 分割) はいずれも意図された変更であり、
それを差し戻す理由が無い。**例外適用の判断であることを本 ADR に残す**のが本節の目的である。

### 決定 2: TESTGATE-02 は「正規化を 1 箇所に寄せる」形で直す

期待値側にも同じ正規化を適用する。**4 モジュールを名指しで除外する形は採らない。**

- **採用**: 正規化関数を 1 つ切り出し、bundle 生成と検査の両方から呼ぶ
- **却下 (a) 落ちる 4 モジュールを assert から外す** — 検査対象が黙って減る。
  今後 `(import Types.TypeInfer)` を持つモジュールが増えるたびに同じ判断を繰り返す
- **却下 (b) `cached_selfhost_bundle` の `replace()` をやめる** — production 側の
  fixture が壊れる。import 行の除去は bundle が単一モジュール束であるために必要
- **却下 (c) `selfhost_module()` 側で除去する** — 生ソースを返す契約が壊れ、
  この関数を使う他の検査 (`CompilerSplit.ls` の包含等) の意味が変わる

### 決定 3: TESTGATE-01 は既存の `file_size` 方式へ寄せる。新規設計はしない

`selfhost_bootstrap_acceptance_file_size.rs` が同じ問題を既に正しく扱っている
(fragment ディレクトリを `read_dir` で列挙 → 親の `include!` マニフェストが全 fragment を
順序どおり含むか検証 → 各 fragment を再帰的に見る)。ops03b / ops03c をこの方式へ寄せる。

- **却下 (a) prefix モードの一致 0 件を `panic!` にするだけ** — 対症療法。
  親が `include!` のみになった構造に追随できていないという原因は残る
- **却下 (b) 検査を消す** — heavy test の `#[ignore]` 漏れを検出する唯一の gate である

### 決定 4: prefix モードの是正は「変異試験」で証明する

無言無効化の是正は、GREEN になっただけでは直った証拠にならない (元々落ちていない)。
fragment から `#[ignore]` を 1 つ剥がし、検査がその test 名を挙げて落ちることを確認し、
backup から復元して `git diff --stat` が空であることまでを証拠とする。

## Evidence

### TESTGATE-02 (2026-08-18, 実測)

RED の再現 (修正前、`cargo test -p lsharp-wasm --test doctools_parity`):

```
thread '...test_support_selfhost_typeinfer_runtime_bundle_cached' panicked at
  crates/lsharp-wasm/tests/e2e/support.rs:1865:9:
assertion failed: bundle.contains(selfhost_module("TypeInferApply.ls").trim())
```

診断どおり、`(import Types.TypeInfer)` を持つ 4 モジュールの 1 番目で落ちる。

修正: `normalize_selfhost_bundle_source()` を切り出し、`cached_selfhost_bundle` と
検査側ヘルパ `bundle_expectation()` の両方から呼ぶ。検査側の `selfhost_module(x).trim()` 12 箇所を
`&bundle_expectation(x)` へ置換した (落ちていなかった `CompilerSplit.ls` / `Parser.ls` も含め、
名指し除外を作らず**一律に**通す)。

GREEN の確認は 6 binary すべてで個別に実行した (workspace 全体の nextest は 7 時間かかるため、
`I-11` と同じく binary 単位で確認する。前例は dev-loop lane speedup ADR)。

| binary | 結果 |
|---|---|
| `lsharp-wasm::doctools_parity` | 7 passed / 0 failed |
| `lsharp-wasm::lsp_diagnostic_parity` | 7 passed / 0 failed |
| `lsharp-wasm::lsp_edge_case_parity` | 7 passed / 0 failed |
| `lsharp-wasm::lsp_stateful_parity` | 7 passed / 0 failed |
| `lsharp-wasm::property_probe_diagnostic` | 7 passed / 0 failed |
| `lsharp-wasm::e2e` | 7 passed / 0 failed (3053 filtered out) |

**非空虚性の証明 (変異試験)**: 新設した
`test_support_bundle_normalization_drops_shared_import_line` は「生ソースは import 行を持ち、
bundle は持たない」を直接固定する。`normalize_selfhost_bundle_source` の本体を
`source.to_string()` へ変異させると**この 1 件だけが FAILED** になり
(他 6 件は pass のまま)、復元後 `git diff --stat` に変異が残らないことを確認した。
この guard が無いと、検査側を生ソースへ戻したとき「両側が生ソース」で静かに通り続ける。

baseline 反映: `workspace-expected-failures.txt` から 6 エントリを削除
(e2e 53 -> 52 / 非 e2e 36 -> 31)。`:141` 付近にあった「5 binary へ重複計上」の注記は、
その FAIL が存在しなくなったので解消の記録へ差し替えた。

### TESTGATE-01

RED (是正前):

```
panicked at crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs:3769:32:
crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_acceptance.rs に
test_e2e_bootstrap_fixed_point_stage2_stage3 が見つからない
```

是正内容 (`crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs`):

1. `gate_source_units()` を追加し、`selfhost_bootstrap_acceptance_file_size.rs` と同じ方式で
   fragment ディレクトリを `read_dir` 列挙 → 親の `include!` マニフェストが全 fragment を
   順序どおり含むことを `assert_eq!` で検証 → 親と全 fragment を `(表示パス, 本文)` で返す。
   ops03b / ops03c の 3 ループ (ops03b / ops03c 厳密名 / ops03c prefix) を全てこれに寄せた。
2. `has_ignore_attribute()` を抽出。`fn` 行の直前の非空行が `#[ignore` で始まるかを見る。
   元のループ (`#[ignore` なら `continue`、`#[test]` かそれ以外なら `break`) と結果は同値である
   — `continue` へ入る条件が「直前の非空行が既に `#[ignore`」だからで、それ以降に
   `has_ignore` が false へ戻る経路は無い。
3. prefix モードに `dead_prefix_rules` を追加。一致 0 件の prefix を明示的に落とす。
   これが無いとルールが死んでも何も起きない (本 issue の「無言で無効化」そのもの)。
4. dead rule の assert が offenders の assert より前にあると、ルールが 1 本死んだだけで
   違反一覧が隠れてしまう。両者を 1 本の assert にまとめ、同時に出るようにした。

**変異試験 (決定 4 の要求)**: 旧「無言で死んでいた」prefix の対象である
`selfhost_bootstrap_four_layer/part_007.rs` から `#[ignore]` を 1 つ剥がすと、
offenders が **164 → 165** になり、増えた 1 件はちょうど
`crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/part_007.rs:7` だった。
backup から復元後、`git diff --numstat -- .../selfhost_bootstrap_four_layer/` は 0 行。
**gate が GREEN にならない以上「通ったから直った」では証明できない**ので、
「fragment の中身が見えているか」を直接測るこの形を証拠とする。

`dead_prefix_rules` は是正後 **0 件** (assert メッセージに prefix ルールの節が出ない)。
`TODO.md` が挙げていた 6 ルール / 215 関数の無言無効化は全て解消した。
偽陽性の可能性は `crates/lsharp-wasm/tests/` に `#[cfg_attr(..., ignore)]` 形が
**0 件**であることで排除した (行スキャン方式が取りこぼす形が存在しない)。

**満たせなかった受入条件**: ops03c は GREEN にならない。検査が復活した結果、
**本物の違反 164 件**が現れたため (`selfhost_cli_core.rs` 158 /
`selfhost_cli_actual_main_args.rs` 5 / `selfhost_native_stage_chain.rs` 1)。
これは本 ADR の範囲外の判断 (規約と実態のどちらが陳腐化しているか) なので、
`I-22` / `TESTGATE-03` へ切り出した。**gate を green にするために prefix ルールを
黙って絞ることはしない。** baseline の FAIL 集合は本作業の前後で不変
(`ops03c` は元から expected FAIL であり、新規 FAIL も pass への転換も無い)。

なお当初 `TODO.md` に書かれていた「現時点では 215 件とも `#[ignore]` を持っており
live な regression は無い」は、分割済み 4 親だけを見た測定だった。非分割ファイルの
違反は厳密名モードの panic に隠れて数えられていない。

`ops03b` (`test_e2e_ops03b_debug_tests_are_ignored`) は是正後 **1 passed**。

### 副産物: 違反がいつ積み上がったか

`I-22` に表として記録した。要点は破壊が **2 段**だったこと:
prefix ルール導入 (2026-05-07 `e9bb354e`) 時点の違反は 0 で、
2026-07-12 の CI 自動実行停止 (`I-19`) の 5 日後から積み上がり始め、
2026-07-27 の fragment 分割 (`a7edffe1`) が「手元で回しても見えない」状態を上塗りした。
**ルールが最初から空回りしていたのではなく、enforcement が止まってから実態が離れた**。

## 受入条件

- `TESTGATE-02`: `workspace-expected-failures.txt` から当該 6 エントリが消え、
  6 binary それぞれで当該 test が pass する
- `TESTGATE-01`: 厳密名モードの `panic!` が解消し、かつ prefix モードが
  変異試験で「見ている」ことを示す
- どちらも `selfhost/src` を触らない (stage0 fingerprint を動かさない)

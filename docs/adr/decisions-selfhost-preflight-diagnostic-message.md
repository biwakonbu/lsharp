# ADR: selfhost preflight の assurance JSON に診断 message を持たせる

- Status: Accepted
- Date: 2026-08-23
- Scope: `selfhost/src/App/EmbeddedCli.ls`, `selfhost/src/App/Cli.ls`
- Related: [I-49](../../ISSUES.md#i-49), [I-45](../../ISSUES.md#i-45),
  [decisions-selfhost-assert-preflight-typecheck.md](decisions-selfhost-assert-preflight-typecheck.md)

## Context

`I-49` で `:assert` preflight の型検査を接続したが、**診断 `message` は空文字列のまま残った**。
`run-test-source-json-preflight` (`Cli.ls:1009` / `EmbeddedCli.ls:958`) は
`assurance-report-json` の最終引数へ literal `""` を渡している。

preflight を通る経路は 3 つあり、いずれも `(count, code, start, end)` の 4 スカラーしか持たない。

| 経路 | code の出どころ | span |
|---|---|---|
| property boundary | `property-runner-boundary-code` (`PropertyRunner.ls:1244`) | **`0 0` を literal で渡す** |
| assertion | `check-canonical-assertions-with-analysis` (`TypeInferAssertions.ls:2183`) | predicate の span |
| case | `check-canonical-cases-with-analysis` (`TypeInferAssertions.ls:2500`) | case 式の span |

### 実測 (2026-08-23)

`selfhost_embedded_cli_runtime_bundle()` を `test input.ls --format json` で回した実測。
既定 CLI の `--format json` は **rust runner へ落ちる**ので、この経路は e2e の bundle lane
でしか観測できない。4 fixture で 990.35s (1 fixture あたり約 250s)。

| fixture | 経路 | firstErrorCode | firstErrorSpan | message |
|---|---|---|---|---|
| `(defn caller [] :assert [(> (nope) 0)] 0)` | preflight assertion | 1001 | 25..37 (`(> (nope) 0)`) | `""` |
| `(defn identity [x] :case [(expect missing 1)] x)` | preflight case | 1001 | 34..41 (`missing`) | `""` |
| `(defn abs [x] :property [(for-all ... :seed 81042 ...)] ...)` | preflight property | 3002 | 0..0 | `""` |
| `(defn succ [x] :invariant (+ x 1) (+ x 1))` | **suite (対照)** | 2 | 26..33 | `:invariant は Bool 必須ですが、Int が推論されました` |

3 経路とも rc=2 / `count=1` / `executed=0` で、**空なのは message だけ**である。
対照の suite 経路は非空 message を既に返しており、差は preflight 側にしかない。

span の精度は経路で違う。case は未定義シンボル単体を指すが、assert は**述語式全体**を指す
(`TypeInferAssertions.ls:1125-1141` が `predicate-start` / `predicate-end` を使うため)。
property boundary は span を持たない。

suite 経路 (`run-test-source-json-suite`) だけは
`first-test-diagnostic-message-with-properties` (`TestRunner.ls:781`) を通じて message を持つ。
つまり **message を組み立てる仕組みは既にリポジトリ内にある** —
`invariant-non-bool-diagnostic-message` (`TestRunner.ls:2192`) が report 境界で文字列を組んでいる。
足りないのは preflight 側の同型の builder である。

### 制約: selfhost は識別子名を復元できない

selfhost の AST は識別子を `name-hash` (`TestRunner.ls:916`) で保持する。
`infer-expr` の失敗結果は `result-error-name-hash` (`TypeInferCore.ls:302`) を持つが、
**hash から文字列へ戻す経路はリポジトリ内に存在しない**。
したがって「未定義シンボル名」を message へ載せる手段は
**ソース本文を span で切り出すこと以外に無い**。

## Decision

**(B) preflight の境界で、`code` と `span` と `src` から message を合成する。**

`run-test-source-json-preflight` へ `src` を渡し、次を組む。

```
[LS<code>] <code に対応する見出し>: <src の span 切り出し> (start..end)
```

span が空 (`start = end = 0`) の property boundary では切り出しを省き、見出しと code だけを載せる。

### D1: 「未定義シンボル名を含む」は source fragment の引用で満たす

`:assert` の span は**述語式全体** (`check-assertion-predicate` の呼び出し元
`TypeInferAssertions.ls:1125-1141` が `predicate-start` / `predicate-end` を使う) であり、
未定義シンボルそのものの span ではない。よって
`(defn caller [] :assert [(> (nope) 0)] 0)` の message には `(> (nope) 0)` が載る。
これは `nope` を**部分文字列として含む**ので受入条件の文言は満たす。

シンボル単体の span (`result-error-start` / `result-error-end`) は infer 結果が持っているが、
それを `firstErrorSpan` へ昇格させると `selfhost_assertion_spans.rs` の既存 pin と
Rust oracle 突合 (`assert_non_bool_invariant_json_matches_rust_oracle`) を同時に動かすことになり、
本 slice の範囲を越える。**span は現状のまま据え置き、message だけを足す。**

### D2: 見出しは code ごとの固定文字列にする

preflight へ到達する code は 5 つに限られる。

| code | 見出し |
|---|---|
| 1001 | `型推論に失敗しました` |
| 1002 | `Bool 必須の述語が Bool になりません` |
| 2004 | `空の contract です` |
| 2005 | `述語が常に真になる vacuous な contract です` |
| 2006 | `空の contract です` |
| 3002 | `未接続の property runner 境界です` |

未知の code は `診断コード <n>` へフォールバックする。番号だけでも空文字列より情報がある。

### D3: `Cli.ls` と `EmbeddedCli.ls` の両方へ同じ builder を置く

2 ファイルは意図的に重複しており、`selfhost_bootstrap_contracts.rs:249` が
片方にしかない contract を検出する。片方だけ直すと bundle 間で挙動が割れる。

## 却下した選択肢

- **(A) 検査 state へ message を通す。** `assertion-check-state-with-span` /
  `case-check-state` は 4 要素の vector で、bounded loop がフィールドを個別の引数として
  10 段以上引き回している。5 要素へ広げると `TypeInferAssertions.ls` の
  `check-*-step-v3` / `-64-loop-bounded` / `-rooted-v3` / `-loop` の 4 段 × 2 系統を
  すべて書き換えることになり、`root_push` / `root_pop` の収支も各段で変わる。
  **得られるものは message の生成位置が数百行上へ移ることだけ**で、文字列の中身は
  結局 span からの切り出しに依存する (上記「識別子名を復元できない」)。費用に対して得が無い。
- **(C) Rust 側の診断を委譲経路で運ぶ。** selfhost lane は Rust を呼ばずに立つことが
  `AGENTS.md` の native selfhost success path の前提である。委譲すると
  「selfhost が検査している」という主張が成立しなくなる。`I-65`
  (`SELFHOST-QUOTE-PARITY-01`) で同じ選択肢を検討する際にもこの理由が効く。
- **(D) message を Rust と逐語一致させる。** `TODO.md` の `ASSERT-DIAG-MESSAGE-01` が
  明示的に含めない範囲としている。Rust 側は `[E0001]` 等の型診断コードを内側に持つが、
  selfhost にはその体系が無い。逐語一致を目標にすると診断コード体系の移植まで引き込む。

## 受入条件

1. `(defn caller [] :assert [(> (nope) 0)] 0)` の selfhost JSON が非空の
   `diagnostics.message` を返し、診断コードと `nope` を含むこと。
2. case の同型 fixture (`(defn identity [x] :case [(expect missing 1)] x)`) が
   同じ形の message を返すこと。
3. property boundary の fixture が非空の message を返すこと
   (span を持たないので切り出しは無い)。
4. 既存の span / code / count / exit code の pin が動かないこと。
5. `Cli.ls` と `EmbeddedCli.ls` が同じ builder を持つこと。

## Evidence

計測環境は 2026-08-23、`cargo test -p lsharp-wasm --test e2e`。
観測は **e2e bundle lane のみ**で行える。既定 CLI の `--format json` は rust runner へ落ちるため
(`LSHARP_DISABLE_EMBEDDED_COMPONENT` と同じ経路)、selfhost の JSON 経路を観測できない。
また **`selfhost/src/**.ls` の編集は凍結済み embedded component artifact を再生成しない**ので、
`target/debug/lsharp` の挙動は本 slice では変わらない。

### RED → GREEN

test: `test_e2e_selfhost_embedded_cli_test_format_json_preflight_diagnostic_message`
(`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs`)

| 段階 | 結果 | 所要 |
|---|---|---|
| RED (実装前) | `FAILED. 0 passed; 1 failed` / `assert preflight の diagnostics.message は非空であるべき` | 249.84s |
| GREEN (実装後) | `ok. 1 passed; 0 failed` | 723.35s |

RED は assert 1 fixture で先に落ちて残り 2 経路まで到達しないため 1 fixture 分の所要、
GREEN は 3 fixture を通すため約 3 倍になる。1 fixture あたり bundle compile + wasm 実行で約 250s。

### 3 経路の実測 message

`(code, span)` は本 slice の実装前に測った値 (Context の表)。message はそこから
`preflight-diagnostic-message` が組む文字列。test が pin したのは **診断コード text と
span 断片の包含**で、下表の全文はその実装からの導出である。

| 経路 | fixture | code | span | message |
|---|---|---|---|---|
| assertion | `(defn caller [] :assert [(> (nope) 0)] 0)` | 1001 | 25..37 | `[LS1001] contract の述語が型検査を通りません: (> (nope) 0) (25..37)` |
| case | `(defn identity [x] :case [(expect missing 1)] x)` | 1001 | 34..41 | `[LS1001] contract の述語が型検査を通りません: missing (34..41)` |
| property | `(defn abs [x] :property [(for-all ...)] ...)` | 3002 | 0..0 | `[LS3002] 未接続の property runner 境界です` |

受入条件 1 の「未定義シンボル名を含む」は、assert 経路では span が述語式全体を指すため
`(> (nope) 0)` の引用として満たしている (D1 のとおり)。case 経路は span が
シンボル単体を指すので `missing` がそのまま出る。

### 受入判定

| # | 受入条件 | 判定 | 根拠 |
|---|---|---|---|
| 1 | assert が非空 message + code + `nope` | 満たす | GREEN test の 3 assertion |
| 2 | case が同型の message | 満たす | 同 test の case fixture |
| 3 | property が非空 message | 満たす | 同 test の property fixture (span 無しで見出しのみ) |
| 4 | 既存 pin が動かない | 満たす | 下記 回帰 |
| 5 | `Cli.ls` / `EmbeddedCli.ls` が同じ builder を持つ | 満たす | `test_e2e_selfhost_preflight_diagnostic_message_builder_is_present_in_both_cli_sources` |

受入条件 5 の contract test は**実装より後に追加した**。TDD の順序としては後追いだが、
`git show HEAD:selfhost/src/App/{Cli,EmbeddedCli}.ls | grep -c 'defn preflight-diagnostic-message'`
が両方 0、worktree が両方 1 であることを確認しており、実装前なら落ちる test であることは検証済み。

### 回帰

| lane | 結果 |
|---|---|
| `selfhost_bootstrap_contracts` | `ok. 20 passed; 0 failed; 1 ignored` / 9.24s (新規 contract test 込み) |
| `selfhost_cli_actual_main_args` | `FAILED. 17 passed; 1 failed; 25 ignored` / 1182.20s |
| `selfhost_cli_core` | `FAILED. 60 passed; 1 failed; 381 ignored` / 195.81s |

**赤 2 本はいずれも本 slice に由来しない。** 受入条件 4 は「既存の pin が動かないこと」なので、
赤を黙って持ち帰らず、原因の特定まで行った。

| 赤 | 原因 | 本 slice との独立性の根拠 |
|---|---|---|
| `selfhost_cli_core::test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms` | property payload が 5 要素から 7 要素へ増えた (`property-runner-form-typed-payload-with-source` が postcondition span と precondition spans を足す) | **`workspace-expected-failures.txt:61` に登録済みの既知 FAIL**。使う bundle は `selfhost_test_runner_runtime_bundle()` で `Cli.ls` / `EmbeddedCli.ls` を含まない |
| `selfhost_cli_actual_main_args::test_e2e_selfhost_cli_main_check_json_aliases` | `914bd9f1` (`I-45`) が 0 引数 defn を `Unit -> body` で登録するようにしたため、`(defn main [] 42)` の `check --json` の `type` が `Int` から `Fn` へ変わった (`render-type-text` は ty-fun を `"Fn"` に潰す) | **凍結済みの `target/debug/lsharp`** (本 slice の未 commit な `.ls` 編集を一切含まない) が既に `{"command":"check","type":"Fn",...}` を返す。本 slice の `Cli.ls` 差分は preflight 4 関数だけで `check` 経路に触れていない |

後者は `I-60` (「0 引数 defn の型を pin する e2e が `I-45` の契約変更で赤のまま放置されている」) が
**「5 本は確定した下限で、全数は未了」**と明記していた続きにあたる。6 本目として `I-60` へ追記し、
pin を新契約へ張り直す作業は別 slice で行う。

`diagnostics.message` を `""` に pin している test (`selfhost_cli_core.rs:8340`) と
Rust oracle との message 比較 (`:8502` 他) はいずれも `#[ignore]` かつ **suite 経路**を見ており、
本 slice の preflight 経路とは交わらない。

### 残渣

- **message の形が suite 経路と揃っていない。** preflight は `[LSxxxx] 見出し: 断片 (start..end)` を
  返すが、suite 経路の `invariant-non-bool-diagnostic-message` は prefix 無しの
  `:invariant は Bool 必須ですが、Int が推論されました` を返す。形の統一は受入条件に入れていない。
- `run-test-source-text` lane は本 slice の対象外で未変更。text 出力の message は空のまま。
- Rust との逐語一致は明示的にスコープ外 (却下案 D)。

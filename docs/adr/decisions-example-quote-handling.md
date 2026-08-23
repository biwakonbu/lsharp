# ADR: `:example` に書かれた quote の扱い

- Status: Accepted
- Date: 2026-08-23
- Scope: `EXAMPLE-QUOTE-01` / `I-62` /
  `crates/lsharp-types/src/metadata_check/diagnostics.rs` の `check_example`
  (対象は `:example` に quote が現れたときの診断のみ。quote/unquote のマクロ展開そのもの、
  selfhost runner 側の診断欠落、lowering 失敗一般の message 整備は含まない)
- Related: [`ISSUES.md` I-62](../../ISSUES.md#i-62)、
  [`:invariant` に書かれた quote の扱い](decisions-invariant-quote-handling.md)、
  [`ISSUES.md` I-59](../../ISSUES.md#i-59)、[`ISSUES.md` I-43](../../ISSUES.md#i-43)、
  [`ISSUES.md` I-65](../../ISSUES.md#i-65)

## Context

`I-59` で `:invariant` の quote は `check_legacy_invariant_types` が弾くようになった。
`:example` は素通しのままで、`I-62` として起票してある。

`I-62` は締め方を 2 つ挙げ、「どちらか一方で足りるのか両方要るのかは、この issue では決めない」と
判断を本 ADR へ預けた。

- **(a)** `check_metadata` に `:example` 用の quote 検出を足す
- **(b)** `lsharp test` の失敗 message に lowering の理由を伝搬させる

### 実測 (2026-08-23、`./target/debug/lsharp`)

`I-62` 本文の実測表は**既定の runner 1 経路だけ**を見ていた。2 経路を分けて測り直すと、
(b) の前提が成り立たないことが分かる。

fixture は 2 つ。

```lisp
(defn caller [x] :example [(caller 'sym)] x)     ; A
(defn caller [x] :invariant (= 'sym 'sym) x)     ; B
```

| fixture | runner | 結果 |
|---|---|---|
| A | selfhost (既定) | `status fail` / `executed 0, failed 1` / **`message` 空** / `count 0` / exit 1 |
| A | rust (`--format json` / `LSHARP_DISABLE_EMBEDDED_COMPONENT=1`) | `message = "[LS1001] テストプログラムの型チェックに失敗: [E0001] 未定義の変数 (undefined): quote/unquote はマクロ展開後に使用できません (63..67)"` / exit 2 |
| B | selfhost (既定) | **`status pass` / `executed 5, failed 0`** |
| B | rust | `[LS1002] [error] caller: :invariant に quote/unquote は書けません (…) (33..37)` |

読み取れる事実は 3 つ。

1. **(b) は rust runner では既に成立している。** 空なのは selfhost runner 側であり、
   これは quote 固有の穴ではなく `run-test-source-json-preflight` が
   `diagnostic-message` を `""` で渡す一般の欠落である (`TODO.md` の `ASSERT-DIAG-MESSAGE-01`、
   `I-49` の残差分)。
2. **rust runner が現に出している message は見出しが誤っている。**
   `[E0001] 未定義の変数 (undefined)` は quote には当てはまらず、
   `I-59` が `:invariant` について潰したのとまったく同じ誤りである。
3. **selfhost runner は B を `pass` と報告する。** message が空なのではなく、
   quote 契約を持たないまま緑を返す。これは (a) でも (b) でも直らない別種の問題で、
   `I-65` として起票した。

`lsharp test` (rust) は `run_metadata_tests` (`crates/lsharp-tooling/src/metadata_test.rs:49`) で
`check_metadata` を先に通し、Error が 1 件でもあれば `[LS1002] <diagnostic>` を返して
compile へ進まない。つまり (a) は `lsharp check` 相当の層で早く落ちるだけでなく、
**rust runner の `lsharp test` の message をそのまま置き換える**。

## Decision

### D1: (a) を採り、(b) は本 slice では採らない

`check_example` に quote 検出を足し、`:invariant` と同型の metadata 固有 Error を出す。

> `:example` に quote/unquote は書けません (実行される例であり、
> quote はマクロ展開後に残らないため)

**(b) を採らない理由**は上記事実 1〜2。rust runner では既に伝搬しており、伝搬していない
selfhost runner 側は quote 固有ではないので `I-62` の担当範囲ではない。
そして (b) だけを足すと、rust runner は「未定義の変数」という誤った見出しを出し続ける。
`I-59` の直後に同じ誤りを `:example` 側で温存する理由が無い。

**「両方要るか」への答えは「(a) だけで `I-62` は閉じる」。** (b) に見えていたものは
`ASSERT-DIAG-MESSAGE-01` (selfhost preflight の message 欠落) と `I-65` (selfhost の quote 契約不在)
の 2 つに分解され、どちらも `:example` に固有ではない。

### D2: 検出範囲は式全体。quote が 1 つでもあれば `:example` 1 件につき Error 1 件

`I-62` は「`:example` の式は任意の呼び出し列なので検出範囲の設計判断が先に要る」と書いたが、
**設計の自由度は実際には無い**。`ir/lower/expr/quote_expr.rs:9` は `Expr::Quote` /
`Unquote` / `UnquoteSplice` を**位置によらず**拒否する。部分的に許す検出範囲を選ぶと、
許した側が lowering で落ちて `I-62` と同じ穴が残る。

したがって `:invariant` と同じ `find_quote_span` (`references.rs:190`、wildcard arm を持たない
網羅 match) を式全体へ当て、最初に見つけた quote の span で 1 件返す。

### D3: 既存の識別子スコープ検査は据え置く。両方出るのを正とする

`check_example` の `collect_var_references` による未定義識別子検査には手を入れない。
結果として `:example [(caller '(a ~nonexistent))]` は **診断 2 件**になる
(未定義識別子 `nonexistent` と quote)。

これは `:invariant` 側の既存構造と一致する。`:invariant` の識別子スコープ検査
(`diagnostics.rs` の `check_invariant`) と quote 検査 (`legacy.rs`) は独立に走り、
同じ入力なら同じく 2 件出る。`:example` だけ片方を抑制する理由が無い。

`I-43` が入れた `contract_scope_unquoted_reference_inside_quote_still_errors` は
「ちょうど 1 件」を assert しているので書き換えになるが、**意図は保存する** --
`~` で戻した参照が検査され続けることを、件数ではなく**両方の Error を名指しして**確認する形へ変える。
緩めるのではなく強くする方向の書き換えである。

## 却下した選択肢

### 案 (b) 単独: `lsharp test` の message に lowering 理由を伝搬させるだけにする

**却下。** rust runner では既に伝搬済みで、残る空 message は selfhost runner の一般の欠落
(`ASSERT-DIAG-MESSAGE-01`)。しかも伝搬している message の見出しが誤っている (事実 2)。
`I-62` の症状は「quote が診断 0 件で通る」ことなので、message を埋めても通ってしまう事実は変わらない。

### 案 (a)+(b) 同時: 本 slice で selfhost の message 欠落も一緒に直す

**却下。範囲が違う。** selfhost preflight の message は assert / case / property を含む
3 経路の共通問題で、`ASSERT-DIAG-MESSAGE-01` が受入条件付きで既に持っている。
ここへ相乗りすると 1 つの修正に台帳項目が 2 つぶら下がり、どちらが正本か分からなくなる。
`I-62` は quote が黙って通ることだけを見る。

### 案 (c): `:example` を quote 対応させる (実行時 Symbol 表現を入れる)

**却下。言語機能の追加であって contract 検査の範囲ではない。**
`:invariant` 側 ADR の案 (a) 却下理由がそのまま当てはまる。
「quote はマクロ展開後には残らない」は `infer/expr.rs:394` と `ir/lower/expr/quote_expr.rs:9` の
2 箇所が独立に実装する言語の契約で、`:example` だけ例外にすると契約が片側で破れる。

### 案 (d): quote を含む `:example` を検査対象外として黙って skip する

**却下。`I-62` の穴そのものである。** `:example` は `test_runner.rs:78` で生成ソースへ
差し込まれて実行されるので、skip しても lowering が拒否する。診断が消えるのではなく、
出る場所が後ろへ移り、span が利用者の書いた `:example` を指さなくなる。

### 案 (e): Warning にする

**却下。** 書いたら必ず実行に失敗する式なので、修正は必須であって推奨ではない。
`:invariant` 側と severity が食い違う理由も無い。

## 受入条件

1. `contract_scope_quoted_symbol_in_example_is_accepted` を
   `contract_scope_quoted_symbol_in_example_reports_metadata_error` へ改名し、
   次の 3 つを assert する -- 診断は**ちょうど 1 件** / 「未定義の識別子」「未定義の変数」を
   **含まない** / `:example` と quote に言及する metadata 固有のメッセージである。
2. `contract_scope_unquoted_reference_inside_quote_still_errors` は診断 2 件になり、
   `nonexistent` に言及する Error と quote に言及する Error が**両方**存在する。
3. `:invariant` 側の 3 つの既存 test (`..._in_invariant_reports_metadata_error` /
   `..._undefined_identifier_in_invariant_still_errors` /
   `..._undefined_identifier_in_example_still_errors`) が緑のまま。
4. rust runner の `lsharp test --format json` が fixture A に対し、
   `diagnostics.message` へ `:example` と quote に言及する `[LS1002]` 系の文字列を載せる
   (現状の `[LS1001] … 未定義の変数 …` から置き換わる)。
5. `cargo test -p lsharp-types` が全 binary 緑、`cargo clippy -p lsharp-types --all-targets` が警告 0。

**selfhost runner (既定経路) の挙動が変わることは受入条件にしない。** `I-65` の担当。

## Evidence

### 実装

1 箇所だけ。`check_example` (`crates/lsharp-types/src/metadata_check/diagnostics.rs`) の
先頭に `find_quote_span` による検出を挿入し、見つかったらその span で Error を 1 件 push する。
`find_quote_span` は `I-59` で追加したものをそのまま再利用した (`references.rs:190`、
wildcard arm を持たない網羅 match なので `Expr` に variant が増えればコンパイルエラーで気付ける)。

既存の `collect_var_references` による識別子スコープ検査には手を入れていない (D3)。
`continue` / early return も置いていないので、両方の Error が並んで出る。

### test

`crates/lsharp-types/src/metadata_check/diagnostics_tests.rs` の 2 本を書き換えた。

- `contract_scope_quoted_symbol_in_example_is_accepted` →
  `contract_scope_quoted_symbol_in_example_reports_metadata_error`
  (`assert_eq!(errors_of(source), Vec::<String>::new())` の 1 行を受入条件 1 の 3 つの assert へ)
- `contract_scope_unquoted_reference_inside_quote_still_errors` は
  「ちょうど 1 件」を「2 件で、`nonexistent` の Error と quote の Error が両方居る」へ

**期待値を実装に合わせて緩めたのではない。** 1 本目は `I-59` が `:invariant` 側で行ったのと
同じ書き換えで、緩い assert のほうが陳腐化していたという裁定 (本 ADR の D1)。
2 本目は件数の assert を**名指しの assert 2 つ**へ置き換えたので、
`I-43` の意図 (`~` で戻した参照が検査され続けること) はむしろ強く固定されている。

### 測定 (2026-08-23)

| 段階 | コマンド | 実測 |
|---|---|---|
| RED | `cargo test -p lsharp-types --lib contract_scope` | `test result: FAILED. 9 passed; 2 failed` — 1 本目が `left: 0 / right: 1`、2 本目が `left: 1 / right: 2` |
| GREEN | 同上 | `test result: ok. 11 passed; 0 failed` (0.01s) |
| 回帰 | `cargo test -p lsharp-types` | test binary 41 本すべて `ok`、`FAILED` 0、exit 0 (最大 binary は `255 passed; 0 failed`) |
| 回帰 | `cargo test -p lsharp-tooling` | `145 passed; 1 failed`。唯一の赤は `api_doc::tests::test_build_api_doc_for_file_preserves_parse_error_code` で、`workspace-expected-failures.txt:139` に登録済みの LS0102 クラスタ (本変更とは無関係) |
| lint | `cargo clippy -p lsharp-types --all-targets` | 警告 0、exit 0 |
| fmt | `cargo fmt --check` | 初回は import 行で diff が出たので `cargo fmt -p lsharp-types` を当て、再実行で clean |

### CLI 実測 (受入条件 4)

fixture `(defn caller [x] :example [(caller 'sym)] x)` に対する
`./target/debug/lsharp test <file> --format json` の `diagnostics`:

| | 変更前 | 変更後 |
|---|---|---|
| `count` | 1 | 1 |
| `firstErrorCode` | 1001 | **1002** |
| `message` | `[LS1001] テストプログラムの型チェックに失敗: [E0001] 未定義の変数 (undefined): quote/unquote はマクロ展開後に使用できません (63..67)` | `[LS1002] [error] caller: :example に quote/unquote は書けません (実行される例であり、quote はマクロ展開後に残らないため) (37..41)` |

code が 1001 → 1002 へ動いたのは `metadata_test.rs:65-70` が「message に『未定義の変数』を含むか」で
分岐しているためで、**誤った見出しが消えたことがそのまま code に現れている**。
span も生成ソースの `63..67` から元ソースの `37..41` (`'sym` の位置) へ移った。

### 受入条件の判定

| # | 条件 | 判定 |
|---|---|---|
| 1 | 改名 + 3 つの assert | 満たした |
| 2 | `'(a ~nonexistent)` が 2 件で両方名指し | 満たした |
| 3 | `:invariant` 側の既存 test 3 本が緑 | 満たした (`contract_scope` フィルタの 11 本に含まれる) |
| 4 | rust runner の message が `[LS1002]` 系へ | 満たした (上表) |
| 5 | `cargo test -p lsharp-types` 全緑 / clippy 0 | 満たした |

**5 つとも文言どおり満たした。** 緩和も読み替えもしていない。

### 影響範囲の確認

`:example` に quote を含む `.ls` は `selfhost/` / `examples/` に **0 件** (grep 実測) なので、
新しい Error でビルドが赤くなるソースは無い。Rust 側の fixture も
`diagnostics_tests.rs` の 2 本だけで、どちらも本 slice で書き換えた。

### 残った問題 (Scope 外)

裁定の過程で、既定の `lsharp test` (selfhost runner) が
`:invariant (= 'sym 'sym)` を `executed 5, failed 0` の**緑**として報告することが分かった。
`I-59` / 本 ADR の診断はどちらも既定経路からは見えない。
`I-65` / `SELFHOST-QUOTE-PARITY-01` として起票し、parity の取り方は別 ADR に委ねた。

`:example` 側の空 `message` は `ASSERT-DIAG-MESSAGE-01` (`I-49` 残差分) の担当のままで、
本 slice では触っていない。

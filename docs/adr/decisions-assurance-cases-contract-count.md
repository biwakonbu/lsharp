# 集計 assurance report の `cases` は contract 数を数える (サンプル数は載せない)

- **Status**: accepted
- **Date**: 2026-08-23
- **Scope**: `implementation_conformance.cases` と `coverage.executed` が
  **ファイル単位の集計 report** で何を数えるか。対象は
  `selfhost/src/Tools/Test/TestRunner.ls` が test result の `actual` slot へ入れる値
  (`:case` / `:invariant` / property の 3 kind) と、
  `crates/lsharp-driver/src/main.rs:1431` の `metadata_test_report_json`。
  `status` / `coverage.failed` / exit code / `seed` / `generator` / `shrinks` は変えない。
- **Related**:
  [`I-68`](../../ISSUES.md#i-68) (本 ADR が解く問題),
  [decisions-selfhost-example-coverage-count.md](decisions-selfhost-example-coverage-count.md)
  (`:example` について同じ判断を先に下した slice。本 ADR はその却下案 D を引き取る),
  [v0.2-evidence-contracts.md](../development/planning/v0.2-evidence-contracts.md)
  (`cases` を導入した設計時の記述)

## 問題

`cases` / `coverage.executed` が何を数えるかについて、**3 者が別のことを言っている。**

| 出所 | `cases` の意味 |
|---|---|
| 設計時の記述 (`v0.2-evidence-contracts.md:159,170`) | サンプル数 (`"cases": 256`) |
| rust runner (`main.rs` の `MetadataTestRun::total()` = `results.len()`) | contract 数 |
| selfhost runner (`TestRunner.ls`) | **kind ごとにバラバラ** |

selfhost の内訳:

| kind | `actual` に入る値 | 結果 |
|---|---|---|
| `:example` | `1` | contract 数 (2026-08-23 に `I-67` で揃えた) |
| `:invariant` | `sample-count` | サンプル数 |
| property | `actual-count` | 実行サンプル数 |
| `:case` | **式の値** | どちらでもない |

`:case` が決定的である。`(expect (f) 3)` は `executed` へ **3** を寄与し、
`(expect (f) 0)` は **0** を寄与して実行数から消える。
**どの契約を採ってもこれは説明できない。** 値と個数を混同している。

## 決定

### 1. 集計 report の `cases` / `coverage.executed` は「実行した contract 数」

全 kind で `actual` に `1` を入れる。`:example` は既にそうなっている。
`:case` / `:invariant` / property を揃える。診断で落ちた場合に `0` を入れる分岐は**変えない**
(`assert_non_bool_invariant_json` が `cases 0` / `executed 0` を live で固定しており、
rust の preflight 値とも一致している)。

### 2. 設計時の「サンプル数」記述と矛盾しない理由

`v0.2-evidence-contracts.md` の `cases` は **Evidence レコード 1 件が持つフィールド**である
(`:152` 「各 Evidence は最低でも次を持つ」)。1 Evidence = 1 contract なので、
そこでの `cases: 256` は「**その contract を 256 サンプル回した**」と読める。

いま出力しているのは **ファイル単位の集計**であり、Evidence レコードではない。
集計でサンプル数を合計すると、`:example` 3 件 (各 1) と property 1 件 (5 サンプル) の
ファイルが `cases 8` になる。**8 が何の個数なのか誰も答えられない。**
kind をまたいで足せる量は contract 数しかない。

したがってこれは設計の撤回ではなく、**粒度の取り違えの是正**である。
per-contract Evidence レコードを出すようになったら、そちらの `cases` はサンプル数でよい。

### 3. サンプル数のフィールドは今は足さない

`I-68` は「サンプル数を残すなら載せ先を決めること」を求めていた。**足さないことを決める。**

- 消費者がいない。サンプル数を読んでいる pin は
  `selfhost_cli_actual_main_args.rs:436` と `native_cli_output.rs:438` の 2 件だけで、
  どちらも「`cases 5` が出ること」を固定しているだけであり、
  **サンプル数を必要としているのではなく現状を写しただけ**である
- `seed` は `0` 固定、`shrinks` は `[]` 固定であり、sampling の情報は**そもそも出ていない**。
  サンプル数だけ足しても sampling plan にはならない
- 足すべき場所は per-contract Evidence レコード (決定 2) であって、集計の新フィールドではない

**再検討の引き金**: per-contract Evidence レコードの出力を実装するとき。
そのときは `seed` / `generator` / `shrinks` と一緒に設計する。

### 4. 既存 pin 2 件は決定に合わせて更新する

`selfhost_cli_actual_main_args.rs:436` / `native_cli_output.rs:438` の `cases 5` は
サンプル数を前提に書かれている。`1` へ更新する。
**これは「テストの期待値を実装に合わせる」ではない** — 契約そのものを本 ADR で変えたので、
契約に追随させる。TDD の禁止事項に当たらないことを明記しておく。

## 却下した案

- **A. selfhost の契約 (サンプル数) を正とし、rust 側を変える。**
  却下。決定 2 の理由で、集計にサンプル数を載せると kind 混在ファイルで意味が壊れる。
  rust を寄せると壊れる側が 2 つになる。
  加えて rust には `:case` の「式の値」に相当する挙動が無いので、
  **selfhost を正とする案は `:case` の扱いを別に決めなければならない。**
  正にする側が自分で説明できていない契約を、正にはできない。

- **B. `cases` はサンプル数、`coverage.executed` は contract 数、と役割を分ける。**
  一見きれいだが、`:example` は両者が一致するので**差が出るのは property だけ**になる。
  読み手は 2 つの数を見比べて初めて「これは sampled だった」と気付く。
  `method` フィールドが既に `sampled-property` を名乗っているので、
  **同じ情報を暗号化して二重に持つことになる。**

- **C. `:case` だけ直し、`:invariant` / property のサンプル数は残す。**
  却下。`:case` の是正だけなら確かに実害 (値を個数として足す) は消えるが、
  同じ 1 フィールドの中で kind によって意味が変わる状態が残る。
  `I-67` を `:example` に閉じて解いた結果として本 issue が生まれており、
  **同じ分割をもう一度やると 3 度目の切り出しになる。**

- **D. `cases` を廃し `contracts` / `samples` の 2 フィールドへ割る。**
  出力 schema の破壊的変更である。`implementation_conformance.cases` を読む pin が
  rust / selfhost 合わせて 9 件あり、`docs/development/planning/v0.2-evidence-contracts.md`
  の記述とも名前が食い違う。`I-68` の影響度は**低**であり、schema 変更に見合わない。
  決定 1 で 2 runner は一致するので、割る必要が無い。

## 波及する pin の事前分類 (doc-RED / 2026-08-23、cargo 非依存の grep で確定)

実装前に、`implementation_conformance.cases` と `coverage.executed` を pin している箇所を
fixture の中身まで見て分類した。**pin 値だけでは「contract 数 2」と「サンプル数 2」を
区別できない**ため、後から突き合わせると考古学になる。

| 位置 | pin | fixture | 変わるか |
|---|---|---|---|
| `native_cli_output.rs:438,440` | `cases`/`executed` == 5 | property 1 本 (`:cases 5`) | **5 → 1** |
| `native_cli_output.rs:708` | `cases > 0` | 同上 (EmbeddedCli 側) | 変わらない (1 > 0) |
| `selfhost_cli_core.rs:8438,8440` | == 0 | 非 bool `:invariant` 1 本 | 変わらない (diagnostic-0 枝) |
| `selfhost_cli_core.rs:8750,8752` | == 2 | `:assert [(truth) (falsehood)]` 1 本 = 述語 2 個 | **2 → 1** |
| `selfhost_cli_core.rs:8806` | `executed` == 0 | 型エラーを含む `:assert` | 変わらない (diagnostic-0 枝) |
| `selfhost_cli_core.rs:8838` | `executed` == 1 | `:assert` 1 本 = 述語 1 個 | 変わらない (両解釈が一致) |
| `selfhost_cli_actual_main_args.rs:436,443` | == 5 | property 1 本 (`:cases 5`) | **5 → 1** |
| `selfhost_cli_actual_main_args.rs:480,482` | == 0 | 非 bool `:invariant` 1 本 | 変わらない (diagnostic-0 枝) |
| `selfhost_cli_actual_main_args.rs:2101,2106` | `cases == executed` | 関係のみを見る | 変わらない (両者が同時に動く) |
| `native-selfhost-dev-source-file-smoke.sh:1483` | `cases`/`executed` == 5 | `$PROPERTY` = property 1 本 (`:cases 5`) | **5 → 1** |
| `metadata_test_cli.rs:60,62,94,96,127` | 2 / 1 / 0 | rust driver 経由 | 変わらない (rust は既に contract 数) |
| `metadata_test_selfhost_case_arity.rs` | `executed() >= 1` | `:case` 各種 | 変わらない (下限比較) |

**更新が要るのは 4 箇所 (test 3 + smoke script 1)**、いずれも「property 1 本の `:cases N` を
N と読んでいた」か「`:assert` の述語数を読んでいた」もの。決定 1 の
「kind をまたいで足せる量は contract 数しかない」を、pin の側から裏返しに示している。

なお `evidence[*].execution.sampling.cases` (`selfhost_evidence_registry/runtime.rs:433`,
`mcp_schema.rs:613`, `native-selfhost-mcp.py:1197` ほか) は**別スキーマで、本 ADR の対象外**。
こちらは contract 単位の Evidence record が持つサンプル数であり、決定 2 が言う粒度の
違いがそのままスキーマ上でも分かれている。触らない。

## Evidence

<!-- doc-GREEN: 実装後に埋める。RED の test 名 / 両 runner の実測 / 上表の「変わるか」の実測結果 -->

## 満たしていないこと

<!-- doc-GREEN: 実装後に埋める -->

# `--ignored` lane 台帳の範囲と test 名の形式

- **Status**: accepted
- **Date**: 2026-08-23
- **Scope**: `docs/development/validation/ignored-lane-expected-failures.txt`,
  `scripts/compare_ignored_lane.py`, `scripts/ci/test-compare-ignored-lane.sh`
- **Related**: [`ISSUES.md` `I-64`](../../ISSUES.md#i-64) (陳腐化 pin が観測されない),
  [`I-23` / `STALE-PIN-01`](../../ISSUES.md#i-23) (台帳の由来),
  [`I-11`](../../ISSUES.md#i-11) (非 ignored lane の baseline),
  [実測記録](../development/operations/ignored-lane-sweep-2026-08-23.md)

## 何が問題か

台帳は `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の 1 module しか測っていない。
ヘッダにその旨は書いてあるが、`IGNORED-STALE-PIN-01` の受入条件は
**「`#[ignore]` 付き e2e を全量実行して赤を列挙する」**であり、1 module では満たせない。

実測すると e2e binary の `#[ignore]` は **1,431 件**あり、台帳が覆うのは `selfhost_native_stage_chain`
の 615 件だけである。残り **816 件は一度も測られていない**。`I-64` が見つけた
`test_e2e_selfhost_cli_lsp_transport_rename_frame` は `selfhost_cli_core` (381 件) にあり、
まさにこの未測定域に属する。台帳の外側で同じ壊れ方が起きているかどうかは、現状では分からない。

範囲を広げると台帳の**名前空間が壊れる**。`compare_ignored_lane.py:32` は
`MODULE_PREFIX = "e2e::selfhost_native_stage_chain::"` を結果行から剥がしてから照合するので、
台帳の 113 件は module 名を持たない裸の test 名になっている。複数 module を混ぜた瞬間、
別 module の同名 test を区別できない。

## 決めたこと

### 1. 台帳の範囲は e2e binary の `#[ignore]` 全量とする

module 単位の台帳を並べるのではなく、1 ファイルで binary 全体を持つ。
測定は module ごとに分割して回すが (下記)、**台帳は分割しない**。

### 2. test 名は `module::test` 形式で持つ

結果行 `test e2e::<module>::<name> ... FAILED` から剥がすのは `e2e::` **だけ**にし、
台帳には `lsharp-wasm::e2e <module>::<name>` と書く。binary の識別は既存の
`lsharp-wasm::e2e ` prefix が担っているので、そこに module を重ねない。

既存 113 件は機械的に `selfhost_native_stage_chain::` を前置して移行する。注記は逐語で保つ。

**移行後、module 名を持たない台帳行は操作ミスとして非 0 で落とす。** 移行漏れを黙って
「未出現」に混ぜると、完走していないのか移行し損ねたのかが区別できなくなる。

### 3. 複数ログの合流規則

module 分割で回す以上、`compare_ignored_lane.py` は複数ログを受け取れなければならない。

| 項目 | 規則 |
|---|---|
| 完走判定 | **ログごとに** 自分の `running N tests` == 自分の結果行ユニーク数、かつ重複 0 |
| 宣言数 | 全ログの `running N` の**和** |
| ログ間の重複 | 同じ `module::test` が 2 ログに出たらエラー (同じ module を 2 回渡している) |
| 差分 | 全ログの和集合に対して従来どおり 4 種 (新規 FAIL / 解消 / 未出現 / 台帳外) |

**この規則は「全 module のログが揃っていること」を検査に変える。** module X の台帳エントリが
あるのに X を覆うログが無ければ、そのエントリは「未出現」になり exit 1 になる。
「18 本揃えるのは運用の約束」ではなく、破ると落ちる不変条件になる。

### 4. 測定は module 分割 + `os.setsid()` で行う

1 プロセスで全量を回すと 12 時間規模になり、途中で失われると全損する。module ごとに
`--ignored <module::>` で filter して順に回し、ログを分ける。
所要が module ごとに記録できる副次効果もある (運用記録に載る)。

`selfhost_native_stage_chain` も**繰り越さずに測り直す**。台帳の数値は `35ea7c32` (2026-08-19) 時点で、
以後 `TestRunner.ls` を 2 slice 分編集している。同 module の FAIL は `selfhost/src/**.ls` を
`read_to_string` してソース本文へ文字列 assertion するものが主体なので、
`TestRunner.ls` の編集はこの pin を動かしうる。**繰り越しは測定ではない。**

## 却下した案

### 却下 A: module ごとに台帳ファイルを分ける

18 ファイルになり、`compare_ignored_lane.py` の呼び出しも 18 回になる。
「どの module の台帳が古いか」を人が覚える運用になり、`I-64` が問題視した
**「誰も見ていないので陳腐化に気付けない」**構造をそのまま再生産する。
1 ファイルなら差分が 1 箇所に出る。

### 却下 B: 裸の test 名のまま module を混ぜる

移行コストは 0 だが、別 module の同名 test を区別できない。
e2e の test 名は module ごとに命名規則が違うので現時点で衝突は無いが、
**衝突が起きたとき静かに誤照合する**のが最悪の壊れ方である。`I-64` と同型。

### 却下 C: `MODULE_PREFIX` を CLI 引数にする

呼び出し側が正しい prefix を渡すことに依存する。渡し間違えると全件「台帳外」になり、
「台帳外はそれ自体を失敗にしない」という既存の判定 (`compare_ignored_lane.py:117`) に
吸われて **exit 0 で通る**。検査が沈黙する方向の失敗なので採らない。

### 却下 D: 全量を 1 プロセスで回して 1 ログにする

`compare_ignored_lane.py` の変更は最小 (prefix の剥がし方だけ) で済むが、
12 時間規模の run が途中で失われると全損する。実際 `I-11` の分割測定は
「時間短縮ではなく部分成果が残ること」を理由に採用した先例がある。

## Evidence

<!-- 測定完了後に埋める。sweep の生ログは /Users/biwakonbu/github/tmp/i64/ -->

- 契約テスト: `scripts/ci/test-compare-ignored-lane.sh` (cargo 非依存)
- 実測: [ignored-lane-sweep-2026-08-23.md](../development/operations/ignored-lane-sweep-2026-08-23.md)

## 満たしていないこと

- **e2e binary の外は依然として測っていない。** `lsharp-types::forward_reference_generalization`
  の 5 件 (`I-46` / `I-48`) は意図的に `#[ignore]` にしてあるが、本台帳の範囲外のままである。
  台帳ヘッダの注記を維持する。
- 台帳と `workspace-expected-failures.txt` は引き続き別ファイルであり、
  `check-workspace-baseline.sh` は本台帳を読まない。ignored lane に自動検証は付かない。

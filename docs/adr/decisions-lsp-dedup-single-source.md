# 診断 dedup の単一正本化 — `merge-duplicate-diagnostics` の廃止

- **Status**: Accepted (2026-08-19 に実装し、受入条件 3 件すべてを実測で満たした)
- **Date**: 2026-08-18 (判断) / 2026-08-19 (実装)
- **Scope**: `selfhost/src/Tools/Lsp/LspServerNav.ls` / `selfhost/src/Tools/Lsp/LspServer.ls` と
  それらを pin する parity test 3 本
- **Related**: `ISSUES.md` の `I-24`、`TODO.md` の `LSP-DEDUP-MERGE-01`、
  [lint dedup identity ADR](decisions-lint-diagnostic-dedup-identity.md)

## 何が問題か

同じ「診断の重複を潰す」概念に対して実装が 2 つある。

| 実装 | 位置 | 呼び出し元 |
|---|---|---|
| `dedup-diagnostics` | `LspServerNav.ls:1293` | `Cli.ls:1440` / `:1687` / `:1702` (**実運用の publish 経路**) |
| `merge-duplicate-diagnostics` | `LspServerNav.ls:1169` | `LspServer.ls:144` の**検証用 `main`** と parity test 3 本のみ |

呼び出し元の全数は実測で確定させた (`grep -rn` を `selfhost/` / `crates/` / `scripts/` / `docs/` に対して実行)。
`merge-duplicate-diagnostics` は実運用経路から 1 箇所も呼ばれていない。

## 実装差の実測

意味論の違いは 2 つあり、**どちらも `merge-duplicate-diagnostics` 側が弱い**。

1. **長さ 2 しか扱わない。** `(if (= len 2) ... diagnostics)` という構造で、
   0 / 1 / 3 件以上の入力は**そのまま返す**。実際の診断リストは 3 件以上になるので、
   これは dedup として機能していない。`dedup-diagnostics` は `dedup-build` が
   全要素を O(n²) で走査する。
2. **lint の rule identity を見ない。** 同一 start span を rule を問わず 1 件へ潰す。
   `dedup-diagnostics` は `dedup-diag-same-span` (`:1240-1246`) で
   `source = 3` (lint) のとき `dedup-diag-same-lint-identity` を追加で要求する。
   これは `I-24` で「rule identity を残す」と裁定済みの意味論であり、
   `merge-duplicate-diagnostics` はその裁定の**逆**を実装している。

## 決定

**`merge-duplicate-diagnostics` を削除し、`dedup-diagnostics` を単一正本にする。**

同時に (a) `LspServer.ls:144` の検証用 `main` を `dedup-diagnostics` 呼び出しへ差し替え、
(b) parity test 3 本 (`lsp_diagnostic_parity.rs:98` / `:121`、
`selfhost_lsp_docs_ops.rs:126` の `TEST-LSP-06`) の呼び出し先を差し替える。

**期待値は変えない。** これは TDD 規約の「test の期待値を実装に合わせて変更しない」に抵触しないため。
pin されている入力は severity 2/1・同一 line 5 col 7・`source = 0` の 2 件で、
`source != 3` なので `dedup-diag-same-span` は line/col 一致だけで同一と判定し、
`dedup-diag-pick-best` が severity の小さい方を選ぶ。**結果は 1 件・severity 1 で現行の pin と一致する。**
すなわち pin 入力の範囲では `dedup-diagnostics` は `merge-duplicate-diagnostics` の厳密な上位互換であり、
差し替えは期待値の書き換えを伴わない。

## 却下した選択肢

- **両方残し、意味論の違いを doc に書く。** 却下。違いは意図ではなく実装漏れ
  (`len == 2` 限定は設計として書きようがない)。同じ概念の実装が 2 つあると、
  次に dedup を直す人がどちらを直すべきか判断できない。`I-24` の裁定を
  片方だけに入れた現状が、まさにその劣化である。
- **`merge-duplicate-diagnostics` を一般化して残し、`dedup-diagnostics` を消す。** 却下。
  実運用経路が使っているのは `dedup-diagnostics` の方であり、`I-24` の裁定もそちらに入っている。
  正本を実運用から外れた側へ寄せる理由が無い。
- **検証用 `main` ごと消す。** 却下。この `main` は selfhost モジュールを
  native/wasm で走らせる際の shape 検証に使われており、dedup とは独立の役割を持つ。
  本 ADR のスコープを越える。

## 受入条件

- `grep -rn 'merge-duplicate-diagnostics'` の hit が 0 (docs の履歴記述を除く)
- parity test 3 本が**期待値を変えずに** pass する
- `Cli.ls` 経路の pin (`test_e2e_selfhost_cli_lsp_stdio_didopen_*` 7 件) が引き続き pass する

## Evidence

実装は 2026-08-19、worktree `/Users/biwakonbu/github/tmp/lsharp-diag-dedup`
(branch `codex/lsp-dedup-01`) で実施。変更は 4 ファイル 7 hunk。

| 受入条件 | 判定 | 実測 |
|---|---|---|
| `merge-duplicate-diagnostics` の hit が 0 (docs 除く) | **満たす** | `grep -rn 'merge-duplicate-diagnostics' selfhost crates scripts` → 0 hit |
| parity test 3 本が**期待値を変えずに** pass | **満たす** | 下表 |
| `Cli.ls` 経路の pin 7 件が引き続き pass | **満たす** | `test result: ok. 7 passed; 0 failed` |

parity test の内訳。

| test | 結果 |
|---|---|
| `lsp_diagnostic_parity::test_e2e_lsp_diagnostic_dedup_keeps_higher_severity` | `ok` |
| `lsp_diagnostic_parity::test_e2e_lsp_diagnostic_dedup_keeps_different_spans` | `ok` |
| `selfhost_lsp_docs_ops::test_e2e_selfhost_lsp_runtime_merge_duplicates` (TEST-LSP-06) | `ok` |

`lsp_diagnostic_parity` は同 binary の 15 件全部が `ok` (144.12s)。TEST-LSP-06 は
`LspServer.ls` の検証用 `main` の出力行を読む test なので、`selfhost_lsp_runtime_*` 10 件を
まとめて回して 10/10 `ok` (44.32s)。後続の TEST-LSP-07 / 08 が同じ出力の 13〜16 行目を
読んでいるため、行数がずれていないことも同時に確認できている。

`Cli.ls` 経路の pin 7 件は `e2e::selfhost_cli_core::test_e2e_selfhost_cli_lsp_stdio_didopen_`
で一括実行し 7/7 `ok`、wall clock 1182.61s (7 件並列、`NATIVE-ROOT-01` の 614 件 sweep と
CPU を共有した状態での実測なので、1 件あたりの所要としては読まないこと)。

**期待値は 1 行も変更していない。** ADR で予測したとおり、pin 入力の diag は
index 5 (`source`) が 0 なので `dedup-diag-same-span` は line/col だけで一致を判定し、
`dedup-diag-pick-best` が severity 1 を選ぶ。`len == 2` に限定されていた旧実装と同じ結果に落ちる。

### 予測との差

無し。削除した関数は production 経路から呼ばれていなかったため、`Cli.ls` 側の
publish 経路には一切の挙動変化が無い。変わったのは検証用 `main` と parity test が
どちらの実装を呼ぶかだけである。

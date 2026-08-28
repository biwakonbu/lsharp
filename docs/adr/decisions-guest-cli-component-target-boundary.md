# guest CLI の component target 境界を e2e 3 件へ反映する

- **Status**: doc-GREEN (focused 5 本まで / lane 未了 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED) / 2026-08-28 (実装) / 2026-08-28 (測定)
- **Scope**: `--target wasi-component` / `--target wasm` を渡す `selfhost_cli_core` の e2e 3 件
  (`TEST-CLI-02-AF6` / `AF6B` / `AF7`) を、guest の capability boundary へ合わせる。
- **含めない範囲**: guest への component packaging の実装。`SMOKE-GATE-03` (CI)。
  `-o` に何が書かれるかの契約 (`CLI-OUTPUT-CONTRACT-01` /
  [出力先契約 ADR](decisions-guest-cli-output-path-contract.md) が持つ)。
- **Related**: `ISSUES.md` の `I-94` / `TODO.md` の `CLI-COMPONENT-TARGET-EXPECT-01` /
  `I-15` (同じ境界を resolved として文書化済み) / `I-93` (同じ commit 由来の兄弟)

## 何が問題か

3 件が `--target wasi-component` / `--target wasm` を渡したうえで `wasm-size:<n>` が
返ることを期待している。guest はその target を **capability boundary で拒否する**ので、
3 件とも `exit code 1` /
`error: wasi-component output requires external component packaging` で落ちる。

`wasm` は `parse-compile-target-name` (`Cli.ls:47`) が `compile-target-component` へ写すので、
`AF7` も component 側である。境界は `run-compile-output` (`Cli.ls:875-889`) と
`run-compile` (`Cli.ls:1437`) の 2 箇所にあるが**条件も message も同一**である。

## 裁定: 境界は仕様。3 件を「境界を期待する」形へ書き換える

**これは「実装が正しいから期待値を直す」ではない。** 境界の正しさは実装出力より前に
別の正本が確定させている。

1. **`I-15` が resolved としてこの境界を文書化している。**
   `EmbeddedCli.ls:1215/1230/1231` の同じ境界について「`run-compile-output` は target が
   `preview1` でなければ**無条件に** `error: wasi-component output requires external
   component packaging` を出して非 0 終了する」と記録済みである。
2. **理由は capability であって未実装ではない。** component packaging には外部ツール
   (`wasm-tools` 相当) が要る。guest は WASI Preview1 の中で動くので単独では遂行できない。
   **時間をかければ実装できる類の欠落ではない。**
3. **同じ問いを既に一度裁定している。**
   [`decisions-default-path-smoke-determinism.md`](decisions-default-path-smoke-determinism.md)
   は smoke script の同型 assertion に対し、**案 X (`wasm-size:` または `コンパイル成功:` の OR)
   を却下**して「既定 target で `wasm-size:` は出ない」を前提として固定した。
   e2e 側だけ別の裁定を採る理由が無い。

したがって欠陥は test 側にある。出所は `I-93` と同じ `2ba93d0a` (2026-07-12) が
`component-output-boundary-message` を入れたときに、`bc752767` (2026-03-31) の test 3 件を
更新しなかったことである。

## 却下した案

**案 A -- guest に component packaging を実装する。却下。**
外部ツールが要るので guest 単独では遂行不能。`I-15` が capability boundary として
既に resolved にしている。本項目は「境界を実装しに行く項目ではない」と `TODO.md` にも書いてある。

**案 B -- 3 件を削除する。却下。**
削除すると **`--target` の値が parse されて正しい枝へ届いていること**を見る e2e が
`selfhost_cli_core` から消える。境界に届かず `unsupported target` や
`missing value for option` で落ちる回帰を拾えなくなる。境界は仕様なのだから、
仕様として pin するのが筋である。

**案 C -- 境界 message だけを assert して exit code を見ない。却下。**
`I-91` の是正は「非 0 終了に stdout を載せる」ことだった。message だけを見ると、
exit code が 0 に退行しても気付けない。**非 0 で落ちることが境界の本体**である。

**案 D -- `AF6B` を preview1 側だけの test に縮める。却下。**
preview1 単独の `wasm-size:` は `AF1` (`compile input.ls`) が既に見ている。
縮めると `AF6B` は重複 test になり、**「target ごとに挙動が変わる」という主題が消える。**

## 採る形と、検査しなくなるもの

| test | 変更後に見るもの |
|---|---|
| `AF6` (`compile … --target wasi-component -o targeted.txt`) | 非 0 終了 + 境界 message。**output file が作られないこと** |
| `AF7` (`build … --output build-target.txt --target wasm`) | 同上。`wasm` alias が component 側へ写ることを含む |
| `AF6B` (`--target wasi-preview1` と `--target wasi-component` の 2 回) | preview1 は `wasm-size:<n>`、component は非 0 + 境界 message |

**検査しなくなるもの (明記が受入条件):**

- **`AF6B` の `preview1_size > component_size`。** component 側が成功しないので比較自体が
  成立しない。**この大小関係を見る test は本変更後どこにも無くなる。** 復活させるとしたら
  guest ではなく host launcher 側 (両 target が成功する経路) に置くべきで、
  それは本 slice の範囲外である。
- **`AF6` / `AF7` の `wasm-size:<n>` 形式。** 同形式は `AF1` / `AF3` / `AF4` / `AF5` が
  引き続き見るので、surface としての被覆は失われない。

## test 名の変更 1 件

`..._compile_target_changes_wasm_size` は「target ごとに wasm-size が変わる」という
**成立しない主張を名前に埋めている**ので、
`..._compile_target_preview1_ok_component_refused` へ改名する。
`AF6` / `AF7` の名前 (`..._compile_target_and_output_path` /
`..._build_output_path_and_target_alias`) は中立なので変えない。

改名は `AGENTS.md` の「rename した module は再計測が必要」に触れるが、
`selfhost_cli_core` は `SWEEP-LANE-RERUN-01` で丸ごと回すので追加の測定は生じない。
`docs/development/planning/compatibility-matrix.md:45` が旧名を持っているので同時に直す。

## Evidence

### 実行

`I-93` と同じ module の隣接する test なので、**1 回の focused run で 5 本まとめて**測った。
以下は共通の実行記録であり、[出力先契約 ADR](decisions-guest-cli-output-path-contract.md)
の Evidence と同一の run を指す。

| 項目 | 値 |
|---|---|
| 実行 | `target/debug/deps/e2e-aa343ded249bec81 --exact --ignored --test-threads 1 <5 本>` |
| 起動 | `/Users/biwakonbu/github/tmp/i93/run_probe.py` を `os.setsid()` で切り離し。pid 53421 (child 53422) |
| ログ | `/Users/biwakonbu/github/tmp/i93/probe.log` |
| 結果 | `test result: ok. 5 passed; 0 failed` / `RUNEXIT=0` / `ELAPSED=1298.10` |
| 母数 | `3078 filtered out` + 5 = **3083** |

母数について 1 点記録しておく。同じ 5 本を修正前に測った harvest (2026-08-28 08:53) は
`3076 filtered out` + 5 = **3081** だった。本 slice は helper 2 個の追加と test 1 件の改名だけで
test を増やしていないので、差の +2 は本 slice 由来ではない。`5fea0b3a`
(「独立 review gate の outcome=pass 条件を selfhost 残り 2 経路へ伝播」) が e2e に 2 本足したのが
実体である (`git show --unified=0 5fea0b3a -- crates/lsharp-wasm/tests/e2e` で確認)。
**次の lane の完走判定はこの新しい母数で行う。**

### 本 ADR が受け持つ 3 本

| test | 変更前 | 変更後 |
|---|---|---|
| `..._compile_target_and_output_path` (`AF6`) | `support.rs:188` `実行に失敗: exit code 1; stdout="error: wasi-component output requires external component packaging\n"` | `ok` |
| `..._compile_target_preview1_ok_component_refused` (`AF6B`、旧 `..._compile_target_changes_wasm_size`) | 同上 | `ok` |
| `..._build_output_path_and_target_alias` (`AF7`) | 同上 | `ok` |

3 件とも失敗文字列が完全に一致していた。経路は 2 通り (`run-compile-output` 2 件 /
`run-compile` 1 件) だが**境界は同一**である。

### 期待値をどう置き換えたか

非 panic 版 helper `try_compile_and_run_with_dir_and_args` (`support.rs`) を足し、
`Result` を helper `assert_component_target_refused` (`selfhost_cli_core.rs`) へ渡す形にした。
helper が見るのは次の 2 点である。

1. `exit code 1` であること (`exit-compile-error` = 1、`Cli.ls:35-36`)
2. 失敗文字列に `error: wasi-component output requires external component packaging` が載ること

guest に stderr は無く `cli-stderr` (`Cli.ls:2291`) が `error: ` を付けて stdout へ出すので、
`format_nonzero_exit_error` (`wasi_runner.rs:156`) が付ける `stdout=` の中に message が入る。
**この可視化は `I-91` の是正が入っていて初めて成り立つ。** 是正前ならこの assertion は書けなかった。

AF6 / AF7 では `remove_dir_all` の**前に** output file の存在を捕まえ、
`!output_exists` を主張している。境界で落ちる経路は `write-file-bytes` に到達しないので、
output file が生成されないことが境界の副次的な証拠になる。

### 検査しなくなるもの (`I-76` と同じ失敗を繰り返さないための明記)

- **`..._compile_target_changes_wasm_size` が見ていた preview1 と component の
  `wasm-size` の大小関係は、本変更後どこの test にも残らない。** component 側が
  そもそも size を出さないので、この比較は原理的に成立しない。改名後の
  `..._compile_target_preview1_ok_component_refused` は preview1 側で `wasm-size > 0` を
  見るだけである。
- AF6 / AF7 が見ていた `wasm-size:<n>` の**書式**は、これらの test では見なくなる。
  ただし `AF1` / `AF3` / `AF4` / `AF5` が preview1 経路で同じ書式を見ているので、
  書式の回帰検出は失われていない。

## 満たせなかったこと

- **`selfhost_cli_core` の lane 再計測をまだ回していない。** 台帳 3 行は**注記を更新しただけで
  削除していない**。削除は `SWEEP-LANE-RERUN-01` の後である。
- **guest への component packaging は実装していない。** 本 ADR は境界を仕様と裁定した
  だけで、境界を動かしていない。外部ツールが要るという `I-15` の判断はそのままである。
- **`--target wasm` が component へ写ることの是非は裁定していない。** `parse-compile-target-name`
  (`Cli.ls:47`) が `wasm` を `compile-target-component` に写すのは本 slice の前からの挙動で、
  AF7 はその写像に依存している。この別名が妥当かは別の判断である。

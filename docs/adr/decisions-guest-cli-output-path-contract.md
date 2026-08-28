# guest CLI の `compile -o` / `build --output` 出力先契約

- **Status**: doc-GREEN (focused 5 本まで / lane 未了 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED) / 2026-08-28 (実装) / 2026-08-28 (測定)
- **Scope**: selfhost guest CLI (`selfhost/src/App/Cli.ls` の `run-compile-output` /
  `run-build-output`) が `-o` / `--output` へ何を書くかの正本確定と、それに依存する e2e 2 件
  (`selfhost_cli_core` の `TEST-CLI-02-AF4` / `AF5`)。
- **含めない範囲**: `--target` 系の期待値 (`CLI-COMPONENT-TARGET-EXPECT-01` /
  [component target ADR](decisions-guest-cli-component-target-boundary.md) が持つ)。
  host launcher 側 (`crates/lsharp-driver`) の `-o` 実装変更。component packaging。
- **Related**: `ISSUES.md` の `I-93` / `TODO.md` の `CLI-OUTPUT-CONTRACT-01` /
  `I-15` (guest 出力の実測表) / `I-90` (同型の「設計ミス例外」前例)

## 何が問題か

e2e 2 件が `compile input.ls -o out.txt` / `build input.ls --output build.txt` の
**output file に stdout summary text が入る**ことを期待している。

```rust
let written = std::fs::read_to_string(dir.join("out.txt")).unwrap();
assert_eq!(written.trim(), lines[0], "compile -o は stdout summary を output file にも書くべき");
```

実装は逆で、output file に wasm binary を書き summary は stdout にしか出さない。
`read_to_string` が binary を読んで `InvalidData "stream did not contain valid UTF-8"` で
panic するため、**assert には一度も到達していない**。実測は `I-93` が持つ。

**この 2 件は「期待値を実装出力に合わせて直す」形になる。** `CLAUDE.md` が禁じる形なので、
`I-90` と同じ精査 -- 実装出力とは独立な根拠を 3 点以上そろえる -- を先に済ませる。

## 裁定: 実装が正本。output file = artifact、stdout = summary

`-o` / `--output` は **artifact の出力先**であり、summary text の出力先ではない。
summary (`wasm-size:<n>`) は stdout にだけ出る。

### 根拠 (実装出力とは独立なもの 5 点)

いずれも「実装を走らせて出た値」ではなく、**別の source / 別の正本が先に書いている契約**である。

1. **Rust driver は output path の拡張子から compile target を推論する。**
   `crates/lsharp-tooling/src/compile.rs:190` の `infer_target_from_output_path` は
   `.component.wasm` -> `WasiComponent`、`.wasm` -> `WasiPreview1`、それ以外 -> `Native` と写す。
   output file が summary text ならこの推論は意味を成さない。**`-o` が artifact path で
   あることに driver の target 解決が依存している。**
2. **driver の MCP 経路は output path を読んで実行する。**
   `crates/lsharp-driver/src/mcp_compile.rs:58` は `std::fs::read(&artifacts.output_path)` を
   `wasm_bytes` として受け、そのまま `run_wasm_wasi` へ渡す。summary text では動かない。
3. **既存 ADR が 4 経路の実測表で契約を確定済みである。**
   [`decisions-default-path-smoke-determinism.md`](decisions-default-path-smoke-determinism.md)
   の Evidence 表 (`:141-145`) は guest 経路 `compile … --target wasi-preview1 -o <out>.wasm` に
   対し **stdout `wasm-size:2904` / output file 先頭 4 byte `0061736d`** を記録している。
   stdout と output file が別物であることは 2026-08-18 に既に正本へ入っている。
4. **運用手順書が同じ形を受入条件として書いている。**
   `docs/development/operations/default-path-migration.md:78` は
   「`wasm-size:<n>` summary を返し**つつ** output file を作ること」を移行の合格条件にしている。
5. **互換表が同じ分担を書いている。** `docs/development/planning/compatibility-matrix.md:45`
   は「guest surface では `wasm-size:<n>` summary を返せる。host launcher default path は
   same summary を保ちつつ output file には実 Wasm / Component bytes を書き」と、
   stdout / output file の分担を明示している。

したがって欠陥は test 側にあり、`CLAUDE.md` の「テストの設計ミスを除く」側に当たる。
**設計ミスの出所は特定できている**: `2ba93d0a` (2026-07-12) が `write-file-bytes` を
導入したとき、`9deab1ce` (2026-03-27) が先に書いた test 2 件の期待値を更新しなかった。
`I-90` の `9175c6e5` と同型である。

## 却下した案

**案 A -- 実装を変えて output file に summary text を書く。却下。**
上記 1 / 2 が壊れる。driver は拡張子から target を推論し、MCP 経路は output file を
実行する。guest だけ summary text を書けば **host と guest で `-o` の意味が割れる**。
`I-15` が記録した guest preview1 経路の出力先頭 `0061736d` とも矛盾する。

**案 B -- output file に summary と binary の両方を書く (先頭に text 行、続けて binary)。却下。**
生成物が wasm module として不正になる。`wasm-tools validate` も `run_wasm_wasi` も通らない。
test 1 本を通すために artifact を壊す取引であり、割に合わない。

**案 C -- test 2 件を削除する。却下。**
`-o` / `--output` が **actual `main` の argv 経由で**効くことを見ている唯一の e2e である
(`compatibility-matrix.md:45-46` が両者を compile / build の証拠 test として名指ししている)。
消すと argv 解釈の回帰を拾えなくなる。**壊れているのは assertion 1 行であって test の主題ではない。**

**案 D -- `read_to_string` を `read` に替えるだけにして、中身は検査しない。却下。**
panic は消えるが「output file に何が書かれるべきか」を誰も検査しない状態になる。
本 ADR が確定させた契約を test が持たないなら、確定させた意味がない。

## 採る形

`std::fs::read` でバイト列として読み、次の 2 つを assert する。

| 検査 | 根拠 |
|---|---|
| 先頭 8 byte が `\0asm\x01\x00\x00\x00` | core module の magic + version。component なら `\x0d\x00\x01\x00` になるので、target が preview1 に解決されたことまで同時に見える |
| **byte 長が stdout の `wasm-size:<n>` と一致する** | `Cli.ls:874-887` は `wasm-size` を `(vector-length wasm-bytes)` から作り、**同じ `wasm-bytes` を `write-file-bytes` へ渡す**。source 上の恒等式であって実測値ではない |

2 つ目は元の assertion より**強い**。元は「stdout と output file が文字列として等しい」だったが、
新しい形は「stdout の数値が output file の実バイト長と一致する」を見る。
stdout summary と artifact の対応は失われない。

## Evidence

### 実行

`I-93` と `I-94` は同じ module の隣接する test を触るので、**1 回の focused run で 5 本まとめて**
測った。以下は共通の実行記録であり、[component target ADR](decisions-guest-cli-component-target-boundary.md)
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

### 本 ADR が受け持つ 2 本

| test | 変更前 | 変更後 |
|---|---|---|
| `..._compile_output_path` (`TEST-CLI-02-AF4`) | `read_to_string(out.txt)` が `InvalidData "stream did not contain valid UTF-8"` で panic | `ok` |
| `..._build_output_path` (`TEST-CLI-02-AF5`) | 同上 | `ok` |

### 期待値をどう置き換えたか

`std::fs::read_to_string` を `std::fs::read` へ変え、3 本の assertion を helper
`assert_preview1_artifact_matches_summary` (`selfhost_cli_core.rs`) の 1 呼び出しへ畳んだ。
helper が見るのは次の 2 点である。

1. output file が core module の magic + version (`\0asm\x01\x00\x00\x00`) で始まること
2. **output file の byte 長が stdout summary の `wasm-size:<n>` と一致すること**

**置き換え後の方が主張は強い。** 変更前は「output file の text == stdout の 1 行目」という
文字列比較だったが、変更後は stdout の数値と artifact の実バイト長を結んでいる。
この恒等式は `Cli.ls:874-887` が `wasm-size` を `(vector-length wasm-bytes)` から作り、
**同じ `wasm-bytes` を `write-file-bytes` へ渡す**ことから source 上で証明できる。
すなわち `-o` が別のバイト列を書いたり summary を捏造したりすれば落ちる。

### `CLAUDE.md` の禁止事項との関係

「テストの期待値を実装に合わせて変更した」形になるが、**根拠は実装の出力ではない。**
上の Decision 節に挙げた 5 点 (host launcher の `infer_target_from_output_path`、
MCP compile が `artifacts.output_path` を wasm として実行すること、既存 ADR の実測表、
運用記録、互換表) はいずれも本 run の出力と独立で、しかも本 test の実装とも独立である。
`テストの設計ミスを除く` 側の例外にあたり、設計ミスの出所は `2ba93d0a` (2026-07-12) が
契約を変えたときに先行 test 2 件を更新しなかったことである (`I-90` と同型)。

## 満たせなかったこと

- **`selfhost_cli_core` の lane 再計測をまだ回していない。** focused 5 本の緑は lane 1 本の
  完走ではない。台帳 2 行 (`docs/development/validation/ignored-lane-expected-failures.txt` の
  `..._compile_output_path` / `..._build_output_path`) は**注記を更新しただけで削除していない**。
  削除は `TODO.md` の `SWEEP-LANE-RERUN-01` が回す 1 本の後である。
- **host launcher (`crates/lsharp-driver`) 側の `-o` は測っていない。** Decision の根拠として
  source を読んだだけで、本 slice では実行していない。Scope 外である。
- **`.component.wasm` 拡張子の経路は guest 側に無い。** host launcher の
  `infer_target_from_output_path` は持つが、guest CLI は拡張子から target を推定しない。
  この非対称は本 ADR では裁定していない (`I-15` の境界の内側にある)。

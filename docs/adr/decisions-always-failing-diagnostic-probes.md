# 構造上必ず赤くなる診断 probe の裁定

- **Status**: doc-GREEN (完了 / 2026-08-28)
- **Date**: 2026-08-27 (doc-RED) / 2026-08-27 (実装) / 2026-08-28 (lane 完走確認)
- **Scope**: `crates/lsharp-wasm/tests/e2e/` の 5 test と、それが使う lookup harness
- **Related**: `I-84` (本 ADR の起点) / `I-81` (発見経路。5 件のうち 1 件) /
  `I-75` (誤分類していた 1 件) / `I-82` と
  [`decisions-probe-subject-unchecked.md`](decisions-probe-subject-unchecked.md) (裏返しの類型)
- **引き取り先**: `TODO.md` の `ALWAYS-RED-PROBE-01` / `VIOLATION-PROBE-STALE-01`。
  どちらも 2026-08-28 に完了・削除済み

## 背景

`I-81` の裁定のために `test_v2_12_self_hosted_stage2_reports_compiler_mode_first_violation_body_diff`
を読んだところ、**body に分岐も `return` も無く、最後に無条件 `panic!` していた**。
`I-81` は当初これを「violation が消えた結果として足場が成立しなくなった」と書いていたが、
そうではない。**この test は一度も緑になったことがない。**

同じ形が他にもあるはずなので `scripts/sweep_always_failing_tests.py` を書いて走査し、**5 件**を得た。
5 件とも `#[ignore]` を持ち、5 件とも ignored lane の台帳に載っている。

## これがなぜ問題か

`scripts/compare_ignored_lane.py` の契約
([`decisions-ignored-lane-ledger-scope.md`](decisions-ignored-lane-ledger-scope.md)) は
**「緑に転じた台帳行は削除する」**ことを前提にしている。台帳は「まだ直っていない赤」の集合であり、
だからこそ行数が減ることが進捗を意味する。

**この 5 件は緑に転じ得ない。** 恒久的に 5 行を占め、未解決の欠陥と区別がつかない。
実際 `I-75` は 1 件を「原因未診断の赤」として保持していたが、原因も何も**成功経路が存在しない**。

> 台帳に載る赤には 2 種類ある — 「直せば消える赤」と「構造上消えない赤」。
> 混ぜると、行数が進捗を表さなくなる。

## 裁定

| # | test | 裁定 |
|---|---|---|
| 1 | `..._reports_compiler_mode_first_violation_body_diff` (`four_layer/part_014.rs:154`) | **極性を反転** |
| 2 | `..._direct_module_resolver_full_inline_mismatch_probe` (`selfhost_cli_core.rs:2870`) | **極性を反転** |
| 3 | `..._representative_crash_offset_maps_to_rust_function` (`stage_chain.rs:26574`) | **削除** |
| 4 | `..._representative_post_entry_call_targets_map_to_source_order` (`stage_chain.rs:26623`) | **削除** |
| 5 | `..._representative_crash_x8_offset_maps_to_source_order` (`stage_chain.rs:26660`) | **削除** + 1 件を実契約へ作り替え |

### 裁定 1: 極性を反転する (#1 / #2)

どちらも「**無いことが正常**」な性質を probe しており、現在の実測は「無い」。

| test | probe している性質 | 現在の実測 |
|---|---|---|
| #1 | stage3 出力に local bound violation (`local.get` 等が宣言範囲外を参照) があるか | **0 件** |
| #2 | 同じ module を 2 回 inline compile して同じ Wasm になるか (`wasm-bytes-eq` / `mismatch-idx`) | mismatch 無し (`.expect` 手前まで到達していない — 要実測) |

したがって、末尾の無条件 `panic!` を**条件付き**に変えるだけでよい。

```rust
// before: 必ず落ちる
let first_bad = *bad_indices.first().expect("... violation があること");
/* ...dump を組み立てる... */
panic!("V2-12 CompilerMode diff: {...}");

// after: 0 件なら緑。再発したら従来と同じ dump で赤
if let Some(&first_bad) = bad_indices.first() {
    /* ...同じ dump を組み立てる... */
    panic!("V2-12 CompilerMode diff: {...}");
}
```

**ダンプは 1 bit も失われない。** 失敗メッセージとしてそのまま残り、
再発したときに従来と同じ情報が出る。名前は主題に合わせて改める
(`..._reports_..._body_diff` → 「violation が無いこと」を表す名前)。

#2 も同型で、`assert!(lines.len() >= 12, ...)` の後に
「`wasm-bytes-eq` が 1 で `mismatch-idx` が負であること」を assert し、
そうでないときに `panic!("mismatch probe: {:?}", lines)` を出す。
主題 (2 回コンパイルの決定性) は隣接する `TEST-CLI-02-M1F0I` が
同じことを別経路で言っており、**主題が実在する契約であることの裏付けになる。**

**必須の前提条件**: 反転する前に、**検出器が本当に検出できることを確かめる**。
確かめずに反転すると、`I-82` と同じ「常に緑で何も見ていない」test ができる。
RED の取り方は `I-79` 形 (b) と同じで、**入力を意図的に壊して赤になることを見る**
(`decisions-harness-swallowed-error-arms.md` の RED 表を見よ)。

### 裁定 2: 削除する (#3 / #4 / #5)

3 件は**特定の native crash 調査で得た生アドレスを直書き**している。

| test | 直書きされたアドレス |
|---|---|
| #3 | `0x6200d0` (pc) / `0x621700` (lr) |
| #4 | `0x1674bc` / `0x16765c` / `0x167684` / `0x1676ac` / `0x169ffc` / `0x16a190` |
| #5 | `0x106d24` |

codegen が 1 命令動けば意味を失う値であり、**契約ではない**。
台帳の注記も `# diagnostic:` と書いており、既に契約でないと認識されている。

**診断出力の引き取り先** (`ALWAYS-RED-PROBE-01` の受入条件):

`run_selfhost_main_representative_aarch64_offset_lookup_harness`
(`stage_chain.rs:26159`, 82 行) が offset → callable_idx → absolute_idx のマッピングを行い、
`representative_selfhost_registration_order()` (`:15518`) が absolute_idx → module::name を引く。
**能力はこの 2 つのヘルパにある。3 件の test はアドレスを差し替えて呼ぶだけの薄い殻である。**

- `representative_selfhost_registration_order` は `:26363` にも利用があるので**残る**
- `run_selfhost_main_representative_aarch64_offset_lookup_harness` は
  **この 3 件でしか使われていない** (5 hits = 定義 1 + 利用 4)。3 件を消すと dead code になる

82 行の L# harness を埋め込んだヘルパで、消すと再構築コストが高い。かといって
`#[allow(dead_code)]` で残すと、**何も呼ばないヘルパは動くかどうか誰も知らないまま腐る**。

したがって #5 を**削除ではなく実契約へ作り替える**。隣接する
`..._representative_layout_offsets` は `layout.entrypoint_offset > 0` を assert しているので、
**その entrypoint_offset を lookup harness に食わせると `App.Main` の main が返る**ことを assert する。
これは直書きアドレスに依存しない本物の契約であり、同時にヘルパを生かし続ける。
期待される module::name は実測で確定する。

**module 外の結合は無い (実測)。** `I-82` の #9 (`test_validate_stage2_wasm`) は
`selfhost_lsp_docs_ops.rs` の厳密名 gate リストと phase11 script に名指しされており、
削除すると別 module が赤くなる。同じ確認を #3 / #4 / #5 に対しても行った結果:

| 確認 | 結果 |
|---|---|
| `selfhost_lsp_docs_ops.rs` の `heavy_tests` (厳密名) | 3 件とも **0 hit** |
| `scripts/` 全体 | 3 件とも **0 hit** |
| prefix ルール `selfhost_native_stage_chain.rs:fn test_e2e_selfhost_main_representative_` | 同 prefix の test は **89 件**。3 件消しても `TESTGATE-01` の dead prefix にはならない |

したがって #3 / #4 の削除と #5 の作り替えは `selfhost_native_stage_chain` の再計測だけで覆える。
**確認したこと自体を記録するのは、次の slice で同じ grep を引き直さないためである。**

### 裁定 3: 台帳の扱い

5 件の台帳行は**実装が済むまで残す**。`compare_ignored_lane.py` が
台帳に無い FAIL を新規 FAIL として exit 1 にするため、先に消すと lane が赤くなる。
注記だけを `引き取り先: I-84` へ揃え、**「構造上消えない赤」であることを明記する。**
実装で緑になった行 (#1 / #2 / #5) は、そのときに削除する。

## 却下した案

### 案 A: 5 件とも削除する

却下。#1 / #2 は「無いことが正常」な本物の性質を probe しており、
極性を反転するだけで恒久的な regression guard になる。**捨てるには惜しい。**
とくに #1 の `local_bound_violations` は生成 Wasm の out-of-bounds local 参照を検出する
実装済みの検出器で、これを捨てると検出器ごと使われなくなる。

### 案 B: 5 件とも `#[ignore]` の理由に `diagnostic` と書いて許容する

却下。3 件は既にそう書かれている (`# diagnostic:`) が、**台帳から消えていない**。
注記は台帳の行を減らさない。「構造上消えない赤」を台帳に残す限り、
行数が進捗を表さないという問題はそのまま残る。

### 案 C: `I-81` の当初案 (b) — violation を含む fixture を与えて足場のまま保つ

却下。**恒久的に赤い test は情報を運ばない。** 常に赤なので、
本物の regression が起きても出力が変わるだけで、赤/緑という一番読まれる信号は動かない。
`I-81` が案 (a) に対して持っていた懸念「violation 再発に気付けなくなる」は、
**極性を反転すれば消える** — 0 件が緑、再発が赤になるので、むしろ気付きやすくなる。

### 案 D: ヘルパを `#[allow(dead_code)]` で残す

却下 (#5 を実契約へ作り替える案を採る)。**何も呼ばないヘルパは腐る。**
82 行の L# harness を埋め込んでおり、selfhost 側の API が動けば黙って壊れる。
`#[allow(dead_code)]` は「使われていない」という事実を隠すだけで、動くことを保証しない。

## Evidence

実装後に埋める。現時点で確定している実測。

| 対象 | 実測 | 取得条件 |
|---|---|---|
| 無条件 `panic!` で終わる `#[test]` | **5 件** (全て `#[ignore]`、全て台帳に在) | `python3 scripts/sweep_always_failing_tests.py`、2026-08-27 |
| 5 件の早期 return | **0 件** (body に `return` も `?;` も無い) | 同走査の brace matching で body を切り出して計数 |
| `run_selfhost_main_representative_aarch64_offset_lookup_harness` の利用 | **5 hits = 定義 1 + 利用 4**。利用は削除対象 3 件のみ | `grep -c` |
| `representative_selfhost_registration_order` の利用 | **5 hits = 定義 1 + 利用 4**。うち `:26363` は削除対象外 | `grep -n` |

### #1 の実装 (2026-08-27)

裁定どおり極性を反転した。末尾の無条件 `panic!` を
`let Some(&first_bad) = bad_indices.first() else { return; };` で包み、violation 0 件を緑にした。
ダンプは失敗メッセージとして残してある。test 名は主題に合わせて
`test_v2_12_self_hosted_stage2_compiler_mode_has_no_local_bound_violation` へ改めた。

**反転する前に検出力を証明した。** 極性を反転しただけでは
「violation が 0 件なのか、検出器が動いていないのか」を区別できない —
`I-82` が扱った「常に緑で何も見ていない」test になる。そこで非 ignore の
`test_local_bound_violation_indices_detects_out_of_range_local` (`part_017.rs`) を新設し、
範囲外の local を仕込んだ入力に対して `local_bound_violation_indices` が実際に検出することを固定した。
**この test が緑であることが、#1 の緑に意味を与えている。**

| 検証 | 結果 |
|---|---|
| `..._has_no_local_bound_violation` の個別実行 | exit 0 (2026-08-27) |
| `test_local_bound_violation_indices_detects_out_of_range_local` (非 ignore) | exit 0 (2026-08-27) |
| `ignored-lane-expected-failures.txt` の該当行 | **削除済み** |

### #2 の実装 (2026-08-27)

#### 前提条件だった実測を先に取った

本 ADR の「満たせなかったこと」は **「#2 の『現在 mismatch は無い』は未実測である」** と書き、
**極性を反転する前に実測すること**を条件にしていた。実測した。

```
mismatch probe: ["7218", "7218", "1", "-1", "-1", "-1", "-1", "-1", "-1", "-1", "-1", "-1"]
```

`selfhost_cli_core.rs:3022`、2026-08-27、`e2e --ignored --nocapture --test-threads 1` の
3 filter 同時走行。読み方は次のとおり。

| 位置 | 値 | 意味 |
|---|---|---|
| 0 / 1 | `7218` / `7218` | 2 回の inline compile が出した Wasm の長さ |
| 2 | `1` | `wasm-bytes-eq` — **バイト単位で一致** |
| 3 | `-1` | `first-function-mismatch` — **食い違う関数は 1 つも無い** |
| 4〜11 | 全て `-1` | mismatch が無いので下流の診断値は全て未定義 |

**推測どおり「mismatch 無し」だった。** ただし推測が当たったことは実測を省ける理由にならない。
`I-81` は「実物を読まずに症状から推測した記述」を後で撤回している (本 ADR の末尾)。

#### 反転と、検出器の自己検査

裁定 1 の文言どおり、末尾の無条件 `panic!` を条件付きに変えた。

```rust
if lines[0] != lines[1] || lines[2] != "1" || lines[3] != "-1" {
    panic!("mismatch probe: {:?}", lines);
}
```

**dump は 1 bit も変わらない。** 再発したときに従来と同じ 12 値がそのまま出る。
test 名は `..._full_inline_mismatch_probe` → `..._full_inline_compile_has_no_mismatch` へ改めた。
`#[ignore = "temporary diagnostic harness for local mismatch inspection"]` の理由文字列も
実態と合わなくなったので落とした。

**検出力の証明は #1 と違う置き方をした。** #1 は別 test (`..._detects_out_of_range_local`) を
新設したが、#2 の検出器は harness 内の L# 関数なので Rust 側から直接叩けない。
そこで **同じ実行の中で検出器へ既知の不一致を食わせ、その結果も出力させる**。

| 追加した出力 | 入力 | 期待 |
|---|---|---|
| 12 | `first-function-mismatch` に `functions1[0]` と `functions1[1]` の 1 要素列 | `0` (検出する) |
| 13 | 同じ検出器に `functions1[0]` を 2 つ | `-1` (一致と判定する) |
| 14 | `wasm-bytes-eq` に `[1,2]` と `[1,3]` | `0` (**同じ長さで**内容の違いを検出する) |
| 15 | 同じ検出器に `[1,2]` を 2 つ | `1` (一致と判定する) |

14 を**同じ長さ**にしたのは意図的である。`wasm-bytes-eq` は長さが違えば
比較ループへ入らず `0` を返すので (`selfhost_cli_core.rs:230-233`)、長さ違いの入力では
**バイト比較ループが動くことを何も示せない。**

この 4 つは主題 assertion の**手前**に置いた。検出器が死んでいるなら、
主題の緑には意味が無い。どちらが壊れたのかが失敗メッセージで分かる順序にした。

#### 検証

| 対象 | 結果 |
|---|---|
| `..._full_inline_compile_has_no_mismatch` の個別実行 | **ok** (215.76s、2026-08-27、`--exact` 完全修飾名) |
| 反転前の同 test | `FAILED` (無条件 `panic!`)。dump は上の表 |
| `cargo clippy -p lsharp-wasm --tests` の `selfhost_cli_core.rs` 分 | 警告 **0** |
| `ignored-lane-expected-failures.txt` の該当行 | **削除済み** (`9b4633a4`)。`selfhost_cli_core` の lane 完走で確認 (2026-08-28) |

**`--exact` は完全修飾名を要求する。** 短縮名で撃つと `running 0 tests` になり、
`RUNEXIT=0` で終わる。**「0 件走って exit 0」を緑と読まないこと。**
本 slice で 1 度これを踏んだ (`ELAPSED=2.21` が異常の指標になった)。

### #3 / #4 / #5 の実装 (2026-08-27)

| # | 位置 | 結果 |
|---|---|---|
| #3 | `stage_chain.rs:26574` | **削除済み** |
| #4 | `stage_chain.rs:26623` | **削除済み** |
| #5 | `stage_chain.rs:26660` | **作り替え済み**。`..._entrypoint_offset_resolves_to_app_main_main` として個別実行 **ok** |

#5 は `run_selfhost_main_representative_aarch64_offset_lookup_harness` (82 行) を生かすための
作り替えであり、`#[allow(dead_code)]` では逃げていない。裁定 2 の「削除する (#3 / #4 / #5)」に
対しては、**#5 だけ削除ではなく作り替えになった** — `TODO.md` の
`ALWAYS-RED-PROBE-01` が既にそう書き換えていたので、本 ADR の裁定 2 の見出しが古い。
見出しは残し、ここに差分を書く。**裁定を静かに書き換えない。**

## 満たせなかったこと

- **5 件とも実装が入り (2026-08-27)、lane 再計測 3 本で確認した (2026-08-28)。**
  `selfhost_native_stage_chain` 613 宣言 / 613 結果行 / FAIL 111 (= 台帳 111 行)、
  `selfhost_cli_core` 381 / 381 / FAIL 21 (= 台帳 21 行)、
  `selfhost_bootstrap_four_layer` 144 / 144 / FAIL 1。3 本とも
  `新規 FAIL 0 / 解消 0 / 未出現 0` で `I-84` は resolved にした。
  台帳行 4 本は `9b4633a4` で削除済みで、comparer が `未出現 0` を返したことにより
  **過不足なく落ちていた**ことが確認できた。lane 実測は [ignored-lane-sweep-2026-08-23.md](../development/operations/ignored-lane-sweep-2026-08-23.md) の `結果 (2 回目 -- 3 module とも完走)`。
- **#1 の lane 再計測も同じ 3 本で解消した (当初は未了)。** 改名したので
  `AGENTS.md` の規約 (`d29cb5a1`) により module 再計測が要り、個別実行の緑では代用できなかった
  (`compare_ignored_lane.py` の完走判定は module 単位のログを要求する)
- **#2 の未実測は解消した (2026-08-27)。** 反転の前に測り、`wasm-bytes-eq=1` /
  `first-function-mismatch=-1` を確認してから反転した。上の「#2 の実装」節が正本
- **#5 の作り替えで期待される `module::name` も実測で確定した** (個別実行 ok)
- **`I-81` の当初の記述が間違っていた。** 「改善した結果として足場が成立しなくなった」は、
  実物を読まずに症状 (`.expect` で落ちる位置) から推測した記述である。
  **落ちる位置が変わったことを、成立していた足場が壊れたことと読んだ。**
  分岐の有無を確かめれば 1 分で分かることだった

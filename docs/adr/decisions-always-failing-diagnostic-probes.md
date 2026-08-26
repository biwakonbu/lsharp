# 構造上必ず赤くなる診断 probe の裁定

- **Status**: doc-RED (裁定は確定、実装は未着手)
- **Date**: 2026-08-27
- **Scope**: `crates/lsharp-wasm/tests/e2e/` の 5 test と、それが使う lookup harness
- **Related**: `I-84` (本 ADR の起点) / `I-81` (発見経路。5 件のうち 1 件) /
  `I-75` (誤分類していた 1 件) / `I-82` と
  [`decisions-probe-subject-unchecked.md`](decisions-probe-subject-unchecked.md) (裏返しの類型)
- **引き取り先**: `TODO.md` の `ALWAYS-RED-PROBE-01` / `VIOLATION-PROBE-STALE-01`

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

## 満たせなかったこと

- **裁定だけで、実装は 1 件も入っていない。** 緑にした test は 0 件で、`I-84` は open のまま
- **#2 の「現在 mismatch は無い」は未実測である。** `assert!(lines.len() >= 12)` の手前で
  落ちていない事実からそう推測しているだけで、`wasm-bytes-eq` の実際の値を見ていない。
  **極性を反転する前に実測すること** — これは本 ADR 自身が `I-82` について書いた
  「実物を確かめずに期待値を固定するな」に当たる
- **#5 の作り替えで期待される `module::name` は決まっていない。** 実測が要る
- **`I-81` の当初の記述が間違っていた。** 「改善した結果として足場が成立しなくなった」は、
  実物を読まずに症状 (`.expect` で落ちる位置) から推測した記述である。
  **落ちる位置が変わったことを、成立していた足場が壊れたことと読んだ。**
  分岐の有無を確かめれば 1 分で分かることだった

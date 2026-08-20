# ADR: 滞留 worktree の取り込み可否判定 (2026-08-20)

- Status: Accepted
- Date: 2026-08-20
- Scope: 滞留 worktree 25 本の棚卸しと、そこから main へ取り込む範囲の確定
- Related: [`ISSUES.md` I-30](../../ISSUES.md#i-30) / [decisions-legacy-test-gc-soak-telemetry.md](decisions-legacy-test-gc-soak-telemetry.md)

## Context

`/Users/biwakonbu/github/tmp/` 配下に worktree が 25 本滞留し、所有者と取り込み状態が
判別できなくなっていた。合計 86 GB の `target/` を抱え、`git cherry` は多数の commit を
`+` (未取り込み) と報告していた。

`git cherry` の `+` は patch-id の不一致でしかなく、**未取り込みの証拠にはならない**。
main 側が同じ意図を別実装で達成していれば patch-id は当然ずれる。実際、`2d4a8165` は
`+` でありながら test も実装も main により先へ進んでいた。したがって判定は
「patch が当たるか」ではなく「**その commit が守ろうとした契約を main が持っているか**」で行う。

## Decision

### 取り込んだもの

| 由来 | 取り込み方 | 根拠 |
|---|---|---|
| `1c0e0584` + `5f162a70` (gc-soak telemetry lane) | cherry-pick (競合なし) | `test-gc-soak-telemetry-contract.sh` が exit 0。参照する test 名 5 件は main に実在 |
| `8afb7c2a` / `e9f94428` (metadata inventory test 2 件) | cherry-pick + 追従 fix | main の `MetadataFormKind` が 17 variant 増えていたため catch-all arm を追加 |
| `0dbc5d11` (nested module の contract owner 修飾) | **hand-merge** | 丸ごと取ると main の `is_contract_form` filtering が消える |
| `84ca54fd` の payload test 2 件 | **test だけ現行 API へ書き直し** | 下記 |
| `lsharp-typeinfer-record-next` の未 commit 実装 | Type.ls の 3 family だけ移植 | 下記 |

### 却下したもの

- **`2d4a8165` (batch family)** — superseded。test も実装も main が先行している。
  取り込むと main の実装を後退させる。
- **`84ca54fd` のパッチ本体** — superseded-by-divergence。当該 commit は
  `generate_tests` を `ContractSuite` inventory 経由へ差し替える設計だが、main の
  `generate_tests` は Case / Assertion / Property を含む **5 種**へ拡張済みで、
  対象だった 2 種 (Invariant / Example) の routing へ戻すことはできない。
  fail-closed 自体は main が別経路で持つ — `metadata_test.rs:74` の
  `run_metadata_tests` が `inventory_contract_suites` を併走させ、
  `ProjectionMismatch` をそのまま `Err` にしている。
  **残すべきは実装ではなく契約**なので、payload test 2 件を現行 API の上へ
  書き直して `crates/lsharp-types/tests/metadata_contract_generation.rs` に置いた。
  main で既に成立しているため RED を経ない characterization test である。
- **`986ac1e3`** — 二つに割れるが両方 superseded。
  `selfhost_cli_core.rs` の `run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, ...)` 包みは
  main が既に全面採用済み (同ファイルに 10 箇所以上)。selfhost 側の converter 統一
  (`contract-forms-to-test-cases`) は、main が `extract-parser-contract-suites` という
  より広い canonical inventory を 5019 行の `TestRunner.ls` に持っており、
  7 ヶ月前のパッチを当て直す形にはならない。
  ただし **当該 commit が予告した drift は現実になった** — `I-30` として起票する。

### batch branch `3b5dbef5` の 5 件 (2026-08-20 追記)

`codex/lsharp-typeinfer-property-aggregation-batch` (`3b5dbef5`) は main と
`1199 files / +95243 / -201160` 離れており、**丸ごとの merge は不可能**。
merge-base `cb9e8d09` に対する 3 方向比較で defn 単位に判定した。

| 対象 | 判定 | 根拠 |
|---|---|---|
| `TypeScheme.ls` + `selfhost_typescheme_loops.rs` | **取り込み (whole-file)** | main が fork 以降 1 度も触っておらず `main == merge-base` が byte 一致。branch は純粋な子孫 |
| `TypeInferRecordDecl.ls` + `selfhost_typeinfer_record_registration.rs` | **取り込み (whole-file)** | 下記 |
| `selfhost_parser_recovery.rs` | **取り込み (回帰 pin)** | main が既に全 assertion を満たす。anchor も main の綴りと一致 |
| `selfhost_parser_collection_helpers.rs` | **却下 + 配線 assertion だけ読み替えて取り込み** | 下記 |
| `TypeInferSignature.ls` | **却下** | 下記 |

#### `TypeInferRecordDecl.ls` を splice ではなく whole-file にした理由

当初は per-defn splice を想定した。main が fork 以降に `f8780d62`
(`feat(selfhost): bound block and record export scans`) で前進しており、
`TypeInferRecordDecl.ls` は `main != merge-base` だったからである。

しかし f8780d62 が入れた 12 defn
(`typeinfer-record-only-contains-*` / `typeinfer-record-remove-unallowed-accessors-*`) は
**branch 側と body が byte 一致**していた。共通 defn 29 件の内訳も
「branch だけが変更」24 / 「main==branch」5 で、**「main だけが変更」と「両方変更」がいずれも 0**。
つまり branch は main の上位集合である。

唯一 branch に無い `typeinfer-predeclare-record-env-with-schema` は
**main の追加ではなく merge-base 由来**で、branch はこれを
`typeinfer-predeclare-record-env-rooted-v3` へ吸収して削除している。参照元は同一ファイル内
1 箇所だけ (`selfhost/` `crates/` 全体を grep して確認) なので、保全すべき main 固有の資産は無い。
**splice すると branch が意図的に消した defn を復活させることになる**ため、whole-file を採る。

#### `selfhost_parser_collection_helpers.rs` を却下した理由

main には既に `selfhost_parser_collection_scanners.rs` (3 test) があり、同じ 2 family
(`vector-set-at` / `defn-signature-param-present`) の bounded 化と 64 境界跨ぎの挙動を
押さえている。branch 版の runtime test は完全に重複する。

ただし branch 版だけが持つ価値が 1 つある — **委譲の配線**を見ている点である。
main 側は名前の存在しか見ておらず、入口が chunk へ委譲しているかは pin していない。
そこで配線 assertion だけを main の綴りへ読み替え、
`test_e2e_selfhost_parser_collection_scanners_delegate_to_bounded_chunks` として
既存ファイルへ足した。読み替えの中身:

| branch の綴り | main の綴り |
|---|---|
| 入口 `(defn vector-set-at-loop` | 入口 `(defn vector-set-at` |
| 入口 → `vector-set-at-step-64` を直接呼ぶ | 入口 → `vector-set-at-rooted-continuation-v3` → `-step-64-loop-bounded` (1 段多い) |

契約 (要素ごとの自己再帰を持たず 64 要素 chunk へ委譲する) は同じで、綴りだけが分岐している。
`ab5f1a01` と同じ読み替えである。

#### `TypeInferSignature.ls` を却下した理由

305 行の branch 専用ファイルだが、実体は **bounded rooted scan ではなく `TypeInfer.ls` の
モジュール分割**である (branch 側 `TypeInfer.ls` の diff は +3013)。main には参照が
1 つも無い (`grep -rn "TypeInferSignature" selfhost/ crates/` が 0 hit)。

取り込むには `TypeInfer.ls` 本体、`crates/lsharp-wasm/tests/common/mod.rs`、
`support.rs` (10 箇所以上) を branch 側へ揃える必要があり、これは 963 commit 分の
分岐を跨ぐ広域リファクタになる。**このファイルが守る契約は 0 件** (専用 test が無い) で、
価値は整理にある。よって取り込まず、分割の意図だけを `TODO.md` へ登録する。

### bounded rooted scan の全域 sweep 結果 (2026-08-20 追記)

上記 5 件は「batch branch が触った test から辿れた範囲」でしかない。同じ branch が
`selfhost/src/**.ls` 全域に入れた bounded rooted scan (`<name>-step-64-loop-bounded` 族) を
merge-base `cb9e8d09` に対する 3 方向比較で洗い直した結果、**未取り込みは 9 ファイル・約 69 family**
あることが分かった。取り込み可否は「main と branch が同じ defn を両方書き換えたか」で決まる。

| ファイル | `main == merge-base` | 同一 / branch のみ / main のみ / 両方 | branch のみの family | 判定 |
|---|---|---|---|---|
| `Types/Type.ls` | -- | (既取り込み) | 3 | 完了 |
| `Types/TypeScheme.ls` | **True** | 0 / 29 / 0 / 0 | 8 | **取り込み (whole-file)** |
| `Types/TypeInferRecordDecl.ls` | False | 5 / 24 / 0 / 0 | 9 | **取り込み (whole-file)** |
| `Types/TypeInferFunctions.ls` | **True** | 7 / 9 / 0 / 0 | 5 | 却下 (下記) |
| `Types/TypeInferCore.ls` | False | 114 / 5 / 1 / 0 | 5 | 却下 (下記) |
| `Types/TypeInfer.ls` | False | 69 / 16 / 5 / **4** | 17 | 却下 (両方変更) |
| `Types/TypeInferPattern.ls` | False | 4 / 0 / 0 / **13** | 2 | 却下 (両方変更) |
| `Types/TypeInferAdt.ls` | False | 4 / 2 / 4 / **4** | 0 | 対象外 (main が独自に前進) |
| `Syntax/Parser.ls` | False | 178 / 26 / 15 / **27** | 20 | 却下 (両方変更) |

#### swap 前に通した 2 つの gate

whole-file take は「入れる差分」だけでなく **「消える defn」** を見ないと安全でない。
`TypeInferRecordDecl.ls` で `typeinfer-predeclare-record-env-with-schema` を取り違えかけた経験から、
取り込み候補 4 ファイルに対して機械的に 2 つ確認した。

1. **import ヘッダ差分** — branch は `Types.TypeInferSignature` が在る別の module topology を持つ。
   incoming ファイルが 1 行でもそれを import していると main の木が壊れる。
   → 4 ファイルとも `(module ...)` / `(import ...)` 行は **main と byte 一致**。通過。
2. **branch が消した main 側 defn の外部参照** — `selfhost/` と `crates/` を語境界付きで grep する。

| ファイル | 消える defn | 外部参照 | 結果 |
|---|---|---|---|
| `TypeScheme.ls` | 0 件 | -- | 通過 |
| `TypeInferRecordDecl.ls` | 1 件 (`typeinfer-predeclare-record-env-with-schema`) | **0 件** | 通過 |
| `TypeInferFunctions.ls` | 9 件 | **4 箇所** (`TypeInfer.ls:371,396,464,474`) | **不通過** |
| `TypeInferCore.ls` | 1 件 (`error-code-recursive-alias`) | **9 箇所** (`TypeInfer.ls` / `App/Cli.ls`) | **不通過** |

#### `TypeInferFunctions.ls` を却下した理由

`main == merge-base` が byte 一致で、`TypeScheme.ls` と同じ「純粋な子孫」の形をしている。
にもかかわらず却下したのは、branch が消した 9 defn のうち 3 つ
(`typeinfer-defn-param-annotation-subst` / `typeinfer-defn-return-annotation-subst` /
`typeinfer-defn-type-param-env`) の移動先が **`Types/TypeInferSignature.ls`** だからである。
このファイルは同じ ADR で既に却下しており (`TypeInfer.ls` の分割を伴うため)、
`TypeInferFunctions.ls` だけを swap すると main の `TypeInfer.ls` の 4 箇所が未定義参照になる。

**`TypeInferSignature.ls` の取り込み判断と不可分**であり、単独では入れられない。
`TYPEINFER-SPLIT-01` の従属項目として扱う。

#### `TypeInferCore.ls` を却下した理由

両方変更が 0 で、差分としては最も素直に見える。しかし branch は `error-code-recursive-alias` を
**branch 全域のどこにも再定義せず削除**しており (branch の `selfhost/src/**.ls` を走査して定義 0 件)、
main はこれを 9 箇所から呼んでいる。branch は recursive alias の診断を別経路へ寄せたと見られるが、
その経路は main に無い。

`error-code-general` 1 件だけを main から残す splice も考えたが、**splice で救えるのは
「main だけが変更した defn」であって「branch が体系ごと差し替えた診断コード」ではない。**
片方だけ持ち込むと診断コードの割り当てが main と branch の混成になる。取り込むなら
diagnostics 側の対応関係を先に決める必要があり、bounded scan の取り込みとは別の判断である。

#### 却下 3 ファイル (両方変更) について

`TypeInfer.ls` / `TypeInferPattern.ls` / `Parser.ls` は、fork 以降に **main と branch が同じ defn を
それぞれ別の方向へ書き換えている**。特に `TypeInferPattern.ls` は共有 17 defn 中 13 が両方変更で、
main 841 行 / branch 786 行と main の方が長い。`Parser.ls` も両方変更 27・main のみ変更 15 で、
main が独自に進めた分の方が大きい。

ここを機械的に merge することはできない。defn 単位で意味を突き合わせる作業が要り、
それは bounded scan の「取り込み」ではなく **移植** である。本 slice の範囲を超えるので、
family 名を全部列挙したうえで `BOUNDED-SCAN-01` として `TODO.md` に登録し、branch ref は消さない。

`TypeInferAdt.ls` だけは branch のみの family が 0 で、main が独自に 291 → 800 行へ変換を
進めている (branch は 629 行)。**取り込むものが無い** ので登録もしない。

### snapshot を実態へ追従させた判断

`metadata_runner_semantics_inventory__rust_runner_metadata_semantics.snap` の
`runner_error_code` を `E0002` → `LS1002`、`runner_error_uses_public_ls_code` を
`false` → `true` に書き換えた。これは snapshot 自身が
`inventory_status: current_behavior_not_final_v0_2_contract` を宣言しており、
**契約ではなく現在の挙動の記録**だからである。契約 snapshot なら実態側を直す。

### test 期待値を実装へ合わせた 1 件

`crates/lsharp-types/tests/metadata_contract.rs:104` の owner 期待値を
`"succ"` → `"Math.succ"` に変更した。通常は禁止 (テストの期待値を実装に合わせない) だが、
以下の理由で追認ではなく予告どおりの置き換えと判定した:

- main 側 `metadata_contract.rs:20-21` の doc comment 自身が、未修飾 owner を
  「後続 inventory slice で追加する」までの**暫定挙動**と明記していた
- `test_generation.rs:131` は生成 test 名を decl 名から組むため、生成される test 名は
  変わらない。変わるのは診断表示上の owner だけである

### `cargo insta accept` はしない

`.snap.new` 14 件は 2 ヶ月分の未レビューな codegen 出力である。一括追認せず、
`/Users/biwakonbu/github/tmp/worktree-salvage-2026-08-20/untracked/lsharp-baseline-a3ae4551/`
へ salvage して据え置く。

## Evidence

取り込み先 worktree: `/Users/biwakonbu/github/tmp/lsharp-absorb-2026-08-20`
(branch `codex/worktree-absorb-2026-08-20`、main `f4a3bb13` から分岐)。

| 検証 | 結果 |
|---|---|
| `bash scripts/ci/test-gc-soak-telemetry-contract.sh` | exit 0 (`GC soak telemetry lane contract passed`) |
| `cargo test --workspace --exclude lsharp-wasm --no-fail-fast` | FAIL 15 件。**全件が `docs/development/validation/workspace-expected-failures.txt` に登録済み**。新規回帰 0 |
| `cargo test -p lsharp-wasm --lib` | 137 passed / 0 failed |
| `cargo test -p lsharp-wasm --test e2e selfhost_type_record` | 4 passed / 0 failed |
| `cargo test -p lsharp-types --test metadata_contract_generation` | 2 passed / 0 failed |

`lsharp-typeinfer-record-next` の RED→GREEN:

- RED: `test_e2e_selfhost_type_record_checks_use_bounded_chunks` が
  「Type.ls record substitution/check/unification should use bounded rooted helpers」で FAIL
- GREEN: 上記 `selfhost_type_record` 4 件 PASS

**満たせなかった / 明示しておく事実**:

- 65 field を跨ぐ挙動 test `test_e2e_selfhost_large_record_checks_preserve_results` は
  **取り込み前の main でも PASS していた**。つまり今回の Type.ls 変更は挙動を変えておらず、
  chunk 境界と rooting の構造を既存 2 family (`type-record-field-type-*` /
  `type-record-fields-eq-*`) へ揃えるものである。「65 field で壊れていたのを直した」ではない
- `lsharp-typeinfer-record-next` の `TypeInferRecord.ls` は main と **byte 一致**まで
  取り込み済みだった。未 landing だったのは `Type.ls` の 3 family
  (`apply-subst-record-fields` / `occurs-check-record-fields` / `unify-record-fields`) だけである
- `84ca54fd` 由来の 2 件は main で既に PASS するため RED を経ていない。回帰 pin である
- e2e 全面 (`selfhost_typeinfer` 以降を含む) の完走は本 slice の範囲外。
  実行時間が 5 時間規模のため、影響範囲 (`selfhost_type_record` / `lsharp-wasm --lib`) に
  絞って検証した


### batch branch `3b5dbef5` の RED→GREEN (2026-08-20)

swap 前の木の健全性を先に固定した (`cargo test -p lsharp-wasm --test e2e selfhost_typeinfer`、
切り離し実行 1936.17s): **191 passed / 0 failed / 7 ignored**。

RED (swap 前、対象 4 test file / 11 test):

| test | RED の内容 |
|---|---|
| `selfhost_typescheme_loops::..._traversals_use_bounded_rooted_chunks` | `TypeScheme traversals should use bounded rooted helpers` |
| `selfhost_typeinfer_record_registration::..._registration_uses_bounded_chunks` | `record schema/accessor registration scan は bounded helper と rooted continuation へ分離するべき` |
| `selfhost_typeinfer_record_registration::..._large_record_registration_preserves_results` | `Mismatch { expected: Con("Vector"), found: Con("Map"), error_code: ArgMismatch }` |

`8 passed / 3 failed`。

GREEN (`TypeScheme.ls` / `TypeInferRecordDecl.ls` を `3b5dbef5` から byte 一致で差し替え後):
**11 passed / 0 failed** (55.74s)。

**明示しておく事実**:

- 3 件目の RED は静的 assertion ではなく **65 record を跨いだ実行時の型不一致**である。
  main 側の record registration scan は chunk 境界を越えると Vector を Map と取り違えていた。
  今回の取り込みは構造を揃えるだけでなく、**実挙動の不具合を 1 件解消している**
- 対して `selfhost_typescheme_loops::..._large_traversals_preserve_results` と
  `selfhost_parser_recovery` / `selfhost_parser_collection_scanners` の計 8 件は
  **swap 前の main で既に PASS していた**。これらは RED を経ていない回帰 pin である
- `TypeInferFunctions.ls` / `TypeInferCore.ls` は gate 不通過のため取り込んでいない。
  `BOUNDED-SCAN-01` に family 名ごと登録した


### swap 後の広域回帰 (2026-08-20)

focused test 4 file の GREEN だけでは swap の影響範囲を覆えないので、`selfhost_type` filter で
234 test を切り離し実行した (pid 9283 / PPID=1 / `CARGO_TARGET_DIR=/Users/biwakonbu/github/tmp/absorb-target`)。

```
test result: FAILED. 226 passed; 1 failed; 7 ignored; 0 measured; 2837 filtered out; finished in 2238.49s
FAILED: e2e::selfhost_type_parser_parity::test_e2e_selfhost_parser_nested_module_decl
```

**合格条件は「0 failed」ではなく「FAILED 集合 ⊆ 台帳」である。**
`test_e2e_selfhost_parser_nested_module_decl` は
`docs/development/validation/workspace-expected-failures.txt` の `:103` と `:150` に登録済みで、
本体 (`selfhost_type_parser_parity.rs:495`) は `(module App (module Sub (defn inner [] 42)))` の
parse を見る test であり、swap した 2 file のどちらにも触れていない。よって swap 由来の回帰ではなく、
新規 FAIL は **0 件**である。

待ち役の background job は途中でハーネスに 2 度停止させられたが、計測本体は `setsid` 切り離し
(PPID=1) なので巻き添えにならず完走した。長時間計測を切り離す運用の有効性がここでも確認できた。

## worktree / branch の片付け (2026-08-20)

「取り込むべきものが残っているか」を **checkout の有無ではなく patch-id** で判定した。
`git cherry main <branch>` が `+` を 1 つも出さない branch は、commit hash が違っても
中身が main に入っている。branch 名や ahead 数だけを見ると、この 46 件を見落とす。

| 分類 | 件数 | 処置 |
|---|---|---|
| main の祖先 (clean) | 17 | checkout 削除 |
| batch tip `3b5dbef5` に包含される | 22 | checkout 削除 (`BOUNDED-SCAN-01` が正本) |
| **patch-id が main と一致 = 取り込み済み** | **46** | checkout 削除 |
| 真に未取り込みの commit を持つ | 29 | **checkout も ref も残す** |
| 実内容のある未 commit 変更あり | 7 | **手を触れない** |

worktree は **110 本 → 37 本**。**branch ref は 1 つも消していない**ので、
`git worktree add <path> <branch>` でいつでも復元できる。ディスクは 335Gi → 338Gi。

### batch family は tip 1 本に畳める

`codex/lsharp-typescheme-batch` / `-typeinfer-apply-batch` / `-type-record-check-batch` は
tip と別系列に見えるが、`git cherry <tip> <sibling>` が 0 件を返す。つまり **tip がこの 3 本の
work を patch-id で完全に含む**。`-type-record-ops-batch` だけ tip に無い commit が 1 件ある。
したがって batch family 26 本の取り込み判断は `BOUNDED-SCAN-01` 1 件に集約してよい。

### 手を触れなかった 7 本

未 commit の実内容 (新規 script / `.ls` 編集) が載っている。`.DS_Store` だけの dirty
13 本とは区別した。

`codex/release-input-bundle` (5) / `codex/v0.2-ec-m1-01` (4) /
`codex/v2-16c-native-selfhost-{doc,install,repl}` (各 2) /
`codex/lsharp-next-type-expression` (5) / `codex/lsharp-typeinfer-record-next` (6)

このうち `codex/lsharp-typeinfer-record-next` の未 commit 分は本 ADR の
「取り込んだもの」で既に main へ landing 済みだが、salvage の突き合わせが済むまで残す。

### merge 前に閉じた 2 つの穴 (2026-08-20)

swap は削除された defn だけでなく、**共有 defn 29 件の body も全面的に書き換えている**。
Gate 1 / Gate 2 はどちらも defn 名と import 行しか見ないので、
**body 本文へ文字列 assertion を張っている test** は検出できない。回帰 run の
filter (`selfhost_type`) の外にそういう test がいると、merge まで気付けない。

`grep -ln 'TypeScheme\|TypeInferRecordDecl' crates/lsharp-wasm/tests/e2e/*.rs` で 14 file を洗い、
filter 外の 8 file を個別に見た。source 本文へ assertion を張っているのは
`selfhost_bootstrap_contracts.rs` の `test_e2e_selfhost_type_responsibility_separation`
(TEST-TYPE-01, `:490`) だけで、述語は 6 つ。swap 後の実測:

| 述語 | 期待 | 実測 |
|---|---|---|
| `(defn mono` を含む | >0 | 1 |
| `(defn poly` を含む | >0 | 2 |
| `(defn generalize` を含む | >0 | 6 |
| `(defn instantiate` を含む | >0 | 17 |
| `(defn free-vars` を含む | >0 | 22 |
| `(defn infer-` を**含まない** | 0 | 0 |

6 つとも満たしている。残る 7 file は module 名の列挙か import 行の assertion で、
`git diff main -- <2 file> | grep -E '^[+-]\(import|^[+-]\(module'` が空
(= import ヘッダは 1 行も動いていない) なので判定は swap 前と同一である。

もう 1 つは台帳の穴。`codex/lsharp-type-record-ops-batch` は batch tip `3b5dbef5` に
含まれない commit を 1 つ持つが (`a5bb397a docs: record Linux evidence for Type record
operations`)、`BOUNDED-SCAN-01` は bounded family だけを、`WORKTREE-ABSORB-02` は
batch family を除外対象としていたため、**この 1 commit だけがどちらの正本にも載らない**
状態になっていた。docs-only 2 file なので `WORKTREE-ABSORB-02` の一覧へ明示的に足し、
同項目の対象を 25 本 → 26 本に改めた。

## ディスク

`target/` 42 本を削除し **86 GB** を回収した (`621Gi used / 251Gi avail` →
`535Gi used / 337Gi avail`)。main repo 自身の `target` と、取り込み作業で使用中の
共有 build dir は除外している。commit / branch / 未 commit の編集はいずれも失っていない
(編集は事前に patch として salvage 済み)。

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

#### 突き合わせの結果 (2026-08-22): salvage すべき内容は 0 件

7 本すべての未 commit 内容を **main の該当ファイルと 1 つずつ突き合わせた**。
結果、**取り込むべきものは 1 つも無かった。** 内訳:

| worktree | 未 commit の中身 | 判定 | 根拠 |
|---|---|---|---|
| `codex/lsharp-typeinfer-record-next` | 998+/114- (`Type.ls` / `TypeInferRecord.ls` / e2e 3 file) | 取り込み済み | 6 file 中 5 file が main と **byte 一致**。`e2e/mod.rs` は main の `mod` 宣言集合の真部分集合 |
| `codex/lsharp-next-type-expression` | 885+/885- (`Cli.ls` / `EmbeddedCli.ls` / `TypeInfer*.ls` 5 file) | 内容ではない | `git diff -w` が**空**。全量が selfhost formatter の再インデント出力で、`let` 継続行の字下げしか変わっていない |
| `codex/v0.2-ec-m1-01` | 113+/11- (invariant scope への引数 bind) | 取り込み済み | `.ls` は `TestRunner.ls:4831` に同じ `bind-params-loop` 行。Rust 生成器は main が **nested `let` の fold** という別解を採っており (`test_runner.rs:90-102`)、branch の flat `let` が依存する逐次束縛を前提にしない分だけ強い。test 3 件も main にある (下記) |
| `codex/release-input-bundle` | 14+/7- (`tar -czf` → producer script 呼び出し) | 取り込み済み | main は呼び出し側 2 本とも変換済み (`native-macos-aarch64-selfhost-release.sh:134` / `native-linux-x86-hostgen-vm-exec.sh:2507`)、contract test も `scripts/ci/test-native-release-input-bundle.py` にある。main 版の producer は未使用の `import os` を落としている |
| `codex/v2-16c-native-selfhost-doc` | 未追跡 `.py` 2 件 | 取り込み済み | 2 件とも main と byte 一致 |
| `codex/v2-16c-native-selfhost-repl` | 未追跡 `.py` 2 件 | 取り込み済み | 差分は docstring 1 行のみで、main 側が日本語へ訳した後 |
| `codex/v2-16c-native-selfhost-install` | 未追跡 `.py` 2 件 | main が先行 | main は 707→957 行 / 351→883 行。worktree 固有に見える 53 行は `install_path_dependency` の**旧形**で、main は staging + rollback + 非 symlink 拒否へ作り替えてある (`89de3680` / `d4250465` / `4c61d68b`) |

`codex/v0.2-ec-m1-01` の test 3 件の所在: `test_invariant_execution_binds_parameter_scope`
(`crates/lsharp-wasm/src/test_runner.rs:486`)、
`test_e2e_selfhost_test_runner_binds_invariant_parameters`
(`crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs:12375`)、
`test_run_metadata_tests_allows_local_let_binding_in_invariant`
(`crates/lsharp-tooling/src/metadata_test_tests/basic.rs:160`、fixture が
`:invariant (let [delta 1] (= result (+ x delta)))` で引数 `x` を参照する)。
運用記録は `rust-boundary-reduction.md` の
`### EC-M1-01 invariant parameter scope parity (2026-07-17)`。

**唯一 main に無いもの**は `codex/release-input-bundle` が `.github/workflows/ci.yml` へ足す
`test-native-release-input-bundle.py` の実行 step 1 つだが、これは **CI 設定なのでスコープ外**。
producer / contract test の実体はどちらも main にあるので、CI 方針を決めるときに
`SMOKE-GATE-03` / `LINT-CLIPPY-01` / `LINT-FMT-01` と併せて判断すればよい。

**「未 commit だから貴重」ではなかった。** 7 本の dirty は、その branch が main へ landing した
**後**に残った残骸 (formatter 出力 / 旧形 / 訳す前の docstring) が大半で、
landing 前の作業中断ではなかった。これで 7 本の worktree は破棄可能になる。

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

## 全 local branch への棚卸しの拡張 (2026-08-22)

上の「片付け」節は **worktree が checkout していた branch** だけを見ていた。`main` が
`13a3786b` になった時点で、対象を **全 local branch 129 本** へ広げて `git cherry` を回し直した。

| 分類 | 件数 |
|---|---|
| `+` が 0 本 = 取り込み済み | 80 |
| `+` を 1 本以上持つ = 未取り込み | 49 |

**この 49 という数字が、以後の未取り込み branch の唯一の正本である。** 上の節の
「真に未取り込み 29」は worktree に載っていた branch だけを分母にした数で、母集団が違う。
両者を足したり引いたりしてはいけない。

49 本の内訳:

| 系列 | 件数 | 扱い |
|---|---|---|
| batch family (`3b5dbef5` を tip とするスタック) | 26 | `BOUNDED-SCAN-01` の family 単位 hand-port のみ。**merge はしない** |
| 単独系列で 25 commit 以上 | 10 | 個別に content diff で判定する |
| 12 commit 以下 | 13 | 個別に content diff で判定する |

batch family 26 本のうち 2 本 (`codex/lsharp-typeinfer-declaration-scans` /
`codex/lsharp-next-type-expression`) は名前に `batch` を含まないが、同一スタック上にある
(それぞれ base から 113 / 107 commit)。名前で family を判定してはいけない。

### batch family を merge できない根拠 (実測)

```
git diff --stat main..3b5dbef5
  => 1209 files changed, 93307 insertions(+), 202114 deletions(-)
```

削除が挿入の 2 倍以上ある。branch 側が消そうとしているのは main が意図的に移設・整理した
ファイル群で、root の `tests/meta_validation.rs` (main が `crates/lsharp-wasm/tests/` へ
移設済み) を復活させる差分まで含む。**lineage が古すぎて、merge は main の整理を巻き戻す。**

### fork の構造 (ledger の正確さのため)

tip `3b5dbef5` に対して 4 本が祖先ではない。それぞれ 1 commit だけ先行する。

| branch | tip からの commit | `git cherry` の `+` |
|---|---|---|
| `codex/lsharp-type-record-check-batch` | 1 | 0 |
| `codex/lsharp-typeinfer-apply-batch` | 1 | 0 |
| `codex/lsharp-typescheme-batch` | 1 | 0 |
| `codex/lsharp-type-record-ops-batch` | 1 | **1** (`a5bb397a` docs: record Linux evidence for Type record operations) |

「tip が family 全部を覆う」は patch-id では真だが **ancestry では偽**。`a5bb397a` だけは
tip に無く、docs のみの commit である。4 本とも origin にあるので ref が消える危険は無い。

### 取り込み済み 80 本の処置

worktree が占有する 9 本を除いた 71 本が削除対象。うち **`main` の祖先 25 本は削除済み**。
残る **patch-id 一致のみの 46 本は未削除** — `git branch -D` が auto mode classifier に
拒否されたため。内容の等価性は確認済みなので、承認が下りれば実行できる。名前と sha は
[`../development/operations/absorbed-branch-refs-2026-08-22.md`](../development/operations/absorbed-branch-refs-2026-08-22.md)
に残す。この台帳が保証するのは sha からの復元ではなく **patch-id としての内容等価性** である
(reflog は約 30 日で expire する)。

### 未取り込み 23 本のうち、小さいものの判定 (2026-08-22)

判定は `git cherry` の commit 数ではなく **touched file の content diff** で行った。
whole-file take や hand-merge で入れた分は patch-id が一致しないため、commit 数は
「取り込むものが残っているか」を答えない。

| branch | commit | 判定 | 根拠 |
|---|---|---|---|
| `codex/legacy-test-01-occur-check` | `59c7dbba` | **取り込む** | main は深い occur-check で LS1003 を返す前に abort する。`INFER-DEPTH-01` / [ADR](decisions-infer-occur-check-depth-bound.md) |
| `codex/lsharp-qualified-record-literal` | `46b3643e` | 取り込み済み | `test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_literal` が main にある |
| `codex/lsharp-qualified-record-accessor` | `d99aafe6` | 取り込み済み | `..._resolves_import_qualified_record_accessor` / `..._filters_import_alias_only_record_accessor` が main にある |
| `codex/lsharp-open-unqualified` | `f6dbd448` | 取り込み済み | `..._filters_import_open_unqualified_definition` が main にある |
| `codex/legacy-test-01-recursion-runtime` | `18170467` | 取り込み済み | main は `runtime_recursion_limits.rs` + [ADR](decisions-legacy-test-runtime-recursion-limit.md) + `validation/runtime-recursion-limit.md` + CI script 2 本を持つ |
| `codex/legacy-module-scc` | `97a9130a` | 取り込み済み | `ModuleGraph::scc_groups()` が main にある (`module_graph.rs:230`) |
| `codex/v0.2-ec-m1-02-inventory` | `0dbc5d11` | 取り込み済み | `33a1e547` で hand-merge 済み。残る `metadata_contract.rs` 差分は **main が branch より進んでいる** (intent module / property defaults / `Binder::source_span` / `inventory_decl_tree`) |
| `codex/v0.2-ec-m1-02-generator-gate` | `84ca54fd` | 却下済み | main は `run_metadata_tests` が `inventory_contract_suites` を併走させる別経路を採った。判断は `crates/lsharp-types/tests/metadata_contract_generation.rs` 冒頭に記録済み |
| `codex/v0.2-ec-m1-02-selfhost-generator` | `986ac1e3` | 却下済み | main は `extract-parser-contract-suites` という別の canonical 経路を採った。予告されていた問題は `I-31` ではなく **`I-30`** へ移してある |
| `codex/todo-active-backlog` | `3ca483e9` | 却下 | 唯一残る差分は `improvement-roadmap.md` の B-2 行で、branch 側は main が足した verified slice 注記を**消す**方向。main が新しい |
| `backup/dev-loop-speedup-pre-rebase` | 2 件 | 取り込み済み | rebase 前の退避 ref。main に同題の `9203de68` / `43d3b905` がある |
| `codex/legacy-test-01-limits` | 4 件 | 却下 | 4 commit すべて main が別形で持つか、main の方が新しい (下記) |
| `codex/legacy-test-01-formatter-blocker` | 2 件 | **部分取り込み** | fixture 修正 (1 hunk) は取り込み、`.ls` の module 再構成と docs は却下 (下記) |
| `codex/legacy-module-scc-cache-contract` | 7 件 | **部分取り込み** | 6 commit は main が別形で持つか main の方が新しい。`265a42c5` の指摘だけ移植した (下記) |
| `codex/lsharp-type-record-ops-batch` | `a5bb397a` | 却下 | main は同じ slice を **別の Linux 実測 (`77f177ab`)** で既に持つ。branch の `be55ac33` 実測は main の履歴に対応しない (下記) |
| `codex/v0.2-diag-api-doc-forwarding-rebased` | 21 件 | **一部が live。`I-02` へ引き継ぐ** | 10 件は取り込み済み、1 件は main が別実装を採用済み。残る **10 件** (`feat: forward ... diagnostics` 8 + cycle/export span 2) は main に無く、その範囲は `I-02` の未解消部分と一致する (下記) |
| `codex/v0.2-ec-m1-06-all-form-differential` | 25 件 | **一部が live。`EC-M1-06` / `EC-M1-07` / `ROOT-SLOT-PROBE-01` へ引き継ぐ** | 7 件は main が別設計で達成済み、1 件は修正の実体だけ取り込み済み。残るのは provenance の field 集合、JSON `contracts` field、mixed JSON と text の CLI coverage、canonical case failure message、schema 文書、root slot probe (下記) |
| `codex/lsharp-wasmgc-atomic-artifact` | 37 件 | **全件取り込み済み** | `git cherry` の `+` は main が後から入れた file 分割 (`wasmgc.rs` -> `wasmgc/` 3 module、`wasmgc_runner.rs` -> 5 module、`wasmgc_probe.rs` -> `part_000..015.rs`) で patch-id が崩れただけ。追加関数 208 個はミス 0、ADR 79 本と `wit/` 2 本も main にある (下記) |

残る未判定は 6 本。batch family の例外 1 本は上表で判定済み。

#### `codex/legacy-module-scc-cache-contract` を 1 commit だけ取り込んだ根拠

7 commit のうち **6 件は main が既に別形で持つか、main の方が先へ進んでいる**。
main は当時の `lib.rs` を `compile_incremental.rs` / `compile_entrypoints.rs` /
`compile_pipeline.rs` / `compile_support.rs` へ分割し、`module_graph_scc.rs` を
`module_graph/scc.rs` へ移しているので、branch の diff はそのままでは当たらない。

| commit | branch が持つもの | main の状態 |
|---|---|---|
| `605539a9` SCC grouping foundation | `module_graph_scc.rs` + `scc_groups()` | `module_graph/scc.rs` の `compute_groups` + `ModuleGraph::scc_groups()` (`module_graph.rs:230`)。test は `module_graph/scc_tests.rs` と `module_graph/tests.rs:251` |
| `f412c141` SCC mutual recursion inference | `module_scc_infer.rs` | `compile_pipeline.rs:418` の `infer_scc_type_surfaces`。`compile_entrypoints.rs:55` / `compile_incremental.rs:83` から呼ばれる |
| `723825a8` formatter を acyclic 化 | facade から式 dispatcher を移す | **明示的に却下**。上記 `codex/legacy-test-01-formatter-blocker` の節を参照 |
| `160d7d19` block 推論を stack-safe に | `TypeInferBlock.ls` を 293 → 139 行へ縮める | main の同ファイルは **660 行**で、branch 版とは 18+/539- の差。branch が回避したかった stack overflow は main では起きない -- `test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` が main で pass する (2026-08-22 実測、66.9s の suite 内) |
| `69f580ef` incremental を SCC group へ一般化 | `lib.rs` の 666 行改修 | `compile_incremental.rs:528` が `group.len() > 1` を見て `compile_multi_file_incremental_scc` へ分岐する |
| `ae24949c` dependency surface で cache を keying | `ModuleCacheEntry::deps_key` + `compile_multi_file_with_cache` | 両方 main にある (`cache.rs:158` / `compile_entrypoints.rs:116`)。ADR は `decisions-legacy-module-*` 30 本の族へ発展済み |
| `265a42c5` analysis/compile cache の readiness 分離 | `ir_ready` / `has_ir()` | **main に無かった。取り込んだ** |

**`265a42c5` は本物の bug 指摘だった。** main の `analyze_multi_file_incremental_with_overrides` は
空の placeholder IR を cache へ入れるのに、compile 側の clean-hit は fingerprint しか見ていない。
`crates/lsharp-tooling/src/compile.rs:259` から呼ばれる公開経路で空 module が返る。
RED を書いて再現し、main の分割後の構造へ移植した。詳細は
[analysis/compile cache 境界 ADR](decisions-legacy-module-analysis-compile-cache-boundary.md) と `I-33`。

**commit 数で判定していたら見落としていた。** 7 件中 6 件が「main が別形で持つ」ため、
branch 全体を却下する誘惑があったが、内容で 1 件ずつ当たった結果 1 件だけ生きていた。

#### `codex/v0.2-diag-api-doc-forwarding-rebased` の 21 commit を分けた根拠

`git cherry` は 12 を返すが、実際の commit は 21。

**2026-08-22 訂正。** 初版はこの 21 件を「7 / 4 / 6 / 4」に割り、module graph の span 4 件を
まとめて取り込み済みとした。**これが誤りだった。** 割り方が subject の見た目 (テーマ) に
引きずられており、commit ごとに main の中身を見ていない。`I-32` で一度やった失敗と同型なので、
下表は **1 commit ずつ main の tree を確認して**組み直したものに差し替える。

| 群 | 件数 | 判定 | 根拠 |
|---|---|---|---|
| `test(selfhost)` / `fix(selfhost)` (property precondition span、assertion oracle、root slot guard) | 8 | 取り込み済み | 各 commit が足す fn / `defn` 名を main へ grep して全件ヒット。`eval-property-precondition-with-index` (`TestRunner.ls:4235`)、`materialize-property-with-span` (`:4631`)、`test_assertion_execution_reports_each_predicate` (`test_runner.rs:505`) など |
| `09b1e154 test(selfhost): promote canonical assert CLI gate` | 1 | **半分だけ取り込み済み** | docs 側 (`rust-boundary-reduction.md:121` の 429.35s 実測と昇格記述) は main に一字一句ある。一方 code 側の `#[ignore]` 解除は main に無い — main は 2026-08-19 `939e4ec9` (TESTGATE-03、heavy e2e 164 件) で**同じ属性を意図的に付け直している**。後の判断が勝つので取り込まない |
| `f9c147bf fix: add stable module graph diagnostic codes` | 1 | 取り込み済み | main の `ModuleGraphError::code()` は同じ `LS3101`〜`LS3104` を返す (`module_graph.rs:85-92`) |
| `23ef8edd feat: preserve module graph import spans` | 1 | 却下 (main が別実装) | branch は `import_spans: HashMap<(String,String), Span>` を graph に持たせて全 variant に `span: Option<Span>` を足す。main は**別解**を採っており、`ModuleNotFoundAt` variant を分けて error 時にだけ `find_import_span` で読み直す (`resolve.rs:260`)。意図は満たされているので branch 実装は入れない |
| `be65dcce feat: preserve module cycle import spans` / `2e94896c feat: preserve package export import spans` | 2 | **main に無い** | main の `span()` は `ModuleNotFoundAt` にしか `Some` を返さず、`CyclicDependency` と `ModuleNotExported` は `None` (`module_graph.rs:94-98`)。`cycle_span` / `import_spans` / `test_build_from_entry_cycle_preserves_closing_import_span` はいずれも main の `crates/` に存在しない |
| `feat: forward ... diagnostics` (cli source / module graph / lowering / codegen / io、repl、lsp、api-doc) | 8 | **main に無い** | 各 commit が足す test 名を main へ grep して全件ミス。実装関数 (`source_diagnostic` / `codegen_diagnostic` / `io_diagnostic` / `module_graph_diagnostic` / `repl_source_diagnostic` / `api_doc_source_diagnostic`) も無い |

**この訂正で live は 6 件から 10 件へ増えた。** 内訳は `feat: forward` 8 + cycle/export span 2。

##### 初版が誤った理由と、以後の判定規則

初版は main に `code()` と `span()` があることを確認し、しかも branch に無い `ModuleNotFoundAt`
variant まで見つけたので、「main の方が進んでいる」と読んだ。**`span()` の match 腕を読んでいなかった。**
`ModuleNotFoundAt` 以外は `None` を返すので、cycle と export の span は main に無い。
branch の該当 2 commit がまさにその 2 variant を埋めるものだった。

以後、この作業では次を規則とする。

- **test 名 grep は triage であって判定ではない。** ヒット 0 件、または 1 件でもミスがある commit は、
  取り込み済みへ入れる前に必ず内容を直接見る。`09b1e154` (抽出名 0)、`2e94896c` (抽出名 0)、
  `be65dcce` (ミス 1) の 3 件は、この規則があれば初版で捕まえられた
- **subject でグルーピングしない。** 「module graph の span 系 4 件」のようなテーマ単位の束ねが
  今回の誤りを生んだ。判定単位は常に commit 1 件

**未取り込みの 10 件は、branch 単位で判断すべきものではなかった。**

**未取り込みの 6 件は、branch 単位で判断すべきものではなかった。**

その中身は「診断へ `NamedSource` と `LabeledSpan` を添えて、code と span を surface まで
構造のまま運ぶ」ことで、これは既に `I-02` (診断 code/span が全 surface に未貫通、状態 in-design)
の未解消部分そのものである。`I-02` は「multi-file / REPL / doc / metadata / native linker /
incremental module・codegen の経路で**文字列化や span 消失が残る**」と書いており、
branch が触る 8 経路と一致する。`DOC-06` も「CLI / LSP / MCP の全診断へ `LS####` を
貫通させる作業は引き続き `I-02` / `imp-02` の範囲に残す」と明示している。

したがって **branch を単位に取り込むのではなく、`I-02` / `imp-02` の設計に沿って
経路ごとに閉じる**。branch は参照実装として残す。

cycle / export span の 2 件も同じ扱いにする。`ModuleGraphError` の全 variant へ span を載せる形は
`23ef8edd` で却下した `import_spans` 方式と不可分なので、main の `ModuleNotFoundAt` +
`find_import_span` 方式へどう寄せるかは `imp-02` の設計判断が要る。

##### この判定の過程で main 側に確認した具体箇所 (`I-02` の証拠として本文へ移した)

- `crates/lsharp-tooling/src/api_doc.rs:188-193` — `build_api_doc_for_file` は
  `miette::miette!("[{}] ... {e}", e.code())` で code を**文字列へ埋めるだけ**で、
  `NamedSource` も `LabeledSpan` も添えない。同 workspace の
  `crates/lsharp-driver/src/main.rs:1739` / `:1776` は添えているので、方式が揃っていない
- `crates/lsharp-lsp/src/util.rs:515 diagnostic_span_from_message` — LSP は
  `LS3102` の span を **診断メッセージの日本語文言 (`"モジュール '"` / `"' が見つかりません"`) を
  文字列検索して**復元している。同 `:493 stable_code_from_message` も `[LSxxxx]` を
  文字列から抜き出す。**文言を変えると span 復元が黙って壊れる**
- 原因は API 境界。`analyze_single_file_incremental` /
  `analyze_multi_file_incremental_with_overrides` は `Result<(), String>` を返すので、
  `ModuleGraphError` が `code()` と `span()` を持っているのに、LSP へ渡る時点で
  構造が捨てられている (`util.rs:616` / `:640`)
- なお `module_graph/resolve.rs:314 extract_imports` が span を捨てるのは **defect ではない**。
  main は error 時にだけ `find_import_span` で読み直す (`:260`) 設計を採っており、
  hot path に span を載せないという取捨選択になっている

#### batch family の例外 `a5bb397a` を却下した根拠

`codex/lsharp-type-record-ops-batch` は tip `3b5dbef5` に含まれない commit を 1 つ持つ
(`a5bb397a docs: record Linux evidence for Type record operations`)。docs 2 ファイル、13 行追加のみ。

**実装側は main に既にある。** `type-record-field-type` / `type-record-fields-eq` の 64 要素
bounded/rooted chunk は `selfhost/src/Types/Type.ls:261-301` にあり、branch が根拠に挙げる 2 test
(`test_e2e_selfhost_type_record_ops_use_bounded_chunks` /
`test_e2e_selfhost_large_record_type_operations_preserve_results`) も
`crates/lsharp-wasm/tests/e2e/selfhost_type_record_ops.rs` にある。**docs だけが未取り込みだった。**

| branch の追記先 | 判定 | 根拠 |
|---|---|---|
| `TODO.md` に `V2-16` の独立 `[~]` 項目を新設 | 却下 | main は同じ内容を `LEGACY-LANG-01` の本文へ畳み込んで持つ (`TODO.md:2140`)。足すと二重計上になる |
| `rust-boundary-reduction.md` に Linux 実測節を追加 | 却下 | main は `### LEGACY-LANG-01 selfhost bounded record type lookup/equality slice (2026-07-31)` を同じ日付で既に持つ |

**却下の決め手は「重複」ではなく「数字が main のものではない」こと。** 両者は同じ変更に対する
独立した Linux x86_64 fixed-point run で、source commit も測定値も違う。

| | branch (`a5bb397a`) | main (`:3213`) |
|---|---|---|
| source commit | `be55ac33` (**main の祖先ではない**) | `77f177ab` |
| stage2/stage3 code length | `11421747` | `11168596` |
| stdout SHA-256 | `2f6eeb64...` | `dad391cd...` |
| artifact id | `be55ac33-type-record-ops` | `77f177ab-type-record-ops` |

branch の節を取り込むと、**main の履歴に存在しない commit の実測値**が main の運用記録に載る。
`docs/development/operations/` は「実測値とその取得条件」の正本なので、取得条件が main で
再現できない値を置いてはならない。なお両 artifact とも作業ツリーには残っていない
(`ci-artifacts/` は空) ため、どちらの数字も再取得はできない。main 側の記録を正とする。

これで batch family 26 本の未判定は 0 になった。family 本体 (bounded scan の実装差分) は
`BOUNDED-SCAN-01` の対象として別に残る。

#### `codex/legacy-test-01-formatter-blocker` を fixture だけ取り込んだ根拠

branch は 2 commit。`632695bf` が docs、`49588db3` が fix。**同じ e2e 2 件を緑にする解が
main と branch で違う**ので、fix を 2 つに割って別々に判定した。

| branch の変更 | 判定 | 根拠 |
|---|---|---|
| e2e fixture へ `selfhost_module("AST.ls")` を足す (2 hunk) | **取り込み** | main の RED を実測で再現し、修正で緑になった (下記) |
| `Formatter.ls` から `format-expr` と式 dispatcher を `FormatterExpr.ls` へ移して acyclic 化 (237+/238-) | 却下 | main は循環を許容する SCC 経路を採った。前提 commit `723825a8` (`fix(selfhost): make formatter modules acyclic`) は main の祖先ではない |
| `632695bf` の imp-03 / imp-07 追記 | 却下 | 内容で確認した。imp-03 の 4097 object / 32769 root stack / `memory.grow` 失敗契約は main の imp-07 `:102-104` が **CI lane 名 (`test-runtime-limits.sh` の 8 exact E2E) つきの新しい形**で持つ。imp-07 追記のうち「`FormatterExpr -> FormatterDecl -> Formatter` の一方向 module graph に修正した」は **main では偽** — main は逆方向の循環を明示的に採っている (上の行と同じ根拠) |

**台帳の診断が誤っていた。** `docs/development/validation/workspace-expected-failures.txt` は
この 2 件を「`ast-defn-signature` が未定義。未実装への RED」と記録していたが、
`ast-defn-signature` は `selfhost/src/Syntax/AST.ls:44` に存在する。実際の原因は
e2e fixture が `AST.ls` を連結していなかった **fixture の欠落**で、未実装ではない。

- RED: `cargo test -q -p lsharp-wasm --test e2e test_e2e_selfhost_formatter` →
  `test result: FAILED. 14 passed; 2 failed`。2 件とも
  `UndefinedVar { name: "ast-defn-signature", span: Span { start: 24644, end: 24662 } }`
- GREEN: 同コマンドで `test result: ok. 16 passed; 0 failed`
- 台帳から 2 行を削除し、誤診断だった旨を同ファイルへ注記した

**acyclic 化を却下した根拠。** main は `ModuleGraph::scc_groups()` (Tarjan) を持ち、
`compile_incremental.rs:370` が `group.len() > 1` を見て SCC 専用経路へ分岐する。
循環は**バグではなく許容される入力**という設計である。branch はその前段 `723825a8` で
循環を消す方向へ倒したが、main はそれを取っていない。同じ 2 件がどちらの道でも緑になる以上、
main が既に持ち test も通っている側を残すのが筋である。

**当初この却下に「循環を契約として張る test が main に無い」という残件を付けて `I-32` を
起票したが、誤りだったので撤回した。** main の循環は事故ではなく
[`decisions-legacy-formatter-scc-imports.md`](decisions-legacy-formatter-scc-imports.md)
(2026-07-24) が**意図して入れたもの**である。同 ADR は `Formatter.ls` 固有の
`try_infer_formatter_trio_batch` 特例を消して generic な `infer_scc_type_surfaces` に寄せる
判断を記録しており、そのために両 module へ `(import Tools.Text.Formatter)` を明示させている。
契約は以下で張られている:

- `lib_tests/incremental_analysis.rs:634` `test_formatter_modules_declare_cross_module_dispatch_imports`
  -- `FormatterExpr.ls` / `FormatterDecl.ls` の source に `(import Tools.Text.Formatter)` 行が
  あることを直接 assert する。循環辺が消えたら落ちる
- `module_graph/tests.rs:251` `test_scc_groups_are_stable_and_dependency_first` /
  `module_graph/scc_tests.rs:6` -- 2 頂点 SCC (`CycleA` / `CycleB`) を作って group 順を固定する
- `lib_tests/multifile_compile.rs` の `test_compile_multi_file_infers_mutual_recursive_scc` ほか
  相互再帰 SCC の compile / incremental / import visibility を覆う 8 本

したがって branch の acyclic 化は「main が別の道を採った」だけでなく、
**main が明示的に却下した方向**である。却下理由は当初の想定より強い。

#### `codex/legacy-test-01-limits` の 4 commit を却下した根拠

commit 単位で見ると 4 件とも main に patch-id が無い。しかし内容で見ると、
main はすべて**別の形で既に持っている**か、**branch より新しい**。

| commit | branch が持つもの | main の状態 |
|---|---|---|
| `4b03182f` parser property contracts | 非 ASCII escape の lexer 修正 + parser property test | 修正は `lexer/` 分割後の `lexer/tokenization.rs:53-70` に `ch.len_utf8()` を消費する形で入っている。test は `property_tests::parser_never_panics_for_bounded_arbitrary_bytes` / `roundtrip_property_tests::pretty_printed_ast_reparses_to_the_same_source` / `lexer::tests::test_invalid_utf8_after_escape_returns_error_without_panic` が同じ契約を張る |
| `6bc274a0` inference property contracts | unify 対称性 / bounded inference の property test | `infer::unify_property_tests::unify_success_is_symmetric` / `infer::inference_property_tests::bounded_expression_inference_never_panics` が main にある |
| `f93d067c` test distribution report | `scripts/test-distribution-report.sh` + その契約 test (bash) | main は `scripts/test-distribution.py` + `scripts/test-test-distribution.sh` を持ち、判断は [ADR](decisions-legacy-test-distribution.md) に記録済み |
| `3bdf6b1c` allocator limit evidence | imp-03 / imp-07 への 2026-07-24 の実測追記 | main の imp-03 は **2026-07-25 の size-class verified slice** を持ち、imp-07 は `test-distribution` / property 4096-case lane / GC soak telemetry まで載っている。branch の記録は main に上書きされている |

**実際に取り込みを試して撤回した。** `4b03182f` / `6bc274a0` の property test を
`crates/lsharp-types/src/infer/property_tests.rs` と
`crates/lsharp-syntax/tests/parser_properties.rs` へ移植して実行したところ、
上記の main 側 test 名がすべて既に pass していた。同じ契約の二重管理になるので revert した。
「commit が残っているから未取り込み」ではないことの実例である。

#### `codex/v0.2-ec-m1-06-all-form-differential` の 25 commit を分けた根拠

merge-base は `941920a4` (2026-07-21)、tip は `60ed461a` (2026-07-23)。
main に対して **1454 commit behind**。`git merge-tree` は conflict を
`docs/adr/decisions-004.jsonl` と `TODO.md` の 7 箇所しか報告しないが、
**これを安全性の根拠にはしない。** branch は `crates/lsharp-driver/src/mcp_server.rs` を
1280 行の単一ファイルとして触るのに対し、main は同ファイルを 160 行まで削って
`mcp_*.rs` へ分割済みである。file 構成が入れ替わった先で code merge が clean に通ること自体が
むしろ危険信号であって、意味の一致を意味しない。**branch 単位の merge はしない。**

25 commit を `test 名 grep` にかけると **ヒット 0 件** (追加された 18 の test 関数名がどれも
main に無い)。しかし前節で定めたとおり grep は triage であって判定ではないので、
1 件ずつ内容を main と突き合わせた。結果は「main が同じ意図を**後から別設計で**満たしている」が
大半で、live な残りは 3 種類に絞れた。

| commit | branch が持つもの | main の状態 | 判定 |
|---|---|---|---|
| `806929ff` migration enum contract | `docs/schemas/legacy-migration.schema.json` (100 行) + `mcp_server.rs` の検証 | enum 契約は **selfhost 側の fail-closed validator** (`legacy-migration-row-schema-valid?`) と [ADR](decisions-v0.2-selfhost-migration-enum-schema.md)、Rust 側 `crates/lsharp-driver/src/mcp_tests.rs:1756` にある。**schema 文書そのものは無い** | 契約は却下、**schema 文書だけ live** |
| `b9cfd0b0` MCP migration row shape | `mcp_server.rs` へ 53 行 | 同上。main の `mcp_server.rs` は分割済みで、行の当たり先が存在しない | 却下 |
| `50a80ad9` schema closure の記録 | `rust-boundary-reduction.md` へ 8 行 | 同ファイルは main 側が先へ進んでいる | 却下 |
| `0ded1928` canonical case failure message | selfhost `TestRunner.ls` が actual/expected span から Rust oracle と同じ message を再構築 | main の `run-cases-loop` は `make-test-result-with-diagnostic` / `-span` しか呼ばず、**message を持たない**。message は Rust oracle (`crates/lsharp-wasm/src/test_runner.rs:297`) 側にしか無い | **live** |
| `b415f8cb` root slot balance | `TypeInfer.ls` / `TypeInferFunctions.ls` の余分な `root_pop` 2 箇所を削除 + `root_slot_probe.rs` (179 行) | **実体の 2 箇所は main で既に balance している** -- `infer-var` は 3 push / 3 pop、`typeinfer-finalize-defn-result-with-env-vars` は 7 push / 7 pop。probe harness だけが無い | fix は取り込み済み、**probe は coverage 残** |
| `2510b108` canonical case を assurance に数える | `Cli.ls` / `EmbeddedCli.ls` に case 計上を追加 | main は `assurance-suite-executed` + `run-test-source-json-suite` という **suite accessor 経由の別設計**で数える | 却下 (別実装) |
| `f233fadd` / `2e5d2050` / `3bba67f0` mixed case+property の JSON report | e2e 3 件 | main は `test_format_json_file` / `..._non_bool_invariant` / `..._property_precondition_span` を持つが、**case と property を混在させた組み合わせは無い** | **coverage 残** |
| `846db44b` text assurance report contract | `selfhost/src/Tools/Test/AssuranceText.ls` (84 行) を新設し Cli/EmbeddedCli で共有 | main は `run-test-source-text` を `Cli.ls:1198` と `EmbeddedCli.ls:1125` へ **それぞれ inline** で持ち、共有 module は作らなかった | 実装は却下 (別実装) |
| `4b80ae15` / `d294f662` / `e0fb42df` text 経路の e2e | e2e 3 件 | main の e2e には `test_format_text_*` が **1 件も無い** (`test_format_json_*` のみ) | **coverage 残** |
| `0ce5da87` text assurance を preflight へ通す | `AssuranceText.ls` 経由で text を preflight 化 | main の `run-test-source-text` は `run-test-source-case-preflight` を通り、`metadata-test-runner-boundary-code` と canonical case 診断の両方で早期 exit する | 却下 (別実装で達成済み) |
| `34e6cdd1` JSON/text preflight sentinel の一致 | 1 行 | main は両経路とも `exit-runtime-error` を返す | 却下 (達成済み) |
| `8b126354` / `d315bce8` / `b79c6394` / `bcc10e03` / `fd516945` / `948b06fc` / `c7f395c1` JSON provenance | `assurance-provenance-json` を 3 fields → **7 fields** へ広げ、`runner` を `"selfhost"` → `(assurance-json-runner)` = `"selfhost-cli"`、`target` を `"unknown"` → `"runtime-selected"` にする + e2e 7 件 | main は `Cli.ls:954` の 3 fields (`runner` / `source_commit` / `artifact_digest`) のまま、`target` も `"unknown"` | **live** (下記の但し書きあり) |
| `27b8ce3a` / `7d1d1dad` / `60ed461a` 全 form の計上と JSON contract counts | `contracts` field 追加 + 全 form の text failure accounting + e2e 6 件 | text 側の全 form accounting は **main が `run-test-source-text` に inline で持つ** (example / invariant / assertion / case / property の 5 種を数え `count-failed-results` を合算する)。**JSON 側の `contracts` field は main に無い** | accounting は却下、`contracts` field は **live** |

25 件の内訳: 却下 (main が別設計で達成済み) **7 件**、実体は取り込み済みで probe だけ残る **1 件**、
live **17 件** (schema 文書 1 / case message 1 / mixed JSON coverage 3 / text coverage 3 /
provenance 7 / `contracts` field を含む all-form 3 -- ただし後述のとおり中身は 3 種類に畳める)。

##### provenance 7 件についての但し書き -- branch も「注入」はしていない

`TODO.md` の `EC-M1-06` と `rust-boundary-reduction.md` の 3 箇所は残件として
「provenance **注入**」と書いている。branch が足す 4 field は
`producer "lsharp-selfhost"` / `tool_version "0.1.0"` / `source_digest "unknown"` /
`timestamp "unknown"` で、**すべて literal 定数**である。つまり branch がやったのは
field 集合を広げる shape の変更であって、実際の commit hash や digest の注入ではない。

したがって「branch を移植すれば `EC-M1-06` の provenance が閉じる」は成り立たない。
7 field という shape が正しいか、`runner` を `"selfhost-cli"` に変えてよいか
(main の Rust 側 report と differential を取る相手が変わる) は `EC-M1-06` の設計判断であり、
**branch を参照実装として引き継ぐが、shape をそのまま採るとは決めない。**

##### live な残りの引き取り先

branch 単位では取り込まない。1454 commit の乖離を跨いで 17 commit を順に当てるより、
main の現行設計の上で書き直す方が安い。内容ごとに正本を分けて引き継ぐ。

| 残るもの | 引き取り先 | 理由 |
|---|---|---|
| provenance の field 集合 / `runner` / `target` ラベル、JSON `contracts` field、mixed JSON と text の CLI coverage、canonical case failure message | `EC-M1-06` | `TODO.md` の受入条件が「全 form、EmbeddedCli、Rust/selfhost report field differential、provenance、2 target evidence を閉じる」と、そのまま同じ範囲を指している |
| `docs/schemas/legacy-migration.schema.json` | `EC-M1-07` | 受入条件が「guide/**schema**/MCP/migration docs を同じ observable contract へ揃える」と schema を名指ししている。enum 契約自体は既に閉じているので、残るのは**機械可読な契約文書が無い**という一点 |
| `root_slot_probe.rs` の regression guard | `ROOT-SLOT-PROBE-01` (新規) | main の `selfhost_rooting_parity.rs` は **codegen が root_push を出すか**を見る test 群で、**bundle 実行後に caller の root slot が残っているか**は見ていない。`NATIVE-ROOT-02/03` は native root API の実装が対象で、ここの持ち主ではない |

##### この branch から学んだこと -- 「後から別設計」は test 名 grep では見えない

25 件のうち 7 件は、main が同じ意図を**別の名前の関数で**満たしていた
(`assurance-result-executed-loop` → `assurance-suite-executed`、
`AssuranceText.ls` → `run-test-source-text` の inline 実装)。
test 名 grep は 25 件すべてを「main に無い」と答えたが、内容で見ると 7 件は取り込む必要が無い。
逆に `b415f8cb` は「main に無い」と答えられた commit だが、**修正の実体は main にあり**、
無いのは probe だけだった。**grep の答えは両方向に間違う。**

#### `codex/lsharp-wasmgc-atomic-artifact` の 37 commit を全件取り込み済みと判定した根拠

merge-base は `0b63ff9f` (2026-07-24)、tip は `35f59fe5`。main に対して **1368 commit behind**。
`git cherry main` が `+` を返すのは 82 commit 中 **37 件**だが、内容で見ると
**37 件すべてが main にある**。`+` の原因は main が後から入れた **file 分割**である。

| branch | main | 分割 |
|---|---|---|
| `crates/lsharp-wasm/src/wasmgc.rs` 731 行 | 429 行 + `wasmgc/instructions.rs` 206 + `wasmgc/validation.rs` 389 + `wasmgc/validation_tests.rs` 11 | `f3b00b49` 系 |
| `crates/lsharp-wasm/src/wasmgc_runner.rs` 733 行 | 121 行 + `_output_writer.rs` 110 + `_component_output.rs` 189 + `_component_preview2.rs` 375 + `_tests.rs` 38 | 同上 |
| `crates/lsharp-wasm/tests/wasmgc_probe.rs` 10300 行 | `include!` 16 行 + `wasmgc_probe/part_000..015.rs` **10769 行** | `f3b00b49` |

分割後は hunk の当たり先が変わるため patch-id が一致しない。**commit 数は「取り込むものが
残っているか」を答えない**という、本 ADR で繰り返し確認してきた事象がここでも起きている。

内容の確認は 3 方向で行った。

1. **37 commit が追加した関数名 208 個を main へ突き合わせ、ミス 0 件。**
   `test_` 接頭辞に限らず `fn` 宣言をすべて拾った (この branch の probe test は
   `wasm_gc_component_cli_fs_runner_*` 命名で `test_` を持たないため、
   `test_` だけを見ると 24 commit が「関数 0 個」に見えて triage が空振りする)
2. **file 内容の包含チェック。** 空白と行コメントを落として一意行を突き合わせると、
   probe は branch 1872 行に対し main 2047 行で **branch 側だけにある行は 4 行**、
   src 3 ファイルは branch 842 行に対し main 1026 行で **54 行**。
   いずれも長い関数シグネチャと `use` の折り返し位置が module 分割で変わっただけで、
   名前 (`wasm_gc_valtype` / `validate_module` / `module_uses_print_string` /
   `string_array_type_index` / `emit_wasm_gc_instructions` / `validate_gc_type_index` /
   `WasmGcEmitOptions` / `ComponentOutputLocals` / `decode_wasmgc_component_run_result`) は
   すべて main の分割先に存在する
3. **docs / wit 側。** branch が足した WasmGC ADR **79 本はすべて main にある**。
   `wit/lsharp-wasmgc-output.wit` は **byte 単位で同一**、`wit/README.md` は差分 0。
   `imp-01-wasmgc-full-migration.md` は branch 1106 行に対し main 1280 行で、
   branch 側だけにある 4 行はいずれも折り返し位置の違い (`Stage 2q の custom CLI` /
   `IrType::FuncRef` / `supported target` はすべて main 側にも含まれる)

branch の名前になっている atomic artifact 化 (`35f59fe5`) は `git cherry` の `+` に入っていない
= patch-id が一致しており、main では
`crates/lsharp-tooling/src/compile_tests_wasmgc_a.rs:66`
`test_compile_file_wasmgc_backend_uses_atomic_artifact_boundary` が張っている。

**判定: 全 37 件取り込み済み。live な残りは無い。** branch ref は削除対象へ回さず据え置く
(`WORKTREE-ABSORB-02` の「branch ref は消さないこと」に従う)。

##### triage 手順への追記 -- `test_` 接頭辞を仮定しない

この branch は probe test を `wasm_gc_component_cli_fs_runner_*` と命名しており、
`test_` で始まらない。`grep -oE '^\+\s*fn (test_[a-z0-9_]+)'` で triage すると
37 件中 24 件が「追加関数 0 個」と出て、**ヒット率 0/0 を「判定不能」ではなく
「取り込み済み」と読み違える**危険がある。triage の抽出は `fn [a-z0-9_]+` で行い、
**抽出数が 0 の commit は triage 失敗として扱って内容を見る。**

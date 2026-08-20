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

## ディスク

`target/` 42 本を削除し **86 GB** を回収した (`621Gi used / 251Gi avail` →
`535Gi used / 337Gi avail`)。main repo 自身の `target` と、取り込み作業で使用中の
共有 build dir は除外している。commit / branch / 未 commit の編集はいずれも失っていない
(編集は事前に patch として salvage 済み)。

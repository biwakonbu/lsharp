# ADR: root lifetime verifier の main exit 免除

- Status: Accepted (verified slice)
- Date: 2026-08-18
- Scope: `SMOKE-GATE-02` / `I-14` / `LEGACY-ROOT-01` / `crates/lsharp-ir/src/root_lifetime.rs`
- Related: [`ISSUES.md` I-14](../../ISSUES.md#i-14)、
  [`decisions-legacy-test-selfhost-rooting-guards.md`](decisions-legacy-test-selfhost-rooting-guards.md)、
  [runtime spec](../language/runtime-spec.md)

## Context

`root_lifetime.rs` は `root_push` / `root_pop` / `root_set` の抽象実行台帳で、関数の出口に
active な root slot が残っていれば `ImbalancedExit` (診断 `LS3003`) で lowering を止める。

一方 `root_push` / `root_pop` / `root_set` は runtime spec が定める**公開 API** であり、
均衡を要求する記述は spec のどこにも無い。この食い違いが `runtime_allocator_closures` の
恒常 FAIL 17 件を生んでいる。判別測定 (verifier を無効化した対照 build) の結果は
**17/17 が verifier 起因、隠れた本物の欠陥 0 件**。詳細と内訳は `I-14` が正本。

同時に、この verifier には実証済みの価値がある。`f8234503` は本検査によって selfhost compiler の
手書き `(root_pop)` 個数バグを 4 件検出している。**検査を弱めることと、検査を消すことは別**であり、
本 ADR は前者だけを扱う。

## Decision

**案 E を採る。`function.is_export && function.name == "main"` が真の関数に限り、
`ImbalancedExit` の判定を免除する。**

判定は `validate_function` の中、2 つの lease helper 分岐 (`ROOT_LEASE_ACQUIRE_HELPER` /
`ROOT_LEASE_RELEASE_HELPER` の早期 return) の**後**に置く。抽象実行そのものは従来どおり
最後まで走らせ、`RootPopUnderflow` / `StaleSlot` / `RootSetWithoutActiveSlot` /
`BranchDepthMismatch` は `main` でも従来どおり報告する。**免除するのは出口検査 1 つだけ**である。

### 健全性の根拠

`main` の出口に残っている root slot は、その直後にプログラムが終了するので **stale になり得ない**。
`ImbalancedExit` が防いでいるのは「解放されない slot を後続コードが踏む」事故だが、
WASI entry の出口にはその「後続コード」が存在しない。

### 述語をこの形にした理由

- `is_export` は `Function` に既存の field で、**新しい field を足さない**。
  `model.rs:67-75` の `Function` は `Default` を derive しておらず、src + tests に 104 箇所の
  literal 構築があるため、field 追加は不釣り合いに侵襲的である。
- `is_export: true` は `crates/lsharp-ir/src/lower/decl.rs:483` の
  `is_export: name == "main"` **1 箇所でしか立たない**。他の全構築点は `false` を渡す。
  つまりこの述語が指すのは「WASI entry として export される関数」だけである。
- 名前だけで判定する (`name == "main"`) より狭く、意図が読める。既存の負テストは
  すべて `is_export: false` で `Function` を組むため、緩和がテストへ漏れ出さないことが
  述語の形から保証される。

## 却下した選択肢

**案 A — verifier 全体を無効化 / 削除する。却下。**
selfhost の 38 ファイルが root API を直接使っており、`f8234503` が捕まえた 4 件の
`(root_pop)` 個数バグごと検査が消える。17 件の FAIL は消えるが、**この検査が過去に実証した
唯一の価値を捨てる**取引になる。安さと引き換えに失うものが大きすぎる。

**案 C — fixture 側を「均衡する形」へ書き換える。却下。**
落ちている fixture は、`main` で root を保持したまま終わること / 意図的に root を積み増して
object table を溢れさせることを**検証対象そのもの**として書いている。均衡させると
テストが検証しようとしている状況が消える。これは検査に合わせてテストの意図を破壊する行為で、
「テストの期待値を実装に合わせて変更しない」という本リポジトリの TDD 規律にも反する。

**案 B2 — `LS3003` を warning へ格下げする。却下。**
17 件は消えるが、`f8234503` が捕まえた種類の本物のバグも同時に warning へ落ちる。
安いが問題を隠す方向で、案 A と同じ損失を段階的に払うだけである。

**案 D — gate script 側の呼び出しを差し替える。不要になった。**
判別測定の前は「gate script が expected-failure を pass 前提で叩いている」ことが
`SMOKE-GATE-02` の主因だと見ていた。測定で主因が verifier 側だと確定したため、
script を触る必要は無くなった。**この旧仮説は取り下げる** (`TODO.md` の該当記述も差し替える)。

## 次スライスへ送るもの (案 B1)

残る 4 件 (`RootPopUnderflow` / `main` ×1、`BranchDepthMismatch` ×3) は本 ADR では扱わない。
これらは「意図的な不均衡」を IR へ伝える明示的な注釈を必要とし、言語表面の追加になる。
別 ADR を起票してから着手する。本スライスの完了条件に**混ぜない**。

## 実装順序 (doc-RED 時点の計画)

1. RED: `main` が root を保持したまま終わる module が `validate_module` を通ることを要求する test。
2. RED: **非 `main`** 関数の不均衡は引き続き拒否されることを要求する test (緩和が広がっていない保証)。
3. RED: `name: "main"` でも `is_export: false` なら拒否されることを要求する test (述語の狭さの pin)。
4. GREEN: `validate_function` に免除を追加。既存の負テストは触らない。
5. baseline (`docs/development/validation/workspace-expected-failures.txt`) から解消分を削除。

## Evidence

すべて 2026-08-18、worktree `codex/gate-fixes-root-lifetime` (base `e9227f3c`) での実測。

- **RED**: `cargo test -p lsharp-ir root_lifetime_ledger` →
  `test_root_lifetime_ledger_accepts_main_holding_root_at_exit` が
  `RootLifetime { error: ImbalancedExit { function: "main", depth: 1 } }` で失敗することを
  実装前に確認した。緩和の広がりを pin する 2 本
  (`..._still_rejects_lowered_non_main_imbalance` / `..._rejects_non_exported_function_named_main`)
  は当初から緑で、これは意図どおり (拒否側の現状を固定するテストのため)。
- **GREEN**: 同 filter → `9 passed; 0 failed`。crate 全体 `cargo test -p lsharp-ir` →
  **`294 passed; 0 failed`** (98.48s)。既存の負テスト 4 本はすべて無改変のまま緑。
- **本丸**: `cargo test -p lsharp-wasm --test e2e runtime_allocator_closures -- --test-threads=1` →
  **`90 passed; 4 failed`** (実施前は `77 passed; 17 failed`)。**13 件解消**で、
  内訳の予測 (`ImbalancedExit` / `main` = 13) と過不足なく一致した。
- **残る 4 件** — いずれも案 B1 の範囲で、免除が届かないことの確認でもある。

  | test | error |
  |---|---|
  | `..._root_runtime_api_tracks_slots_and_values` | `RootPopUnderflow { function: "main", instruction_index: 14 }` |
  | `..._runtime_object_table_grows_past_initial_capacity` | `BranchDepthMismatch { function: "alloc-rooted" }` |
  | `..._runtime_root_stack_grows_past_initial_capacity` | `BranchDepthMismatch { function: "push-roots" }` |
  | `..._runtime_root_stack_growth_preserves_root_api` | `BranchDepthMismatch { function: "push-roots" }` |

  `main` の `RootPopUnderflow` が残っていることが、免除が出口検査だけに閉じている実証になる。
- **baseline**: `workspace-expected-failures.txt` の entry 数 **108 → 95** (13 行削除)。
  e2e クラスタの見出しも `70 FAIL` → `57 FAIL` へ更新した。
- **gate**: `scripts/ci/test-gc-rooting.sh` → **rc=0** (実施前は rc=101)。
  `scripts/ci/test-selfhost-rooting-guards.sh` → rc=0 (回帰なし)。
- **lint**: `cargo clippy -p lsharp-ir --all-targets` → 警告 1 件だが
  `module_graph/resolve.rs:261` の `redundant_closure` で、本変更が触っていない既存箇所。

### 満たせなかった受入条件

- **`scripts/ci/test-runtime-limits.sh` は rc=101 のまま**。叩く先の
  `..._object_table_grows_past_initial_capacity` が案 B1 の範囲だからで、
  これは本 ADR の「含めない範囲」に明記した想定どおりの結果である。緑化は次スライスに残る。
- **`scripts/ci/check-workspace-baseline.sh` は再実行していない。** 同 script は
  workspace 全体の nextest 実測 (5 時間級) を入力に取る。代わりに、削除した 13 件が
  実際に pass へ転じたこと・残した 4 件が実際に fail し続けることを、当該 binary の
  直接実行で 1 件ずつ確認した (上記 `90 passed; 4 failed`)。
- `crates/lsharp-wasm` の lib test `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap`
  は引き続き `RootSetWithoutActiveSlot { function: "main" }` で落ちる。baseline の
  `[lsharp-wasm]` 行として登録済みの既知 FAIL で、免除対象が出口検査だけである以上
  本変更では動かない (動いていたら緩和が広すぎた証拠になる)。

## Consequences

`main` で root を保持し続ける公開 API の使い方が lowering を通るようになる。
一方これは `SMOKE-GATE-02` の第 1 段であり、`LEGACY-ROOT-01` の解消でも、
`scripts/ci/test-runtime-limits.sh` の緑化でもない。後者は案 B1 まで進んで初めて閉じる。

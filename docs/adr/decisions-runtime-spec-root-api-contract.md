# ADR: root 管理 API の契約を二段 tier で定める

- Status: Accepted (verified slice)
- Date: 2026-08-18
- Scope: `RUNTIME-SPEC-01` / `I-17` / `docs/language/runtime-spec.md`
  (`RUNTIME-SPEC-01` は本 ADR で閉じ、native 側の残件は `NATIVE-ROOT-01` / `I-21` へ移した)
- Related: [`ISSUES.md` I-17](../../ISSUES.md#i-17)、
  [`decisions-root-lifetime-intentional-imbalance-annotation.md`](decisions-root-lifetime-intentional-imbalance-annotation.md)、
  [runtime spec](../language/runtime-spec.md)、[native backend spec](../language/native-backend-spec.md)

## Context

[runtime spec](../language/runtime-spec.md) は `root_push` / `root_pop` / `root_set` を
公開 API として挙げるが、定めているのは**役割の一行説明と、空 root stack への `root_pop` の
境界挙動 1 項目だけ**である。`root_push` / `root_set` の戻り値、root stack の容量上限、
`root_set` 失敗時の観測可能性は未定義のまま実装だけが先行している (`I-17`)。

契約が薄いこと自体より、**契約が実装より薄いまま backend が増えている**ことが問題である。
本スライスの調査で、wasm backend と native backend の root API は既に**挙動が食い違っている**
ことが分かった (`I-21`)。仕様が沈黙している以上、どちらが正しいとも言えない状態が続く。

## Decision

**契約を二段 tier に分ける。tier 1 は全 backend が満たすべき言語レベルの契約、
tier 2 は wasm backend が現に満たしている観測可能性の契約とし、backend 任意とする。**

### tier 1 — 全 backend 必須

1. **`root_push` は追加した slot の index を返す。** 返る値は push 前の stack 高さに等しく、
   同じ関数内で `root_set` の slot 引数として使える。
2. **`root_set` は書き込んだ slot の index を返す。**
3. **空の root stack に対する `root_pop` は trap せず、stack を変更せずに `0` を返す。**
   (既に spec にある 1 項目。tier 1 へ移すだけで内容は変えない)
4. **root stack の容量は動的で、固定上限を定めない。** 容量を確保できなくなった時点で trap する。

### tier 2 — 観測可能性 (backend 任意)

5. **有効でない slot への `root_set` は trap する。**
6. **失敗の記録 (failure ledger) を持つ backend は、`failure_count` を失敗回数として増やす。**
   付随する `failure_slot` / `failure_top` は **`failure_count > 0` のときにのみ意味を持つ**。

### なぜ tier を分けるか

native backend の実測 (`I-21`) が、tier 1 の項目 3 すら満たしていない実装の存在を示している。
一段の契約にすると、**spec を書いた瞬間に既存 backend が違反状態になる**か、
違反を避けるために契約を実装の最小公倍数まで薄めるかの二択になる。
tier を分ければ「言語として要求すること」と「wasm backend が現に提供していること」を
別々に書けるので、どちらも薄めずに済む。

### 契約に書かないもの

**実装の数値と手順は契約に含めない。** 初期容量 32768 slot、容量が尽きたときの倍々拡張、
拡張時に旧テーブルを解放せず捨てていること、新テーブルを `max(heap_ptr, memory_end)` へ
8 byte 境界で置くこと — これらはいずれも wasm backend の**実装事実**であって、
backend を跨ぐ契約ではない。契約は項目 4 の「動的・固定上限なし・枯渇したら trap」までとし、
数値は e2e test が pin し続ける。

## 却下した選択肢

**案 A — 全項目を全 backend 必須の単一契約として書く。却下。**
`I-21` のとおり aarch64 native の `emit-root-pop-aarch64` は空 stack ガードを持たず、
`sub x27, x27, #8` を無条件に実行してから load する。**spec に既にある項目 3 に違反している**。
x86-64 に至っては `root_push` が `xor eax, eax` で引数を捨てて常に 0 を返し、
`root_pop` は emitter そのものが無い。単一契約にすると spec を書いた時点で
native lane 全体が違反になり、しかも本スライスは native の実装を範囲に含めていない
(`TODO.md` の `RUNTIME-SPEC-01` が「含めない範囲」に明記)。**書けない契約を書かない。**

**案 B — 未定義のまま残す。却下。**
これが `I-17` そのものである。未定義のまま backend が 2 つに増えた結果が `I-21` の食い違いで、
放置しても差は広がるだけである。

**案 C — failure ledger の無条件書き込みをそのまま契約へ昇格する。却下。**
実装は `failure_slot` / `failure_top` を **bounds check の前に、成功・失敗を問わず毎回**
`GlobalSet` している。これを契約にすると「直前の呼び出しの引数が読める」ことを
全 backend へ要求することになるが、それは診断のための実装都合であって言語の契約ではない。
実装との整合点は「trap 直後に読めば失敗した呼び出しの値が入っている」ことなので、
契約は項目 6 の形 (`failure_count > 0` のときにのみ意味を持つ) にとどめる。
**無条件書き込みという実装の性質は `I-17` 本文に記録として残す。**

**案 D — `root_push` の戻り値を「push 後の stack 高さ」と定める。却下。**
実装は push 前の top を返す。`root_set` の slot 引数としてそのまま渡せる値であることが
API として意味を持つので、実装のほうが正しい。契約を実装に合わせる。

## 台帳の訂正を含める

`I-17` 本文が持っていた次の 2 つの記述は、本スライスの実測で不正確だと分かった。
**doc-GREEN で訂正する。**

1. 「`root_set` の失敗 → failure ledger へ記録してから trap する」 —
   記録は失敗時に限らない (案 C の却下理由)。
2. 「native backend は現時点で `lsharp_root_pop` を実装していない」 —
   **シンボルとしては真だが、挙動としては偽**。aarch64 は IR opcode 74/75/76 を
   インライン展開して実 root stack を持つ。詳細は `I-21` へ切り出す。

## 実装順序 (doc-RED 時点の計画)

1. `I-21` を起票する (native backend の root API 非適合)。
2. `runtime-spec.md` の 内部管理 API 節を tier 1 / tier 2 へ書き換える。
3. 凍結中の lib test `test_root_set_invalid_slot_records_failure_ledger_before_trap` の
   fixture へ `:roots-unbalanced` を足して解凍し、tier 2 の項目 5/6 を pin する。
   **assertion は触らない。**
4. tier 1 の項目 1 と 4 は既存 e2e が pin しているので、doc-GREEN で名指しする。新規 test は作らない。
5. 解凍が成功したら baseline から該当行を削除し、クラスタ見出しの件数も更新する。

## Evidence

すべて 2026-08-18、worktree `codex/spec-and-parser-parity` (base `2c2a50e4`) での実測。

### tier 1 を pin する test

**新規 test は作っていない。** 既存の e2e 1 本が項目 1/2/3 を同時に pin していることを確認した。

`crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs:1021`
`test_e2e_root_runtime_api_tracks_slots_and_values` は
`root_push 111` / `root_push 222` / `root_set slot0 333` / `root_pop` ×3 の出力を
`["0", "1", "0", "222", "333", "0"]` に固定している。

| 出力 | 意味 | 契約 |
|---|---|---|
| `0` / `1` | `root_push` が push 前の top を返す | tier 1 項目 1 |
| `0` | `root_set` が書き込んだ slot を返す | tier 1 項目 2 |
| `222` / `333` | pop が値を返す | -- |
| 末尾の `0` | 空 stack への `root_pop` | tier 1 項目 3 |

項目 4 (容量が動的) は
`test_e2e_runtime_root_stack_grows_past_initial_capacity` (`:534`) が pin している。
**実行結果**: 2 本とも `ok` (`2 passed; 0 failed`)。

### tier 2 を pin する test — 凍結の解凍

`crates/lsharp-wasm/src/wasi_tests/core.rs` の
`test_root_set_invalid_slot_records_failure_ledger_before_trap` は
`workspace-expected-failures.txt` に登録された既知 FAIL で、**一度も緑になったことがなかった**。
原因は assertion ではなく fixture で、`(defn main [] (root_set 0 42))` が
root lifetime verifier の `RootSetWithoutActiveSlot { function: "main" }` で
lowering 段に止められ、assertion まで到達していなかった。

fixture のソースへ `:roots-unbalanced "<理由>"` を注釈して免除した
(`root_lifetime.rs` の `validate_function` が抽象実行ごと早期 return する)。
**assertion は 1 文字も変えていない。**

実行結果は `1 passed`。当初の懸念だった `error.contains("<wasm function 24>")` も、
`failure_slot == 0` / `failure_top == 0` / `failure_count == 1` も**そのまま成立した**。
つまりこの test が書かれた当時の期待値は今も正しく、凍結していたのは fixture 側の問題だけだった。

- `cargo test -p lsharp-wasm --lib` → **`137 passed; 0 failed`**。
- baseline `workspace-expected-failures.txt` の entry 数 **90 → 89**。
  非 e2e クラスタの見出しも `37 FAIL` → `36 FAIL` へ更新し、削除の理由と日付を注記した。

### 契約の根拠にした実装 (一次ソース)

| 契約 | 実装 |
|---|---|
| tier 1 項目 1 | `wasi/root.rs:119-128` — store の後 `LocalGet(TOP_LOCAL)` (push 前の値) を `I64ExtendI32U` |
| tier 1 項目 3 | `wasi/root.rs` `emit_root_pop_func` — `I32Eqz` で分岐し `I64Const(0)` |
| tier 1 項目 4 | `wasi.rs:83` `ROOT_STACK_SLOT_CAPACITY: i32 = 32768`、`root.rs:40-42` の `I32Const(2)` / `I32Mul` で倍々、`memory.grow` 失敗で `Unreachable` |
| tier 2 項目 6 | `wasi/root.rs:190-224` — `failure_slot` / `failure_top` は `I32GeU` の**前**に無条件 `GlobalSet`、`failure_count` だけが失敗分岐の中 |
| 非適合 (`I-21`) | `selfhost/src/Backend/Native/NativeCodegen.ls` の `emit-root-pop-aarch64` (空 stack ガード無し) / `emit-root-push-x86` (`xor eax, eax`) |

### 満たせなかった受入条件

- **native backend を tier 1 準拠にしていない。** `TODO.md` の `RUNTIME-SPEC-01` が
  「含めない範囲」に明記した想定どおりの結果で、非適合は `I-21` として起票した。
  spec の準拠状況表にも違反として明記してある。**契約を書いた結果、違反が可視化された状態**であり、
  これは隠さずそのまま残す。
- **`scripts/ci/check-workspace-baseline.sh` は再実行していない。** 同 script は workspace 全体の
  nextest 実測 (5 時間級) を入力に取る。代わりに、削除した 1 件が実際に pass へ転じたことを
  当該 binary の直接実行で確認した (`137 passed; 0 failed`)。
- **tier 2 は backend 任意にした。** 全 backend 必須にできなかったのは案 A の却下理由のとおり。

## Consequences

root API の契約が実装に追いつく。一方 tier 2 を backend 任意にしたことで、
**native lane は契約を満たさないまま spec 準拠を名乗れる**状態が残る。
これは意図した妥協で、native の実装を範囲外に置いた以上の帰結である。
tier 1 の項目 3 に対する native の違反は `I-21` が正本として持ち続ける。

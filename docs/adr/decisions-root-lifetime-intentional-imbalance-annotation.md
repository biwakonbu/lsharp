# ADR: 意図的な root 不均衡の明示注釈 (`:roots-unbalanced`)

- Status: Proposed (doc-RED)
- Date: 2026-08-18
- Scope: `SMOKE-GATE-02` / `I-14` / `I-17` / `LEGACY-ROOT-01` /
  `crates/lsharp-ir/src/root_lifetime.rs` / `crates/lsharp-syntax/src/parser/metadata.rs`
- Related: [`ISSUES.md` I-14](../../ISSUES.md#i-14)、[`ISSUES.md` I-17](../../ISSUES.md#i-17)、
  [`main` exit 免除 ADR](decisions-root-lifetime-main-exit-exemption.md)、
  [runtime spec](../language/runtime-spec.md)

## Context

[`main` exit 免除 ADR](decisions-root-lifetime-main-exit-exemption.md) は
`runtime_allocator_closures` の恒常 FAIL 17 件のうち 13 件を解消し、残り 4 件を
「意図的な不均衡を IR へ伝える明示的な注釈が要る」として本 ADR へ送った。

残る 4 件は 2 つのクラスに割れる。

| test | error | 形 |
|---|---|---|
| `..._root_runtime_api_tracks_slots_and_values` | `RootPopUnderflow { function: "main", instruction_index: 14 }` | push 2 回 / pop **3 回**。3 回目の pop が返す `0` が assertion そのもの |
| `..._runtime_object_table_grows_past_initial_capacity` | `BranchDepthMismatch { function: "alloc-rooted" }` | `if` の then=0 / else=1 で合流 |
| `..._runtime_root_stack_grows_past_initial_capacity` | `BranchDepthMismatch { function: "push-roots" }` | 同上 |
| `..._runtime_root_stack_growth_preserves_root_api` | `BranchDepthMismatch { function: "push-roots" }` | 同上 |

この 2 クラスは、verifier が既に持っている 2 つの**関数名ハードコード**と正確に対応する。

- `ROOT_LEASE_ACQUIRE_HELPER` (`typeinfer-builtin-root-value`) — slot を積んだまま返り、
  呼び出し側が解放する。`BranchDepthMismatch` / `ImbalancedExit` と同じ「出口が非 0」の形。
- `ROOT_LEASE_RELEASE_HELPER` (`typeinfer-builtin-release-roots`) — 自分が積んでいない slot を
  pop する。`RootPopUnderflow` と同じ形。**この分岐は抽象実行そのものを丸ごと省略している。**

つまり必要なのは新概念ではなく、**既にハードコードされている暗黙の許可を、
ソース側から宣言できる形へ一般化すること**である。

`RootPopUnderflow` については、もう一つ前提の確認が要る。
`main` exit 免除の健全性根拠は「spec が均衡を要求していない」ことだったが、
**runtime spec は `root_pop` を空 stack に対して呼んだときの挙動を定義していない**
(`docs/language/runtime-spec.md:78-80,84,142` は一行の役割表と no-op 互換の許容だけ)。
実装は定義している — `crates/lsharp-wasm/src/wasi/root.rs:145-152` の `emit_root_pop_func` は
`top == 0` を明示的に分岐し、top を動かさずに `0` を返す。
落ちている test はこの `0` を期待している。**契約が実装より薄い**状態である。

## Decision

### 1. spec を実装に追いつかせる (最小)

`root_pop` を空の root stack に対して呼んだ場合、root stack を変更せず `0` を返す、と
runtime spec に一行足す。新しい挙動を決めるのではなく、`wasi/root.rs` が既に持っている
挙動を契約へ引き上げるだけである。

これにより `RootPopUnderflow` は「**spec 上は合法だが、既定では拒否する厳しめの検査**」となり、
`main` exit 免除 ADR と同じ論法の上に乗る。検査を消すのではなく、注釈で個別に外す。

**root API 契約の薄さ全体 (`root_push` / `root_set` の戻り値、slot 上限、失敗時の観測可能性) は
本 ADR では埋めない。** 気づいた事実として `I-17` に登録し、そこを正本とする。

### 2. `:roots-unbalanced "<理由>"` を `defn` の metadata directive として追加する

```lisp
(defn push-roots [n]
  :roots-unbalanced "root stack の grow を確認するため、意図的に slot を積み増したまま返る"
  (if (<= n 0)
    0
    (do (root_push n) (push-roots (- n 1)))))
```

- `Metadata` の**素の field** (`roots_unbalanced: Option<String>`) として持つ。
  `MetadataFormKind` の variant は足さない。先例は `transitions` (`ast.rs:105-106`)。
- 理由文字列は**必須**にする。注釈が「検査を黙らせる呪文」ではなく
  **レビュー可能な宣言**であることが、この設計の唯一の存在理由だからである。
- payload を string にしたのは parser 都合でもある。metadata ループは `Colon + Symbol` で
  再入するため、`:roots :unchecked` のように payload を keyword にすると
  payload 側が次の directive として読まれる。`:doc` と同じ string payload にすればこの罠は無い。
- 同期点は **2 箇所**ある。`try_parse_metadata` の `match` と
  `decl.rs:688` の directive allowlist。片方だけだと directive が型注釈として誤読される。

### 3. 免除は IR の struct field ではなく、検証の第 2 引数で渡す

```rust
pub struct RootLifetimeExemptions { functions: HashSet<String> }
pub fn validate_module(module: &Module, exemptions: &RootLifetimeExemptions) -> Result<(), _>
```

集合は `lower/program.rs:223` の**唯一の production 呼び出し点**で AST から組み立てる。
そこには `program` がまだ scope にあるので、IR を経由する必要が無い。

- IR の `Function` / `Module` に field を足さない。literal 構築点は `Function {` **107 箇所** /
  `Module {` **83 箇所**あり、どちらも `Default` を derive していないので不釣り合いに侵襲的である。
- field を足さないので **`dump()` 出力が変わらず、insta snapshot が 1 件も動かない**。
  本リポジトリには未レビューの snapshot 差分が 14 件溜まっており、これを増やさない意味は大きい。
- test 呼び出し点 (`lower/tests.rs:26`、`lower/tests/wasm_gc_and_roots.rs:406,438`) は
  空集合を渡す。既存の負テストの意味は変わらない。

**代償**: 免除は IR に載らないので、serialize した `Module` を後段で再検証する consumer には
届かない。現時点で検証点は lowering 1 箇所だけなので許容する。検証点が増えたら再考する。

### 4. 注釈された関数は抽象実行を丸ごと省略する (粗い免除)

`validate_root_lease_release` が既に取っている形をそのまま使う。
`StaleSlot` だけ残すような細粒度の免除は採らない (理由は下記)。

## 却下した選択肢

**案 P — compiler 生成の root 操作と、ユーザーが書いた root 操作を区別する (provenance)。却下。**
「ユーザーが書いた root 操作は検査しない」という方向は、一見すると注釈より自動的で良く見える。
しかし `f8234503` がこの verifier で捕まえた 4 件は、いずれも
`selfhost/src/Backend/Wasm/Compiler.ls` の**手書き `(root_pop)` の個数バグ**である。
ユーザー由来を免除すると、**この verifier が過去に実証した唯一の価値をそのまま捨てる**ことになる。
向きが逆である。

**案 F — IR の `Function` / `Module` に免除 field を足す。却下。**
literal 構築点 107 / 83 箇所の全書き換えを要求する。`Default` が無いので機械的にも重い。
さらに `dump()` 出力が変わると insta snapshot が動き、未レビュー分 14 件の上に差分を積む。
得られるものは「免除が IR に載る」ことだけで、検証点が 1 箇所しか無い現状では釣り合わない。

**案 N — 既存の lease helper と同じく、関数名の allowlist を verifier 側へ足す。却下。**
実装は最小 (定数を 4 つ足すだけ) だが、**ソースを読んでも免除されていることが分からない**。
既存の 2 件が既にこの問題を持っており、それを 6 件へ増やす方向は逆行である。
本 ADR はむしろ既存 2 件を将来この注釈へ寄せる下地を作る。

**案 G — 免除を細粒度にする (出口検査と branch 合流だけ外し、`StaleSlot` は残す)。却下。**
一見きめ細かいが、branch の深さが揃わない状態から先の `roots` ベクタは意味を失っており、
そこで残す `StaleSlot` / `RootSetWithoutActiveSlot` は半盲の検査にしかならない。
「揃わない深さをどう合流させるか」という新しい意味論を発明する必要もある。
既存の `validate_root_lease_release` は同じ状況で抽象実行ごと省略しており、その先例に合わせる。

**案 B2 — `LS3003` を warning へ格下げする。却下 (前 ADR から継続)。**
本物のバグも同時に warning へ落ちるため。

### 「fixture を書き換える」は前 ADR の却下案 C の再来ではない

前 ADR は **案 C (fixture を均衡する形へ書き換える)** を「テストが検証しようとしている状況が
消える」ため却下した。本 ADR で fixture に触るのはこれとは別物である。

- assertion は **1 バイトも変えない**。期待値も、root 操作の回数も、制御構造も変えない。
- 足すのは「この不均衡は意図である」という**宣言 1 行**だけで、実行される L# の意味は変わらない。
- 結果として、注釈された 4 fixture はそのまま**注釈機能自体の正の e2e テスト**になる。

前者は検証対象を壊す変更、後者は検証対象に説明を付ける変更で、向きが違う。

## 含めない範囲

- **既存の lease helper 2 件 (`typeinfer-builtin-root-value` / `typeinfer-builtin-release-roots`) を
  本注釈へ移行すること。** これらは selfhost source に居るため、編集すると embedded component の
  再ビルドと cache key の再計算 (`I-16` で安定化させたばかり) を巻き込む。将来作業として残す。
- **root API 契約の全面的な補完** — `I-17` が正本。
- `crates/lsharp-wasm` lib の `test_root_set_invalid_slot_records_failure_ledger_before_trap`
  (`RootSetWithoutActiveSlot`) — 別要因の既知 FAIL。

## 受入条件

1. 注釈あり / なしを対にした test が GREEN (注釈なしの不均衡は**引き続き拒否される**こと)。
2. `scripts/ci/test-runtime-limits.sh` が rc=0 (現在 rc=101)。
3. `docs/development/validation/workspace-expected-failures.txt` から残り 4 件を削除
   (95 → 91 entry、e2e クラスタ見出し 57 → 53)。

## 実装順序 (doc-RED 時点の計画)

1. RED: parser が `:roots-unbalanced "..."` を受理し `Metadata` に載せる test。
2. RED: 注釈された関数の不均衡が `validate_module` を通る test。
3. RED: **注釈の無い**同形の関数は引き続き拒否される test (緩和が広がらない保証)。
4. RED: 注釈が**同じ module の別関数へ漏れない** test。
5. GREEN: parser 2 箇所 → `RootLifetimeExemptions` → `validate_function` の順に実装。
6. 4 fixture へ注釈を足し、e2e 4 件と gate script を緑化。
7. baseline から 4 行削除。

## Evidence

(実装後に埋める)

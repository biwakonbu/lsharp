# ADR: 深い occur-check を診断可能にする (2026-08-22)

- **Status**: accepted
- **Date**: 2026-08-22
- **Scope**: `crates/lsharp-types` の `infer_expr` 再帰入口、`Substitution::compose`、`Expr::App` の環境更新
- **Related**: `LEGACY-TEST-01` / `WORKTREE-ABSORB-02` /
  [`decisions-worktree-absorption-2026-08-20.md`](decisions-worktree-absorption-2026-08-20.md) /
  `docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md`

## Context

`codex/legacy-test-01-occur-check` の `59c7dbba` (`perf(types): bound deep occur-check inference`)
が未取り込みだった。main は `crates/lsharp-types/tests/infer_limits.rs` で

- nested type annotation depth 32/64/128
- wide record 128/256 fields
- 浅い self-application `(defn omega [f] (f f))` の LS1003

までは押さえているが、**入れ子の深い式を通した self-application** を測っていない。

実測 (2026-08-22, main `3afa5ba7`):

```
(defn occur [value] (value (ref-new (ref-new ... value))))   # depth 32 / 64 / 128
=> thread has overflowed its stack / fatal runtime error: stack overflow, aborting (SIGABRT)
```

診断 (LS1003) を返す前に process ごと abort する。**型エラーが型エラーとして観測できない。**

parser 側は既に同じ問題を segmented stack で解いている
(`crates/lsharp-syntax/src/parser/{expr,pattern,type_expr}.rs` が `stacker::maybe_grow`)。
推論側だけが素の再帰のままだった。

## Decision

### 採る: `infer_expr` の再帰入口を segmented stack で包む

`infer_expr` を薄い wrapper にし、本体を `infer_expr_inner` へ移して
`stacker::maybe_grow(RED_ZONE, SEGMENT_SIZE, ...)` で包む。parser と同じ方式だが、
**推論の frame は parser より大きい**ので red zone / segment を大きく取る。

| | red zone | segment |
|---|---|---|
| parser (`parse_expr` 他) | 64 KiB | 1 MiB |
| `infer_expr` (本 ADR) | 256 KiB | 2 MiB |

浅い式では `maybe_grow` は残量チェックだけで通常経路のまま走る。深い式だけが segment を掘る。

### 採る: `Substitution::compose` の空 fast path

`compose` は両辺を走査して新しい `BTreeMap` を作る。片方が空なら結果はもう片方そのものなので、
`clone()` を返して走査を省く。深い再帰では各 frame が `compose` を呼ぶため、
frame あたりの仕事量がそのまま深さ方向に効く。

### 採る: `Expr::App` で不要な環境更新を省く

従来は引数ごとに `current_env.apply_subst(&s)` を必ず実行していた。

- `subst` が空なら `env` をそのまま使い、`TypeEnv` の複製を作らない
- **最後の引数の後**の `apply_subst` は誰も読まないので実行しない

`apply_subst` は環境全体を走査するので、引数が多い呼び出しほど効く。

**この最適化は意味論を変えてはならない。** 先行引数が確定させた substitution が後続引数の
環境へ伝わらなくなると、`(pair (f 1) (f true))` が型エラーにならなくなる。
そのため最適化より先に `multi_argument_application_propagates_prior_argument_substitution`
を pin した (LS1004 を要求する)。

### 却下: 深さ上限を設けて明示エラーを返す

「depth N を超えたら専用の診断」を返す案。却下する。

- 上限値に根拠が無い。64 で落ちるのは stack 消費量の都合で、言語の契約ではない
- 正しい答え (LS1003) を返せる入力に対して別のエラーを返すことになる
- 上限を跨ぐ入力が「前は通ったのに」と壊れる。segmented stack にはこの断絶が無い

### 却下: `59c7dbba` の test / fixture をそのまま取り込む

branch は `tests/inference_limits.rs` + `tests/support/inference_limit_fixture.rs` +
`benches/inference_limits.rs` を新設し、nested type / wide record / occur-check を
共通 fixture へ寄せる。却下する。

main は既に `tests/infer_limits.rs` と `benches/infer_limits.rs` を持ち、nested type と
wide record を**別の書き方で**押さえている。branch 版を入れると同じ契約が 2 箇所に増える。
**main に無い契約は occur-check の深さと多引数 substitution 伝播の 2 つだけ**なので、
それだけを現行ファイルの上へ書き直す。`84ca54fd` を
`metadata_contract_generation.rs` へ寄せたときと同じ扱いである。

## Evidence

すべて 2026-08-22、main `3afa5ba7` の上で実測。

### RED → GREEN

`crates/lsharp-types/tests/infer_limits.rs` に 2 test を追加してから実装した。

| | 結果 |
|---|---|
| RED (`cargo test -p lsharp-types --test infer_limits`) | `occur_check_reports_infinite_type_at_documented_depths` で **`fatal runtime error: stack overflow, aborting` (SIGABRT)**。他 4 件は pass |
| GREEN (同上) | **5 passed / 0 failed**, 0.19s |

RED は assert 失敗ではなく **process abort** である。診断を返す前に落ちるという問題の形が
そのまま出ている。

`multi_argument_application_propagates_prior_argument_substitution` は RED 時点でも pass する。
これは修正対象ではなく、`Expr::App` の環境更新を絞る変更が意味論を壊さないことを押さえる pin である。

### crate 全体

`cargo test -p lsharp-types` — **222 + 各 integration binary すべて 0 failed**
(最大は unit 222、次いで `metadata_contract` 62、`review_*` 群 29/30 ほか)。

### workspace

`cargo test --workspace --lib --bins` — 失敗は
`lsharp-lsp util::tests::incremental_module_parse_diagnostics_forward_stable_code` **1 件のみ**。
これは `docs/development/validation/workspace-expected-failures.txt:133` に登録済みの
既知 FAIL で、**新規 FAIL は 0 件**。合格判定は `0 failed` ではなく
`FAILED 集合 ⊆ 台帳` である。

### 満たせなかった検証

- **`cargo clippy -p lsharp-types -- -D warnings` は通していない。** main の時点で
  `review_trust_store.rs:120` の `collapsible_if` が 3 経路で compile error になるため、
  この crate では clippy を gate として使えない。`git stash` した状態でも同じ 3 件が出ることを
  確認しており、本変更が持ち込んだものではない。事実を `I-31` / `LINT-CLIPPY-01` として登録した。
- **`cargo fmt -p lsharp-types -- --check` は `benches/infer_limits.rs` の import 順で 1 件差分が出る。**
  本 slice が触っていないファイルの既存差分で、本変更のファイルには差分が無い。
- **e2e (`lsharp-wasm`) は回していない。** 5 時間規模のため本 slice の範囲外とした。
  `Expr::App` の環境更新を絞る変更は推論結果に影響しうるので、次の広域回帰で確認する必要がある。

### 未検証の境界

- selfhost / native 側の同等の深さ境界。`selfhost/src/Types/TypeInfer.ls` は
  segmented stack を持たないので、同じ入力で同じ問題が残っている可能性が高い。未測定。
- `INFER_STACK_SEGMENT_SIZE` の 2 MiB という値に測定上の根拠は無い。parser の 1 MiB に対して
  推論 frame が大きいことを見込んだ倍取りである。depth 128 で足りることだけが実測で言える。

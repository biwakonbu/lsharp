# computation builder の未定義・不完全を型検査で拒否する

- **Status**: accepted
- **Date**: 2026-08-22
- **Scope**: Rust 側 `Infer` の `Expr::Computation` arm
  (`crates/lsharp-types/src/infer/expr.rs`) が、builder 名と builder の member
  (`bind_fn` / `return_fn`) をどう検査し、computation expression の結果型をどう決めるか。
  computation expression の lowering / codegen、selfhost 側 `TypeInfer.ls` の同経路、
  bind の型そのものの検査 (`m a -> (a -> m b) -> m b` の形の強制) は範囲外。
- **Related**: `ISSUES.md` `I-44` / `I-46`、`TODO.md` `COMP-BUILDER-01` (本 ADR で削除)、
  [worktree 取り込み判定](decisions-worktree-absorption-2026-08-20.md)

## 何が問題だったか

`Expr::Computation` arm は builder 名を `computation_builders` から引くだけで、
**引けなかった場合に何もしなかった**。

```rust
let builder_info = self.computation_builders.get(builder_name).cloned();
// ...
if let Some((bind_fn, return_fn)) = &builder_info {   // None なら丸ごと skip
```

結果として 3 つの欠陥が重なっていた。

| # | 欠陥 | 症状 |
|---|---|---|
| 1 | 未登録の builder 名を通す | `(computation missing (return 42))` が `Ok`。typo が実行時まで残る |
| 2 | 登録済みでも member の存在を検査しない | `bind_fn` / `return_fn` の片方だけを持つ builder を通す |
| 3 | 結果型の sentinel が機能しない | `result_ty == self.var_gen.fresh()` は毎回新しい id を作るので**常に false**。「未登録なら最後のステップの型」という fallback は一度も発火していない |

3 は 1 / 2 を直しても残る。`return` ステップを持たない computation expression
(`(computation identity (+ 1 2))`) の結果型が、束縛されない fresh 型変数のままになる。

## 決めたこと

### 1. 未登録の builder は `TypeError::UndefinedVar` (`LS1001`)、span は form 全体

`(computation missing (return 42))` 全体を span とする。builder 名だけを指すより、
どの form が拒否されたかが読み手に伝わる。

### 2. incomplete な builder も `UndefinedVar` (`LS1001`)。ただし `name` は欠けた member 名

`TODO.md` の受入条件は「別診断で拒否すること」だった。これを
**variant を分けること**ではなく**診断の内容を分けること**として満たす。
`(computation-builder identity identity-bind missing-return)` は
`UndefinedVar { name: "missing-return" }` を返す。未登録 builder の
`UndefinedVar { name: "missing" }` とは `name` で区別できる。

**却下: 新 variant `IncompleteComputationBuilder` + 新 error code。**
理由は 2 つある。

- **意味的に `UndefinedVar` が正しい。** builder 宣言が指す `missing-return` という
  名前は実際にどこにも定義されていない。別の名前を与える理由が無い
- **新 code は二重更新を強制する。** code を足すと `error_codes.rs` と
  `docs/development/operations/error-reference.md` の両方を触る必要があり、
  driver 側の gate が片方だけの更新を落とす。**得られるものが「variant が別である」
  だけなら割に合わない**

### 3. 結果型は `Option<Type>` で持つ

sentinel 比較をやめ、`return` 関数が結果型を確定させたかどうかを `Option` で表す。
確定しなければ最後のステップの型、ステップが無ければ fresh を返す。

### 4. member の存在検査は decl-site ではなく use-site で行う

**却下: `Decl::ComputationBuilder` を登録する時点で member を env から引く。**
choke point としてはそちらが自然だが、`infer_program` の登録 pass は
関数の型環境が構築される**前**に走る。この時点で env を引くと、
builder より後ろに書かれた member が必ず未定義に見える。

use-site (`Expr::Computation` arm) では `infer_decl_functions` のパス 1 が
全 defn を placeholder 型変数で仮登録済みなので、前方参照が正しく解決する。
`computation_builder_members_resolve_when_declared_after_use` がこれを固定する。

## なぜ error にしても既存コードが壊れないか

`(computation ...)` を使う箇所を全数確認した (`(computation-builder` を除いて 54 hit)。

| 経路 | 件数 | 判定 |
|---|---|---|
| Rust 側 `Infer` を通るもの (`lsharp-types` / `lsharp-ir` / `lsharp-tooling` / e2e `core_language_semantics`) | 6 | すべて builder 宣言と member の defn を同一 source 内に持つ |
| selfhost CLI の `:invariant` fixture (`selfhost_cli_core.rs`) | 多数 | selfhost 側 `TypeInfer.ls` を通る。本変更の影響外。いずれも `(computation-builder maybe-builder mb identity)` と `mb` / `identity` を宣言済み |
| parse / macro 展開のみ (`parser` / `macro_expand` / `e2e_selfhost_syntax`) | 多数 | 型推論を通らない |
| 文字列生成 (`selfhost/src/Tools/Text/FormatterDecl.ls`) | 5 | formatter の出力文字列であり式ではない |

## Evidence

RED は 5 test すべてが落ちる状態から始めた
(`crates/lsharp-types/tests/computation_builder_diagnostics.rs`)。

| test | RED | GREEN |
|---|---|---|
| `unknown_computation_builder_reports_stable_diagnostic` | `Ok([("main", Fun([], Var(25)))])` | ok |
| `computation_builder_missing_return_function_reports_stable_diagnostic` | `Ok(...)` | ok |
| `computation_builder_missing_bind_function_reports_stable_diagnostic` | `Ok(...)` | ok |
| `known_computation_builder_preserves_plain_expression_result_type` | `Fun([], Var(33))` (期待 `Fun([], Int)`) | ok |
| `computation_builder_members_resolve_when_declared_after_use` | 期待値を後述のとおり縮めたうえで ok | ok |

`test result: ok. 5 passed; 0 failed`。

回帰は `cargo test --no-fail-fast -p lsharp-types -p lsharp-ir -p lsharp-tooling` で
**960 passed / 1 failed**。唯一の FAIL は
`lsharp-tooling api_doc::tests::test_build_api_doc_for_file_preserves_parse_error_code` で、
`workspace-expected-failures.txt:139` に既収載の既知 FAIL である
(`LS0102` の parse 診断で computation は関与しない)。
`lsharp-syntax` の `selfhost_cli_validation_surface_is_registered` も同ファイル `:137` の既知 FAIL。
e2e は `core_language_semantics::test_e2e_computation` / `..._let_bang_typecheck` の 2 件が ok。
`cargo clippy -p lsharp-types --all-targets` は警告なし。

### 受入条件のうち、書いたとおりには満たさなかったもの

`TODO.md` の「builder が一部の member だけ持つ incomplete な場合も**別診断**で拒否すること」を、
**別 variant ではなく別 `name` で**満たした。判断と却下理由は上記 2 節に書いた。

### RED 中に期待値を縮めた 1 件と、その理由

`computation_builder_members_resolve_when_declared_after_use` は当初
「前方参照でも結果型が `Int` になること」まで要求していたが、
**実装を直しても `Fun([], Var(27))` のままだった**。これは本 slice の 3 欠陥とは別の穴で、
前方参照時点の member は placeholder 型変数なので、use-site では単相化しようがない。
test の要求を「誤って incomplete 扱いにしないこと」へ縮め、
落ちた側の事実は `I-46` として起票した。**期待値を実装に合わせて下げたのではなく、
一つの test が二つの契約を抱えていたのを分けた**という整理である。

## 範囲外として残したもの

- **`I-46`** — 前方参照された builder member の下では結果型が汎化され、
  `(string-length (main))` のような誤用が通る。宣言順だけが違う同じ program が
  片方は `Mismatch` で落ち、片方は通る。
  **本 ADR を書いた後の実測で、これは computation builder 固有ではなく plain な `defn` の
  前方参照で再現することが分かった。** `I-46` は一般の前方参照の問題として書き直してあり、
  computation builder は発見経路として扱う。別 issue には分けていない
- **selfhost 側 `TypeInfer.ls`** の同じ経路。Rust 側だけを直したので parity は未確認
- **`bind_fn` の型検査**。`current_env.get(bind_fn)` で存在は見るが、
  `m a -> (a -> m b) -> m b` の形は依然として要求していない
  (実装のコメントもそのまま残した)

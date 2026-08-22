# ADR: selfhost の 0 引数 `defn` を `Unit -> body` として登録する (2026-08-22)

- **Status**: accepted
- **Date**: 2026-08-22
- **Scope**: `selfhost/src/Types/TypeInfer.ls` の `infer-defn-predeclared` (param-count 0 の分岐)
- **Related**: `ISSUES.md` `I-45` / `I-49` / `TODO.md` `CASE-ZERO-ARITY-01` /
  [`decisions-worktree-absorption-2026-08-20.md`](decisions-worktree-absorption-2026-08-20.md) の群 3

## Context

`:case [(expect (zero) 1)]` のように **引数を取らない `defn`** を canonical `:case` の
`expect` 内で呼ぶと、`cases:0` / `executed:0` のまま `status:"fail"` / exit 1 になっていた。
期待値が合っているか外れているかに関係なく同じ結果になるため、`:example` から `:case` への
移行が機械的にできない。

観測は selfhost lane に固有である。`lsharp test` は `--format json` を付けない限り
embedded selfhost component へ委譲され (`crates/lsharp-driver/src/main.rs:1080`)、
Rust 実装 (`--format json` / `LSHARP_DISABLE_EMBEDDED_COMPONENT=1`) は同じソースを
`status=pass cases=1 executed=1` / exit 0 で通す。

原因は evaluator ではなく型推論の内部不整合だった。

| 箇所 | 0 引数をどう扱っていたか |
|---|---|
| `TypeInfer.ls:466-486` `infer-defn-predeclared` | param-count 0 の `defn` を **body の型そのもの** (`zero : Int`) で env へ登録 |
| `TypeInferApply.ls:688-716` `infer-apply-legacy-raw` | argc 0 の apply に **`Unit -> a`** を要求 |
| `TypeInferApply.ls:33-45` `infer-lambda` | param-count 0 の lambda を **`Unit -> body`** で構築 |
| Rust `lsharp-types` | 0 引数 `defn` を `Fun([], Con("Int"))` で保持 |

食い違うのは `infer-defn-predeclared` の 1 箇所だけである。unify が落ちると
`check-case-expectation` (`TypeInferAssertions.ls:1481-1535`) が `infer-expr` の失敗を
一律 `canonical-case-type-error-code` = 1001 へ潰し、`EmbeddedCli.ls:1065-1078` の
preflight が suite 生成前に短絡するので `cases:0` になる。

## Decision

`infer-defn-predeclared` の param-count 0 分岐で `fun-ty = (mk-fun (mk-unit) body-ty)` を作り、
placeholder との unify と `typeinfer-finalize-defn-result-with-env-vars` の両方へ `fun-ty` を渡す。
param-count 1 以上の分岐 (`infer-defn-parameterized-predeclared`) が
`typeinfer-build-curried-fun` の結果を同じ 2 箇所へ渡しているのと形を揃えた。

`typeinfer-defn-return-annotation-subst` へは **unwrapped の `body-ty` のまま**渡す。
これは戻り値注釈 (`:returns` 相当) と body の型を突き合わせる処理であり、
関数型を渡すと注釈側と形が合わなくなる。

## 却下した選択肢

- **apply 側を緩める** (`infer-apply-legacy-raw` の argc 0 で非関数の callee を許す)。
  同じ selfhost の `infer-lambda` が 0 引数 lambda を `Unit -> body` にしているので、
  lambda と `defn` で 0 引数の意味が割れる。Rust 実装 (`Fun([], _)`) からも離れる。
  収束先を 2 つ作る修正であり、`:case` が緑になっても不整合は残る。
- **`check-case-expectation` の 1001 を握り潰す / preflight を `:case` で無効化する。**
  症状 (`cases:0`) は消えるが、型の食い違いはそのまま残り、
  今度は本物の型エラーが preflight をすり抜ける。安全側の壊れ方を危険側へ倒す変更になる。
- **Rust 側を selfhost に合わせる** (0 引数 `defn` を body の型で持つ)。
  Rust lane は既に正しく、`(expect zero 1)` に対して `actual=() -> Int, expected=Int` と
  正確に報告できている。正しい方を壊す向きなので採らない。

## Evidence

RED は `crates/lsharp-driver/tests/metadata_test_selfhost_case_arity.rs` (新規)。
受入条件どおり `lsharp test` の **exit code と `coverage.executed` の両方**を見て、
arity 1 の control を同じ fixture 群に置いた。

| test | 修正前 | 修正後 |
|---|---|---|
| `selfhost_case_zero_arity_actual_side_is_executed_and_passes` | FAIL (`executed=0 code=1001`) | ok |
| `selfhost_case_zero_arity_expected_side_is_executed_and_passes` | FAIL (`executed=0 code=1001`) | ok |
| `selfhost_case_zero_arity_mismatch_is_executed_and_fails` | FAIL (`executed=0`、実行されずに fail) | ok |
| `selfhost_case_arity_one_control_is_executed_and_passes` | ok | ok |
| `selfhost_case_arity_one_mismatch_control_is_executed_and_fails` | ok | ok |

修正前の 3 FAIL / 2 pass は「arity だけが変数」であることを示している。

### 影響範囲の計測

`I-48` の前例 (類似の修正で失敗 defn が 0 → 262 件になった) を踏まえ、着地前に計測した。

非 e2e の 6 crate を `--no-fail-fast` で全 target 走らせた (2026-08-22、修正後)。

```bash
cargo test --no-fail-fast -p lsharp-driver -p lsharp-types -p lsharp-ir \
  -p lsharp-tooling -p lsharp-syntax -p lsharp-lsp
```

**1592 passed / 15 failed。** 失敗 15 件は
`docs/development/validation/workspace-expected-failures.txt` が
この 6 crate について挙げている 15 件と **完全に一致** (`diff` で 0 行差)。
新規 FAIL は 0 件、pass へ転じた expected も 0 件である。

自己適用も確認した。修正後の selfhost から作った embedded component で
selfhost 自身の entry を全モジュールごとコンパイルできる。

```bash
./target/debug/lsharp compile selfhost/src/App/EmbeddedCli.ls -o /tmp/selfapp.wasm
# => コンパイル成功: ... (1211823 bytes) / exit 0 / real 0m55.6s
```

`I-48` の前例で問題になった「修正パッチ下で selfhost 自身が型検査を通らなくなる」形は
本修正では起きていない。

### 計測していない範囲

- **selfhost の自己適用 (stage chain)**。`#[ignore]` lane
  (`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`) と
  `./stage0` を要する bootstrap は本 slice では回していない。
  修正後の selfhost が**自分自身を**コンパイルできるかは未検証である。
- workspace 全体の e2e lane (実測 5h38m)。

### `I-46` / `I-48` との関係

`lsharp compile` は同じ 0 引数呼び出しを含むプログラムを修正前から通していた。
compile 経路は pass-1 の生 placeholder を unify するため矛盾が顕在化せず、
確定した analysis env を見る `:case` preflight だけが露出させていたと見られる。
**本修正は placeholder の穴 (`I-46` / `I-48`、`TypeInfer.ls:485`) を閉じない。**
前方参照経由の呼び出しは従来どおり素通りするので、`INFER-FORWARD-GEN-01` の
`[BLOCKED: I-48]` は本修正では解けない。

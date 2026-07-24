# ADR: 型推論の bounded expression property

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-types/src/infer.rs`
- Related: `imp-07-test-verification-infrastructure.md`, `TODO.md` の `LEGACY-TEST-01`

## Context

型 unification の成否対称性だけでは、parser から `Infer::infer_program` までの式境界で発生する panic を検出できない。型推論全体には
既に多数の fixture があるが、if/let/lambda/application/do/annotation/quote を bounded に組み合わせる property がなかった。

## Decision

- `inference_property_tests::arb_expression_source` は深さ 3、各 collection 最大 2 の小さな expression source を生成する。
- 生成 source は `(defn main [] ...)` に包み、同じ source を syntax parser と `Infer::infer_program` に通す。型が成立しない組み合わせも入力として許容し、
  property の契約は `TypeError` の内容ではなく panic が発生しないことに限定する。
- local property は 64 cases とし、各ケースで新しい `Infer` を作る。これにより state の持ち越しで結果が隠れることを避ける。
- generator と property は `cfg(test)` に閉じ、公開型推論 API、runtime dependency、Wasm output を変更しない。

## Evidence

- RED: property を先に追加し、expression generator 未定義による compile error を確認した。
- GREEN: `cargo test -p lsharp-types bounded_expression_inference_never_panics` が 64 cases で成功した。
- Regression: `cargo test -p lsharp-types -- --nocapture --test-threads=1`（unit 209、integration 46）、`cargo clippy -p lsharp-types --lib --tests -- -D warnings`、
  changed file の rustfmt check、`git diff --check` が成功した。

## Consequences

bounded expression の parser → type inference 境界で panic regression を検出できる。type error の診断 parity、無限型の性能限界、nightly 4096 cases、
AST generator の cross-crate 再利用、GC leak/limit、rooting stress、native stage0 の証跡は未完了であり、`LEGACY-TEST-01` を完了扱いにはしない。

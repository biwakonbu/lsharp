# ADR: 型推論 unify の bounded property symmetry

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-types/Cargo.toml`, `crates/lsharp-types/src/infer.rs`
- Related: `imp-07-test-verification-infrastructure.md`, `TODO.md` の `LEGACY-TEST-01`

## Context

`lsharp-types` の unification は関数型・型適用・型変数・heap handle 互換など複数の分岐を持つ。既存の例示テストだけでは、型の組み合わせを
入れ替えたときに成功/失敗の成否がずれる回帰を十分に検出できない。一方、これは型推論全体の property や両 target の native parity を完了させる
判断とは別の、局所的な契約である。

## Decision

- `proptest` は `lsharp-types` の dev-dependency に限定し、runtime / 配布依存には追加しない。
- `unify_property_tests::arb_type` は `Con` の代表型、型変数、`Fun`、`App` を深さ 3・要素数 0〜2 に制限して生成する。
- 各方向で新しい `Infer` を作り、同じ span で `unify(a, b)` と `unify(b, a)` を実行する。property の契約は `is_ok()` の一致に限定し、
  置換 map の具体的な形や型変数番号の割り当て順を契約にしない。
- 通常の local test は 64 cases とし、入力サイズを bounded に固定する。nightly/fuzz、式全体の推論、AST generator の再利用はこの slice の対象外とする。

## Evidence

- RED: property test を実装より先に追加し、`proptest` 未導入時の unresolved crate/macro と `Strategy` 解決エラーを確認した。
- GREEN: `cargo test -p lsharp-types unify_success_is_symmetric` が 64 cases で成功した。
- Regression: `cargo test -p lsharp-types -- --nocapture --test-threads=1`（208 tests）、`cargo clippy -p lsharp-types --lib --tests -- -D warnings`、
  changed file の rustfmt check、`git diff --check`、`scripts/audit_docs.sh` がすべて成功した。workspace 全体の `cargo fmt --all -- --check` は
  既存の別ファイルの formatting drift を検出するため、この slice の gate には採用していない。

## Consequences

型 unification の成否対称性に対する bounded regression を通常の local test で検出できる。これは `LEGACY-TEST-01` の verified slice に留まり、
parser の roundtrip、生成式の型推論 panic/infinite-loop、nightly 4096 cases、GC leak/limit、rooting stress、native stage0 の証跡は未完了である。

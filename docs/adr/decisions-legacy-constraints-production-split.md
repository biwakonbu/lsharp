# ADR: `constraints.rs` の production 責務分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/constraints.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

inline test を分離した後も制約評価、制約階層、型変換、runtime check の production が一つの module に残っていた。各責務を独立してレビューし、regex/runtime などの変更が他の制約経路へ混ざる衝突を減らす必要があった。

## Decision

- `constraints/eval.rs` に制約エラー・評価結果・整数/文字列評価・境界値生成を移す。
- `constraints/hierarchy.rs` に階層解決・互換性判定を移す。
- `constraints/conversion.rs` に conversion info/kind と型変換分析を移す。
- `constraints/runtime.rs` に runtime check/condition と runtime condition 生成を移す。
- 親 module は責務別 module 宣言と既存 public symbol の `pub use`、test module 宣言だけを保持する。既存の `constraints::...` public path と test namespace は維持する。

## Evidence

- 分離前後の focused gate `cargo test -p lsharp-types constraints:: -- --nocapture`: 43 passed。
- `constraints.rs` は 543 行から 31 行へ縮小し、production modules は eval 187 行、hierarchy 149 行、conversion 131 行、runtime 68 行となった。
- `cargo test -p lsharp-types`、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check`、`bash scripts/audit_docs.sh` が pass。

## Consequences

制約責務ごとの差分レビューと focused test 実行が可能になり、I-01 の file-size boundary と I-08 の test isolation を前進させられる。制約 semantics の拡張、I-01 / I-08 aggregate、native/selfhost parity は後続であり、この ADR の verified slice だけで完了扱いにはしない。

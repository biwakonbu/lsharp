# ADR: self-application の occurs-check 診断 contract

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-types/tests/infer_limits.rs`
- Related: `imp-07-test-verification-infrastructure.md`, `TODO.md` の `LEGACY-TEST-01`

## Context

型推論の bounded expression property は panic がないことを確認するが、無限型を検出したときの公開診断分類までは固定しない。
Hindley–Milner の代表的な failure boundary である self-application を fixture として、occurs check が `InfiniteType` と安定診断 code `LS1003` を返す契約を明示する。

## Decision

- `(defn omega [f] (f f))` を parse → `Infer::infer_program` に通し、成功してはならないことを検証する。
- `TypeError::InfiniteType` の variant と `TypeError::code() == "LS1003"` を同時に確認する。メッセージ全文や型変数番号は契約に含めない。
- test は integration file に置き、public parser/inference boundary を利用する。private `unify` の内部実装や substitution map の形へ依存しない。

## Evidence

- RED: fixture test を実装前に追加し、focused `cargo test -p lsharp-types --test infer_limits self_application_reports_infinite_type` を実行した。
- GREEN: self-application fixture は `InfiniteType` と `LS1003` を返し、focused test が成功した。
- changed integration file の rustfmt、`git diff --check`、`scripts/audit_docs.sh` が成功した。workspace 全体 `cargo fmt --all -- --check` は既存の別ファイル formatting drift を検出するため採用していない。

## Consequences

occur-check の代表的な診断境界を regression として保てる。一方、深い型の性能上限、recursion stack limit、式全体の diagnostic/span parity、nightly cases、GC leak/limit、
native stage0 の証跡は未完了であり、`LEGACY-TEST-01` aggregate を完了扱いにはしない。

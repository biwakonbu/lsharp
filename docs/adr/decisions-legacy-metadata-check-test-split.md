# ADR: `metadata_check.rs` の inline test module 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/metadata_check.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`metadata_check.rs` は metadata diagnostics、property/test generation production と、metadata validation 22 件・test generation 9 件の計 31 件の unit test が同じ 1,191 行の file に混在していた。production の変更境界と generated-test fixture のレビューを分ける必要があった。

## Decision

- 既存の `#[cfg(test)] mod tests` と `test_generation_tests` namespace を維持する。
- test body を `metadata_check/tests.rs` と `metadata_check/test_generation_tests.rs` へ移動する。
- `metadata_check.rs` は production と module declarations だけを持ち、公開型、diagnostic semantics、generated test contract、fixture は変更しない。

## Evidence

- 分離前後の focused `cargo test -p lsharp-types metadata_check:: -- --nocapture`: 31 passed。
- `cargo test -p lsharp-types`: unit 209、integration 3/4/1/6/30/3/2 が全て pass。
- `metadata_check.rs` は 1,191 行から 846 行へ縮小し、tests files は 238/95 行となった。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check` が pass。

## Consequences

metadata diagnostics と generated-test fixtures を独立してレビュー・実行できるようになった。metadata production の責務分割、property/unit 拡張、`I-01` / `I-08` aggregate、selfhost/native parity は後続であり、この verified slice だけで完了扱いにはしない。

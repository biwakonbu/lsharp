# ADR: `macro_expand.rs` の production 責務分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-syntax/src/macro_expand.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

inline test を分離した後も `macro_expand.rs` は 995 行で、エラー型、built-in/computation helper、再帰展開・quote substitution が一つの module に残っていた。macro semantics の変更を含めずに責務境界を明確にし、今後の feature 差分の衝突を減らす必要があった。

## Decision

- `macro_expand/error.rs` に `MacroExpansionStep`、`MacroExpandError` と診断 helper を移す。
- `macro_expand/builtins.rs` に built-in registration、`cond`、`|>`、computation desugaring を移す。
- `macro_expand/expand.rs` に再帰式展開と quote substitution を移す。
- 親 module は `MacroExpander` の state、高位 program/decl orchestration、module 宣言を保持し、既存の `MacroExpandError` / `MacroExpansionStep` path は `pub use` で維持する。

## Evidence

- 分割前後の focused gate `cargo test -p lsharp-syntax macro_expand:: -- --nocapture`: 35 passed。
- `macro_expand.rs` は 995 行から 186 行へ縮小し、production modules は `error.rs` 89 行、`builtins.rs` 292 行、`expand.rs` 450 行となった。
- 展開 semantics、公開 API、既存 test namespace を変更せず、syntax crate 全体の回帰・clippy・rustfmt・`git diff --check`・docs 監査を passした。

## Consequences

macro の error/built-in/recursive expansion 差分を独立してレビューでき、I-01 の file-size boundary と I-08 の test isolation を同時に前進させられる。parser production、constraints の production split、macro の built-in further decomposition、I-01 / I-08 aggregate、selfhost/native parity は後続であり、この ADR の verified slice だけで完了扱いにはしない。

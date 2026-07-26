# ADR: atomic/durable writer test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-driver/src/atomic_write.rs`,
  `crates/lsharp-driver/src/atomic_write/tests.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), `LEGACY-MAINT-01`, `EC-M2-03`

## Context

`atomic_write.rs` は M2-03 の `--emit-manifest` durable write production と、symlink
replacement を含む regression test を同じ module に保持していた。production parent の
責務を明確にし、rename failure の後始末を明示的な contract として残すため、test-only
fixture を child module へ分離する。

## Decision

inline `tests` module を `atomic_write/tests.rs` へ移動し、parent には
`write_durable_atomic`、temporary path、file/directory sync、test module declaration だけを
残す。既存の crate-private `atomic_write::write_durable_atomic` path、same-directory temp
file、`sync_all` → `rename` → parent directory sync の順序、symlink replacement semantics は
変更しない。rename が失敗した場合に temporary file を削除し、destination directory を
保持する contract test を追加する。

## Evidence

- Baseline で rename failure cleanup contract test が既存 writer behavior に対して passした。
- RED: `#[path = "atomic_write/tests.rs"] mod tests;` を先に追加し、
  `cargo test -p lsharp-driver atomic_write::tests::atomic_write_removes_temporary_file_when_rename_fails`
  を実行して `E0583`（child test module 不在）を確認。
- GREEN: child へ移動後、symlink replacement と rename failure cleanup を含む
  `atomic_write::tests` 2件が pass。
- `cargo test -p lsharp-driver --bin lsharp`: 160 pass。
- `cargo clippy -p lsharp-driver --bin lsharp --tests -- -D warnings`、workspace check、
  対象 Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

test fixture の ownership が child に集約され、production parent は 126 行から 69 行へ
縮小した。M2-03 の atomic/durable writer の observable behavior と failure cleanup boundary
は維持される。selfhost/native manifest parity、release-level durability evidence、
EC-M2-03 aggregate、I-01 / I-08 は未完了であり、`EC-M2-03` と `LEGACY-MAINT-01` は
verified partial のまま継続する。

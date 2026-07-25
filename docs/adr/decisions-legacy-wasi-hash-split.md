# ADR: WASI FNV-1a hash helper split
 failure と package-wide test-only lint debt は今回の差分外として残る。
- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-stdin-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__fnv1a_hash` の String length-header 読み取り、byte loop、FNV offset
basis/prime、tombstone 回避は独立した code-emission 責務だが、WASI entrypoint と同じ
parent にあり、別 runtime 変更との衝突単位を増やしていた。

## Decision

- `emit_fnv1a_hash_func` を `crates/lsharp-wasm/src/wasi/hash.rs`（109 行）へ移動する。
- Preview1/Preview2 の helper registration は `hash` module 経由にし、function ordering と
  import/index contract は維持する。
- String header `[tag:i32=1][len:i32][bytes]`、FNV-1a offset basis `2166136261`、prime
  `16777619`、`0` / `-1` の `+2` tombstone 回避、i64 return contract は変更しない。
- `hash_tests.rs` の module seam test で hash function body の登録を固定する。

## Evidence

- RED: `hash` module が空の状態で seam test を実行し、helper の unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib hash_module_emits_fnv1a_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib`（89 passed / 1 件は既存 root-lifetime failure）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI FNV-1a hash code-emission の責務分離だけを扱う。hash の Rust/native
selfhost parity、dynamic memory layout、全公開 command、Mac Apple Silicon / Linux x86_64
native stage0 の完了を意味しない。既存の `vector-push-pair-rooted-v3` selfhost fixture
failure と package-wide test-only lint debt は今回の差分外として残る。

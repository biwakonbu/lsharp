# ADR: `lsharp-ir/src/lib.rs` の tail test-only 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-ir/src/lib.rs`
- Related: I-01 / I-08 / `imp-06-large-file-decomposition`

## Context

`lsharp-ir/src/lib.rs` は IR 定義、module linking、multi-file compile、incremental
cache と unit test を同居させ、origin/main の基準で 5462 行だった。末尾には linker、
import deduplication、multi-file compile、fingerprint、incremental compile、memory
instruction、selfhost collision の 7 test module（61件）が連続していた。

## Decision

末尾の 7 test module を `crates/lsharp-ir/src/lib_tests.rs` へ移動し、親には次の
test-only include だけを残す。

```rust
#[cfg(test)]
include!("lib_tests.rs");
```

`include!` により既存の `linker_tests` 等の module path、private helper access、公開 API
を維持する。production と cfg(test) tracking helper（incremental counter 等）は今回
の移動対象から外し、runtime/compiler logic は変更しない。

## Evidence

- `lib.rs`: 5462 行 → parent 3080 行
- `lib_tests.rs`: 2383 行、7 module / 61 tests
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir`: 257 passed
- `cargo test -p lsharp-ir --doc`: 0/0 passed
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`: passed
- 対象 files の Rust 2024 rustfmt、`git diff --check`: passed
- default stack では既存 `incremental_compile_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` が stack overflow。large-stack gate では pass するため、今回の移動差分外 boundary として残す。

## Consequences

- linking / incremental compile の production code と tail test fixture の ownership が分かれ、後続の `ir.rs` / `linker.rs` / `compile.rs` 分割の差分を小さくできる。
- 親は依然 3080 行であり、IR production split と I-01 / I-08 aggregate は未完了。
- formatter incremental fixture の default-stack overflow は別の stack/fixture 改修タスクとして維持する。

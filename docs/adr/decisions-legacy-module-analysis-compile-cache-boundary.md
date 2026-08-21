# ADR: analysis-only cache と compile cache の readiness 分離

- Status: Accepted (verified slice)
- Date: 2026-08-22
- Scope: `crates/lsharp-ir/src/cache.rs`, `crates/lsharp-ir/src/compile_incremental.rs`
- Related: `LEGACY-MODULE-01`, `I-33`, [worktree 取り込み判定](decisions-worktree-absorption-2026-08-20.md),
  [`decisions-legacy-module-incremental-scc.md`](decisions-legacy-module-incremental-scc.md)

## Context

`analyze_multi_file_incremental_with_overrides` は AST と型 surface を cache へ入れるが IR は作らない。
`build_module_cache_entry` (`compile_support.rs:108`) が入れる `Module` は全 field 空の placeholder である。

一方 compile 側の clean-hit 判定は `entry.fingerprint() == fingerprint` だけを見ていた。
そのため **analyze の直後に compile すると、空の placeholder IR がそのまま返る**。
`compile_multi_file_with_cache` は `crates/lsharp-tooling/src/compile.rs:259` から呼ばれる公開経路なので、
analysis を先に走らせる呼び出し側は黙って空 module を受け取ることになる。

## Decision

`ModuleCacheEntry` に `ir_ready: bool` を持たせ、`set_ir` が呼ばれたときだけ true にする。
`has_ir()` で公開し、**compile 側の clean-hit 判定 2 箇所**に条件として足す。

- `compile_incremental.rs:486` -- `sorted_files.len() == 1` の単一 module 経路
- `compile_incremental.rs:540` -- 非 SCC の multi-module 経路 (`changed_modules` の算出)

**却下した案**: `ir().functions.is_empty()` で判定する。空の IR は有効なプログラムでも
起こり得る (宣言だけの module) ので、内容から readiness を推論してはならない。明示フラグにする。

**変更しない範囲**:

- **analysis 側の clean-hit 判定は触らない** (`compile_incremental.rs:386` / `:325`)。
  analysis は IR を必要としないので、readiness を要求すると型解析結果の再利用が壊れる。
- SCC 経路の `all_clean` 早期 return (`compile_incremental.rs:26`) も触らない。
  こちらは `cache.linked_module()` の存在で守られており、`set_linked_module` は
  compile 経路 2 箇所 (`:212` / `:694`) からしか呼ばれない。

## Evidence

`codex/legacy-module-scc-cache-contract` @ `265a42c5` の指摘を main で再現し、修正を移植した。
branch の diff をそのまま当てることはできない (main は `lib.rs` を
`compile_incremental.rs` / `compile_entrypoints.rs` / `compile_support.rs` へ分割済み)。

- RED: `test_compile_multi_file_with_cache_materializes_ir_after_analyze_only_cache` が
  `analysis-only cache hit must not return an empty IR module` で落ちた
- GREEN: 同 test が pass。単一 module 経路の
  `test_compile_single_module_with_cache_materializes_ir_after_analyze_only_cache` も追加した
- `cargo test -p lsharp-ir`: 301 passed / 0 failed

## Residual risk

公開 driver の fresh-vs-cached artifact/runtime parity、cache telemetry、disk persistence、
selfhost parity、Mac/Linux native evidence は未検証のままである。`LEGACY-MODULE-01` は active に残す。

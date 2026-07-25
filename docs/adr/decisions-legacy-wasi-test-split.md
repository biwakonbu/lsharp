# ADR: `wasi.rs` の test-only 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: I-01 / I-08 / `imp-06-large-file-decomposition`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper、Wasmtime fixture を同じ
ファイルに持ち、origin/main の基準で 5208 行だった。末尾の `#[cfg(test)] mod tests`
は 28 件の unit test と compile/run fixture helper を含み、production と test-only の
責務が混在していた。

## Decision

末尾の test-only module 全体を `crates/lsharp-wasm/src/wasi_tests.rs` へ移動する。
親には次だけを残す。

```rust
#[cfg(test)]
include!("wasi_tests.rs");
```

`include!` を使って従来の `wasi::tests` module path、private helper へのアクセス、公開
API を維持する。production logic、fixture body、runtime contract は変更しない。

## Evidence

- `wasi.rs`: 5208 行 → production parent 4568 行
- `wasi_tests.rs`: 647 行
- focused `cargo test -p lsharp-wasm 'wasi::tests'`: 28 件中 27 pass、1 件は既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure
- `cargo test -p lsharp-wasm --lib`: 86 pass / 1 fail（同じ既存 failure）
- `cargo test -p lsharp-wasm --doc`: 0/0 pass
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`: pass
- 対象 files の Rust 2024 rustfmt、`git diff --check`: pass
- `cargo clippy -p lsharp-wasm --lib --tests -- -D warnings` は、移動した既存 test
  closure lint と `native_cli_output` / E2E の既存 lint debt のため fail。今回 lint を
  新規修正することはせず、failure boundary として記録する。

## Consequences

- `wasi.rs` の production と Wasmtime test fixture の ownership が分かれ、後続の
  `wasi/layout.rs`、GC、I/O、preview entrypoint 分割の差分が小さくなる。
- `wasi.rs` の production 責務はまだ 800 行を大きく超えるため、I-01 / I-08 と
  wasi production split は未完了である。
- 既存 root-lifetime failure と package-wide clippy lint debt は別タスクとして残す。

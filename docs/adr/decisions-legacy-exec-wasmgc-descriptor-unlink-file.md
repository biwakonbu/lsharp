# ADR: Legacy Exec WasmGC descriptor unlink-file-at lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC Component filesystem descriptor operations

## Context

Stage 2ag で write-enabled named preopen descriptor の
`descriptor.remove-directory-at` と host directory deletion を actual Component で検証した。
次の filesystem mutation boundary として、通常 file の `descriptor.unlink-file-at` と resource
drop を同じ `wasmgc-cli-fs` world で固定する必要がある。

## Decision

`wasm_gc_component_cli_fs_runner_unlinks_file_and_drops_resources` を追加し、二つの read-write named
preopen を Component runner に渡す。guest は最初の preopen descriptor に `to-unlink.txt` を渡して
`descriptor.unlink-file-at` を呼び、canonical result の success discriminant を確認した後、preopen
descriptor を drop する。実行後に stdout が空、exit code が 0、host 側の `to-unlink.txt` が存在しない
ことを確認する。fixture directory と second preopen directory はテスト終了時に削除する。

これは file unlink の host artifact と resource lifetime を actual Component で検証するが、descriptor
operation 全体、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native evidence、
native/selfhost parity を完了扱いにはしない。

## Evidence

- `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_unlinks_file_and_drops_resources -- --nocapture`
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` (58 tests)
- `cargo test -p lsharp-wasm --lib component_adapter::tests -- --nocapture` (5 tests)
- `cargo test -p lsharp-tooling --lib wasmgc_backend -- --nocapture` (31 tests)
- WasmGC probe/workspace clippy (`-D warnings`)
- `rustfmt --edition 2024 --check`、`git diff --check`、`bash scripts/audit_docs.sh`

## Consequences

`descriptor.unlink-file-at` の success/error result、host file deletion、preopen drop、Component exit
の境界が再現可能な fixture として残る。残る descriptor operation、artifact/runtime differential、
対応二 target の native evidence は TODO の `[~]` aggregate として継続する。

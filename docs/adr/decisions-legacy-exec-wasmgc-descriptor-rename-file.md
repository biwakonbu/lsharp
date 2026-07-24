# ADR: Legacy Exec WasmGC descriptor rename-at lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC Component filesystem descriptor operations

## Context

Stage 2ah で write-enabled named preopen descriptor の `descriptor.unlink-file-at` と host file
deletion を actual Component で検証した。次の filesystem mutation boundary として、二つの descriptor
を関係させる `descriptor.rename-at` と host artifact の移動を同じ `wasmgc-cli-fs` world で固定する
必要がある。

## Decision

`wasm_gc_component_cli_fs_runner_renames_file_and_drops_resources` を追加し、二つの read-write named
preopen を Component runner に渡す。guest は最初の preopen descriptor に `old.txt` を渡し、同じ
descriptor を destination base として `renamed.txt` へ `descriptor.rename-at` を呼ぶ。canonical result
の success discriminant と preopen descriptor の drop を確認し、stdout empty、exit code 0、host 側で
source が消え、destination の bytes が `hello` のまま残ることを同じ実行で確認する。fixture directory
と second preopen directory はテスト終了時に削除する。

これは same-directory file rename の host artifact と resource lifetime を actual Component で検証
するが、descriptor operation 全体、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64
native evidence、native/selfhost parity を完了扱いにはしない。

## Evidence

- `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_renames_file_and_drops_resources -- --nocapture`
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` (59 tests)
- `cargo test -p lsharp-wasm --lib component_adapter::tests -- --nocapture` (5 tests)
- `cargo test -p lsharp-tooling --lib wasmgc_backend -- --nocapture` (31 tests)
- WasmGC probe/workspace clippy (`-D warnings`)
- `rustfmt --edition 2024 --check`、`git diff --check`、`bash scripts/audit_docs.sh`

## Consequences

`descriptor.rename-at` の success/error result、同一 directory の source/destination artifact、preopen
drop、Component exit の境界が再現可能な fixture として残る。残る descriptor operation、artifact/runtime
differential、対応二 target の native evidence は TODO の `[~]` aggregate として継続する。

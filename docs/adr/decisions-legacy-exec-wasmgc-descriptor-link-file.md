# ADR: Legacy Exec WasmGC descriptor link-at lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC Component filesystem descriptor operations

## Context

Stage 2ak で symbolic link target の `descriptor.readlink-at` string result を actual Component で検証
した。次の filesystem mutation boundary として、old-path-flags を含む `descriptor.link-at` の hard
link creation と source/destination artifact を同じ `wasmgc-cli-fs` world で固定する必要がある。

## Decision

`wasm_gc_component_cli_fs_runner_creates_hard_link_and_drops_resources` を追加し、二つの read-write
named preopen を Component runner に渡す。guest は最初の preopen descriptor に old-path-flags `0`、
`source.txt`、同じ descriptor を destination base、`hardlink.txt` を渡して `descriptor.link-at` を
呼ぶ。canonical result の success discriminant と preopen descriptor の drop を確認し、stdout empty、
exit code 0、source と hard link の bytes がともに `hello` であることを同じ実行で確認する。fixture
directories はテスト終了時に削除する。

これは same-directory hard link の host artifact と resource lifetime を actual Component で検証するが、
descriptor operation 全体、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
evidence、native/selfhost parity を完了扱いにはしない。

## Evidence

- `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_creates_hard_link_and_drops_resources -- --nocapture`
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` (62 tests)
- `cargo test -p lsharp-wasm --lib component_adapter::tests -- --nocapture` (5 tests)
- `cargo test -p lsharp-tooling --lib wasmgc_backend -- --nocapture` (31 tests)
- WasmGC probe/workspace clippy (`-D warnings`)
- `rustfmt --edition 2024 --check`、`git diff --check`、`bash scripts/audit_docs.sh`

## Consequences

`descriptor.link-at` の old-path-flags、success/error result、source/destination hard-link artifact、
preopen drop、Component exit の境界が再現可能な fixture として残る。残る descriptor operation、
artifact/runtime differential、対応二 target の native evidence は TODO の `[~]` aggregate として継続する。

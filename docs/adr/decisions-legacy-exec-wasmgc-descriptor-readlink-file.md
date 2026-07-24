# ADR: Legacy Exec WasmGC descriptor readlink-at lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC Component filesystem descriptor operations

## Context

Stage 2aj で write-enabled named preopen descriptor の `descriptor.symlink-at` と host symbolic-link
artifact を actual Component で検証した。次の read-side boundary として、symbolic link の target string
を `descriptor.readlink-at` の canonical `result<string, error-code>` payload から読み戻し、resource drop
まで閉じる必要がある。

## Decision

`wasm_gc_component_cli_fs_runner_reads_symlink_target_and_drops_resources` を追加し、二つの read-write
named preopen を Component runner に渡す。fixture の `link.txt -> target.txt` symlink を最初の preopen
descriptor から `descriptor.readlink-at` で読み、result discriminant が success であることを確認する。
success payload の guest linear-memory `(ptr, len)` を custom stdout に渡して `target.txt` を出力し、
preopen descriptor を drop する。stdout `target.txt`、exit code 0、host symlink target unchanged を同じ
実行で確認する。fixture directories はテスト終了時に削除する。

これは relative symbolic-link target の string result と host artifact を actual Component で検証するが、
descriptor operation 全体、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64 native
evidence、native/selfhost parity を完了扱いにはしない。

## Evidence

- `cargo test -p lsharp-wasm --test wasmgc_probe wasm_gc_component_cli_fs_runner_reads_symlink_target_and_drops_resources -- --nocapture`
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` (61 tests)
- `cargo test -p lsharp-wasm --lib component_adapter::tests -- --nocapture` (5 tests)
- `cargo test -p lsharp-tooling --lib wasmgc_backend -- --nocapture` (31 tests)
- WasmGC probe/workspace clippy (`-D warnings`)
- `rustfmt --edition 2024 --check`、`git diff --check`、`bash scripts/audit_docs.sh`

## Consequences

`descriptor.readlink-at` の success/error result、canonical string payload、host symlink target、preopen
drop、Component exit の境界が再現可能な fixture として残る。残る descriptor operation、artifact/runtime
differential、対応二 target の native evidence は TODO の `[~]` aggregate として継続する。

# ADR: WasmGC descriptor is-same-object の actual Component 検証

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `wasmgc-cli-fs` / `wasi:filesystem/types@0.2.3`

## Context

`descriptor.is-same-object` は二つの descriptor が同じ underlying filesystem object を指すかを
返す。synthetic import の接続だけでは、hard link を介した object identity、Component canonical
bool、descriptor drop の境界を証明できない。

## Decision

実際の `wasmgc-cli-fs` Component から同じ preopen 上の `source.txt` と host hard link
`hardlink.txt` を開き、`descriptor.is-same-object` の bool result が true になる契約を検証する。
実行後は二つの file descriptor と preopen descriptor を drop し、source/hard-link の bytes、
stdout、`wasi:cli/run` exit code を確認する。

## Evidence

- Test: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Test: `wasm_gc_component_cli_fs_runner_compares_same_file_descriptors_and_drops_resources`
- Focused gate: `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture`
- Expected boundary: `descriptor.is-same-object` returns true for two descriptors opened on hard-linked
  files; both files retain `hello`, stdout is empty, and exit code is 0.

## Residual risk

これは descriptor is-same-object の verified partial slice である。descriptor metadata/hash、
remaining filesystem operations、Wasm artifact/runtime differential、Mac Apple Silicon/Linux
x86_64 native evidence、native/selfhost parity は別途必要であり、aggregate completion とは扱わない。

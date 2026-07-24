# ADR: WasmGC CLI の descriptor sync lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 `wasi:filesystem/types`

## Context

Stage 2ab で `descriptor.sync-data`、Stage 2ac で poll list を actual Component に接続したが、
descriptor の data と metadata をまとめて同期する `descriptor.sync` の canonical result/error
boundary と resource cleanup は未検証だった。

## Decision

- 既存の `wasmgc-cli-fs` world と二つの read-only named preopen を使い、最初の preopen から
  `input.txt` を `descriptor.open-at` で開く。
- `descriptor.sync` の `result<_, error-code>` を canonical retptr で受け取り、success discriminant
  を確認する。read-only descriptor でも host implementation が no-op success とする Preview2
  契約を actual file で固定する。
- descriptor と preopen を drop し、Component exit 0、stdout empty、host bytes unchanged
  (`hello`) を同じ実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_syncs_descriptor_and_drops_resources` は actual Component で
  `descriptor.sync`、success result、resource drop、exit 0、host bytes を一つの実行で確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は Stage 2ad を含む 54 tests
  passed。component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff
  check、docs audit も成功した。

## Consequences

- `descriptor.sync` の actual Component success/drop boundary が verified partial slice になる。
- 残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

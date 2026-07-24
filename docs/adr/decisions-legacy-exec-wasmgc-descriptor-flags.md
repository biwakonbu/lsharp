# ADR: WasmGC CLI の descriptor get-type/get-flags lifecycle

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC CLI Component、WASI Preview2 filesystem descriptor

## Context

Stage 2y までに named preopen、descriptor read/write/stat、stream write/append、directory entry、
error/drop を検証したが、open 済み descriptor の動的 type と access flags を Component canonical
ABI から取得する経路は未検証だった。

## Decision

- 既存の `wasmgc-cli-fs` world を使い、二つの read-only named preopen を順序付きで渡す。
- 最初の preopen から `input.txt` を `descriptor.open-at` で開き、`descriptor.get-type` が
  `regular-file`、`descriptor.get-flags` が `read` bit を返すことを確認する。
- `result<descriptor-type, error-code>` と `result<descriptor-flags, error-code>` の canonical
  result discriminant と byte-aligned payload を guest 側で検証し、word offset を暗黙の契約にしない。
- descriptor と二つの preopen を drop し、Component exit 0、stdout empty、host bytes unchanged
  (`hello`) を同じ実行で確認する。

## Evidence

- `wasm_gc_component_cli_fs_runner_reports_descriptor_type_and_flags` は actual Component で
  preopen table、open-at、get-type、get-flags、resource drop、exit 0、host bytes を一つの実行で
  確認する。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は 50 tests passed（Stage 2z を含む）。
- component adapter 5 tests、WasmGC tooling 31 tests、対象 clippy、rustfmt、git diff check、docs audit
  が成功する。

## Consequences

- descriptor type/flags の canonical result layout と resource lifecycle が verified partial slice に
  なった。
- pollable、残る descriptor operation、artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
  native/selfhost parity は未完了であり、aggregate task は `[~]` のまま維持する。

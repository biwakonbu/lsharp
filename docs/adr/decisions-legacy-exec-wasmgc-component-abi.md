# ADR: WasmGC output の Component canonical ABI

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC `print-string` を Component Model output interface へ接続する ABI

## Context

Stage 2k で、WasmGC の `env.print-string` を既存 `wit-component` adapter へ直接渡す経路は
未実装として明示拒否した。Component Model の `list<u8>` は core module では GC reference ではなく、
linear memory 上の `(ptr: i32, len: i32)` として lowering される。bridge を実装する前に、この ABI と
bytes の ownership を固定する必要がある。

## Decision

`wit/lsharp-wasmgc-output.wit` を WasmGC output の候補 interface とし、次の契約を採用する。

- package は `lsharp:wasmgc-output@0.1.0`、interface は `stdout`、function は
  `write(bytes: list<u8>)` とする。
- core import module/name は
  `lsharp:wasmgc-output/stdout@0.1.0` / `write`、lowered signature は `(i32, i32) -> ()` とする。
- core module は exported `memory` を持つ。入力 list の canonical lowering だけなら
  `cabi_realloc` は要求しないため、host が呼び出し中に bytes を読み取る borrow-like 契約にする。
- `write` は一回の呼び出しで bytes 全量を消費する。host 側の write error は component trap/error
  として返し、chunk の再順序化・黙った切り捨て・GC reference の component 境界越しの移送は行わない。

## Evidence

- `test_componentize_linear_list_u8_output_exposes_canonical_pair_contract` は、この WIT world と
  `(i32, i32) -> ()` の linear core import、exported memory を `componentize_core_module` へ渡し、
  Component validation を通過させる。
- 同 test が、WasmGC の GC array reference と canonical pair の差を bridge 実装の前提として固定する。

## Consequences

- ABI、version、interface/name、memory ownership を次の GC→linear copy 実装から参照できる。
- これは linear probe と WIT contract の verified partial slice であり、WasmGC array を実際に
  memory へ copy する codegen、WASI `fd_write`、Component runner、native/selfhost parity は未完了である。
- `list<u8>` を UTF-8 text とみなすのは L# の `String` semantics を接続する層の責務であり、ABI 自体は
  opaque bytes を運ぶ。Unicode code-point semantics をこの ABI の検証へ混ぜない。

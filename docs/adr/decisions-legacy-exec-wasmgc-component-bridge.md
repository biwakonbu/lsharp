# ADR: WasmGC `print-string` の Component Model 境界

- Status: Accepted (verified boundary)
- Date: 2026-07-24
- Scope: WasmGC core module の Component Model 変換

## Context

Stage 2j までで、WasmGC core module の `env.print-string` は packed `i8` array reference を
host の `std::io::Write` へ渡せるようになった。一方、既存の `component_adapter` は
`wit-component` の Preview1 adapter と WIT の canonical ABI を前提にしており、GC reference を
`list<u8>` などの WIT 値へ暗黙に変換する経路を持たない。

この境界を曖昧にすると、WasmGC artifact を linear-memory/WASI component として誤って包装したり、
失敗を一般的な `env` import エラーのまま利用者へ返したりする。

## Decision

`componentize_core_module` は、`env.print-string` を含む core module の component 化失敗を
`WasmGC component bridge は未実装です` として明示する。

- `wit-component` の component 化準備・adapter 登録・encode の各段階で、`env::print-string` の
  import interface 解決エラーを WasmGC bridge 未実装へ分類する。
- 既存の linear/WASI component 経路の generic error は従来の phase と world 名を保持する。
- WasmGC core に WASI/component import を暗黙追加しない。`std::io::Write` adapter の成功を
  Component Model 成果物の成功へ拡大解釈しない。

## Evidence

- `test_componentize_wasmgc_core_reports_missing_gc_component_bridge` は、実際の WasmGC
  `PackedByteArray` と `env.print-string` import を componentize し、GC array reference を
  WIT import interface へ変換できない failure boundary と診断語を固定する。
- 現行 `wit-component 0.245.1` の componentize 経路は `env::print-string` を WIT world の
  import interface として解決できず、`module requires an import interface named env` で停止する。

## Consequences

- WasmGC backend が `WasiPreview1` / `WasiComponent` target へ誤って fallback する経路を、
  component adapter API の境界でも検出できる。
- 現時点の verified slice は「安全な明示拒否」であり、WasmGC の公開 component artifact や
  WASI stdout parity を完了したものではない。
- 次の実装では、GC array を canonical ABI の `list<u8>` へ渡す設計を先に固定し、core module 内の
  array→linear-memory copy、WASI `fd_write` の partial/error semantics、component runner の
  actual runtime を別々に RED/GREEN で検証する必要がある。

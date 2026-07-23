# ADR: WasmGC packed String byte array

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

Stage 2a〜2d で `StringBytes` は WasmGC の `array<i32>` として literal、byte access、equality、
concat、valid-range substring を実行できるようになった。しかし UTF-8 byte の各要素を 32-bit
value として保持するため、String の storage contract は WasmGC の packed array 表現へまだ
到達していなかった。

## Decision

`StringBytes` の GC type は既存 record/ADT の type index を変更せず、末尾の同じ index に
`GcTypeKind::PackedByteArray` として登録する。WasmGC emitter はこの種別を mutable
`array(i8)` (`StorageType::I8`) へ変換する。既存の IR `ArrayNewFixed` / `ArrayNewDefault` /
`ArraySet` / `ArrayLen` は再利用し、packed byte の読み出しだけ `array.get_u` へ分岐する。

`PackedByteArray` は WasmGC backend の array 命令として検証するが、linear-memory emitter と
native Apple Silicon emitter では従来の array 外部境界を維持し、暗黙の fallback は行わない。
linear backend の String representation と codegen は変更しない。

## Evidence

- `wasm_gc_emitter_uses_unsigned_get_for_packed_byte_array` が `i8` array に `255` を格納し、
  `array.get_u` の実行結果 `255` を Wasmtime で確認する。signed `array.get` なら負値になるため、
  packed storage と unsigned byte access の契約を同時に固定する。
- `test_compile_file_wasmgc_backend_reads_utf8_byte_as_unsigned` が実際の String lowering から
  UTF-8 の `é` の先頭 byte `195` を Wasmtime で確認する。
- `wasm_gc_lowering_registers_string_bytes_as_packed_array` が WasmGC lowering の `StringBytes`
  登録種別を `PackedByteArray` として固定する。
- WasmGC backend focused suite 27 件（String length/get/parameter/equality/concat/substring を含む）と
  WasmGC probe 9 件が成功し、既存の String operation pipeline が packed type index を通過する。
- `cargo check -p lsharp-ir -p lsharp-wasm -p lsharp-tooling` を成功させる。

## Consequences

- WasmGC StringBytes の storage が packed `i8` になり、byte access は unsigned semantics で
  実行できるようになった。既存の `array<i32>` 用 IR と record/ADT type index は維持する。
- Unicode code-point semantics、invalid substring range diagnostics、String の print/WASI/component
  bridge、GC mutation の公開契約、Mac/Linux native、selfhost compiler は未完了である。
  `LEGACY-EXEC-01` は active のまま残す。

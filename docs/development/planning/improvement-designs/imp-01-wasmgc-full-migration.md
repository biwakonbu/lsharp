# imp-01: WasmGC 完全移行 (v2-07 補遺)

> 対象 issue: [D-01](../../../../ISSUES.md#d-01) (i64 フォールバック)、[D-02](../../../../ISSUES.md#d-02) (GADT)、
> [D-03](../../../../ISSUES.md#d-03) (HKT)、[D-04](../../../../ISSUES.md#d-04) (Computation Expression)、
> [D-06](../../../../ISSUES.md#d-06) (動的ディスパッチ)、[D-09](../../../../ISSUES.md#d-09) (selfhost ADT 表現)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase B-1
>
> **正本**: [v2-designs/v2-07-wasmgc-optional-backend.md](../v2-designs/v2-07-wasmgc-optional-backend.md)。
> 本書は v2-07 の方針 (バックエンド構成・型マッピング・優先順位・`--backend=wasmgc` フラグ) を変更せず、
> 「現行コードのどこを、どの順で、どう無回帰に置き換えるか」の補遺を与える。

## 現状の正確な把握 (2026-06-12 コード検証済み)

WasmGC への移行に必要な **IR レベルの語彙は既に存在する**。欠けているのは emit 層のみ:

| 層 | 現状 | 場所 |
|----|------|------|
| IR 型 | `IrType::Ref(u32)` が GC 型インデックスを保持済み | `crates/lsharp-ir/src/lib.rs` (IrType enum) |
| IR 命令 | `StructNew(u32)` / `StructGet(u32,u32)` / `StructSet(u32,u32)` / `RefCast(u32)` が定義済み | `crates/lsharp-ir/src/lib.rs:213-216` |
| IR モジュール | `Module.gc_types` フィールドが GC 型定義を保持する枠を持つ | `crates/lsharp-ir/src/cache.rs:9-15` (empty_module 参照) |
| 型変換 | `ir_to_wasm_valtype` が `Ref(_) => ValType::I64` へフォールバック | `crates/lsharp-wasm/src/emit.rs:16` |
| 命令 emit | `StructNew(_) => I64Const(0)` / `StructGet => nop` / `StructSet => Drop;Drop;I64Const(0)` / `RefCast => nop` のスタブ | `crates/lsharp-wasm/src/emit.rs:195-212` |
| 実動経路 | レコード/ADT の実体はリニアメモリ allocator + mark-sweep GC で動作 (`emit_wasm_wasi`) | `crates/lsharp-wasm/src/wasi.rs:102` (preview1), `:742` (component) |
| トレイト | マングル名 `TraitName_TypeName_methodName` による静的解決 (lowering 時) | `crates/lsharp-ir/src/lower/` |
| クロージャ | 自由変数解析 + lambda lifting | `crates/lsharp-ir/src/lower/closure.rs`, `lower/mod.rs` |

つまり `emit.rs:195-212` のスタブは「リニアメモリ経路では StructNew 系 IR 命令が
生成されない」前提で安全に nop 化されているもので、WasmGC backend はこの 4 命令を
本物の GC 命令に変換する emitter を新設すれば成立する。

## 依存バージョンの前提 (着手前の必須確認)

- workspace は `wasm-encoder = "0.245"` / `wasmtime = "29"` (`Cargo.toml:24-26`)
- リポジトリ内に `Config::wasm_gc(true)` 等の GC proposal 設定は存在しない (未使用)
- **Step 0 として確認すること**:
  1. wasm-encoder 0.245 の GC 型エンコード API (`SubType` / `CompositeType` / `StructType` /
     `FieldType` / `StorageType`) で必要な型セクションが出力できるか
  2. wasmtime 29 の `Config::wasm_gc(true)` で `struct.new` / `struct.get` / `ref.cast` を
     含むモジュールが実行できるか (最小 .wat を手書きして wasmtime API で実行する
     スパイクテストを 1 本書く)
  3. 不足する場合は wasmtime / wasm-encoder の更新を先行タスク化し、
     更新後に既存 E2E 全件 green を確認する

## Stage 0 検証結果 (2026-07-24)

依存バージョンの前提を、最小の self-contained module で検証した。

- `wasm-encoder 0.245.1` の `TypeSection::struct_` と
  `Instruction::{StructNew, StructGet}` で `struct { i64 }` を符号化できる。
- `wasmtime 29.0.1` は `Config::wasm_gc(true)` を有効にした engine で、その module を
  検証・instantiate・実行できる。`struct.new` で `42` を格納し、`struct.get` で読み出す
  `read-field` の結果 `42` を確認した。
- GC feature を有効化しない engine では同じ module が検証拒否されるため、backend は
  WasmGC capability を明示的に有効化する必要がある。
- 実行契約は `crates/lsharp-wasm/src/wasmgc.rs` と
  `crates/lsharp-wasm/tests/wasmgc_probe.rs` に固定した。

これは依存 API と runtime capability の確認であり、L# IR の records/ADT lowering、CLI の
`--backend=wasmgc`、文字列・closure・trait の移行、対応 target の native E2E を完了した
ことを意味しない。次の実装単位は Stage 1 の IR 型登録と records の WasmGC emitter である。

## Stage 1 検証済み slice (2026-07-24)

L# IR から self-contained WasmGC core module を生成する最小 emitter を追加した。

- `crates/lsharp-wasm/src/wasmgc.rs::emit_wasm_wasmgc` が `Module.gc_types` の struct 定義を
  type section へ出力し、`IrType::Ref` を concrete heap type の nullable reference として
  関数・local・struct field へ反映する。
- `StructNew` / `StructGet` / `StructSet` / `RefCast` を WasmGC 命令へ変換し、mutable field、
  nested reference field の実行を Wasmtime で確認した。
- Stage 1 がまだ扱わない linear-memory / global / indirect-call 命令は、i64 fallback や
  無効な Wasm を出力せず、明示的な codegen error で停止する。
- `crates/lsharp-wasm/tests/wasmgc_probe.rs` の IR emitter tests 8 件が、生成・検証・
  instantiate・actual execution と未対応命令の拒否を固定する。

この slice は Rust `lsharp-wasm` の IR emitter に加えて、直接の record literal/field access
へ接続する前段までを扱う。ADT lowering、WASI/component runtime、supported 2 targets の native
artifact/runtime、selfhost ADT 表現は未完了である。

## Stage 1.5 検証済み slice: CLI backend 選択 (2026-07-24)

compiler integration の最初の境界として、linear と WasmGC を明示的に選択できる API/CLI を追加した。

- `lsharp_tooling::compile::CompileBackend::{Linear,WasmGc}` と
  `compile_file_with_backend` を追加し、既存 `compile_file` は `Linear` を選ぶ互換 wrapper とした。
- `lsharp compile --backend wasmgc --target web-wasm` は同じ parse/type/lower パイプラインから
  `emit_wasm_wasmgc` を呼び、core Wasm を生成して Wasmtime の `main` 実行まで確認する。
- WasmGC backend は現在 `web-wasm` target のみを受け付け、WASI/component/native との組み合わせは
  `LS4001` で明示拒否する。backend 未指定時の既定値は linear のまま維持する。
- 明示 backend がある compile/build は embedded component guest へ delegation せず、host の選択した
  backend を実行する。これにより guest が未対応の flag を受け取って成功を隠すことを防ぐ。

これは CLI/API の選択境界と最小 core runtime の証跡であり、一般 records/ADT lowering、
WASI/component runtime、Mac/Linux 2 target の native E2E、selfhost compiler からの backend 選択は
まだ未完了である。

## Stage 1.75 検証済み slice: record lowering と user call (2026-07-24)

既存 linear lowering を壊さずに、WasmGC compile path の直接 record 表現を接続した。

- `lsharp_ir::lower::LowerBackend::{Linear,WasmGc}` と `Lower::with_backend` を追加し、
  WasmGC 選択時の record 型を `IrType::Ref(gc_type_index)`、record field の型も nested record
  参照へ変換する。既存 `Lower::new()` は Linear のまま維持する。
- record literal / direct field access / generated field accessor / record update の function
  signature と extra local 型を GC struct の参照型に揃え、`{Point ...}`、nested record、update を
  Wasmtime で実行する。
- lowering が予約する 17 個の runtime import 論理 index と、WasmGC core module の local function
  index の差を emitter で明示的に remap する。runtime import / `CallImport` は未対応として診断し、
  unknown function を出力しない。
- file import を含む compile は module graph/linker が WasmGC 型境界をまだ持たないため `LS4001` で
  明示拒否し、linear backend の成功を WasmGC evidence に流用しない。
- 証跡: `test_compile_file_wasmgc_backend_executes_record_access`、
  `test_compile_file_wasmgc_backend_executes_nested_record_access`、
  `test_compile_file_wasmgc_backend_executes_record_update`、WasmGC probe 8 件。

## Stage 1.75a 検証済み slice: scalar ADT constructor と pattern (2026-07-24)

record slice と同じ WasmGC core compile path に、非パラメトリック ADT の最小構築・分岐を接続した。

- `TypeDef` ごとに GC struct type を登録し、フィールド 0 を `i64` の tag、後続を variant 間で
  共有する `i64` payload slot とする。各 constructor は tag と payload を `StructNew` で生成し、
  nullary constructor は残りの slot を 0 で埋める。
- constructor pattern は `StructGet(type, 0)` の tag 比較で分岐し、`Var` / wildcard payload を
  `StructGet` して typed local へ束縛する。literal pattern、GADT、パラメトリック ADT はこの
  slice の外側であり、WasmGC 未対応診断へ止める。
- `test_compile_file_wasmgc_backend_executes_adt_constructor_and_match` が `(Just 42)` と
  `Nothing` の両方を実際に構築・match し、`--backend=wasmgc --target=web-wasm` の Wasmtime
  実行結果を確認する。WasmGC の user-call 経路では linear GC root 操作を挿入しないため、
  root stack / allocator / strings / collections の一般対応をこの証跡へ拡大解釈しない。

この段階で records と scalar ADT の direct construction/access/pattern は検証済みだが、
ADT の typed payload と nested pattern、GC root/allocator、strings、WASI/component、Mac/Linux
2 target、selfhost compiler は次の残件として維持する。

## Stage 1.75b 検証済み slice: nested ADT payload と pattern (2026-07-24)

scalar ADT の共通 `i64` slot を一段拡張し、ADT を別の ADT payload として保持する最小経路を接続した。

- program 内の ADT struct index を先に予約し、宣言順に依存せず `TypeExpr::Named` の ADT/record
  payload を concrete `IrType::Ref` として解決する。各 variant の field type と共通 slot type が
  一致しない場合は `LS3001` で止め、未対応 payload を i64 に暗黙変換しない。
- nested constructor pattern は親の tag と payload `StructGet` を確認した後、子 ADT の tag を
  再帰的に検査する。nested pattern の不一致は次の match arm へ戻り、変数束縛には payload の
  Ref 型名を引き継ぐため、その値をさらに `match` できる。
- `test_compile_file_wasmgc_backend_executes_nested_adt_constructor_and_pattern` と
  `test_compile_file_wasmgc_backend_preserves_nested_adt_binding_type` が `Box (Just 42)` /
  `Box Nothing` の成功・fallback を Wasmtime で実行する。literal pattern と String/parametric
  payload は明示拒否テストで境界を固定する。

GADT、parametric representation、GC root/allocator、strings、WASI/component、supported 2 targets、
selfhost compiler は未完了である。

## Stage 1.75c 検証済み slice: nullable ADT reference payload (2026-07-24)

variant によって payload が欠損する ADT でも、WasmGC の nullable concrete reference を明示的に生成する
経路を追加した。

- `RefNull(type_idx)` を IR に追加し、GC type index の link remap、WasmGC validation/emitter、linear
  backend の互換 fallback を接続する。ADT constructor の欠損 Ref slot は `ref.null concrete` を
  `StructNew` 前に積む。
- `test_compile_file_wasmgc_backend_executes_nullable_adt_payload` が `Present (Just 42)` と
  `Present Nothing` を実行し、nested `Just` の不一致が wildcard arm へ進む結果 `42` を Wasmtime で
  確認する。既存 linear constructor と WasmGC probe の回帰も同じ gate で通す。

nullable slot は concrete Ref の default に限定し、literal pattern、GADT、parametric representation、
GC root/allocator、strings、WASI/component、supported 2 targets、selfhost compiler は未完了である。

## Stage 1.75d 検証済み slice: scalar literal ADT pattern (2026-07-24)

ADT payload の `Int` / `Bool` / `Unit` literal pattern を、暗黙の linear fallback ではなく WasmGC
`StructGet` と value comparison で検査する経路を追加した。

- constructor tag が一致した後、literal payload は `I64Eq` の結果で nested pattern sequence を継続し、
  不一致時は同じ scrutinee の次の arm へ進む。
- `test_compile_file_wasmgc_backend_executes_integer_adt_literal_pattern` が `Just 42` / `Just 41` と
  Bool `Set true` の成功・fallback を Wasmtime で実行して結果 `2` を確認する。
- Float/String literal と record pattern はこの scalar slice の外側で、`LS3001` の明示拒否を維持する。

この slice は literal の型表現全体を閉じたものではなく、GADT、parametric representation、GC
root/allocator、strings、WASI/component、supported 2 targets、selfhost compiler は未完了である。

## Stage 1.75e 検証済み slice: record pattern field checks (2026-07-24)

WasmGC の record pattern を ADT pattern と同じ field sequence lowering へ接続し、field 値の
scalar literal と nested record pattern を実行できるようにした。

- record の concrete GC struct から field を typed local へ取り出し、`Int` / `Bool` / `Unit`
  literal は `I64Eq` で比較する。比較不一致時は同じ scrutinee の次の match arm へ fallback し、
  一致時だけ後続 field と body を評価する。
- nested record field は child の GC struct fields を sequence へ展開するため、親・子の literal
  を組み合わせても fallback の境界を失わない。record field の `Ref` 型は typed local と
  `local_type_names` へ保持し、後続の field access / nested match で i64 へ暗黙変換しない。
- `test_compile_file_wasmgc_backend_executes_record_literal_pattern_with_fallback` と
  `test_compile_file_wasmgc_backend_executes_nested_record_literal_pattern` が成功・不一致を
  Wasmtime で実行し、String literal など未対応表現は
  `test_wasmgc_backend_rejects_unsupported_record_string_literal_pattern` で `LS3001` を確認する。

Float/String の value representation、nominal runtime cast、WASI/component、supported 2 targets、
selfhost compiler は未完了であり、`LEGACY-LANG-01` / `LEGACY-EXEC-01` の aggregate 完了条件には
到達していない。

## Stage 1.75f 検証済み slice: typed type-application payload slots (2026-07-24)

ADT payload の `TypeExpr::App` を、型引数を実行時表現へ持ち込まず head type の concrete GC
reference として解決する経路を追加した。

- `Wrapper (Wrapped (Inner Int))` のような既知 ADT/record への type application は、payload を
  `IrType::Ref` として登録し、source field type name も constructor pattern へ引き継ぐ。
- variant 間で payload 型が異なる場合に i64/Ref を同じ slot へ無理に詰めないよう、各 variant の
  field を共通 struct 内の variant-specific typed slot へ配置する。未使用 slot は `I64Const 0`
  または concrete `RefNull` で初期化し、constructor/pattern の field offset を同じ表で参照する。
- `test_compile_file_wasmgc_backend_executes_type_application_payload` が type application payload
  の nested constructor match を Wasmtime で実行し、42 を確認する。
- self-recursive `TypeExpr::App` は Wasmtime 29 の GC collection で再現性のある内部 panic が発生する
  ため、`test_wasmgc_backend_rejects_recursive_type_application_payload_explicitly` で `LS3001` に
  停止する。これは GADT の型 refinement や recursive runtime support の完了を意味しない。

GADT の self-recursive representation、HKT、nominal runtime cast、WASI/component、supported 2 targets、
selfhost compiler は未完了であり、`LEGACY-EXEC-01` の完了条件には到達していない。

## Stage 1.75g 検証済み slice: scalar GADT refinement execution (2026-07-24)

return type を持つ non-recursive GADT の scalar constructor/pattern を、variant-specific typed slot
と既存の型推論 refinement の組み合わせで WasmGC 実行へ接続した。

- `(: (IntLit Int) (Expr Int))` と `(: (BoolLit Bool) (Expr Bool))` の constructor は、各 payload
  を concrete typed slot へ生成する。`get-int` は `Expr Int`、`get-bool` は `Expr Bool` として推論され、
  異なる refinement の constructor を渡す呼び出しは `LS1004` で拒否する。
- `test_compile_file_wasmgc_backend_executes_scalar_gadt_refinement` が Int/Bool の各 arm を
  Wasmtime で実行し、`42 + 1 = 43` を確認する。self-recursive `Expr Int` payload、GADT の recursive
  evaluator、HKT はこの slice の対象外である。

scalar GADT の型 refinement は verified だが、GADT 全体の recursive representation、nominal runtime
cast、WASI/component、supported 2 targets、selfhost compiler は未完了であり、`LEGACY-EXEC-01` の
aggregate 完了条件には到達していない。

## Stage 1.75h 検証済み slice: computation return と bind 境界 (2026-07-24)

Computation Expression の WasmGC path で、scalar `return` と未対応 `let!` / `do!` の境界を
成功・拒否の両方で固定した。

- builder の `return` は通常の scalar function call として lowering し、
  `test_compile_file_wasmgc_backend_executes_computation_return` が `add-one 41 = 42` を Wasmtime
  で確認する。これは monadic bind 全体の完了ではなく、return-only の verified slice である。
- `let!` / `do!` は continuation を表す GC closure と bind call が必要だが、WasmGC backend は
  まだ funcref/closure を emit できない。従来の「式を評価してローカルへ格納するだけ」の挙動を
  成功扱いしないため、`LS3001` の明示診断で止める。
- `test_wasmgc_backend_rejects_computation_bind_without_gc_closure` が、この failure boundary と
  `computation` / `closure` の診断語を固定する。linear backend の既存経路は変更しない。

この slice は D-04 の return-only contract と未対応 bind の安全な境界を閉じる。実際の bind、
multi-step monadic runtime、Stage 3 の GC closure/funcref、HKT、WASI/component、supported target
の native evidence、selfhost compiler は未完了である。

## Stage 2a 検証済み slice: scalar String GC array (2026-07-24)

Stage 2 の先行境界として、String の値表現を WasmGC の concrete array reference へ接続した。

- record/ADT の既存 GC type index をずらさないよう、program の GC struct 群の末尾に
  `StringBytes` (`array<i32>`) を登録する。String literal は UTF-8 bytes を
  `ArrayNewFixed`、`string-length` は `ArrayLen`、`string-char-at` は `ArrayGet` へ lowering する。
- `String` の function parameter と record field は同じ array reference type を使い、user call の
  Wasm function signature と local type を concrete ref に揃える。linear backend は従来の i64 pointer
  表現を変更しない。
- `test_compile_file_wasmgc_backend_executes_string_array_length`、`..._get`、
  `..._passes_string_array_to_user_function` が Wasmtime actual execution を固定する。

これは Stage 2 の scalar value slice であり、packed `i8` storage、Unicode code-point semantics、
concat/substring、print/WASI/component bridge、GC mutation、supported target の native
evidence、selfhost compiler は未完了である。

## Stage 2b 検証済み slice: scalar String GC equality (2026-07-24)

Stage 2a の `StringBytes` array を使い、WasmGC の `string-eq` を linear runtime import なしで
実行できるようにした。

- 二つの concrete array reference の長さを先に比較し、同じ場合だけ `array.get` の byte loop を
  回して全要素を比較する。長さ不一致や最初の byte 不一致は false とし、空配列同士は true とする。
- 比較対象を local に保存するため、String の user function parameter からの呼び出しも同じ
  `StringBytes` reference 型で処理する。linear backend の `__string_eq` 経路は変更しない。
- `test_compile_file_wasmgc_backend_executes_string_equality` が同長一致、同長不一致、長さ不一致、
  空配列同士を user function 経由で Wasmtime 実行し、結果 `15` を固定する。既存 linear string-eq
  4 件も回帰確認する。

これは byte-level equality の verified slice であり、packed `i8` representation、Unicode
code-point semantics、concat/substring、print/WASI/component bridge、GC mutation、supported target
の native evidence、selfhost compiler は未完了である。

## Stage 2c 検証済み slice: scalar String GC concat (2026-07-24)

Stage 2a の mutable `StringBytes` array に `string-concat` を接続した。

- 二つの String reference の `array.len` を足して `array.new_default` で結果 array を作り、
  lhs/rhs をそれぞれ index loop と `array.get` / `array.set` でコピーする。動的な長さと空文字列を
  linear runtime import なしで扱う。
- String parameter を受ける user function の戻り値も concrete `StringBytes` reference として
  `string-length` / `string-char-at` へ渡せることを actual Wasmtime で確認する。linear backend の
  `__string_concat` root/import 経路は変更しない。
- `test_compile_file_wasmgc_backend_executes_string_concat` が `"hello" + " world"` の長さと、
  `"a" + "b"` の byte access を user function 経由で実行し、合成結果 `109` を固定する。既存
  linear concat E2E 10 件も回帰確認する。

これは scalar concat の verified slice であり、packed `i8` representation、Unicode code-point
semantics、substring、print/WASI/component bridge、GC mutation、supported target の native
evidence、selfhost compiler は未完了である。

## Stage 2d 検証済み slice: scalar String GC substring (2026-07-24)

Stage 2a の mutable `StringBytes` array に valid-range の `substring` を接続した。

- `end - start` の長さで `array.new_default` を作り、source array の `start + index` を
  `array.get` して結果へ `array.set` する。空 range は長さ 0 の array として扱う。
- String parameter を受ける user function の戻り値を `string-length` / `string-char-at` へ渡す
  actual core Wasm 実行を固定する。linear backend の root/import/memory.copy 経路は変更しない。
- `test_compile_file_wasmgc_backend_executes_string_substring` が `"hello world"[6..11]` の長さと
  byte access、空 range を user function 経由で実行し、結果 `116` を固定する。既存 linear
  substring E2E の回帰も確認する。

これは valid byte-range substring の verified slice であり、invalid range 診断、packed `i8`
representation、Unicode code-point semantics、print/WASI/component bridge、GC mutation、
supported target の native evidence、selfhost compiler は未完了である。

## Stage 2e 検証済み slice: packed String byte array (2026-07-24)

Stage 2a〜2d の `StringBytes` storage を WasmGC の packed `i8` array へ置き換えた。

- 既存 record/ADT の GC type index をずらさないよう、StringBytes を同じ末尾 index の
  `GcTypeKind::PackedByteArray` として登録し、Wasm type section では mutable `array(i8)` を出力する。
- IR の array.new/array.set/array.len は再利用し、packed array の `array.get` だけを
  `array.get_u` へ選択する。`255` を格納した byte の読み出しが `255` になる Wasmtime probe で
  signed access への退行を防ぐ。
- `wasm_gc_lowering_registers_string_bytes_as_packed_array`、
  `wasm_gc_emitter_uses_unsigned_get_for_packed_byte_array`、
  `test_compile_file_wasmgc_backend_reads_utf8_byte_as_unsigned` と既存の String compile suite が、
  lowering → emitter → actual core Wasm の境界を固定する。

これは packed byte storage の verified slice であり、Unicode code-point semantics、invalid range
診断、print/WASI/component bridge、GC mutation の公開契約、supported target の native evidence、
selfhost compiler は未完了である。

## Stage 2f 検証済み slice: substring invalid range boundary (2026-07-24)

WasmGC `substring` の invalid byte range を、allocation/array access 前の明示境界へ固定した。

- start/end を i64 のまま保持し、負値、`start > end`、source byte length 超過を検証してから i32
  へ変換する。これにより i64 の巨大値を wrap して有効値に見せる経路を閉じる。
- source/start/end がすべて literal で静的に判定できる場合は `LS3001` と source span を返す。
  動的な場合は `if(unreachable)` guard を出力し、Wasm runtime の explicit trap として停止する。
- linear backend の tagged pointer/memory.copy/import 経路は変更せず、既存 valid byte-range
  substring の結果を維持する。

これは invalid range の fail-closed boundary の verified slice であり、dynamic trap の structured
diagnostic message、Unicode code-point semantics、print/WASI/component bridge、GC mutation の公開
契約、supported target の native evidence、selfhost compiler は未完了である。

## Stage 2g 検証済み slice: `print-string` external import boundary (2026-07-24)

WasmGC の String reference を host 側へ渡す最小の external boundary を、未対応 runtime import の
暗黙 fallback なしで固定した。

- lowering が予約する `Call(4)` を WasmGC backend の `print-string` 呼び出しとしてだけ認識し、
  `Module.gc_types` の `PackedByteArray` を `StringBytes` の concrete heap type として選ぶ。
- backend は `(ref null $StringBytes) -> ()` の function type と `env.print-string` import を必要時
 だけ materialize する。既存の `Module.imports` がある場合はそれを先に保持し、synthetic import の
  後ろへ user function index を remap する。
- `Call(4)` 以外の未対応 runtime logical index、`CallImport`、WASI/component/native の host
  integration は従来どおり明示的 codegen error とし、linear backend の import ABI は変更しない。
- direct WasmGC probe は生成 module の validation、`env.print-string` の型確認、stub import による
  instantiate、`main` 実行までを確認する。compiler pipeline test は
  `--backend=wasmgc --target=web-wasm` の `print-string` source が同じ import を出すことを確認する。

これは host callback が GC array の bytes を読み stdout へ出す実装ではない。GC reference の host
側 read contract、WASI fd_write / component adapter、native/selfhost runtime、Unicode code-point
semantics は後続 task として残す。host-side read contract は Stage 2h で閉じる。

## Stage 2h 検証済み slice: host-side packed String read (2026-07-24)

Stage 2g の synthetic import を、Wasmtime host callback が concrete `PackedByteArray` から
unsigned bytes へ変換する境界まで進めた。

- `crates/lsharp-wasm/src/wasmgc_host.rs::create_print_string_import` は import signature を
  `(ref null (concrete array i8)) -> ()` に限定し、i64 pointer、abstract array、i32 array、result
  付き callback を受け付けない。
- callback は `Val::AnyRef` を null / array に明示 downcast し、array type の `i8` を確認した後に
  length と各 byte を `ArrayRef::len` / `ArrayRef::get` で読み出す。null、非 array、length/get
  failure、unsigned i8 範囲外、sink error は Wasm host error として返し、成功時だけ sink へ
  immutable `&[u8]` を渡す。
- direct WasmGC probe は UTF-8 bytes `[195, 169]` の実読出し、null reference の runtime trap、
  non-packed import signature の拒否を固定する。compiler pipeline test は source の `print-string`
  を同じ callback で実行し、bytes を確認する。

これは WASI fd_write / component adapter の実装ではなく、公開 runner へ接続する前の host read
slice である。runner の stdout sink 契約は Stage 2i で閉じ、native/selfhost runtime、Unicode
code-point semantics、supported target の native evidence は後続 task として残す。

## Stage 2i 検証済み slice: WasmGC runner stdout sink (2026-07-24)

Stage 2h の host callback を、WasmGC core module の公開 runner から実際に呼び出す経路へ接続した。

- `crates/lsharp-wasm/src/wasmgc_runner.rs::run_wasm_wasmgc_with_stdout_sink` は WasmGC engine を
  明示的に有効化し、`env.print-string` 以外の import を WASI fallback なしで拒否する。
- runner は exported `main: () -> i64` を実行し、結果を i32 exit code へ検証変換する。各
  `print-string` 呼び出しの packed bytes は sink へ一度だけ全量渡し、sink が `Err` を返した場合は
  再試行せず Wasm error として返す。
- `run_wasm_wasmgc_capture` / `run_wasm_wasmgc` は同じ runner 経路で stdout を UTF-8 として capture
  し、non-zero exit code と invalid UTF-8 を既存 runner と同じ Result 境界で返す。
- direct probe は UTF-8 bytes の sink 到達、sink failure、未知 import の明示拒否を固定する。
  compiler pipeline test は source compile artifact を公開 runner で実行し、stdout と exit code を
  確認する。

これは WASI Preview1/Preview2 や Component Model の runner を置き換える実装ではない。WASI/component
adapter、Unicode code-point semantics、native/selfhost runtime、supported target の native evidence
は後続 task として残す。

## Stage 2j 検証済み slice: WasmGC stdio Write adapter (2026-07-24)

Stage 2i の chunk sink を実際の Rust writer へ接続し、partial write と flush の境界を固定した。

- `crates/lsharp-wasm/src/wasmgc_runner.rs::run_wasm_wasmgc_to_writer` は writer を同期的な
  `Arc<Mutex<_>>` sink へ包み、各 `print-string` chunk を `Write::write_all` で全量消費する。
- `write_all` による partial write の再試行、`WriteZero` / write error の停止、正常実行後の `flush`
  と flush error の伝播を明示する。chunk の再順序化、黙った切り捨て、WASI fd_write への暗黙
  fallback は行わない。
- direct probe は 1 byte writer、WriteZero、write error、flush error を固定する。WASI/component
 adapter や public CLI の output ownership はこの writer slice の対象外である。

これは `std::io::Write` を通じた core runner の host adapter であり、WASI Preview1/Preview2 や
Component Model の guest adapter を実装したものではない。Unicode code-point semantics、WASI/component
I/O、native/selfhost runtime、supported target の native evidence は後続 task として残す。

## Stage 2k 検証済み slice: WasmGC Component bridge の明示拒否 (2026-07-24)

Stage 2j の core runner を既存の Component Model encoder へ渡すとき、GC reference を WIT の
canonical ABI へ暗黙変換しない失敗境界を固定した。

- `componentize_core_module` は、`env::print-string` の import interface 解決に失敗した場合、
  一般的な `env` import エラーではなく `WasmGC component bridge は未実装です` として返す。
- 実際の `PackedByteArray` と `env.print-string` を含む WasmGC core module を
  `wit-component 0.245.1` へ渡す RED を追加し、GC array reference を WIT `list<u8>` 相当へ
  変換する実装が無いことを確認する。
- linear/WASI component の generic error、WasmGC core runner、`std::io::Write` adapter の
  成功経路は変更しない。WASI/component への暗黙 fallback や fake component artifact は作らない。

これは component artifact の実装完了ではなく、未実装の guest bridge を誤って成功扱いしない
verified boundary である。次の実装 task は、(1) GC array→canonical `list<u8>` の ABI 設計、
(2) core module 内の array→linear-memory copy と WASI `fd_write` の partial/error parity、
(3) Component Model の actual instantiate/runner evidence、の三つに分ける。

## Stage 2l 検証済み slice: output `list<u8>` canonical pair 契約 (2026-07-24)

GC array を Component Model へ接続する前段として、WIT と core module の ABI を linear probe で
固定した。

- `wit/lsharp-wasmgc-output.wit` は `lsharp:wasmgc-output@0.1.0` の `stdout` interface と
  `write(bytes: list<u8>)` を定義する。bytes は呼び出し中だけ有効な opaque byte list とし、
  UTF-8/code-point 解釈は上位の String/runtime 層へ残す。
- `wit-component` の canonical lowering では core import
  `lsharp:wasmgc-output/stdout@0.1.0::write` が `(i32, i32) -> ()` となり、core module の
  exported `memory` を読む。入力 list だけの contract では `cabi_realloc` を要求しない。
- `test_componentize_linear_list_u8_output_exposes_canonical_pair_contract` が、この WIT world と
  pair signature を componentize/validate する。WasmGC `env.print-string` の GC reference を
  この pair へ copy する実装はまだ追加しない。

これは ABI と ownership の verified partial slice であり、WasmGC array→linear-memory copy、
WASI `fd_write`、host error/trap parity、Component runner、native/selfhost parity は未完了である。

## Stage 2m 検証済み slice: packed GC array→linear-memory output bridge (2026-07-24)

Stage 2l で固定した canonical pair を、WasmGC core module の実行経路へ接続した。

- `emit_wasm_wasmgc_component_output` は `print-string` の packed `i8` array を一時 local に保持し、
  `array.len` / `array.get_u` の要素ループで exported linear memory へコピーする。memory は
  `ceil(len / 65536)` pages を先に grow し、失敗時は trap とする。
- コピー後は `lsharp:wasmgc-output/stdout@0.1.0::write` の `(i32, i32) -> ()` import を呼び出す。
  `create_component_output_import` は pointer/length の符号、overflow、memory 範囲を検証し、
  呼び出し中だけ bytes を host sink へ渡す。sink error は trap として伝播する。
- `run_wasm_wasmgc_component_output_capture` はこの import だけを解決し、WASI や GC reference
  import へ fallback しない。生成 core module の WIT componentize/validation も actual bytes で
  固定した。
- RED/green は `wasm_gc_component_output_copies_packed_array_to_linear_memory_import`、
  `wasm_gc_component_output_rejects_invalid_linear_memory_range`、
  `wasm_gc_component_output_propagates_sink_failure_as_trap`、
  `wasm_gc_component_output_componentizes_against_wit_world` で固定する。

これは GC→linear-memory と canonical host sink の verified partial slice であり、WASI
`fd_write` の partial/error/errno、flush/exit ordering、Component actual instantiate/runner、
native/selfhost parity は未完了である。次の実装 task は canonical output sink を WASI
`fd_write` または Component host implementation へ接続することとする。

## Stage 2n 検証済み slice: canonical output の `fd_write` handler 契約 (2026-07-24)

Stage 2m の canonical output sink を、WASI `fd_write` に置き換え可能な host handler boundary へ
接続した。

- `run_wasm_wasmgc_component_output_to_writer` は canonical bytes を `Write::write_all` で消費し、
  partial write / WriteZero / write error を fail-closed に扱い、main の exit code を受け取った
  後だけ `flush` する。
- `run_wasm_wasmgc_component_output_to_fd_write` は `(fd, bytes) -> Result<nwritten, errno>` の
  handler を受け取り、stdout fd を明示的に渡す。partial write は再試行し、zero、over-report、
  errno は error/trap として返す。
- `wasm_gc_component_output_fd_write_retries_partial_writes`、
  `wasm_gc_component_output_fd_write_propagates_errno`、
  `wasm_gc_component_output_fd_write_rejects_zero_and_overreported_counts`、
  `wasm_gc_component_output_writer_flushes_after_nonzero_exit` が observable contract を固定する。

これは actual `WasiP1Ctx`/Preview2 host implementation ではなく、fd_write semantics を差し替え
可能にした verified partial slice である。実 WASI context、fd table/rights、Component actual
instantiate/runner、native/selfhost parity は未完了である。

## Stage 2o 検証済み slice: `wasmgc-output` Component actual instantiate (2026-07-24)

Stage 2m/2n の core output を WIT Component として実際に instantiate し、host interface を
解決する経路を追加した。

- `run_wasm_wasmgc_component_output_component_with_stdout_sink` は
  `lsharp:wasmgc-output/stdout@0.1.0` の `write(list<u8>)` を Component Linker に定義する。
  Component 側の `list<u8>` は host callback で `Vec<u8>` として lift され、sink error は
  Component trap として返る。
- `main: func() -> s64` export を actual Component API で呼び出し、s64 exit code を i32 へ検証
  変換する。WASI Preview1/Preview2 linker へ暗黙に fallback しない。
- `wasm_gc_component_output_component_runner_executes_wit_host` と
  `wasm_gc_component_output_component_runner_propagates_sink_failure` が、生成 core bytes の
  componentize、Component validation、instantiate、host output、trap propagation を固定する。

これは custom `wasmgc-output` world の actual Component slice であり、WASI Preview2 `wasi:cli/run`
接続、fd table/rights、Mac Apple Silicon/Linux x86_64 artifact/runtime、native/selfhost parity は
未完了である。

## Stage 2p 検証済み slice: custom output の Preview2 stdout stream 接続 (2026-07-24)

Stage 2o の custom Component host を、実 WASI Preview2 context と stdout resource へ接続した。

- `run_wasm_wasmgc_component_output_component_with_preview2_stdout` は `WasiCtxBuilder`、
  `ResourceTable`、`WasiView` を組み立て、同じ Component linker に
  `wasmtime_wasi::add_to_linker_sync` と custom `wasmgc-output` interface を登録する。
- custom `stdout.write(list<u8>)` callback は `wasi:cli/stdout.get-stdout` 相当の resource を
  `WasiImpl` 経由で取得し、`check-write` → `write`（permit 分割）→ `flush` を実行してから
  resource を解放する。stdout は `WasiCtxBuilder.stdout(MemoryOutputPipe)` へ集約する。
- `wasm_gc_component_output_component_runner_connects_preview2_stdout_stream` が、componentize、
  Preview2 linker/context、stdout stream、exit code、UTF-8 output を actual runtime で固定する。

これは custom world の Preview2 context/stream verified partial slice であり、custom world 自体が
`wasi:cli/run` を import/export する接続、fd table/rights の公開契約、Mac Apple Silicon/Linux
x86_64 artifact/runtime、native/selfhost parity は未完了である。

## Stage 2q 検証済み slice: WasmGC CLI world の `wasi:cli/run` 接続 (2026-07-24)

Stage 2p の Preview2 stdout stream 接続を、custom `wasmgc-output` package の command world と
`wasi:cli/run` export へ拡張した。

- `wit/lsharp-wasmgc-output.wit` に `wasmgc-cli` world を追加し、custom `stdout` import と
  `wasi:cli/run@0.2.3` export を同じ package/version の正本として固定した。
- `emit_wasm_wasmgc_component_cli` は WasmGC `main: () -> i64` の後ろに
  `wasi:cli/run@0.2.3#run: () -> i32` wrapper を追加し、main の値を drop して成功 exit code `0`
  を返す。未使用 WASI import は生成しない。
- `run_wasm_wasmgc_component_cli_with_preview2_stdout` は nested interface export を解決し、
  Stage 2p と同じ `WasiCtx`/stdout resource を通って `wasi:cli/run` を実行する。
- `wasm_gc_component_output_cli_world_rejects_core_without_wasi_cli_run_export` が core 側の
  欠落を RED に固定し、`wasm_gc_component_output_cli_backend_emits_canonical_run_export` と
  `wasm_gc_component_cli_runner_executes_wasi_cli_run_with_preview2_stdout` が componentize、
  validation、actual Preview2 runtime を GREEN に固定する。

これは custom CLI world の `wasi:cli/run` verified partial slice であり、fd table/rights の全契約、
proc-exit/error result parity、Mac Apple Silicon/Linux x86_64 artifact/runtime、native/selfhost
parity は未完了である。

## Stage 2r 検証済み slice: WasmGC CLI の exit/result parity (2026-07-24)

Stage 2q の `wasi:cli/run` runner に、Preview2 の終了境界と command result の non-zero semantics を
追加した。

- `wasmgc-cli` world は `wasi:cli/exit@0.2.3` import を明示し、`proc_exit` 相当の capability を
  custom stdout/run contract と同じ WIT 正本で宣言する。未使用の別 WASI capability は追加しない。
- `run_wasm_wasmgc_component_cli_with_preview2_stdout` は `wasi:cli/run` 呼び出しが
  `wasmtime_wasi::I32Exit`（直接 error chain または rendered trap）で終了した場合、その status code
  を trap として捨てず `ExecutionOutput.exit_code` へ返す。通常の `bool`/`result` return は従来通り
  0/1 へ decode する。
- `wasm_gc_component_cli_runner_maps_wasi_cli_exit_to_exit_status` は actual component の
  `wasi:cli/exit` host call を、`wasm_gc_component_cli_runner_maps_failed_wasi_cli_run_result_to_exit_status`
  は failed `wasi:cli/run` result をそれぞれ exit code 1 として固定する。

これは exit/result parity の verified partial slice であり、fd table/rights の公開契約、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2s 検証済み slice: WasmGC CLI の preopen/rights 境界 (2026-07-24)

Stage 2r の exit/result parity に、WASI Preview2 filesystem descriptor の preopen table と rights
を明示的に渡す境界を追加した。

- `wasmgc-cli-fs` world は `wasi:filesystem/preopens@0.2.3` と `types@0.2.3` を明示的に import する。
  通常の `wasmgc-cli` world へ filesystem capability を暗黙追加しない。
- `Preview2PreopenRights` は `read_only()` と `read_write()` を持ち、CLI/output runner の
  `...with_preopen_rights` API が `WasiCtxBuilder.preopened_dir` へ directory/file rights をそのまま
 渡す。既存 API は後方互換の read-write default を使う。
- `wasm_gc_component_cli_fs_runner_enforces_preopen_rights` は actual Component で
  `get-directories` と `descriptor.open-at(create, write)` を通し、preopen なしを exit 1、
  read-only を `read-only` 相当の exit 1、read-write を file creation/exit 0 として固定する。

これは preopen table/rights の verified partial slice であり、descriptor の全 operation、stream
read/write、fd close/lifecycle、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
native/selfhost parity は未完了である。

## Stage 2t 検証済み slice: 名前付き preopen と descriptor read stream lifecycle (2026-07-24)

Stage 2s の単一 host path preopen を、guest-visible な名前付き preopen table と descriptor の
read stream lifecycle へ拡張した。

- `Preview2Preopen` は host path、guest path、directory/file rights を一つの capability として保持し、
  `...with_preview2_stdout_and_preopens` API が複数の preopen を `WasiCtxBuilder` へ順序通り登録する。
  既存の `Option<&Path>` API は guest path `"."` の read-write default wrapper として維持する。
- `wasmgc-cli-fs-streams` world は `wasi:io/streams@0.2.3` を明示的に import し、filesystem/types
  が返す `input-stream` resource と stream methods を同じ Component resource boundary で解決する。
- `wasm_gc_component_cli_fs_runner_reads_named_preopen_stream_and_drops_resources` は actual Component
  で guest path `data` の read-only preopen から `input.txt` を `descriptor.open-at`、
  `descriptor.read-via-stream`、`input-stream.blocking-read` の順に読み、stdout に `hello` を返した後、
  input-stream と descriptor の resource-drop を実行して exit 0 になることを固定する。

これは named preopen と read-stream/drop の verified partial slice であり、descriptor の direct
`read`/`write`/`stat`、write/append stream、close-after-error、directory-entry stream、pollable、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2u 検証済み slice: descriptor direct read と EOF lifecycle (2026-07-24)

Stage 2t の stream path に加え、`wasi:filesystem/types` の direct `descriptor.read` を既存の
`wasmgc-cli-fs` world で actual Component から実行できることを固定した。

- `descriptor.read(length, offset)` の `result<tuple<list<u8>, bool>, error-code>` を canonical ABI で
  lift し、guest の `cabi_realloc` が返却 bytes を linear memory に受ける。
- `wasm_gc_component_cli_fs_runner_reads_descriptor_directly_and_reports_eof` は guest path `data` の
  read-only preopen から `input.txt` を direct read して `hello` を stdout へ出力し、その後 offset 5
  から length 1 を read して empty list と EOF bool を確認する。最後に descriptor を resource-drop
  して exit 0 を返す。
- EOF bool は exact-length read の時点では false になり得るため、0-byte read を別の observable
  operation として要求する。この意味論を曖昧な「短い read」判定へ置き換えない。

これは direct read/EOF/drop の verified partial slice であり、direct `write`/`stat`、read-directory、
write/append stream、close-after-error、directory-entry stream、pollable、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2v 検証済み slice: write/append stream lifecycle (2026-07-24)

Stage 2t/2u の read path に対応して、`wasi:io/streams` の output resource を descriptor の
write/append operation から host filesystem bytes まで閉じた。

- `wasm_gc_component_cli_fs_runner_writes_and_appends_streams_then_drops_resources` は read-write
  named preopen から create+truncate/write flags の descriptor を開き、`write-via-stream(0)` と
  `blocking-write-and-flush` で `hello` を作る。
- 同じ descriptor を drop した後に既存ファイルを再度開き、`append-via-stream` と
  `blocking-write-and-flush` で `!` を追加する。output-stream、descriptor、preopen の drop と
  `wasi:cli/run` exit 0 を actual Component で確認し、host 側の最終 bytes が `hello!` になることを
  検証する。
- stream result の error discriminant を成功扱いにせず、write/flush が失敗した場合は resource を
  解放して非成功 result へ収束させる。

これは write/append stream の verified partial slice であり、direct `write`/`stat`、read-directory、
close-after-error、directory-entry stream、pollable、Wasm artifact/runtime differential、Mac Apple
Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2w 検証済み slice: descriptor direct write/stat lifecycle (2026-07-24)

Stage 2v の output-stream path に対応して、filesystem descriptor の direct `write` と `stat` を
同じ named preopen の実ファイル境界まで閉じた。

- `wasm_gc_component_cli_fs_runner_writes_descriptor_directly_and_stats_file` は二つの read-write
  named preopen を受け取り、`descriptor.open-at(create+truncate, write)` で `output.txt` を開く。
- `descriptor.write(buffer, offset)` の canonical `list<u8>` / `result<filesize, error-code>` を使って
  offset 0 に `hello` を書き、戻り値の書込長 `5` を確認する。
- 同じ descriptor の `descriptor.stat` から regular-file type と size `5` を確認し、descriptor と
  preopen を drop した後に `wasi:cli/run` が exit 0 になること、host bytes が `hello` になることを
  actual Component で固定する。

これは direct write/stat の verified partial slice であり、read-directory、close-after-error、
directory-entry stream、pollable、Wasm artifact/runtime differential、Mac Apple Silicon/Linux
x86_64、native/selfhost parity は未完了である。

## Stage 2x 検証済み slice: direct write error と descriptor drop lifecycle (2026-07-24)

Stage 2w の成功 path に加えて、read-only descriptor で direct write が失敗した後の resource lifecycle
を actual Component で閉じた。

- `wasm_gc_component_cli_fs_runner_drops_descriptor_after_direct_write_error` は既存 `input.txt` を
  read-only descriptor として開き、`descriptor.write` の result discriminant が error になることを
  確認する。
- エラー後に descriptor と preopen を drop し、`wasi:cli/run` が exit 0 で収束すること、host bytes が
  元の `seed` から変化しないことを確認する。成功扱いにしたり、error path で resource を残したり
  しない契約を固定する。

これは direct write error/drop の verified partial slice であり、read-directory、directory-entry
stream、pollable、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost
parity は未完了である。

## Stage 2y 検証済み slice: read-directory と directory-entry stream lifecycle (2026-07-24)

Stage 2x の descriptor error/drop path に加えて、named preopen の directory descriptor から
directory-entry stream を取得し、entries と end-of-stream を Component ABI で確認した。

- `wasm_gc_component_cli_fs_runner_reads_directory_entries_and_drops_stream` は二つの read-only named
  preopen を受け取り、最初の directory descriptor に `descriptor.read-directory` を呼ぶ。
- `directory-entry-stream.read-directory-entry` の `option<directory-entry>` を canonical ABI で liftし、
  `input.txt` の regular-file entry を custom stdout へ出力した後、二回目の read が `none` になることを
  確認する。`.` / `..` を期待値へ混ぜず、WASI が明示する directory entry semantics を保つ。
- directory-entry stream と directory descriptor を drop して `wasi:cli/run` が exit 0 になることを
  actual Component で固定する。

これは read-directory/entry-stream/drop の verified partial slice であり、pollable、残る descriptor
operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2z 検証済み slice: descriptor get-type/get-flags lifecycle (2026-07-24)

Stage 2y の directory entry path に加えて、open 済み descriptor の動的 type と access flags を
Component canonical ABI から取得し、resource を解放する境界を閉じた。

- `wasm_gc_component_cli_fs_runner_reports_descriptor_type_and_flags` は二つの read-only named
  preopen を受け取り、最初の preopen から `input.txt` を `descriptor.open-at` で開く。
- `descriptor.get-type` の `regular-file` と `descriptor.get-flags` の `read` bit を result の
  discriminant/payload として確認する。descriptor type/flags の enum/flags payload は byte
  alignment を保ち、未初期化の word offset を契約にしない。
- descriptor と両 preopen を drop し、`wasi:cli/run` exit 0、stdout empty、host bytes unchanged
  (`hello`) を actual Component の一実行で固定する。

これは descriptor type/flags/drop の verified partial slice であり、pollable、残る descriptor
operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2aa 検証済み slice: input-stream pollable lifecycle (2026-07-24)

Stage 2t の input-stream resource に、non-blocking I/O の readiness boundary を追加した。

- `wasm_gc_component_cli_fs_runner_subscribes_and_polls_input_stream` は二つの read-only named
  preopen から `input.txt` を `descriptor.open-at` → `descriptor.read-via-stream` で開く。
- `input-stream.subscribe` で child `pollable` を作り、`pollable.block` でデータ準備を待った後に
  `pollable.ready` が true になることを actual Component で確認する。subscribe 直後の
  non-blocking `ready` の値を常に true と仮定しない。
- pollable、input-stream、descriptor、両 preopen を drop し、`wasi:cli/run` exit 0、stdout empty、
  host bytes unchanged (`hello`) を一実行で固定する。

これは input-stream の subscribe/block/ready/drop の verified partial slice であり、poll list API、
残る descriptor operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
native/selfhost parity は未完了である。

## Stage 2ab 検証済み slice: descriptor sync-data lifecycle (2026-07-24)

Stage 2z の descriptor metadata path に加えて、open descriptor の data synchronization operation と
success/error の drop boundary を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_syncs_descriptor_data_and_drops_resources` は二つの read-only
  named preopen を受け取り、最初の preopen から `input.txt` を `descriptor.open-at` で開く。
- `descriptor.sync-data` の `result<_, error-code>` discriminant が success になることを確認する。
  read-only descriptor でも POSIX-compatible host implementation が成功扱いにする契約を、単なる
  synthetic import ではなく実ファイルで固定する。
- descriptor と両 preopen を drop し、`wasi:cli/run` exit 0、stdout empty、host bytes unchanged
  (`hello`) を同一実行で確認する。

これは descriptor sync-data/drop の verified partial slice であり、poll list API、残る descriptor
operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2ac 検証済み slice: poll list lifecycle (2026-07-24)

Stage 2aa の単一 pollable boundary に加えて、`wasi:io/poll.poll` の borrowed pollable list と
ready index の canonical list ABI を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_polls_subscribed_input_stream_list` は read-only named preopen
  から input-stream を作り、subscribe → block → ready の後に pollable handle の list を `poll` へ渡す。
- `poll` が返す `list<u32>` の length `1` と ready index `0` を guest linear memory で確認する。
  empty list や別 index を成功扱いにせず、list input/output の realloc・memory boundary を component
  canonical ABI の実行で固定する。
- pollable、input-stream、descriptor、preopen を drop し、`wasi:cli/run` exit 0、stdout empty、
  host bytes unchanged (`hello`) を一実行で確認する。

これは poll list と resource-drop の verified partial slice であり、残る descriptor operation、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ad 検証済み slice: descriptor sync lifecycle (2026-07-24)

Stage 2ab の data-only synchronization に加えて、open descriptor の metadata/data synchronization
operation `descriptor.sync` と success/error の drop boundary を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_syncs_descriptor_and_drops_resources` は二つの read-only named
  preopen を受け取り、最初の preopen から `input.txt` を `descriptor.open-at` で開く。
- `descriptor.sync` の `result<_, error-code>` discriminant が success になることを確認する。
  read-only descriptor でも POSIX-compatible host implementation が成功扱いにする契約を、単なる
  synthetic import ではなく実ファイルで固定する。
- descriptor と preopen を drop し、`wasi:cli/run` exit 0、stdout empty、host bytes unchanged
  (`hello`) を同一実行で確認する。

これは descriptor sync/drop の verified partial slice であり、残る descriptor operation、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ae 検証済み slice: descriptor set-size lifecycle (2026-07-24)

Stage 2ad の descriptor synchronization に加えて、write-enabled open descriptor の
`descriptor.set-size` operation と host artifact boundary を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_sets_descriptor_size_and_drops_resources` は二つの read-write
  named preopen を受け取り、最初の preopen から `input.txt` を write descriptor として開く。
- `descriptor.set-size(7)` の `result<_, error-code>` success discriminant を確認し、host file が
  `hello` から `hello\0\0` へ拡張されることを byte-for-byte で確認する。
- descriptor と preopen を drop し、`wasi:cli/run` exit 0、stdout empty、Component 実行後の host
  artifact を同一テストで確認する。

これは descriptor set-size の verified partial slice であり、残る descriptor operation、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2af 検証済み slice: descriptor create-directory-at lifecycle (2026-07-24)

Stage 2ae の file-size mutation に加えて、write-enabled preopen descriptor の
`descriptor.create-directory-at` path mutation と host directory artifact を actual Component で
検証した。

- `wasm_gc_component_cli_fs_runner_creates_directory_and_drops_resources` は二つの read-write named
  preopen を受け取り、最初の preopen descriptor に `created` path を渡す。
- `descriptor.create-directory-at` の `result<_, error-code>` success discriminant を確認し、host
  側に `created/` directory が生成されることを実行後に確認する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、host directory artifact を
  同じ実行で確認する。

これは descriptor create-directory-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ag 検証済み slice: descriptor remove-directory-at lifecycle (2026-07-24)

Stage 2af の directory creation に加えて、write-enabled preopen descriptor の
`descriptor.remove-directory-at` path mutation と host directory deletion artifact を actual Component
で検証した。

- `wasm_gc_component_cli_fs_runner_removes_directory_and_drops_resources` は二つの read-write named
  preopen を受け取り、fixture の `to-remove/` directory を最初の preopen descriptor から削除する。
- `descriptor.remove-directory-at` の `result<_, error-code>` success discriminant を確認し、host
  側の directory が実行後に存在しないことを確認する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、削除済み host artifact を
  同じ実行で確認する。

これは descriptor remove-directory-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ah 検証済み slice: descriptor unlink-file-at lifecycle (2026-07-24)

Stage 2ag の directory removal に加えて、write-enabled preopen descriptor の
`descriptor.unlink-file-at` path mutation と host file deletion artifact を actual Component で
検証した。

- `wasm_gc_component_cli_fs_runner_unlinks_file_and_drops_resources` は二つの read-write named
  preopen を受け取り、fixture の `to-unlink.txt` file を最初の preopen descriptor から削除する。
- `descriptor.unlink-file-at` の `result<_, error-code>` success discriminant を確認し、host
  側の file が実行後に存在しないことを確認する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、削除済み host artifact を
  同じ実行で確認する。

これは descriptor unlink-file-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2ai 検証済み slice: descriptor rename-at lifecycle (2026-07-24)

Stage 2ah の file unlink に加えて、二つの read-write named preopen と同一 directory 内の
`descriptor.rename-at` path mutation を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_renames_file_and_drops_resources` は最初の preopen descriptor
  に `old.txt` を渡し、同じ descriptor を destination base として `renamed.txt` へ rename する。
- `descriptor.rename-at` の `result<_, error-code>` success discriminant を確認し、host 側で
  source が消え、destination の bytes が `hello` のまま保持されることを確認する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、rename 済み host artifact を
  同じ実行で確認する。

これは descriptor rename-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2aj 検証済み slice: descriptor symlink-at lifecycle (2026-07-24)

Stage 2ai の file rename に加えて、write-enabled preopen descriptor の
`descriptor.symlink-at` path mutation と host symbolic-link artifact を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_creates_symlink_and_drops_resources` は二つの read-write named
  preopen を受け取り、最初の preopen descriptor に `target.txt` を old path、`link.txt` を new path
  として渡す。
- `descriptor.symlink-at` の `result<_, error-code>` success discriminant を確認し、host 側の
  symlink target が `target.txt`、symlink 経由の bytes が `hello` になることを確認する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、symlink 済み host artifact を
  同じ実行で確認する。

これは descriptor symlink-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2ak 検証済み slice: descriptor readlink-at lifecycle (2026-07-24)

Stage 2aj の symbolic-link creation に加えて、host symlink の target string を
`descriptor.readlink-at` の Component canonical string result から読み戻す境界を actual Component で
検証した。

- `wasm_gc_component_cli_fs_runner_reads_symlink_target_and_drops_resources` は二つの read-write
  named preopen を受け取り、fixture の `link.txt -> target.txt` symlink を最初の preopen descriptor
  から読む。
- `descriptor.readlink-at` の `result<string, error-code>` success discriminant と string payload
  (`ptr`, `len`) を guest linear memory から lift し、custom stdout へ `target.txt` を出力する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、host symlink target unchanged、stdout
  `target.txt` を同じ実行で確認する。

これは descriptor readlink-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2al 検証済み slice: descriptor link-at lifecycle (2026-07-24)

Stage 2ak の symbolic-link readback に加えて、old-path-flags を明示した
`descriptor.link-at` hard-link mutation と host artifact を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_creates_hard_link_and_drops_resources` は二つの read-write named
  preopen を受け取り、最初の preopen descriptor に old-path-flags `0`、`source.txt`、同じ
  destination base、`hardlink.txt` を渡す。
- `descriptor.link-at` の `result<_, error-code>` success discriminant を確認し、host 側で source
  と hard link の両方が存在し、bytes `hello` を保持することを確認する。
- preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、hard-link host artifact を
  同じ実行で確認する。

これは descriptor link-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2am 検証済み slice: descriptor is-same-object lifecycle (2026-07-24)

Stage 2al の hard-link artifact に加えて、同一 underlying object を指す二つの descriptor を
`descriptor.is-same-object` で比較する Component 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_compares_same_file_descriptors_and_drops_resources` は host fixture
  の `source.txt` と hard link `hardlink.txt` を二つの descriptor として同じ preopen から開く。
- `descriptor.is-same-object` の bool result が true になることを確認し、`wasi:cli/run` exit 0 と
  source/hard-link の bytes `hello` unchanged を同じ実行で確認する。
- 二つの file descriptor と preopen descriptor を drop し、stdout empty と resource lifecycle を
  確認する。

これは descriptor is-same-object の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2an 検証済み slice: descriptor metadata-hash lifecycle (2026-07-24)

Stage 2am の object identity に加えて、descriptor metadata の 128-bit hash を Component canonical
record として読み出す `descriptor.metadata-hash` 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_reads_stable_descriptor_metadata_hash_and_drops_resources` は
  `source.txt` を read-only descriptor として開き、二回の `metadata-hash` 呼び出しを実行する。
- 二回の `result<metadata-hash-value, error-code>` success discriminant と `lower` / `upper` の
  64-bit payload が一致することを guest linear memory 上で確認し、同一 metadata に対する stable
  hash の境界を固定する。
- descriptor と preopen descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、host bytes
  `hello` unchanged を同じ実行で確認する。

これは descriptor metadata-hash の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2ao 検証済み slice: descriptor metadata-hash-at lifecycle (2026-07-24)

Stage 2an の descriptor metadata hash に加えて、directory descriptor と相対 path から同じ
128-bit metadata hash を取得する `descriptor.metadata-hash-at` 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_reads_stable_metadata_hash_at_and_drops_resources` は named
  preopen の directory descriptor に path-flags `0`、`source.txt`、二つの retptr を渡す。
- 二回の `result<metadata-hash-value, error-code>` success discriminant と `lower` / `upper` の
  64-bit payload が一致することを guest linear memory 上で確認する。
- directory descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、host bytes `hello`
  unchanged を同じ実行で確認する。

これは descriptor metadata-hash-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2ap 検証済み slice: descriptor stat-at lifecycle (2026-07-24)

Stage 2ao の path-based metadata hash に加えて、directory descriptor と相対 path から
`descriptor.stat-at` の canonical `descriptor-stat` record を取得する境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_stats_file_at_and_drops_resources` は named preopen の directory
  descriptor に path-flags `0` と `source.txt` を渡す。
- `descriptor.stat-at` の `result<descriptor-stat, error-code>` success discriminant、regular-file
  type `6`、size `5` の record payload を guest linear memory の canonical offsets で確認する。
- directory descriptor を drop し、`wasi:cli/run` exit 0、stdout empty、host bytes `hello`
  unchanged を同じ実行で確認する。

これは descriptor stat-at の verified partial slice であり、残る descriptor operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity
は未完了である。

## Stage 2aq 検証済み slice: descriptor set-times-at lifecycle (2026-07-24)

Stage 2ap の path-based stat に加えて、write-enabled named preopen の directory descriptor から
`descriptor.set-times-at` の `new-timestamp` variant と result/drop 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_sets_file_times_at_without_changing_no_change_values` は read-write
  named preopen の directory descriptor に path-flags `0` と `source.txt` を渡し、access/modify の
  timestamp をともに `no-change` として呼び出す。
- `result<_, error-code>` success discriminant を確認し、directory descriptor を drop する。実行後の
  host file bytes `hello`、stdout empty、`wasi:cli/run` exit 0 を同じ実行で確認する。

これは descriptor set-times-at の verified partial slice であり、`now` / explicit `timestamp` payload、
descriptor.set-times、残る descriptor operation、Wasm artifact/runtime differential、Mac Apple
Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ar 検証済み slice: descriptor set-times lifecycle (2026-07-24)

Stage 2aq の path-based timestamp boundary に加えて、write-enabled named preopen の open descriptor
から `descriptor.set-times` の `new-timestamp` variant と result/drop 境界を actual Component で
検証した。

- `wasm_gc_component_cli_fs_runner_sets_descriptor_times_without_changing_no_change_values` は
  `input.txt` を write descriptor として開き、access/modify timestamp をともに `no-change` として
  呼び出す。
- `result<_, error-code>` success discriminant を確認し、file/preopen descriptor を drop する。実行後の
  host file bytes `hello`、stdout empty、`wasi:cli/run` exit 0 を同じ実行で確認する。

これは descriptor set-times の verified partial slice であり、`now` / explicit `timestamp` payload、
directory descriptor mutation、残る descriptor operation、Wasm artifact/runtime differential、Mac
Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2as 検証済み slice: descriptor advise lifecycle (2026-07-24)

Stage 2ar の descriptor timestamp boundary に加えて、read-only named preopen の open file descriptor
から `descriptor.advise` の `advice` enum、success result、resource-drop 境界を actual Component で
検証した。

- `wasm_gc_component_cli_fs_runner_advises_descriptor_and_drops_resources` は `input.txt` を read-only
  descriptor として開き、offset `0`、length `5`、`advice::normal` を渡す。
- `result<_, error-code>` success discriminant を確認し、file/preopen descriptor を drop する。実行後の
  host file bytes `hello`、stdout empty、`wasi:cli/run` exit 0 を同じ実行で確認する。

これは descriptor advise の verified partial slice であり、他の advice variant、range/error behavior、
残る descriptor operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
native/selfhost parity は未完了である。

## Stage 2at 検証済み slice: output-stream blocking-write-zeroes lifecycle (2026-07-24)

Stage 2as の descriptor advisory boundary に加えて、read-write named preopen の output stream から
`blocking-write-zeroes-and-flush` の zero-fill、flush、result/drop、host artifact 境界を actual Component
で検証した。

- `wasm_gc_component_cli_fs_runner_writes_zeroes_and_drops_resources` は `zeros.bin` を create+truncate+
  write で開き、`write-via-stream(0)` で得た output stream に 3 zero bytes の blocking write/flush を
  実行する。
- success result を確認して output stream、file descriptor、preopen を drop し、host file が `[0, 0, 0]`、
  stdout empty、`wasi:cli/run` exit 0 になることを同じ実行で確認する。

これは output-stream zero-fill の verified partial slice であり、直接 `write-zeroes` の check-write
contract、separate flush / blocking-flush、stream error、残る streams operation、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2au 検証済み slice: output-stream check-write/write/flush lifecycle (2026-07-24)

Stage 2at の zero-fill boundary に加えて、read-write named preopen の output stream から
`check-write` の permit、direct `write`、`flush`、`blocking-flush`、result/drop、host artifact 境界を
actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_checks_writes_and_flushes_stream_then_drops_resources` は
  `checked.txt` を create+truncate+write で開き、`write-via-stream(0)` で得た output stream に
  `check-write` を実行する。
- 正の permit を確認した後、permit 以下の `hello` を直接 `write` し、`flush` と `blocking-flush` の
  success result を確認する。output stream、file descriptor、preopen を drop し、host file が
  `hello`、stdout empty、`wasi:cli/run` exit 0 になることを同じ実行で確認する。

これは output-stream readiness/write/flush の verified partial slice であり、`write-zeroes` の
check-write contract、stream error/resource failure、`subscribe`/poll readiness、`splice`、input
stream の残る operation、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、
native/selfhost parity は未完了である。

## Stage 2av 検証済み slice: output-stream direct write-zeroes lifecycle (2026-07-24)

Stage 2au の readiness/write/flush boundary に加えて、read-write named preopen の output stream
から `check-write` の permit を取得して直接 `write-zeroes` を行う precondition、
`blocking-flush`、result/drop、host artifact 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_writes_zeroes_after_check_write_then_drops_resources` は
  `direct-zeroes.bin` を create+truncate+write で開き、`write-via-stream(0)` で得た output stream に
  `check-write` を実行する。
- 4 bytes 以上の permit を確認した後、`write-zeroes(4)` と `blocking-flush` の success result を
  確認する。output stream、file descriptor、preopen を drop し、host file が `[0, 0, 0, 0]`、
  stdout empty、`wasi:cli/run` exit 0 になることを同じ実行で確認する。

これは output-stream direct zero-fill の verified partial slice であり、stream error/resource
failure、zero-length write、`subscribe`/poll readiness、`splice`、input stream の残る operation、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了
である。

## Stage 2aw 検証済み slice: output-stream splice / blocking-splice lifecycle (2026-07-24)

Stage 2av の direct zero-fill boundary に加えて、read-write named preopen の input/output stream
から borrowed input resource を渡す `splice`、non-blocking の partial-transfer を補完する
`blocking-splice`、result/drop、host artifact 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_splices_input_into_output_and_drops_resources` は `input.txt` を
  read descriptor、`spliced.txt` を create+truncate+write descriptor として開き、それぞれ
  `read-via-stream(0)` / `write-via-stream(0)` で input/output stream を取得する。
- `output-stream.splice(5)` の success result を確認した後、`blocking-splice(5)` を呼ぶ。direct
  operation が要求長未満を返す場合も含め、host file が `hello`、stdout empty、`wasi:cli/run` exit 0
  になることを同じ実行で確認し、全 resource を drop する。

これは output/input splice の verified partial slice であり、exact transferred-byte count、stream
error/resource failure、zero-length splice、poll readiness、input stream の残る operation、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ax 検証済み slice: input-stream skip / blocking-skip lifecycle (2026-07-24)

Stage 2aw の splice boundary に加えて、read-only named preopen の input stream から non-blocking
`skip` の partial result、残量を補完する `blocking-skip`、`blocking-read` の host stdout artifact、
result/drop 境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_skips_input_stream_then_reads_remaining_bytes` は `input.txt`
  (`hello!`) を開き、`read-via-stream(0)` で input stream を取得する。
- `skip(2)` の success count を受け取り、`2 - count` を `blocking-skip` に渡して合計 2 bytes を消費
  する。続く `blocking-read(4)` が `llo!` を返し、stdout、`wasi:cli/run` exit 0、input stream /
  descriptor / preopen drop を同じ実行で確認する。

これは input-stream skip の verified partial slice であり、stream error/resource failure、EOF/
zero-length skip、poll readiness、`read` の non-blocking data contract、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ay 検証済み slice: input-stream read / blocking-read lifecycle (2026-07-24)

Stage 2ax の skip boundary に続き、read-only named preopen の input stream で non-blocking
`read` の上限・空リスト契約と、残量を `blocking-read` で補完する Component 境界を検証した。

- `wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof`
  は `input.txt` (`hello`) を開き、`read-via-stream(0)` で input stream を取得する。
- `read(0)` が success かつ空 list を返すことを確認した後、`read(5)` の list length が要求値を
  超えないことを確認し、取得した bytes を stdout に渡す。
- `5 - first_read_len` を `blocking-read` に渡し、残りの bytes を stdout に渡した後、input
  stream、descriptor、preopen を drop して `wasi:cli/run` exit 0、stdout `hello` を確認する。

これは input-stream read の verified partial slice であり、stream error/closed、複数回の partial read、
poll readiness、Wasm artifact/runtime differential、Mac Apple
Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2az 検証済み slice: input-stream EOF read (2026-07-24)

Stage 2ay の `hello` fixture を読み切った後、non-blocking `read(1)` が stream error ではなく
success の空 list を返す EOF boundary を同じ actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof`
  は `read(0)`、`read(5)`、残量の `blocking-read` で `hello` を消費する。
- 直後に `read(1)` を呼び、result discriminant が success、list length が 0 であることを確認し、
  EOF 確認 marker `E` を stdout に追加する。input stream、descriptor、preopen の drop と
  `wasi:cli/run` exit 0、stdout `helloE` を同じ実行で確認する。

これは regular-file EOF の verified partial slice であり、stream error/closed、複数回の partial read、
poll readiness、Wasm artifact/runtime differential、Mac Apple
Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2ba 検証済み slice: input-stream empty source read (2026-07-24)

空の regular file を input stream として開いた場合も、non-blocking `read(1)` が stream error
ではなく success の空 list を返す境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_reads_empty_input_stream_as_empty_success` は空の `input.txt`
  を read-only named preopen から開き、`read-via-stream(0)` で input stream を取得する。
- `read(1)` の result discriminant が success、list length が 0 であることを確認し、empty-source
  marker `Z` を stdout に渡す。input stream、descriptor、preopen を drop して `wasi:cli/run`
  exit 0、stdout `Z` を同じ実行で確認する。

これは empty source の verified partial slice であり、stream error/closed、複数回の partial read、
poll readiness、Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost
parity は未完了である。

## Stage 2bb 検証済み slice: input-stream pollable empty/EOF readiness (2026-07-24)

空の regular file から作った input stream でも、`subscribe` → `pollable.block` が完了し、
`pollable.ready` が true になる EOF/readiness boundary を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_polls_empty_input_stream_as_ready` は空の `input.txt` を
  read-only named preopen から開き、`read-via-stream(0)` → `input-stream.subscribe` を呼ぶ。
- `pollable.block` 後に `pollable.ready` が true であることを確認し、ready marker `R` を stdout に
  渡す。pollable、input stream、descriptor、preopen を drop して `wasi:cli/run` exit 0、stdout
  `R` を確認する。既存の非空 fixture でも同じ marker を観測し、ready path を実行証跡にする。

これは input-stream pollable の empty/EOF readiness verified partial slice であり、stream error/
closed、poll list の empty list input trap、Wasm artifact/runtime differential、Mac Apple Silicon/Linux
x86_64、native/selfhost parity は未完了である。

## Stage 2bc 検証済み slice: poll list empty/EOF readiness (2026-07-24)

`wasi:io/poll.poll` に渡す borrowed pollable list でも、empty input stream の EOF readiness が
ready index として返ることを actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_polls_empty_input_stream_list_as_ready` は空の `input.txt` から
  input stream と pollable を作り、`pollable.block` → `pollable.ready` の後に list `[pollable]` を
  `poll` へ渡す。
- `poll` の result list length `1` と ready index `0` を確認し、marker `P` を stdout に渡す。
  pollable、input stream、descriptor、preopen を drop して `wasi:cli/run` exit 0、stdout `P` を
  確認する。非空 `hello` fixture でも同じ marker を観測する。

これは poll list の empty/EOF readiness verified partial slice であり、stream error/closed、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux
x86_64、native/selfhost parity は未完了である。

## Stage 2bd 検証済み slice: poll list empty input trap (2026-07-24)

`wasi:io/poll.poll` の空 list 入力が trap になる契約を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_traps_on_empty_poll_list` は空の `input.txt` から input stream と
  pollable を作り、list 長 `0` の borrowed pollable list を `poll` に渡す。
- Component 実行が正常な `ExecutionOutput` ではなく error になり、error 境界に `poll` が含まれることを
  確認する。これは空 list trap が `wasi:cli/run` の成功や stdout marker に変換されないことを示す。

これは poll list の empty input trap verified partial slice であり、stream error/closed、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2be 検証済み slice: input-stream blocking-read EOF closed (2026-07-24)

empty source に対する `input-stream.blocking-read` は、non-blocking `read` のように success の空 list
を返すのではなく、`stream-error::closed` を返す契約を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_blocking_reads_empty_input_stream_reports_closed` は空の
  `input.txt` から input stream を作り、`blocking-read(1)` を呼ぶ。
- outer result が error になり、`stream-error` の closed case（discriminant `1`）を確認して marker `C`
  を stdout に渡す。input stream、descriptor、preopen を drop して `wasi:cli/run` exit 0、stdout `C`
  を同じ実行で確認する。

これは blocking-read EOF/closed の verified partial slice であり、`last-operation-failed` と
filesystem error-code downcast、複数回の blocking partial read、Wasm artifact/runtime differential、
Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2bf 検証済み slice: poll list multiple ready indices (2026-07-24)

`wasi:io/poll.poll` に同じ input stream から派生した二つの pollable を渡した場合、両方の ready
index が result list に返ることを actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_polls_multiple_input_stream_pollables_as_ready` は空の
  `input.txt` から input stream を作り、二つの `input-stream.subscribe` を呼ぶ。
- 二つの pollable をそれぞれ `block` / `ready` で確認し、`poll` の result list length `2`、index `0`
  と `1` を確認する。marker `P` を stdout に渡し、二つの pollable、input stream、descriptor、preopen
  を drop して `wasi:cli/run` exit 0 を確認する。

これは poll list の複数 ready index verified partial slice であり、`last-operation-failed` と
filesystem error-code downcast、異なる input source の複数 pollable、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2bg 検証済み slice: poll list multiple input sources (2026-07-24)

同じ preopen 配下の異なるファイルから作った二つの input stream を poll list に渡した場合、
両方の ready index `0` / `1` が result list に返ることを actual Component で検証した。WASI の
contract は index の集合を定めるため、host の内部走査順に依存せず、`[0, 1]` / `[1, 0]` の両方を受け入れる。

- `wasm_gc_component_cli_fs_runner_polls_multiple_input_sources_as_ready` は `source-a.txt` と
  `source-b.txt` をそれぞれ descriptor 経由で `read-via-stream` し、独立した二つの
  `input-stream.subscribe` を呼ぶ。両方を EOF の fixture として source の違いに焦点を固定する。
- 二つの pollable を `block` / `ready` で確認して `poll` の result list length `2`、ready index の値が
  `0` と `1` の集合であることを guest 側で検証する。二つの pollable、stream、descriptor、preopen の
  drop を先に完了してから marker `P` を stdout に渡し、exit code `0` を同じ actual Component 実行で
  確認する。

これは異なる input source の複数 ready index verified partial slice であり、pollable が異なる
ready/error 状態になる場合の projection、Wasm artifact/runtime differential、Mac Apple
Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2bh 検証済み slice: input-stream last-operation-failed の filesystem error-code downcast (2026-07-24)

filesystem descriptor から作った input stream の `blocking-read` が返す
`stream-error::last-operation-failed` を、`wasi:filesystem/types` の
`filesystem-error-code` で filesystem `error-code` へ downcast できる境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_maps_stream_failure_to_filesystem_error_code` は read-only preopen の
  `input.txt` から descriptor/input stream を作り、offset `u64::MAX` の `read-via-stream` に続けて
  `blocking-read(1)` を呼ぶ。失敗の outer result と `stream-error` の
  `last-operation-failed` case（discriminant `0`）を確認する。
- error resource を borrowed `filesystem-error-code` に渡し、option が `Some`、payload が
  `error-code::invalid`（discriminant `12`）であることを確認する。marker `E`、`wasi:cli/run` exit
  code `0`、error/input stream/descriptor/preopen の drop を同じ actual Component 実行で固定する。

これは filesystem read の invalid-offset downcast verified partial slice であり、他の OS error mapping、
Wasm artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2bi 検証済み slice: output-stream last-operation-failed の filesystem error-code downcast (2026-07-24)

filesystem descriptor から作った output stream の `blocking-write-and-flush` が返す
`stream-error::last-operation-failed` も、`filesystem-error-code` で filesystem `error-code` へ
downcast できる境界を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_maps_output_stream_failure_to_filesystem_error_code` は read-write
  preopen の `output.txt` に offset `u64::MAX` の `write-via-stream` を作り、`blocking-write-and-flush(1)`
  を呼ぶ。outer result が error、`stream-error` の `last-operation-failed` case（discriminant `0`）を確認する。
- error resource を borrowed `filesystem-error-code` に渡し、option が `Some`、payload が
  `error-code::invalid`（discriminant `12`）であることを確認する。marker `O`、`wasi:cli/run` exit
  code `0`、error/output stream/descriptor/preopen の drop を同じ actual Component 実行で固定する。

これは filesystem output write の invalid-offset downcast verified partial slice であり、非同期
`write` 後の `check-write` error、他の OS error mapping、Wasm artifact/runtime differential、Mac
Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2bj 検証済み slice: 非同期 output-stream write 後の blocking-flush failure downcast (2026-07-24)

output stream の non-blocking `write` が開始した filesystem I/O を `blocking-flush` で待機し、失敗を
`stream-error::last-operation-failed` として受け取って `filesystem-error-code` へ downcast できる
状態遷移を actual Component で検証した。

- `wasm_gc_component_cli_fs_runner_maps_async_output_stream_failure_to_filesystem_error_code` は
  read-write preopen の `output.txt` に offset `u64::MAX` の output stream を作る。`check-write` の
  permit を確認してから `write(1)` を開始し、`blocking-flush` が完了待ち後に返す outer error と
  `last-operation-failed` case（discriminant `0`）を確認する。
- error resource を borrowed `filesystem-error-code` に渡し、option が `Some`、payload が
  `error-code::invalid`（discriminant `12`）であることを確認する。marker `A`、`wasi:cli/run` exit
  code `0`、error/output stream/descriptor/preopen の drop を同じ actual Component 実行で固定する。

これは非同期 output write → blocking-flush の invalid-offset downcast verified partial slice であり、
pending write 後の non-blocking `flush` projection、他の OS error mapping、Wasm artifact/runtime
differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## Stage 2bk 検証済み slice: pending output-stream check-write failure の filesystem error-code downcast (2026-07-24)

output stream の non-blocking `write` を開始した後、`output-stream.subscribe` から
`pollable.block` で I/O 完了を待ち、再度 `check-write` を呼ぶ pending failure 状態を actual
Component で検証した。

- `wasm_gc_component_cli_fs_runner_maps_pending_output_stream_failure_to_filesystem_error_code` は
  read-write preopen の `output.txt` に offset `u64::MAX` の output stream を作る。`check-write` の
  permit を確認してから `write(1)`、`output-stream.subscribe`、`pollable.block` を順に呼び、完了後の
  `check-write` が返す outer error と `stream-error::last-operation-failed` case（discriminant `0`）を
  確認する。`result<u64, stream-error>` の 8-byte payload alignment を保った canonical ABI layout
  （stream-error case `+8`、error handle `+12`）もこの probe で固定する。
- error resource を borrowed `filesystem-error-code` に渡し、option が `Some`、payload が
  `error-code::invalid`（discriminant `12`）であることを確認する。marker `C`、`wasi:cli/run` exit
  code `0`、error/pollable/output stream/descriptor/preopen の drop を同じ actual Component 実行で
  固定する。

これは pending output write → subscribe/block → check-write の invalid-offset downcast verified
partial slice であり、non-blocking `flush` 後の projection、他の OS error mapping、Wasm
artifact/runtime differential、Mac Apple Silicon/Linux x86_64、native/selfhost parity は未完了である。

## 実装戦略

### Stage 0: backend フラグの配線

- CLI には既に 4 target (wasi-preview1 / wasi-component / web-wasm / native) の分岐が
  `crates/lsharp-driver/src/main.rs` の compile コマンド処理にある。`--backend=wasmgc`
  (デフォルト: linear) の選択と `web-wasm` target 制約まで検証済みであり、records/ADT の
  lowering 分岐と runtime 境界は後続 stage とする
- 新規ファイル `crates/lsharp-wasm/src/wasmgc.rs` に `emit_wasm_wasmgc()` を新設
  (`emit_wasm_wasi` と同じシグネチャ。imp-06 のファイル分割方針に合わせ 800 行以内で分割)

### Stage 1: Records / ADT → struct 型

1. lowering: record literal/direct field access、field accessor、record update の WasmGC 型変換は
   Stage 1.75 で検証済み。残る ADT 構築・フィールドアクセス・pattern を
   `StructNew` / `StructGet` / `StructSet` / `RefCast` IR 命令 + `Module.gc_types` へ接続する
   (IR 命令は定義済みのため lowering の分岐追加のみ)
2. emit: `wasmgc.rs` で型セクションに struct 型を出力し、4 命令を
   `struct.new type_idx` / `struct.get type_idx field_idx` / `struct.set` / `ref.cast` へ変換。
   `ir_to_wasm_valtype` の wasmgc 版は `Ref(idx) => ValType::Ref(RefType { heap_type: Concrete(idx), nullable: true })`
3. ADT は v2-07 のとおり tagged struct (フィールド 0 にタグ i32、以降ペイロード)。
   パターンマッチの lowering はタグ読み出し (`StructGet(t, 0)`) + `RefCast` で分岐
4. 検証: `examples/gadt.ls` の「型チェックのみ」スタブを外し、
   `--backend=wasmgc` での実行 E2E を追加 (D-02 の resolved 条件)

### Stage 2: Strings → array i8

- 文字列リテラル/操作を `array.new_data` / `array.get_u` / `array.len` へ。
  WASI fd_write へ渡す際は array → リニアメモリへのコピーが必要
  (`array.copy` 不可のため要素ループまたは `array.init_data` の逆操作を helper 化)
- Stage 2k の明示拒否、Stage 2l の `(ptr, len)` ABI、Stage 2m の GC array→linear-memory copy、
  Stage 2n の fd_write handler semantics、Stage 2o の custom Component instantiate、Stage 2p の
  Preview2 stdout stream 接続、Stage 2q の custom CLI `wasi:cli/run` 接続を前提に、次の
  WASI/component adapter 実装を observable contract ごとに分ける。次に fd table/rights と
  proc-exit/error result parity を閉じ、最後に Preview2 artifact/runtime と native/selfhost parity
  を検証する。
  synthetic import の instantiate 成功、host callback 単体の byte read、core runner 単体の success、
  writer adapter 単体の success は、公開 component print 完了の証拠に数えない。

### Stage 3: Closures → funcref + env struct

- 現行の lambda lifting (`lower/closure.rs`) は維持し、env をリニアメモリ tuple から
  struct 型へ置換。呼び出しは `call_ref` (typed funcref)。
  `IrType::FuncRef` は既存のため IR 拡張は env の型インデックス保持のみ
- 検証: `examples/hkt.ls` / `examples/computation.ls` の実行 E2E (D-03 / D-04 の resolved 条件)

### Stage 4: トレイト vtable

- 現行のマングル名静的解決 (`TraitName_TypeName_methodName`) は維持 (性能上有利)
- 追加で「trait object 型」を導入する場合のみ vtable struct
  (メソッドごとの funcref フィールド) + `call_ref` を生成。
  言語構文 (存在型) の設計は本書スコープ外とし、まず IR/emit の機構のみ用意する
  (D-06 の resolved 条件は「動的ディスパッチの最小 E2E」)

### Stage 5: selfhost ADT 表現の切替 (D-09)

- selfhost コンパイラ (selfhost/src/) の整数タグ + Vector 表現を struct ベースへ移行。
  bootstrap 固定点 (stage chain) の再生成・一致検証が必須のため、Stage 1-4 が
  安定した後に単独で計画する (本書では方針のみ固定)

## 無回帰戦略

- デフォルト backend は linear のまま。既存 E2E は全件無変更で走り続ける
- wasmgc backend の E2E は既存シナリオのパラメタライズ追加とし、
  stdout の byte 一致を両 backend で比較する差分テストを置く
- `emit.rs:195-212` のスタブは linear backend が StructNew 系命令を受け取らない限り
  そのまま残してよい。Stage 1 で lowering が分岐した後、linear 経路で当該命令が
  生成されないことを debug_assert で固定し、最終的に
  「wasmgc backend では `crates/lsharp-wasm/src/emit.rs` の TODO 4 件に到達しない」
  ことをもって D-01 を resolved とする

## 完了条件 (issue との対応)

| Stage | 解消 issue | 証跡 |
|-------|-----------|------|
| 1 | D-01 (主要部), D-02 | gadt.ls 実行 E2E、struct 型セクションのスナップショット |
| 2 | D-01 (残部) | 文字列系 E2E の両 backend green |
| 3 | D-03, D-04 | hkt.ls / computation.ls 実行 E2E |
| 4 | D-06 | 動的ディスパッチ最小 E2E |
| 5 | D-09 | bootstrap 固定点の wasmgc backend 一致 |

## ステータス

Stage 0 (依存 API / runtime capability probe)、Stage 1 の IR emitter、Stage 1.5 の CLI 選択、
Stage 1.75 の direct record lowering/update/user call、scalar ADT constructor/pattern、nested ADT
payload/pattern、nullable ADT reference payload、scalar literal pattern、record pattern field check
slice、typed type-application payload slice、scalar GADT refinement execution slice、computation
return-only slice と bind 明示拒否境界、Stage 2a の scalar String GC array slice、Stage 2b の
scalar String GC equality slice、Stage 2c の scalar String GC concat slice、Stage 2d の scalar
String GC substring slice、Stage 2e の packed String byte array slice、Stage 2f の substring invalid
range boundary、Stage 2g の `print-string` external import boundary、Stage 2h の host-side packed
String read、Stage 2i の WasmGC runner stdout sink、Stage 2j の `std::io::Write` adapter、Stage 2k の
WasmGC Component bridge 明示拒否、Stage 2l の output `list<u8>` canonical pair 契約、Stage 2m
の packed GC array→linear-memory output bridge、Stage 2n の fd_write handler 契約、Stage 2o の
custom `wasmgc-output` Component actual instantiate、Stage 2p の Preview2 stdout stream 接続、
Stage 2q の custom CLI `wasi:cli/run` 接続は 2026-07-24 に検証済み。ADT
の全表現、Stage 2 の残り (Unicode code-point semantics / WASI-component I/O /
native-selfhost parity)、Stage 3 以降
(closures / traits / selfhost)、supported target
の actual runtime evidence は未完了であり、`LEGACY-EXEC-01` の完了条件には到達していない。

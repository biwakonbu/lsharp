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
String GC substring slice は 2026-07-24 に検証済み。ADT の全表現、Stage 2 の残り
(packed strings / invalid-range diagnostics / I/O)、Stage 3 以降
(closures / traits / selfhost)、supported target
の actual runtime evidence は未完了であり、`LEGACY-EXEC-01` の完了条件には到達していない。

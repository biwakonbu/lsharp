# Native Backend 仕様

> **Status: Native-only official replacement track active (2026-05-09)**
>
> V2-08〜V2-10 で Darwin arm64 の actual native self-regeneration と experimental native-only RC は完了した。
> 次の目標は native-only official replacement track として、host launcher + embedded guest component 配布を rollback compatibility へ降格し、native-only を公式配布へ完全置換すること。
> ただし Tier1 target matrix には未完了 blocker が残るため、V2-13〜V2-15 を TODO.md の正本に積む。
> 詳細は `TODO.md` の現在の残タスク一覧、`docs/development/planning/v2-designs/v2-08-native-backend-self-regeneration.md`、`docs/development/planning/v2-designs/v2-09-wasm-native-differential-zero.md`、`docs/development/planning/v2-designs/v2-10-native-only-rc-distribution.md` および `backend-boundary.md` を参照。

## 目的

本書は、L# compiler の native backend が満たすべき契約を定義する。
native backend は既存の `LoweredModule` を入力として受け取り、target ごとの object file と最終バイナリを生成する。

## 適用範囲

本書が扱うのは次の領域である。

- native backend の対象ターゲット
- internal ABI と external ABI
- stack frame と GC-safe point の前提
- object emitter と linker 連携
- 決定的コード生成の要件

以下は v1 の対象外だったが、Native-only official replacement track では Tier1 完全置換 blocker として扱う。

- tail call 最適化
- デバッグ情報の完全サポート
- C ABI を越えた汎用 FFI 面の拡張
- JIT や動的ロード
- Windows 向けネイティブ成果物 (**BLOCKED**: `x86_64-pc-windows-msvc` の object/link/runtime 契約が未実装)

## パイプライン上の位置づけ

```text
LoweredModule
  -> NativeCodegen
  -> NativeArtifact (.o)
  -> Linker
  -> Native Binary
```

native backend は `LoweredModule` を入力とし、frontend や lowering の意味解析を再実装しない。
IR の意味を変える必要がある場合は、backend 内で吸収せず `backend-boundary.md` の境界設計へ戻る。

## 対象ターゲット

v1 で対象とするターゲットは次のとおりである。

- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-linux-gnu`

Native-only official replacement track で公式 Tier1 置換に必要な target matrix は次のとおりである。

| target | 現状 | 置換 blocker |
|---|---|---|
| `aarch64-apple-darwin` | actual native self-regeneration / experimental RC 完了 | stable 公式導線への昇格 |
| `x86_64-apple-darwin` | spec 対象、実行 artifact coverage は未完了 | actual native self-regeneration と release smoke |
| `x86_64-unknown-linux-gnu` | active priority: Linux x86_64 server priority track で先行検証中 | actual Linux native self-regeneration / runtime-link smoke |
| `x86_64-pc-windows-msvc` | **BLOCKED** | COFF/PE runtime/link/smoke と Authenticode gate |

### Linux x86_64 server priority track

サーバー用途を優先するため、`x86_64-unknown-linux-gnu` は full Tier1 公式置換より先に V2-13a として切り出す。Ubuntu x86_64 VM / GitHub Actions `ubuntu-latest` runner を正本の実行環境とし、まず `NativeTarget` descriptor、ELF object emitter、x86_64 codegen exact-byte smoke を `native-linux-x86-smoke` で required CI に固定する。

開発中の inner loop は GitHub Actions ではなくローカル VM で回す。`scripts/ci/native-linux-x86-local-vm-smoke.sh` は Linux x86_64 VM 上で descriptor / ELF emitter と canonical `program.o` / `runtime.o` / `linker-response.txt` / `program.native` runtime-link smoke を短時間で確認する。QEMU x86_64 VM では selfhost exact-byte suite が重いため、local smoke には含めず、actual native self-regeneration の調査へ進む前の fast gate として扱う。

host 側の selfhost `emit-native` で生成した Linux x86_64 code artifact を VM 内でリンク・実行する split smoke は `scripts/ci/native-linux-x86-hostgen-vm-exec.sh` で固定する。このスクリプトは `LSHARP_NATIVE_LINUX_X86_CODE_ARTIFACT` を指定して host-side selfhost artifact generation test を実行し、`limactl` 経由で Ubuntu x86_64 VM に `code.bin` を渡し、VM 内で `program.native` の `actual_exit_code` を確認する。

この track の受理は Linux VM 上の証跡を必須にする。macOS arm64 ローカルだけでは Linux ELF runtime/link smoke と actual native self-regeneration を完了扱いにしない。

追加ターゲットを導入する場合でも、`LoweredModule` や runtime 契約の共有を前提にしなければならない。

## Target descriptor schema

`NativeTarget` は単なる triple 文字列ではなく、backend 差分を閉じ込めるための **target descriptor** として扱う。v1 では少なくとも次の論理項目を持つことを前提にする。

| 区分 | 項目 | 役割 |
|------|------|------|
| identity | `triple_id`, `arch`, `os`, `object_format` | target の基本識別 |
| ABI | `word_size`, `endianness`, `stack_alignment` | 値表現とフレーム生成の基礎 |
| calling convention | `arg_registers`, `return_registers`, `callee_saved`, `caller_saved` | 関数呼び出し規約 |
| emit policy | `text_section`, `rodata_section`, `data_section`, `bss_section`, `symbol_prefix`, `visibility_policy` | object 出力時の section / symbol 規約 |
| relocation policy | `reloc_call`, `reloc_abs`, `reloc_data` | codegen / emit が使う relocation 種別 |
| toolchain | `linker_flavor`, `response_file_style` | 最終 link の実行方法 |
| runtime | `runtime_object_name`, `runtime_symbol_prefix`, `gc_root_policy` | `runtime.o` との接続規約 |

実装言語上の表現は struct, record, vector のいずれでもよいが、**codegen / emit / linker 本体が target 固有 if を散らさず、この descriptor を参照するだけで切り替わること**を acceptance とする。

## v1 delivery model

v1 の native backend は、**compiler core が全てを自前で完結させること**よりも、**selfhost 後も維持できる artifact 契約を固定すること**を優先する。

責務分担は次で固定する。

- **compiler core (`NativeTarget` / `NativeCodegen` / `NativeEmit` / `Linker`)**
  - `LoweredModule` から target-specific native artifact を生成する
  - `program.o`, `linker-response.txt`, `program.native` の契約を決める
  - target 差分を descriptor と runtime boundary に閉じ込める
  - source order / stable sort / no timestamp の determinism を守る
- **external toolchain**
  - 最終 link を担当する
  - Darwin では `ld64`、Linux では `ld.lld` を優先し、必要に応じて `ld` fallback を許容する
- **runtime artifact**
  - `runtime.o` は compiler core に埋め込まず、target ごとの別 artifact として扱う
  - tier1 配布物では対応する `runtime.o` を同梱することを推奨する
  - 開発中の shadow path では、CI や補助 build step による `runtime.o` 生成を許容する

Mach-O / ELF の完全手書きは v1 の必須要件ではない。shadow path を閉じるために必要であれば、`NativeEmit` が補助的な外部 object-generation path を使って `program.o` へ到達してよい。ただし default-path cutover 前には、product path の artifact 契約と再現性を固定しなければならない。

## ABI 契約

### Internal ABI

L# 内部の呼び出し規約は、少なくとも次を満たす。

- 引数と戻り値は machine word 単位で受け渡す
- 複合値は pointer 経由で扱う
- 複数戻り値は v1 では導入しない
- タグ付き word 表現を保持する
- レジスタ利用は caller-save 優先とする
- tail call は v1 では前提にしない

### External ABI

native backend の外部境界は runtime を介して定義する。論理的な runtime API (`alloc_words`, `alloc_bytes`, `print`, `read_file`, `clock_now_millis` など) は、native 側では `lsharp_` 接頭辞付き symbol へ写像する。

- エントリポイントは `runtime init -> L# main 呼び出し` の thin stub で構成する
- 外部公開シンボルは runtime boundary に限定する
- CLI、LSP、REPL は同一 compiler core の別エントリとして構成する
- C ABI 互換は runtime boundary でのみ保証する

v1 で想定する代表的な公開シンボルは次のとおりである。

- `lsharp_runtime_init`
- `lsharp_alloc_words`
- `lsharp_alloc_bytes`
- `lsharp_print`
- `lsharp_eprint`
- `lsharp_read_file`
- `lsharp_write_file`
- `lsharp_file_exists`
- `lsharp_read_dir`
- `lsharp_clock_now_millis`
- `lsharp_root_push`
- `lsharp_root_pop`
- `lsharp_root_set`

## ターゲット別 calling convention

### x86_64

| 項目 | 規約 |
|------|------|
| 引数 | `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` |
| 戻り値 | `rax` |
| callee-save | `rbx`, `rbp`, `r12`-`r15` |
| stack alignment | 16-byte |

### aarch64

| 項目 | 規約 |
|------|------|
| 引数 | `x0`-`x7` |
| 戻り値 | `x0` |
| callee-save | `x19`-`x28` |
| stack alignment | 16-byte |

Linux `x86_64` でも、基本的な word 単位の ABI 契約は同様に扱う。

## スタックフレームと GC

### Stack frame

stack frame は概ね次の構成を持つ。

- return address
- saved registers
- local slots
- spill slots
- outgoing argument area

### GC-safe point

GC-safe point は runtime 仕様と揃え、少なくとも次に置く。

1. 関数呼び出しの前後
2. loop backedge

native backend は、これらの地点で root 情報が正しく追跡できるコードを生成しなければならない。

## Object Emitter

native backend は relocation 付き object file を出力する。
Mach-O と ELF の差分は target descriptor に閉じ込め、codegen の共通部分からは section 名や relocation 種別の差分を直接参照しない。

標準的な artifact セットは次のとおりである。

| 成果物 | 役割 |
|--------|------|
| `program.o` | ユーザープログラム本体 |
| `runtime.o` | runtime 実装 |
| `linker-response.txt` | linker へ渡す補助情報 |
| `program.native` | 最終ネイティブバイナリ |

Mach-O / ELF を完全に手書きすることは v1 の必須要件ではない。必要に応じて補助的な object-generation path を許容するが、最終 link は response file 経由で system linker に委譲し、artifact 契約と determinism は compiler 側で担保する。

## 決定的コード生成

native backend は再現可能な成果物を生成しなければならない。
少なくとも次の条件を守る。

- 関数順、静的データ順、シンボル番号、relocation 順は source order と stable sort に基づく
- ビルド時刻、ホストパス、乱数 ID を成果物へ埋め込まない
- v1 ではデバッグ情報を既定で無効とする
- 同一 commit を同一条件で 2 回ビルドした場合、成果物ハッシュが一致することを目標とする

## Link と Runtime の関係

native backend は、runtime を別 object として扱う前提を持つ。
最終バイナリは `program.o` と `runtime.o` の結合で成立し、runtime 依存を codegen 内へハードコードしてはならない。

## 関連文書

- [`backend-boundary.md`](./backend-boundary.md)
- [`runtime-spec.md`](./runtime-spec.md)
- [`../development/planning/v2-designs/v2-08-native-backend-self-regeneration.md`](../development/planning/v2-designs/v2-08-native-backend-self-regeneration.md)
- [`../development/planning/v2-designs/v2-09-wasm-native-differential-zero.md`](../development/planning/v2-designs/v2-09-wasm-native-differential-zero.md)

# Native Backend 仕様

> **Status: Native-only official replacement track active (2026-05-09)**
>
> V2-08〜V2-10 で Darwin arm64 の actual native self-regeneration と experimental native-only RC は完了した。
> 次の目標は native-only official replacement track として、host launcher + embedded guest component 配布を rollback compatibility へ降格し、native-only を公式配布へ完全置換すること。
> Supported product/release targets は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) の 2 つに固定する。`x86_64-apple-darwin` と Windows (`x86_64-pc-windows-msvc`) は support scope 外であり、公式置換 blocker として扱わない。
> V2-13〜V2-15 はこの 2 target を正本として TODO.md に積む。
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

以下は native backend の初期対象外であり、supported product/release targets の blocker として扱わない。

- tail call 最適化
- デバッグ情報の完全サポート
- C ABI を越えた汎用 FFI 面の拡張
- JIT や動的ロード
- Windows 向けネイティブ成果物 (`x86_64-pc-windows-msvc` は out of support scope)

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

Native-only official replacement track で公式配布として扱う Supported product/release targets は次の 2 つである。

| target | 現状 | 置換 blocker |
|---|---|---|
| `aarch64-apple-darwin` | Mac Apple Silicon。actual native self-regeneration / experimental RC 完了 | stable 公式導線への昇格 |
| `x86_64-unknown-linux-gnu` | Linux x86_64。server priority track で先行検証中 | actual Linux native self-regeneration / runtime-link smoke |

Unsupported product/release targets は次のとおりである。既存の internal diagnostic や historical design は残せるが、公式配布や release blocker には含めない。

| target | 扱い |
|---|---|
| `x86_64-apple-darwin` | out of support scope。Rosetta / Mach-O smoke は internal diagnostic のみ |
| `x86_64-pc-windows-msvc` | out of support scope。COFF/PE / Authenticode は公式配布 blocker ではない |
| `aarch64-unknown-linux-gnu` | out of support scope。Linux ARM は tier2 ではなく将来再評価事項 |

### Linux x86_64 server priority track

サーバー用途を優先するため、`x86_64-unknown-linux-gnu` は Mac Apple Silicon と並ぶ supported product/release target として V2-13a で先行固定する。actual native self-regeneration の正本実行環境は Mac Apple Silicon 上の Ubuntu x86_64 Lima VM とし、GitHub Actions の required CI には含めない。`NativeTarget` descriptor、ELF object emitter、x86_64 codegen exact-byte smoke はローカルの `native-linux-x86-smoke` / Lima replay で確認する。

開発中の inner loop は GitHub Actions ではなくローカル VM で回す。`scripts/ci/native-linux-x86-local-vm-smoke.sh` は Linux x86_64 VM 上で descriptor / ELF emitter と canonical `program.o` / `runtime.o` / `linker-response.txt` / `program.native` runtime-link smoke を短時間で確認する。QEMU x86_64 VM では selfhost exact-byte suite が重いため、local smoke には含めず、actual native self-regeneration の調査へ進む前の fast gate として扱う。

VM は repo 管理の `scripts/ci/lima/lsharp-linux-x86.yaml` から作成する。この設定は x86_64 QEMU、4 CPU、20GiB memory、12GiB disk、host mount なしに固定し、provision 後の apt cache も削除する。

```bash
limactl create --name lsharp-linux-x86 scripts/ci/lima/lsharp-linux-x86.yaml
limactl start lsharp-linux-x86
```

`scripts/ci/native-linux-x86-hostgen-vm-exec.sh` は replay 前に `/tmp` の空き容量が既定 4GiB 以上あることを確認し、actual transport の既定 chunk を 64 にする。失敗時も `LSHARP_NATIVE_LINUX_X86_KEEP_VM_WORK_DIR=1` を明示しない限り VM workdir を削除し、ローカル `ci-artifacts/native-linux-x86-hostgen-vm/` は current/reuse artifact を保護しながら最新 8 世代へ制限する。調査で一時的に全世代を残す場合は `LSHARP_NATIVE_LINUX_X86_ARTIFACT_RETENTION_COUNT` を明示的に増やす。

host 側の selfhost `emit-native` で生成した Linux x86_64 code artifact を VM 内でリンク・実行する split smoke は `scripts/ci/native-linux-x86-hostgen-vm-exec.sh` で固定する。このスクリプトは `LSHARP_NATIVE_LINUX_X86_CODE_ARTIFACT` を指定して host-side selfhost artifact generation test を実行し、`limactl` 経由で Ubuntu x86_64 VM に `code.bin` を渡し、VM 内で `program.native` の `actual_exit_code` を確認する。

actual self-regeneration の重い `stage1 -> stage2` harvest に入る前の fail-fast 診断として、同スクリプトは `LSHARP_NATIVE_LINUX_X86_STAGE1_PROGRESS=1` を受け取り、materialized `actual-stage1/program.native` から `actual-stage1-progress.txt` / `actual-stage1-progress-stderr.txt` を artifact 化できる。この progress artifact は parser / defn body shape の早期切り分け用であり、`stage2` / `stage3` transport の byte-for-byte compare を置き換えるものではない。

full actual Linux native self-regeneration の未完了 blocker は、AArch64 actual stage23 と同等の **x86 selfhost runtime helper parity**、argc/argv と linear memory/data image を seed する **Linux runtime trampoline**、および release artifact として link 可能な **real ELF object/link artifact** の 3 点に分けて管理する。const-42 VM link/run は x86 codegen と Linux 実行環境の接続確認であり、この 3 点を完了扱いにはしない。

`scripts/ci/native-linux-x86-hostgen-vm-exec.sh` は host-side selfhost `emit-object` で生成した ELF64 relocatable `program.o` を VM に渡し、VM 内の `object-runtime.s` trampoline で `argc` / tagged argv vector を x86_64 SysV 側（`%r14` / `%r15`）へ seed して `object-program.native` を実行する。これにより Linux runtime trampoline と real ELF object/link artifact の const-42 slice は VM 証跡付きで固定済み。さらに `argv-program.o` / `argv-char-program.o` は同じ trampoline 上で command-line-arg + string-length / string-char-at helper を実行し、`argv-object-program.native seedling` が exit 8、`argv-char-object-program.native seedling` が exit 101 を返すことを確認する。`print-program.o` は print helper を Linux syscall 経由で実行し、`print-object-program.native` が exit 0 / stdout `42\n` を返すことを確認する。`vector-program.o` / `ref-program.o` は heap-backed vector/ref helper を実行し、`vector-object-program.native` が exit 42、`ref-object-program.native` が exit 99 を返すことを確認する。`substring-program.o` / `string-concat-program.o` は tagged string helper を実行し、`substring-object-program.native seedling` が exit 4、`string-concat-object-program.native seed ling` が exit 8 を返すことを確認する。`map-program.o` / `map-size-program.o` / `file-exists-program.o` は map/file helper を実行し、`map-object-program.native` が exit 42、`map-size-object-program.native` が exit 1、`file-exists-object-program.native file-exists-target.txt` が exit 1 を返すことを確認する。map/file helper 追加後の representative gap report では `selfhost_unsupported_x86_64` は空である。

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

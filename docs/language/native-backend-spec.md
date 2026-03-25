# Native Backend 仕様

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

以下は v1 の対象外とする。

- tail call 最適化
- デバッグ情報の完全サポート
- C ABI を越えた汎用 FFI 面の拡張
- JIT や動的ロード
- Windows 向けネイティブ成果物

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

追加ターゲットを導入する場合でも、`LoweredModule` や runtime 契約の共有を前提にしなければならない。

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

native backend の外部境界は runtime を介して定義する。

- エントリポイントは `runtime init -> L# main 呼び出し` の thin stub で構成する
- 外部公開シンボルは runtime boundary に限定する
- CLI、LSP、REPL は同一 compiler core の別エントリとして構成する
- C ABI 互換は runtime boundary でのみ保証する

v1 で想定する代表的な公開シンボルは次のとおりである。

- `lsharp_runtime_init`
- `lsharp_alloc`
- `lsharp_print`
- `lsharp_read_file`
- `lsharp_write_file`
- `lsharp_clock_now`

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

Mach-O / ELF を完全に手書きすることは v1 の必須要件ではない。必要に応じて `cc` や `ld` へ委譲してよい。

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

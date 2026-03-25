# Native Backend 仕様 (v1)

## 概要
L# compiler の Native backend は既存 Lowered IR から NativeInstr への 1 段変換を行い、
platform linker でネイティブバイナリを生成する。

## 対象ターゲット (v1)
- x86_64-apple-darwin
- aarch64-apple-darwin
- x86_64-unknown-linux-gnu

## 内部 ABI (P11-2b-1)
- 引数と戻り値は machine word 単位
- 複合値は pointer 参照
- 複数戻り値なし
- タグ付き word 表現を維持
- caller-save 優先
- tail call は v1 非対応

## 外部 ABI (P11-2b-2)
- エントリポイント: runtime init → L# main 呼び出しの thin stub
- 外部公開シンボル: lsharp_runtime_init, lsharp_alloc, lsharp_print, lsharp_read_file, lsharp_write_file, lsharp_clock_now
- CLI/LSP/REPL は同一 compiler core の別エントリ
- C ABI 互換は runtime boundary のみ

## スタックとレジスタ (P11-2b-3)
### x86_64
- 引数: rdi, rsi, rdx, rcx, r8, r9
- 戻り値: rax
- callee-save: rbx, rbp, r12-r15
- stack alignment: 16-byte

### aarch64
- 引数: x0-x7
- 戻り値: x0
- callee-save: x19-x28
- stack alignment: 16-byte

### Stack frame レイアウト
- return addr / saved regs / local slots / spill slots / outgoing arg area
- GC-safe point: call 前後と loop backedge

## Object Emitter (P11-2b-4)
- 出力: relocation 付き .o
- Mach-O / ELF の section 名・symbol visibility・relocation type を target descriptor に切り出し
- Runtime は別 object として出力
- 標準 artifact: program.o, runtime.o, linker-response.txt, program.native

## Deterministic Codegen (P11-2b-5)
- 関数順・静的データ順・シンボル番号・relocation 順は source order + stable sort
- ビルド時刻・ホストパス・ランダム ID を埋め込まない
- デバッグ情報は v1 では無効
- 再現性検証: 同一 commit 2 回ビルドでハッシュ一致

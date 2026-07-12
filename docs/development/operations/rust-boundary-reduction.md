# Rust 依存境界の縮小

## 目的

L# の通常開発を `cargo build` の待ち時間から切り離す。対象の product/release target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) のみとする。

この文書の完了条件は Rust workspace の物理削除ではない。通常の L# ソース編集、コンパイル、テストで Rust toolchain を必要としないことと、Rust の責務を bootstrap/oracle/rollback に限定することである。

## 現在の事実

- selfhost compiler の fixed-point は、supported target の stage chain で確認済みである。
- native-only `program.native` の release smoke は `--version` と `--help` だけを実証対象にしている。source-file command の成功は release readiness の根拠ではない。
- `selfhost/src/App/Cli.ls` と `selfhost/src/App/EmbeddedCli.ls` の `compile -o` / `build -o` は現在 `wasm-size:<n>` summary を `write-file` する。actual Wasm bytes を出力する contract ではない。
- selfhost compiler IR は `write-file` (opcode `89`) と内部 `write-file-bytes` (opcode `90`) を持ち、両 native target の runtime helper は byte-level ABI test で固定した。まだ `App.Cli` は summary text を書く実装であり、native `program.native` の source-file output は単独では完了していない。
- Rust host launcher は embedded guest の command execution、actual artifact write、Wasm component capability wiring を担当している。prebuilt launcher を使う利用者は Rust toolchain を必要としないが、Rust 実装への runtime dependence は残る。
- known rollback host-launcher archive を stage0 seed にした local bootstrap は、`cargo` を PATH から外した状態で stage2 再生成、`check`、actual Wasm output まで実行できる。これは no-Cargo development loop の証跡である。
- `fetch-stage0.sh` が要求する GitHub Release archive/checksum の公開契約は未接続である。fresh clone が自動で stage0 を取得できることは、この時点では未達とする。

## V2-16a: Cargo なしの開発ループ

`scripts/selfhost-dev.sh` は stage0 package を入力にして stage1/stage2 を組み、stage2 launcher に通常の argv を委譲する。script 自体は `cargo` を呼ばない。

stage0 directory には少なくとも `lsharp` と `lsharp.component.wasm` が必要である。既知の rollback host-launcher archive を展開した directory を入力にできる。

```bash
STAGE0_DIR=/path/to/lsharp-host-launcher \
  ./scripts/selfhost-dev.sh check examples/fib.ls

STAGE0_DIR=/path/to/lsharp-host-launcher \
  ./scripts/selfhost-dev.sh compile examples/fib.ls -o fib.component.wasm
```

初回、または `ENTRY_FILE` / `selfhost/src/**/*.ls` の fingerprint が変わった場合は stage1/stage2 を再生成する。常に再生成したい場合は `--bootstrap` を渡す。

```bash
STAGE0_DIR=/path/to/lsharp-host-launcher \
  ./scripts/selfhost-dev.sh --bootstrap check examples/fib.ls
```

runner は `LSHARP_PATH` と `LSHARP_DISABLE_EMBEDDED_COMPONENT` を除去してから bootstrap と stage2 command を実行する。外部 launcher への意図しない delegation を避けるためである。`--help` を stage2 へ渡す場合は `-- --help` を使う。

この経路は `cargo` を編集・実行の hot path から外すが、immutable な Rust 製 host launcher を bootstrap compatibility boundary として使う。Rust 実装を不要にした証拠ではない。

## 目標境界

| 範囲 | 通常開発で使うもの | V2-16 完了後の Rust の役割 |
|---|---|---|
| bootstrap | stage0 package と stage2 再利用 | stage0 取得と緊急復旧のみ |
| parse/check/fmt/test | selfhost stage2 または native CLI | oracle 比較のみ |
| compile/build output | native CLI が actual bytes を書く | oracle 比較と bootstrap のみ |
| install/repl/lsp/doc | native selfhost または明示した外部 tool | hidden host-launcher fallback なし |
| release verification | 両 target の native artifact E2E | optional differential と rollback 検証 |

immutable prebuilt host launcher に委譲する executable は、移行中の開発 runner としては許容する。これは edit/run loop から `cargo` を外すが、base implementation が Rust を必要としなくなったことは意味しない。

## 残りの順序

1. `V2-16b`: byte-output ABI と両 target の native `write-file` runtime helper は追加済み。`App.Cli` / `EmbeddedCli` の compile/build を actual Wasm bytes 出力へ移し、component sidecar と同じ WASI Preview1 file ABI を selfhost Wasm runtime に接続する。
2. `V2-16c`: host-only public command を native selfhost 実装へ移すか、外部 tool の責務として明示する。planned response や warmup-only REPL は parity と扱わない。
3. `V2-16d`: Mac Apple Silicon と local Lima Linux x86_64 VM で、native `program.native` 単独の source-file command suite を実行する。stage0 からの再生成後も同じ gate を通す。
4. `V2-16e`: 通常の開発/test 手順から Rust を外し、Rust workspace を stage0/oracle/rollback に限定する。

## 必須証跡

- `scripts/selfhost-dev.sh` の fixture が bootstrap、stage2 reuse、source change refresh、forced rebuild、command delegation を検証し、script に `cargo` がないことを検証する。
- native `compile -o` と `build -o` が両 supported target で nonempty Wasm bytes と `\0asm` header を生成する。
- native `parse`、`check`、`fmt`、`test`、`compile -o`、`build -o` が host launcher を process path に置かず real source file に対して動く。
- stage0-to-stage2 rebuild 後にも同じ native command suite が通る。
- documentation が prebuilt executable の toolchain-free use と Rust runtime dependence の撤去を区別する。

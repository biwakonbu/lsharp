# Default path migration

公開コマンドの既定実装を Rust パイプラインから L# selfhost / native toolchain へ移すための運用メモ。

## 目的

- `lsharp` コマンドの **default path** を段階的に L# 実装へ切り替える。
- 切替ごとに parity / smoke / CI gate を通し、Rust 経路は fallback ではなく shadow/oracle 比較用に縮退させる。
- Phase 11 完了時には、エンドユーザーが Rust 実装を意識せずに native 配布物だけで開発フローを完走できる状態を目指す。

## 現状

2026-03-25 時点では **第 1 段**のみ完了している。

- `crates/lsharp-driver/src/main.rs` に `LSHARP_PATH` 予約コメントと compiler path の移行余地がある。
- `scripts/ci/default-path-smoke.sh` が、ビルド済み `lsharp` バイナリ単体で `check` / `compile` を実行できることを検証する。
- `.github/workflows/ci.yml` の `default-path-smoke` は `ci-gate` / `ci-gate-v2` の required job に含まれる。

この段階では **バイナリ経路の固定**まではできているが、実際の default implementation はまだ主に Rust 版であり、完全な Rust 非依存 default path ではない。

## 切替順

`docs/development/planning/phase11-implementation-plan.md` の OPS-05 に従い、default path の切替順は次で固定する。全 13 コマンドを対象とする。

### コマンド別移行マトリクス

| # | コマンド | 現在の default | 移行先 | Parity テスト | ステータス |
|---|----------|---------------|--------|--------------|-----------|
| 1 | `compile` | Rust (`lsharp-wasm`) | L# (`selfhost/Compiler.ls`) | `test_e2e_selfhost_wasm_native_differential` | 🔶 第1段完了 |
| 2 | `check` | Rust (`lsharp-types`) | L# (`selfhost/TypeInfer.ls`) | `test_check_selfhost_typeinfer_standalone_import_path` | 🔶 第1段完了 |
| 3 | `parse` | Rust (`lsharp-syntax`) | L# (`selfhost/Parser.ls`) | `test_e2e_selfhost_parser_*` | 🔶 smoke 通過 |
| 4 | `test` | Rust (`lsharp-driver`) | L# (`selfhost/TestRunner.ls`) | — | ⬜ 未着手 |
| 5 | `build` | Rust (`lsharp-driver`) | L# (`selfhost/Cli.ls`) | — | ⬜ 未着手 |
| 6 | `fmt` | Rust (`lsharp-driver`) | L# (`selfhost/Formatter.ls`) | `test_e2e_selfhost_formatter_*` | 🔶 部分実装 |
| 7 | `lsp` | Rust (`lsharp-lsp`) | L# (`selfhost/LspServer.ls`) | `test_e2e_selfhost_lsp_*` | 🔶 部分実装 |
| 8 | `docs` | Rust (`lsharp-docs`) | L# (`selfhost/DocTools.ls`) | `test_e2e_selfhost_doctools_*` | 🔶 部分実装 |
| 9 | `review` | Rust (`lsharp-docs`) | L# (未定) | — | ⬜ 未着手 |
| 10 | `doc-ack` | Rust (`lsharp-docs`) | L# (未定) | — | ⬜ 未着手 |
| 11 | `doc-check` | Rust (`lsharp-docs`) | L# (未定) | — | ⬜ 未着手 |
| 12 | `install` | Rust (`lsharp-driver`) | L# (未定) | — | ⬜ 未着手 |
| 13 | `repl` | Rust (`lsharp-driver`) | L# (未定) | — | ⬜ 未着手 |

凡例: 🔶 = 進行中 / 部分完了、⬜ = 未着手

### 切替の前提条件

各コマンドは、切替前に以下を満たしてから active row を L# 側へ更新する。

- parity test が存在する
- golden / snapshot が安定している
- smoke test が CI で blocking になっている
- Rust 経路を shadow/oracle へ下げても triage 可能である

## 現在の blocking gate

### `default-path-smoke.sh`

`scripts/ci/default-path-smoke.sh` は次を確認する。

1. `cargo build -p lsharp-driver -q`
2. `target/debug/lsharp check examples/fib.ls`
3. `target/debug/lsharp compile examples/fib.ls -o <tmp>`
4. 生成物が空でないこと

この smoke は「`cargo run` ではなく、配布対象に近い `lsharp` バイナリ経路が動く」ことを保証する第 1 段の gate である。

### `main.rs` の path 予約

`crates/lsharp-driver/src/main.rs` では、`LSHARP_PATH` を将来の selfhost / 外部 compiler path として予約している。

- 現状: Rust 実装を内蔵した `lsharp` バイナリ
- 移行中: `LSHARP_PATH` を通じて compiler root / binary path を差し替え可能にする
- 最終像: `lsharp` が native selfhost toolchain を既定で選び、Rust 経路は fallback ではなく shadow になる

## 完了までに残ること

- `compatibility-matrix.md` の `Default path` を切替順どおり L# へ更新する
- `CLI-02`, `LSP-02`, `FMT-01`, `DOC-02` の parity を満たす
- `OPS-03` の shadow/oracle lifecycle を整理し、Rust 経路を比較用途へ限定する
- `scripts/release-playbook.sh` と `scripts/smoke_test_readme.sh` を native 配布物前提へ更新する
- Rust 未導入の fresh clone で Quick Start が通ることを `OPS-07` で検証する

## 証跡

- `scripts/ci/default-path-smoke.sh`
- `crates/lsharp-driver/src/main.rs`
- `.github/workflows/ci.yml`
- `docs/development/planning/compatibility-matrix.md`
- `docs/development/planning/phase11-implementation-plan.md#ops-05-default-path-migration`

## 更新規則

- default path の active 実装が変わったら、同じ PR でこの文書と `compatibility-matrix.md` を更新する。
- smoke / parity / release 手順のいずれかが変わった場合も、この文書を evidence と同期させる。

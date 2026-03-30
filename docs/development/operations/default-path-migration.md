# Default path migration

公開コマンドの既定実装を Rust 内蔵パイプラインから **host launcher + embedded guest Wasm component** 構成へ移すための運用メモ。

## 目的

- `lsharp` コマンドの **default path** を段階的に L# 実装へ切り替える。
- 切替ごとに parity / smoke / CI gate を通し、Rust 経路は fallback ではなく shadow/oracle 比較用に縮退させる。
- Phase 11 / Phase 13 移行後は、エンドユーザーが Rust workspace を意識せず **単一バイナリの host launcher と埋め込み guest component** で開発フローを完走できる状態を目指す。

## 現状

2026-03-30 時点では **第 1 段 + delegation hook + selfhost Wasm smoke + command-surface 明文化** まで完了している。

- `crates/lsharp-driver/src/main.rs` の `LSHARP_PATH` は、external host launcher executable・その配置ディレクトリ・preview1 `.wasm` selfhost artifact・Preview2 `.component.wasm` guest artifact を受け付ける。
- `scripts/ci/default-path-smoke.sh` が、ビルド済み `lsharp` バイナリ単体の Rust built-in `compile` に加え、`selfhost/src/App/SmokeCli.ls` を stage1 `.wasm` artifact として生成し、`check` / `fmt` / `compile` の日常 smoke を `LSHARP_PATH=<stage1.wasm>` で検証する。さらに executable path / directory path delegation と invalid path error も検証する。
- `scripts/ci/compile-phase11-inputs.sh` も `LSHARP_BIN` を受け取り、Phase 11 固定入力セットを `lsharp` バイナリ経路でコンパイルする。
- `fresh-clone-smoke` が clean checkout 相当のコピー上で `lsharp` host launcher を再ビルドし、default-path smoke + 代表的な selfhost / stdlib compile を実行する。
- `.github/workflows/ci.yml` の `default-path-smoke` は `ci-gate` / `ci-gate-v2` の required job に含まれる。
- `.github/workflows/ci.yml` の `fresh-clone-smoke` も required job に含まれる。
- `cargo test -p lsharp-driver --test default_path_delegation` が、`LSHARP_PATH` の executable path / directory path / preview1 `.wasm` artifact / Preview2 `.component.wasm` artifact / invalid path error を固定する。
- `selfhost/src/App/Cli.ls` は `init` を除く公開 13 CLI サブコマンド (`parse` / `check` / `compile` / `build` / `test` / `review` / `doc-ack` / `doc-check` / `install` / `repl` / `lsp` / `fmt` / `doc`) すべてに対応する selfhost surface を持つ。ただし現行 host launcher の組み込み clap surface で直接実行できるのは `compile` / `build` / `test` / `review` / `doc-ack` / `doc-check` / `install` / `repl` / `lsp` / `doc` の Rust-backed subset であり、`parse` / `check` / `fmt` は当面 `LSHARP_PATH` delegation を要求する shadow command として扱う。
- `selfhost/src/App/SmokeCli.ls` は `parse` / `check` / `fmt` / `compile` / `build` だけに絞った narrow selfhost smoke entrypoint で、LSP/doc/test 依存を持たない。これは STR-03 の non-circular smoke 専用であり、full CLI parity や `.component.wasm` cutover の正本ではない。

この段階では **バイナリ経路の固定**に加えて **non-circular な selfhost Wasm smoke** まではできているが、実際の default implementation はまだ主に Rust host launcher 内蔵経路であり、完全な guest component 既定起動には至っていない。

## 切替順

`docs/development/planning/phase11-implementation-plan.md` の OPS-05 に従い、default path の切替順は次で固定する。対象は `init` を除く公開 13 CLI サブコマンドである。

### コマンド別移行マトリクス

| # | コマンド | 組み込み default path | selfhost surface の現況 | `LSHARP_PATH` delegation 時 | 次の主要 gate |
|---|----------|----------------------|-------------------------|-----------------------------|---------------|
| 1 | `parse` | shadow command (`LSHARP_PATH` 必須) | `selfhost/src/App/Cli.ls` + `selfhost/src/Syntax/Parser.ls` に parse text/diagnostics subset あり | 外部 `lsharp parse ...` へ argv 全体を委譲 | AST pretty-print / diagnostics snapshot parity |
| 2 | `check` | shadow command (`LSHARP_PATH` 必須) | `selfhost/src/App/Cli.ls` + `selfhost/src/Types/TypeInfer.ls` に type/diagnostics subset あり | 外部 `lsharp check ...` へ argv 全体を委譲 | type display / diagnostics JSON parity |
| 3 | `compile` | Rust (`lsharp-wasm`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Backend/Wasm/Compiler.ls` + `selfhost/src/Backend/Wasm/WasmEmit.ls` に compile surface あり | 外部 `lsharp compile ...` へ argv 全体を委譲 | artifact / `-o` / bootstrap fixed-point parity |
| 4 | `build` | Rust (`lsharp-driver`) | `selfhost/src/App/Cli.ls` に compile alias surface あり | 外部 `lsharp build ...` へ argv 全体を委譲 | project build contract / artifact parity |
| 5 | `test` | Rust (`lsharp-driver`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Test/TestRunner.ls` に metadata suite subset あり | 外部 `lsharp test ...` へ argv 全体を委譲 | metadata semantics / exit code parity |
| 6 | `review` | Rust (`lsharp-docs`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Doc/DocTools.ls` に deterministic review text surface あり | 外部 `lsharp review ...` へ argv 全体を委譲 | diagnostics schema / severity / exit code parity |
| 7 | `doc-ack` | Rust (`lsharp-docs`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Doc/DocTools.ls` に ack text surface あり | 外部 `lsharp doc-ack ...` へ argv 全体を委譲 | state/update semantics parity |
| 8 | `doc-check` | Rust (`lsharp-docs`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Doc/DocTools.ls` に check text surface あり | 外部 `lsharp doc-check ...` へ argv 全体を委譲 | schema / failure surface parity |
| 9 | `install` | Rust (`lsharp-driver`) | `selfhost/src/App/Cli.ls` に dry-run install plan surface あり | 外部 `lsharp install ...` へ argv 全体を委譲 | package/archive/checksum parity |
| 10 | `repl` | Rust (`lsharp-driver`) | `selfhost/src/App/Cli.ls` に warmup session summary surface あり | 外部 `lsharp repl` へ argv 全体を委譲 | interactive loop / history / runtime stability gate |
| 11 | `lsp` | Rust (`lsharp-lsp`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Lsp/LspServer.ls` + `selfhost/src/Tools/Lsp/JsonRpc.ls` に capability / handler subset あり | 外部 `lsharp lsp` へ argv 全体を委譲 | JSON-RPC transport / snapshot parity / soak gate |
| 12 | `fmt` | shadow command (`LSHARP_PATH` 必須) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Text/Formatter.ls` に canonical text surface あり | 外部 `lsharp fmt ...` へ argv 全体を委譲 | roundtrip / idempotency / CLI-LSP parity |
| 13 | `doc` | Rust (`lsharp-docs`) | `selfhost/src/App/Cli.ls` + `selfhost/src/Tools/Doc/DocTools.ls` + `selfhost/src/Tools/Doc/HtmlDoc.ls` に doc title/body + HTML subset あり | 外部 `lsharp doc ...` へ argv 全体を委譲 | JSON/HTML snapshot / distribution parity |

### この表の読み方

- **組み込み default path**: `LSHARP_PATH` を設定しない通常起動時に、driver が使う内蔵実装。2026-03-30 時点では Rust-backed subset (`compile` / `build` / `test` / `review` / `doc-ack` / `doc-check` / `install` / `repl` / `lsp` / `doc`) のみが clap surface に載っており、`parse` / `check` / `fmt` は `LSHARP_PATH` を要求する shadow command のまま。
- **selfhost surface**: `selfhost/src/App/Cli.ls` から見える L# 側の公開面。`Default path` の切替前でも、実装の存在と narrow parity 証跡はここで追跡する。
- **`LSHARP_PATH` delegation**: executable / directory path については、行ごとの feature flag ではなく **process-entry delegation**。`main.rs` は clap dispatch 前に `LSHARP_PATH` を評価し、設定されていれば受け取った argv 全体を外部 `lsharp` host launcher binary へ渡す。
- **`LSHARP_PATH=<*.wasm>` / `LSHARP_PATH=<*.component.wasm>`**: preview1 selfhost artifact の場合は host launcher が current dir + argv を付けて Wasm を起動し、Preview2 component artifact の場合も component runner で current dir + argv を付けて起動する。前者は STR-03 の daily smoke 用、後者は C-1 の launcher-side component execution scaffold であり、どちらも final embedded cutover そのものではない。
- したがって、`LSHARP_PATH` は「built-in default を L# に切り替えた」ことを意味しない。現時点では **Rust built-in default を維持したまま、外部 selfhost/host-launcher binary と preview1 Wasm artifact を shadow smoke できる hook** と読む。

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
2. `target/debug/lsharp compile examples/fib.ls -o <tmp>` が Rust built-in path で通ること
3. `target/debug/lsharp compile selfhost/src/App/SmokeCli.ls -o <stage1.wasm>` で narrow selfhost artifact を生成できること
4. `LSHARP_PATH=<stage1.wasm>` で `check smoke_input.ls` / `fmt smoke_input.ls` / `compile smoke_input.ls -o smoke_output.txt` が current dir 上の相対 path を使って成功すること
5. `LSHARP_PATH=<delegate-exec> target/debug/lsharp --version` が delegate 側の stdout / exit code をそのまま返すこと
6. `LSHARP_PATH=<delegate-dir> target/debug/lsharp --version` が `<delegate-dir>/lsharp` へ委譲できること
7. 不正な `LSHARP_PATH` が明示エラーになること

この smoke は「`cargo run` ではなく、配布対象に近い `lsharp` host launcher バイナリ経路が動く」ことに加え、「default path を external host launcher へ差し替える delegation hook」と「preview1 selfhost Wasm artifact を daily smoke に使う non-circular path」が壊れていないことも保証する gate である。

補足:

- `selfhost/src/App/SmokeCli.ls` は daily smoke 専用の narrow artifact である。
- `fmt` は canonical formatted input の roundtrip smoke、`compile -o` は `wasm-size:<n>` summary file を使う smoke であり、full CLI parity / final artifact parity は `CLI-02` / `P13-A-2` / `P13-B-2` 側の gate で追う。

### `compile-phase11-inputs.sh`

`scripts/ci/compile-phase11-inputs.sh` は次を確認する。

1. `LSHARP_BIN` が指定されていればそれを使い、未指定なら `target/debug/lsharp` をビルドする
2. selfhost / stdlib / examples の固定入力セットを `"$LSHARP_BIN" compile ...` で順にコンパイルする
3. CI `bootstrap` job / release playbook から同じスクリプトを共有し、`cargo run -- compile` の多重呼び出しを避ける

これにより、Phase 11 入力セットの compile gate も default-path migration の証跡として使えるようになった。

### `main.rs` の path delegation

`crates/lsharp-driver/src/main.rs` では、`LSHARP_PATH` を selfhost / 外部 host launcher への delegation hook として使う。加えて `crates/lsharp-driver/build.rs` は build-time 環境変数 `LSHARP_EMBED_COMPONENT_PATH` が設定されていれば、その `.component.wasm` を `include_bytes!` 用の埋め込み資産として取り込み、`LSHARP_PATH` 未設定時の default path 候補として起動できる。

- 現状: Rust 実装を host launcher に内蔵した `lsharp` バイナリだが、`LSHARP_PATH` を通じて compiler launcher executable・配置ディレクトリ・preview1 `.wasm` selfhost artifact・Preview2 `.component.wasm` guest artifact を差し替え可能
- 追加済み: build-time `LSHARP_EMBED_COMPONENT_PATH` を使えば、custom build の host launcher に guest component を `include_bytes!` で埋め込み、`LSHARP_PATH` 未設定時でも embedded guest へ委譲できる
- 追加済み: runtime `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` を与えると、custom build でも embedded guest を明示的に無効化し、内蔵 Rust/clap path へ戻せる
- 移行中: invalid path は明示エラーにしつつ、外部 host launcher path へ委譲できる
- 最終像: `lsharp` が埋め込み guest component を既定で選び、Rust workspace は host launcher / component tooling context として残存しつつ、旧内蔵経路は fallback ではなく shadow になる

補足:

- executable / directory delegation は **`compile` だけでなく全 13 CLI サブコマンドに対する process-entry hook** である
- `LSHARP_PATH` が設定されている場合はそちらが最優先で、build-time に埋め込んだ guest component は fallback/default 候補としてのみ使う
- `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` が設定されている場合、embedded guest は起動せず、host launcher の built-in path を優先する
- `default-path-smoke.sh` は `--version` で executable / directory delegation を smoke しつつ、preview1 `.wasm` artifact については `check` / `fmt` / `compile` の relative-path smoke を追加で回す。Preview2 `.component.wasm` artifact は現状 runner/delegation の narrow test で固定し、embedded cutover 時に smoke へ昇格させる。

## 完了までに残ること

- `compatibility-matrix.md` の `Default path` を切替順どおり L# へ更新する
- `CLI-02`, `LSP-02`, `FMT-01`, `DOC-02` の parity を満たす
- `OPS-03` の shadow/oracle lifecycle を整理し、Rust 経路を比較用途へ限定する
- `scripts/release-playbook.sh` と `scripts/smoke_test_readme.sh` を host launcher + embedded component 配布前提へ更新する
- Rust 未導入の fresh clone で Quick Start が通ることを `OPS-07` で検証する

## 証跡

- `scripts/ci/default-path-smoke.sh`
- `scripts/ci/compile-phase11-inputs.sh`
- `crates/lsharp-driver/src/main.rs`
- `crates/lsharp-driver/tests/default_path_delegation.rs`
- `.github/workflows/ci.yml`
- `docs/development/planning/compatibility-matrix.md`
- `docs/development/planning/phase11-implementation-plan.md#ops-05-default-path-migration`

## 更新規則

- default path の active 実装が変わったら、同じ PR でこの文書と `compatibility-matrix.md` を更新する。
- smoke / parity / release 手順のいずれかが変わった場合も、この文書を evidence と同期させる。

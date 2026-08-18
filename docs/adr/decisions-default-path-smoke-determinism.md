# ADR: default-path-smoke の経路別決定論化と CI での binary 供給

- Status: Accepted (verified slice)
- Date: 2026-08-18
- Scope: `SMOKE-GATE-01` / `I-15` / `OPS-05` / `scripts/ci/default-path-smoke.sh` / `.github/workflows/ci.yml`
- Related: [`ISSUES.md` I-15](../../ISSUES.md#i-15)、
  [`decisions-root-lifetime-main-exit-exemption.md`](decisions-root-lifetime-main-exit-exemption.md)

## Context

`scripts/ci/default-path-smoke.sh` は「ビルド済み `lsharp` binary の既定経路」を守るゲートである。
ところが `:39` / `:54` の assertion は既定 target (`wasi-component`) の stdout に `wasm-size:` を
要求しており、**この条件は最初から成立しない**。`wasm-size:` を出すのは embedded guest だけで、
guest は既定 target を遂行しないからである。詳細と 4 経路の実測は `I-15` が正本。

重要なのは、これが**製品の欠陥ではなくゲートの欠陥**だという点である。driver の host fallback
(`crates/lsharp-driver/src/main.rs:900-928`) は設計どおりの挙動であり、既定経路は rc=0 で
有効な Wasm を書き出している。ゲートは「壊れたら落ちる」ではなく「常に落ちる」状態にあった。

同時に、この矛盾が CI で発覚しなかった理由は別にある。`default-path-smoke` job は
`cargo build --bin lsharp` を持たず `LSHARP_BIN` も渡していないため、走れば `:24-28` の
「binary が無い」で落ちる。実際には直近 5 run すべてで job が skipped だったので、
**そもそも一度も走っていない**。ゲートの条件を直すだけでは、この盲点は閉じない。

**実装中に判明したこと (doc-RED からの訂正)。** 腐敗は `:39` / `:54` の 1 件ではなく、
走らないゲートの背後に **3 層**溜まっていた。

| 層 | 箇所 | 症状 |
|---|---|---|
| 1 | `:39` / `:54` の既定経路 assertion | 条件が原理的に成立しない (doc-RED 時点で認識済み) |
| 2 | guest `check` / `test` の assertion | 期待が plain text のままで、製品側は structured JSON へ移行済み |
| 3 | `selfhost/src/App/SmokeCli.ls` | `(import App.CompilerMode)` 欠落でソースが**コンパイルできない** |

「ゲートが一度も走らなかったので、3 層の腐敗が誰にも見えないまま積み上がった」が正しい
Context である。これは第 2 層 (CI への binary 供給) の必要性を、doc-RED 時点の理由より強く裏付ける。

## Decision

**2 層で直す。条件を緩めるのではなく、経路ごとに分けて両方を検査する。**

### 第 1 層 — script の経路別 assertion

既定経路と guest 経路は**別の契約**なので、別の case として書く。

| case | 呼び出し | 要求する stdout | 追加検査 |
|---|---|---|---|
| 既定 (component) | `compile … -o <rel>.component.wasm` | `コンパイル成功:` を含む | 非空 + 先頭 4 byte `0061736d` |
| guest (preview1) | `compile … --target wasi-preview1 -o <rel>.wasm` | `wasm-size:` を含む | 同上 |
| 既定 (build) | `build … --output <rel>.component.wasm` | `コンパイル成功:` を含む | 同上 |
| guest (build/preview1) | `build … --target wasi-preview1 --output <rel>.wasm` | `wasm-size:` を含む | 同上 |

**これは現状よりゲートが強くなる。** 現状 2 case が常に落ちていたのに対し、4 case が
それぞれ実際に守るべき契約を検査する。とくに guest 経路は現状どこのゲートにも載っていない。

判定文字列は `コンパイル成功:` の**前置き部分だけ**を見る。`(18506 bytes)` の数値は
入力や codegen の変化で動くため、assertion に含めない。

### 第 2 層 — CI job への binary 供給

`.github/workflows/ci.yml` の `default-path-smoke` job に `cargo build --bin lsharp` step を足し、
script step へ `LSHARP_BIN` を渡す。`needs: [test]` は順序を作るだけで `target/` を共有せず、
`Swatinem/rust-cache` は workspace member の binary を保持しないので、**明示ビルドが要る**。

## 却下した選択肢

**案 X — assertion を `wasm-size:` または `コンパイル成功:` の OR にする。却下。**
2 case は通るようになるが、「どちらが出ても合格」は**どちらの契約も検査しない**に等しい。
host fallback が意図せず既定経路になった退行を、このゲートは永久に見逃す。
ゲートを緑にすることと、ゲートが仕事をすることは別である。

**案 Y — 既定経路の case を削除し guest 経路だけ残す。却下。**
このゲートの名前と存在理由 (`OPS-05`: ビルド済み binary の既定経路) を失う。
実際にユーザーが叩くのは既定経路であり、そこを外すのは本末転倒。

**案 Z — ゲートを「packaged binary 前提」と明示し、dev build では skip する。却下。**
`I-15` の実測どおり、boundary は guest 側で**無条件**なので packaged binary でも
`wasm-size:` は出ない。前提を明示しても条件は成立せず、問題が移動するだけである。
この旧仮説自体を `TODO.md` から取り下げる。

## 含めない範囲

- **job が skipped だった理由そのもの**は本 ADR では扱わない。第 2 層は「走ったときに
  正しく走る」ことだけを保証する。トリガ条件の調査は別件として残る。
- ~~`scripts/ci/default-path-smoke.sh:68-335` (embedded guest の parse/check/test/review/doc-* と
  `LSHARP_PATH` 委譲) は現状で通っており、触らない。~~
  **この記述は実測で 2 度否定された (下記 Evidence)。** 当該範囲のうち guest `check` / `test` の
  2 assertion が陳腐化しており、さらに `:279` が叩く `SmokeCli.ls` はコンパイルできなかった。
  「script 全体 rc=0」を受入条件に置いた以上、これらは本スライスの範囲に入る。
  実測で無改変のまま通ったのは `parse` / `review` / `review --json` / `review --format json` /
  `doc-ack` / `doc-ack --trailer` / `doc-check` / `doc-check --strict` と `LSHARP_PATH` 委譲側で、
  こちらは触っていない。
- `EMBEDCACHE-01` (build.rs の cache key) は独立のスライス。

## 実装順序 (doc-RED 時点の計画)

1. RED: 無改変の script を dev binary に対して実行し、`:39` で落ちることを確認する。
2. GREEN: `:31-66` を経路別 4 case へ書き換える。`-o` は preopen 内の**相対パス**を保つ。
3. GREEN: `ci.yml` の `default-path-smoke` job に build step と `LSHARP_BIN` を足す。
4. 検証: script 全体 rc=0 と `bash -n`。

## 追加で決めたこと (実装中)

### 陳腐化 assertion の期待値を書き換える根拠

本リポジトリの TDD 規律は「テストの期待値を実装に合わせて変更しない」を要求する。
今回 guest `check` / `test` の 2 assertion を書き換えたのは、**契約側が別に存在し、
script 側が古かったことを一次証拠で示せた**場合に限る例外である。

| 対象 | 契約の正本 | script 側の旧期待 |
|---|---|---|
| `lsharp check` | `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_contracts.rs:234-235`「EmbeddedCli の check JSON report は failureKinds を返す必要がある」 | `diagnostics:0` (2026-03-31 `bc752767` 由来) |
| `lsharp test` | `docs/development/planning/v0.2-evidence-contracts.md:165-180` (v0.2 structured assurance report) | `examples:1` / `invariants:1` / `failures:0` |

どちらも「実装がこう出力するから期待値を合わせた」のではなく、**別の正本が既にその形式を
契約として固定しており、script だけが追随していなかった**。逆向き (契約側を script に
合わせる) は契約テストを壊すので採れない。

### `SmokeCli.ls` の import 欠落を本スライスで直す判断

`:279` が叩く `selfhost/src/App/SmokeCli.ls` は `build-wasm-bytes-wasi`
(`selfhost/src/App/CompilerMode.ls:6089` 定義) を呼びながら `(import App.CompilerMode)` を
持たず、host compile が `[LS1001] [E0001] 未定義の変数` で落ちていた。

これは製品ソースの欠陥でありゲートの欠陥ではないので、当初は別 issue へ切り出す想定だった。
**probe で 1 行の import 追加だけで解消し、波及が無いことを実測できた**ため、本スライスに含める。
判断基準は「受入条件 (script 全体 rc=0) を満たすのに不可欠か」と「修正が同一 slice に収まるか」で、
両方を満たした。連鎖していた場合は `I-16` として切り出し、第 2 層ごと保留する予定だった。

同 import は `App/Main.ls` / `App/Cli.ls` / `App/EmbeddedCli.ls` / `App/PipelineSmoke.ls` の
4 兄弟モジュールが既に持っており、`SmokeCli.ls` だけが欠けていた。

## Evidence

すべて 2026-08-18、worktree `codex/gate-fixes-root-lifetime` (base `e9227f3c`)、
`cargo build --bin lsharp` (rc=0) が生成した dev binary での実測。

### 前提の確定 (4 経路の実測)

| 呼び出し | stdout | 経路 | 先頭 4 byte |
|---|---|---|---|
| `compile examples/fib.ls -o <out>.component.wasm` | `コンパイル成功: … (18506 bytes)` | host fallback | `0061736d` |
| `compile … --target wasi-preview1 -o <out>.wasm` | `wasm-size:2904` | guest | `0061736d` |
| `build … --output <out>.component.wasm` | `コンパイル成功: … (18506 bytes)` | host fallback | `0061736d` |
| `build … --target wasi-preview1 --output <out>.wasm` | `wasm-size:2904` | guest | `0061736d` |

4 件とも rc=0。`examples/fib.ls` は 136 byte で `standalone-preview1-input-layout-safe?`
の 1024 byte 制限内にある。既定 target で `wasm-size:` が出ないことが確定した。

### RED → GREEN

- **RED (第 1 層)**: 無改変の script → `ERROR: embedded compile output mismatch` で `:39` 停止。
- **RED (第 3 層)**: 4 case 書き換え後に `:279` →
  `selfhost/src/App/SmokeCli.ls: [LS1001] [E0001] 未定義の変数 (undefined): build-wasm-bytes-wasi (4105..4126)`、rc=1。
  import を外した状態で `cargo test -p lsharp-driver --test default_path_delegation
  test_driver_delegates_to_wasm_cli_artifact_via_lsharp_path` → **FAILED**、
  panic message は上と同一の `[LS1001]`。因果を pin した。
- **GREEN (第 3 層)**: import 追加後の同 test → **`1 passed; 0 failed`** (56.20s)。
  host compile 単体でも `コンパイル成功: … (761488 bytes)` rc=0。
- **GREEN (script 全体)**: `bash scripts/ci/default-path-smoke.sh` → 最終行 `default-path-smoke: OK`、
  **rc=0**。`bash -n scripts/ci/default-path-smoke.sh` も通る。
- **crate**: `cargo test -p lsharp-driver --test default_path_delegation` → `35 passed; 11 failed`。
  残る 11 件はすべて baseline 登録済みの既知 FAIL で、本変更が触っていない
  embedded component 委譲クラスタである。

### baseline の更新

`test_driver_delegates_to_wasm_cli_artifact_via_lsharp_path` が pass へ転じたため、
`docs/development/validation/workspace-expected-failures.txt` から 1 行削除した
(「expected が pass に転じたら非 0」という checker の規則に従う)。
非 e2e クラスタの見出しを `38 FAIL` → `37 FAIL`、entry 総数 **95 → 94**。

### CI 側 (第 2 層)

`.github/workflows/ci.yml` の `default-path-smoke` job に `cargo build --bin lsharp` step を追加。
`LSHARP_BIN` は**渡さない**。script 既定の `$ROOT/target/debug/lsharp` が絶対パスであるのに対し、
job から相対値を渡すと script 内の `cd "$EMBED_SMOKE_DIR"` を跨いだ時点で解決できなくなるためで、
これは doc-RED 時点の計画 (「`LSHARP_BIN` を渡す」) からの訂正である。
YAML は `yaml.safe_load` で構文確認済み。

### 満たせなかった受入条件

- **CI 側の変更はローカルで検証できていない。** job が実際に緑になるかは push 後の
  1 run を見るまで確定しない。ローカルで確認できたのは「script 自体は rc=0」と
  「YAML が構文として妥当」の 2 点だけである。
- **job が skipped だった理由そのものは未解明のまま**。「含めない範囲」に明記したとおりで、
  第 2 層は「走ったときに正しく走る」だけを保証する。トリガ条件の調査は残件。
- `check-workspace-baseline.sh` は再実行していない (workspace 全体の nextest 実測を
  入力に取るため)。代わりに削除した 1 件が pass へ転じたことを当該 test の直接実行で確認した。

## Consequences

既定経路と guest 経路の双方が、ビルド済み binary に対して実際に検査されるようになる。
一方 CI 側の変更はローカルで検証できないので、その旨を doc-GREEN で明示する。

# パフォーマンスベンチマーク実行

L# コンパイラのパフォーマンスを計測し、Rust/Go/JS と比較するためのコマンド。

## 引数

$ARGUMENTS = ベンチマーク対象 (例: "all", "compile", "wasm-size", "runtime", "memory", "compare")

## 計測対象

| 引数 | 内容 | 手段 |
|------|------|------|
| `compile` | パイプライン各ステージ速度 (parse/infer/lower/codegen) | `cargo bench -p lsharp-wasm` (criterion) |
| `wasm-size` | 生成 Wasm バイナリサイズ | `scripts/bench-wasm-size.sh` |
| `runtime` | Wasm 実行時間 | `wasmtime` + `/usr/bin/time` |
| `memory` | コンパイル時メモリ使用量 (RSS) | `/usr/bin/time -l cargo run -- compile` |
| `compare` | Rust/Go/JS との実行時間・メモリ・サイズ比較 | `scripts/bench-compare.sh` |
| `all` | 上記すべて | 順次実行 |

## ワークフロー

### Step 1: 基盤確認
- criterion が `Cargo.toml` に追加済みか確認 (`cargo bench --help` が動くか)
- `crates/lsharp-wasm/benches/` ディレクトリの存在を確認
- `scripts/bench-wasm-size.sh` / `scripts/bench-compare.sh` の存在を確認
- 未導入の基盤があれば、導入手順を提示して対処する

### Step 2: 計測実行

引数に応じて以下を実行する。引数が空の場合は `all` として扱う。

#### compile
```bash
cargo bench -p lsharp-wasm --bench compiler_pipeline
```
- parse / infer / lower / codegen / full_pipeline の各ステージ計測結果を確認
- 前回結果との差分が criterion 出力に含まれるので、回帰がないか確認

#### wasm-size
```bash
scripts/bench-wasm-size.sh
```
- 各 example の Wasm バイナリサイズを表示
- ベースラインとの差分を確認
- 10% 以上の増加があれば WARNING として報告

#### runtime
```bash
/usr/bin/time -l cargo run -- compile examples/fib.ls -o /tmp/lsharp-bench.wasm 2>&1
/usr/bin/time -l wasmtime /tmp/lsharp-bench.wasm 2>&1
```
- コンパイル時間と実行時間を個別に計測
- メモリ使用量 (RSS) も `/usr/bin/time -l` 出力から取得

#### memory
```bash
/usr/bin/time -l cargo run -- compile examples/fib.ls -o /tmp/lsharp-bench.wasm 2>&1
```
- maximum resident set size (RSS) を確認
- CPU 使用率 (%) を確認

#### compare
```bash
scripts/bench-compare.sh
```
- Rust / Go / JS / L# の fibonacci(35) 実行結果を比較
- 実行時間、メモリ使用量、バイナリサイズの 3 軸で比較表を生成

### Step 3: 結果レポート

計測結果を以下の形式でサマリとして報告する:

```
## ベンチマーク結果

### コンパイル速度 (criterion)
| ステージ | 時間 | 前回比 |
|---------|------|--------|
| parse   | Xms  | +Y%    |
| ...     | ...  | ...    |

### Wasm バイナリサイズ
| ファイル | サイズ | ベースライン比 |
|---------|--------|---------------|
| fib.ls  | X bytes | +Y%          |

### 言語比較 (fibonacci 35)
| 言語 | 実行時間 | RSS メモリ | バイナリサイズ |
|------|---------|-----------|-------------|
| Rust | Xms     | Y MB      | Z KB        |
| Go   | ...     | ...       | ...         |
| JS   | ...     | ...       | N/A         |
| L#   | ...     | ...       | ...         |
```

### Step 4: 回帰チェック

- 10% 以上の速度低下 → **WARNING** として明示
- 10% 以上のバイナリサイズ増加 → **WARNING** として明示
- 回帰がある場合、原因の推定と改善案を提示

### Step 5: レポート生成

計測結果を `docs/development/validation/BENCHMARK.md` に出力する:
```bash
scripts/bench-report.sh
```
- GitHub 上で確認可能な Markdown レポートが生成される
- コミットすればリポジトリで誰でも閲覧可能
- `--skip-bench` オプションで前回結果からレポートのみ再生成も可能

### Step 6: ベースライン更新 (任意)

ユーザーに確認の上、`scripts/bench-wasm-size.sh --save-baseline` でベースラインを更新するか判断する。

## 包括的計測項目 (将来対応含む)

| 計測項目 | 現在 | 将来 |
|---------|------|------|
| CPU 使用率 | `/usr/bin/time` | ✅ |
| GPU 使用率 | N/A (Wasm/WASI) | WebGPU 連携時 |
| メモリ使用量 | `/usr/bin/time -l` RSS | ✅ |
| GC 挙動 | N/A | WasmGC 統計 API 利用時 |
| バイナリサイズ | `wc -c` | ✅ |
| コンパイル速度 | criterion | ✅ |
| DOM 操作 | N/A | wasm-bindgen 導入時 |

## 重要なルール

- ベンチマーク結果は **数値で** 示す（主観的な「速い」「遅い」は避ける）
- 比較対象 (Rust/Go/JS) との差を **パーセンテージ** で明示する
- 回帰が見つかった場合は、変更を元に戻すか改善するかをユーザーと相談する
- ベースラインの更新はユーザー承認後のみ行う

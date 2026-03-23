# L# パフォーマンスベンチマーク レポート

> 計測日時: 2026-03-24 01:19:39
> Git: `8d09bd1` (main)
> プラットフォーム: Darwin arm64 (25.3.0)

---

## サマリ

| 項目 | 値 |
|------|-----|
| L# コンパイル時間 (fib.ls) | 1.91 s |
| L# コンパイル RSS メモリ | 11.0 MB |
| L# Wasm 実行時間 (fib 35) | 0.07 s |
| L# Wasm 実行 RSS メモリ | 21.5 MB |
| L# Wasm 平均サイズ | 1.7 KB |
| コンパイル成功数 | 14 / 14 |

---

## 言語比較 (fibonacci 35)

### コンパイル速度

| 言語 | コンパイル時間 | コンパイル RSS メモリ | CPU 使用率 |
|------|-------------|-------------------|-----------|
| Rust (`rustc -O`) | 0.32 s | 87.7 MB | N/A |
| Go (`go build`) | 0.22 s | 64.8 MB | N/A |
| L# (`lsharp compile`) | 1.91 s | 11.0 MB | N/A |
| JS (Node.js) | N/A (インタプリタ) | N/A | N/A |

### 実行速度

| 言語 | 実行時間 | 実行 RSS メモリ | CPU 使用率 |
|------|---------|---------------|-----------|
| Rust (ネイティブ) | 0.37 s | 1.4 MB | N/A |
| Go (ネイティブ) | 0.37 s | 3.5 MB | N/A |
| L# (wasmtime) | 0.07 s | 21.5 MB | N/A |
| JS (Node.js) | 0.07 s | 43.2 MB | N/A |

### バイナリサイズ

| 言語 | バイナリサイズ |
|------|-------------|
| Rust | 432.5 KB |
| Go | 2.3 MB |
| L# (Wasm) | 1.6 KB |
| JS | N/A (ソースコード実行) |

> **注**:
> - 時間は `/usr/bin/time` の real 時間 (壁時計時間)。単位は秒 (s)。
> - RSS メモリは maximum resident set size。単位は MB。
> - L# の実行時間は wasmtime ランタイム起動オーバーヘッドを含む。
> - L# のコンパイル時間は cargo の起動オーバーヘッドを含む (純粋なコンパイル時間は criterion を参照)。

---

## 詳細結果

### L# パイプライン内部速度 (criterion)

```
Benchmarking parse/fib
Benchmarking parse/fib: Warming up for 3.0000 s
Benchmarking parse/fib: Collecting 100 samples in estimated 5.0063 s (2.1M iterations)
Benchmarking parse/fib: Analyzing
parse/fib               time:   [2.3285 µs 2.3892 µs 2.4724 µs]
                        change: [-2.6685% +2.6989% +8.8267%] (p = 0.36 > 0.05)
Benchmarking parse/factorial
Benchmarking parse/factorial: Warming up for 3.0000 s
Benchmarking parse/factorial: Collecting 100 samples in estimated 5.0094 s (1.8M iterations)
Benchmarking parse/factorial: Analyzing
parse/factorial         time:   [2.7502 µs 2.8569 µs 2.9979 µs]
                        change: [-6.5809% -3.4539% -0.1155%] (p = 0.04 < 0.05)
Benchmarking parse/hello
Benchmarking parse/hello: Warming up for 3.0000 s
Benchmarking parse/hello: Collecting 100 samples in estimated 5.0017 s (10M iterations)
Benchmarking parse/hello: Analyzing
parse/hello             time:   [453.12 ns 461.81 ns 474.73 ns]
                        change: [-24.853% -19.349% -14.491%] (p = 0.00 < 0.05)
Benchmarking infer/fib
Benchmarking infer/fib: Warming up for 3.0000 s
Benchmarking infer/fib: Collecting 100 samples in estimated 5.9296 s (20k iterations)
Benchmarking infer/fib: Analyzing
infer/fib               time:   [280.39 µs 303.09 µs 334.11 µs]
                        change: [-17.212% -4.3207% +13.398%] (p = 0.61 > 0.05)
Benchmarking infer/factorial
Benchmarking infer/factorial: Warming up for 3.0000 s
Benchmarking infer/factorial: Collecting 100 samples in estimated 5.8287 s (15k iterations)
Benchmarking infer/factorial: Analyzing
infer/factorial         time:   [361.29 µs 386.01 µs 425.21 µs]
                        change: [-24.953% -19.343% -13.181%] (p = 0.00 < 0.05)
Benchmarking infer/hello
Benchmarking infer/hello: Warming up for 3.0000 s
Benchmarking infer/hello: Collecting 100 samples in estimated 5.2253 s (81k iterations)
Benchmarking infer/hello: Analyzing
infer/hello             time:   [57.120 µs 59.129 µs 61.424 µs]
                        change: [-10.890% -1.5223% +8.0423%] (p = 0.77 > 0.05)
Benchmarking lower/fib
Benchmarking lower/fib: Warming up for 3.0000 s
Benchmarking lower/fib: Collecting 100 samples in estimated 5.0077 s (3.0M iterations)
Benchmarking lower/fib: Analyzing
lower/fib               time:   [1.4210 µs 1.4850 µs 1.5626 µs]
                        change: [+7.4078% +15.044% +24.817%] (p = 0.00 < 0.05)
Benchmarking lower/factorial
Benchmarking lower/factorial: Warming up for 3.0000 s
Benchmarking lower/factorial: Collecting 100 samples in estimated 5.0040 s (2.7M iterations)
Benchmarking lower/factorial: Analyzing
lower/factorial         time:   [1.4918 µs 1.5597 µs 1.6410 µs]
                        change: [-8.3952% -0.5735% +7.1236%] (p = 0.89 > 0.05)
Benchmarking lower/hello
Benchmarking lower/hello: Warming up for 3.0000 s
Benchmarking lower/hello: Collecting 100 samples in estimated 5.0038 s (5.1M iterations)
Benchmarking lower/hello: Analyzing
lower/hello             time:   [817.24 ns 851.98 ns 895.12 ns]
                        change: [+4.5002% +10.104% +17.251%] (p = 0.00 < 0.05)
Benchmarking codegen/fib
Benchmarking codegen/fib: Warming up for 3.0000 s
Benchmarking codegen/fib: Collecting 100 samples in estimated 5.0095 s (591k iterations)
Benchmarking codegen/fib: Analyzing
codegen/fib             time:   [7.2400 µs 7.6243 µs 8.1125 µs]
                        change: [-6.9706% -0.6193% +5.6397%] (p = 0.85 > 0.05)
Benchmarking codegen/factorial
Benchmarking codegen/factorial: Warming up for 3.0000 s
Benchmarking codegen/factorial: Collecting 100 samples in estimated 5.0116 s (697k iterations)
Benchmarking codegen/factorial: Analyzing
codegen/factorial       time:   [7.1619 µs 7.8812 µs 8.9999 µs]
                        change: [+2.7226% +9.4622% +17.919%] (p = 0.01 < 0.05)
Benchmarking codegen/hello
Benchmarking codegen/hello: Warming up for 3.0000 s
Benchmarking codegen/hello: Collecting 100 samples in estimated 5.0296 s (758k iterations)
Benchmarking codegen/hello: Analyzing
codegen/hello           time:   [7.4118 µs 8.5627 µs 10.271 µs]
                        change: [-3.9187% +3.9925% +14.085%] (p = 0.45 > 0.05)
Benchmarking full_pipeline/fib
Benchmarking full_pipeline/fib: Warming up for 3.0000 s
Benchmarking full_pipeline/fib: Collecting 100 samples in estimated 5.6665 s (15k iterations)
Benchmarking full_pipeline/fib: Analyzing
full_pipeline/fib       time:   [285.24 µs 292.36 µs 302.59 µs]
                        change: [-6.6643% -0.8403% +4.7239%] (p = 0.80 > 0.05)
Benchmarking full_pipeline/factorial
Benchmarking full_pipeline/factorial: Warming up for 3.0000 s
Benchmarking full_pipeline/factorial: Collecting 100 samples in estimated 5.1297 s (10k iterations)
Benchmarking full_pipeline/factorial: Analyzing
full_pipeline/factorial time:   [720.23 µs 787.46 µs 849.49 µs]
                        change: [+56.035% +70.862% +86.777%] (p = 0.00 < 0.05)
Benchmarking full_pipeline/hello
Benchmarking full_pipeline/hello: Warming up for 3.0000 s
Benchmarking full_pipeline/hello: Collecting 100 samples in estimated 5.4163 s (35k iterations)
Benchmarking full_pipeline/hello: Analyzing
full_pipeline/hello     time:   [149.69 µs 165.80 µs 184.93 µs]
                        change: [+124.25% +144.28% +166.93%] (p = 0.00 < 0.05)
```

<details>
<summary>criterion 全出力</summary>

```
warning: function `emit_write_heap_header` is never used
   --> crates/lsharp-ir/src/lower/mod.rs:515:15
    |
515 | pub(crate) fn emit_write_heap_header(
    |               ^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `lsharp-ir` (lib) generated 1 warning
warning: unused variable: `args_get_idx`
  --> crates/lsharp-wasm/src/wasi.rs:58:9
   |
58 |     let args_get_idx: u32 = 2;
   |         ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_args_get_idx`
   |
   = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `gc_type_count`
  --> crates/lsharp-wasm/src/wasi.rs:79:9
   |
79 |     let gc_type_count = module.gc_types.len() as u32;
   |         ^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_gc_type_count`

warning: `lsharp-wasm` (lib) generated 2 warnings (run `cargo fix --lib -p lsharp-wasm` to apply 2 suggestions)
    Finished `bench` profile [optimized] target(s) in 0.37s
     Running benches/compiler_pipeline.rs (target/release/deps/compiler_pipeline-1e05cb3c97e3076f)
Gnuplot not found, using plotters backend
Benchmarking parse/fib
Benchmarking parse/fib: Warming up for 3.0000 s
Benchmarking parse/fib: Collecting 100 samples in estimated 5.0063 s (2.1M iterations)
Benchmarking parse/fib: Analyzing
parse/fib               time:   [2.3285 µs 2.3892 µs 2.4724 µs]
                        change: [-2.6685% +2.6989% +8.8267%] (p = 0.36 > 0.05)
                        No change in performance detected.
Found 13 outliers among 100 measurements (13.00%)
  3 (3.00%) high mild
  10 (10.00%) high severe
Benchmarking parse/factorial
Benchmarking parse/factorial: Warming up for 3.0000 s
Benchmarking parse/factorial: Collecting 100 samples in estimated 5.0094 s (1.8M iterations)
Benchmarking parse/factorial: Analyzing
parse/factorial         time:   [2.7502 µs 2.8569 µs 2.9979 µs]
                        change: [-6.5809% -3.4539% -0.1155%] (p = 0.04 < 0.05)
                        Change within noise threshold.
Found 11 outliers among 100 measurements (11.00%)
  3 (3.00%) high mild
  8 (8.00%) high severe
Benchmarking parse/hello
Benchmarking parse/hello: Warming up for 3.0000 s
Benchmarking parse/hello: Collecting 100 samples in estimated 5.0017 s (10M iterations)
Benchmarking parse/hello: Analyzing
parse/hello             time:   [453.12 ns 461.81 ns 474.73 ns]
                        change: [-24.853% -19.349% -14.491%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 6 outliers among 100 measurements (6.00%)
  3 (3.00%) high mild
  3 (3.00%) high severe

Benchmarking infer/fib
Benchmarking infer/fib: Warming up for 3.0000 s
Benchmarking infer/fib: Collecting 100 samples in estimated 5.9296 s (20k iterations)
Benchmarking infer/fib: Analyzing
infer/fib               time:   [280.39 µs 303.09 µs 334.11 µs]
                        change: [-17.212% -4.3207% +13.398%] (p = 0.61 > 0.05)
                        No change in performance detected.
Found 16 outliers among 100 measurements (16.00%)
  4 (4.00%) high mild
  12 (12.00%) high severe
Benchmarking infer/factorial
Benchmarking infer/factorial: Warming up for 3.0000 s
Benchmarking infer/factorial: Collecting 100 samples in estimated 5.8287 s (15k iterations)
Benchmarking infer/factorial: Analyzing
infer/factorial         time:   [361.29 µs 386.01 µs 425.21 µs]
                        change: [-24.953% -19.343% -13.181%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 10 outliers among 100 measurements (10.00%)
  5 (5.00%) high mild
  5 (5.00%) high severe
Benchmarking infer/hello
Benchmarking infer/hello: Warming up for 3.0000 s
Benchmarking infer/hello: Collecting 100 samples in estimated 5.2253 s (81k iterations)
Benchmarking infer/hello: Analyzing
infer/hello             time:   [57.120 µs 59.129 µs 61.424 µs]
                        change: [-10.890% -1.5223% +8.0423%] (p = 0.77 > 0.05)
                        No change in performance detected.
Found 8 outliers among 100 measurements (8.00%)
  5 (5.00%) high mild
  3 (3.00%) high severe

Benchmarking lower/fib
Benchmarking lower/fib: Warming up for 3.0000 s
Benchmarking lower/fib: Collecting 100 samples in estimated 5.0077 s (3.0M iterations)
Benchmarking lower/fib: Analyzing
lower/fib               time:   [1.4210 µs 1.4850 µs 1.5626 µs]
                        change: [+7.4078% +15.044% +24.817%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 8 outliers among 100 measurements (8.00%)
  4 (4.00%) high mild
  4 (4.00%) high severe
Benchmarking lower/factorial
Benchmarking lower/factorial: Warming up for 3.0000 s
Benchmarking lower/factorial: Collecting 100 samples in estimated 5.0040 s (2.7M iterations)
Benchmarking lower/factorial: Analyzing
lower/factorial         time:   [1.4918 µs 1.5597 µs 1.6410 µs]
                        change: [-8.3952% -0.5735% +7.1236%] (p = 0.89 > 0.05)
                        No change in performance detected.
Found 15 outliers among 100 measurements (15.00%)
  4 (4.00%) high mild
  11 (11.00%) high severe
Benchmarking lower/hello
Benchmarking lower/hello: Warming up for 3.0000 s
Benchmarking lower/hello: Collecting 100 samples in estimated 5.0038 s (5.1M iterations)
Benchmarking lower/hello: Analyzing
lower/hello             time:   [817.24 ns 851.98 ns 895.12 ns]
                        change: [+4.5002% +10.104% +17.251%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 11 outliers among 100 measurements (11.00%)
  5 (5.00%) high mild
  6 (6.00%) high severe

Benchmarking codegen/fib
Benchmarking codegen/fib: Warming up for 3.0000 s
Benchmarking codegen/fib: Collecting 100 samples in estimated 5.0095 s (591k iterations)
Benchmarking codegen/fib: Analyzing
codegen/fib             time:   [7.2400 µs 7.6243 µs 8.1125 µs]
                        change: [-6.9706% -0.6193% +5.6397%] (p = 0.85 > 0.05)
                        No change in performance detected.
Found 12 outliers among 100 measurements (12.00%)
  6 (6.00%) high mild
  6 (6.00%) high severe
Benchmarking codegen/factorial
Benchmarking codegen/factorial: Warming up for 3.0000 s
Benchmarking codegen/factorial: Collecting 100 samples in estimated 5.0116 s (697k iterations)
Benchmarking codegen/factorial: Analyzing
codegen/factorial       time:   [7.1619 µs 7.8812 µs 8.9999 µs]
                        change: [+2.7226% +9.4622% +17.919%] (p = 0.01 < 0.05)
                        Performance has regressed.
Found 11 outliers among 100 measurements (11.00%)
  4 (4.00%) high mild
  7 (7.00%) high severe
Benchmarking codegen/hello
Benchmarking codegen/hello: Warming up for 3.0000 s
Benchmarking codegen/hello: Collecting 100 samples in estimated 5.0296 s (758k iterations)
Benchmarking codegen/hello: Analyzing
codegen/hello           time:   [7.4118 µs 8.5627 µs 10.271 µs]
                        change: [-3.9187% +3.9925% +14.085%] (p = 0.45 > 0.05)
                        No change in performance detected.
Found 12 outliers among 100 measurements (12.00%)
  4 (4.00%) high mild
  8 (8.00%) high severe

Benchmarking full_pipeline/fib
Benchmarking full_pipeline/fib: Warming up for 3.0000 s
Benchmarking full_pipeline/fib: Collecting 100 samples in estimated 5.6665 s (15k iterations)
Benchmarking full_pipeline/fib: Analyzing
full_pipeline/fib       time:   [285.24 µs 292.36 µs 302.59 µs]
                        change: [-6.6643% -0.8403% +4.7239%] (p = 0.80 > 0.05)
                        No change in performance detected.
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) high mild
  4 (4.00%) high severe
Benchmarking full_pipeline/factorial
Benchmarking full_pipeline/factorial: Warming up for 3.0000 s
Benchmarking full_pipeline/factorial: Collecting 100 samples in estimated 5.1297 s (10k iterations)
Benchmarking full_pipeline/factorial: Analyzing
full_pipeline/factorial time:   [720.23 µs 787.46 µs 849.49 µs]
                        change: [+56.035% +70.862% +86.777%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild
Benchmarking full_pipeline/hello
Benchmarking full_pipeline/hello: Warming up for 3.0000 s
Benchmarking full_pipeline/hello: Collecting 100 samples in estimated 5.4163 s (35k iterations)
Benchmarking full_pipeline/hello: Analyzing
full_pipeline/hello     time:   [149.69 µs 165.80 µs 184.93 µs]
                        change: [+124.25% +144.28% +166.93%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) high mild
  1 (1.00%) high severe
```

</details>


### Wasm バイナリサイズ一覧

| ファイル | サイズ |
|---------|--------|
| `computation.ls` | 1.6 KB |
| `constrained.ls` | 1.8 KB |
| `factorial.ls` | 1.7 KB |
| `fib.ls` | 1.6 KB |
| `gadt.ls` | 1.8 KB |
| `hello.ls` | 1.6 KB |
| `hkt.ls` | 1.7 KB |
| `module.ls` | 1.6 KB |
| `nested-module.ls` | 1.7 KB |
| `record.ls` | 1.7 KB |
| `trait-where.ls` | 1.6 KB |
| `trait.ls` | 1.6 KB |
| `type-alias.ls` | 1.6 KB |
| `types.ls` | 1.9 KB |

| **平均** | **1.7 KB** |

---

## 計測環境

| 項目 | 値 |
|------|-----|
| OS | Darwin 25.3.0 |
| Arch | arm64 |
| Rust | rustc 1.93.0 (254b59607 2026-01-19) |
| Go | go version go1.25.8 darwin/arm64 |
| Node.js | v22.21.0 |
| wasmtime | wasmtime 43.0.0 (be23469ec 2026-03-20) |

---

## 計測対象の網羅性

| 計測項目 | ステータス | 手段 |
|---------|----------|------|
| コンパイル速度 | ✅ 全言語比較 | `/usr/bin/time` + criterion |
| 実行速度 | ✅ 全言語比較 | `/usr/bin/time` |
| CPU 使用率 | ✅ 全言語比較 | `/usr/bin/time` |
| メモリ使用量 (RSS) | ✅ 全言語比較 | `/usr/bin/time -l` |
| バイナリサイズ | ✅ 全言語比較 | `wc -c` |
| GPU 使用率 | N/A | Wasm/WASI に GPU アクセスなし |
| GC 挙動 | 将来対応 | WasmGC 統計 API 利用時 |
| DOM 操作 | 将来対応 | wasm-bindgen 導入時 |

---

*このレポートは `scripts/bench-report.sh` で自動生成されました。*

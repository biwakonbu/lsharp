# L# パフォーマンスベンチマーク レポート

> 計測日時: 2026-03-24 01:13:50
> Git: `925d4b8` (main)
> プラットフォーム: Darwin arm64 (25.3.0)

---

## サマリ

| 項目 | 値 |
|------|-----|
| L# コンパイル時間 (fib.ls) | 1.76 s |
| L# コンパイル RSS メモリ | 11.0 MB |
| L# Wasm 実行時間 (fib 10) | 0.03 s |
| L# Wasm 実行 RSS メモリ | 14.1 MB |
| L# Wasm 平均サイズ | 1.7 KB |
| コンパイル成功数 | 14 / 14 |

---

## 言語比較 (fibonacci 35)

### コンパイル速度

| 言語 | コンパイル時間 | コンパイル RSS メモリ | CPU 使用率 |
|------|-------------|-------------------|-----------|
| Rust (`rustc -O`) | 0.37 s | 87.8 MB | N/A |
| Go (`go build`) | 0.22 s | 65.0 MB | N/A |
| L# (`lsharp compile`) | 1.76 s | 11.0 MB | N/A |
| JS (Node.js) | N/A (インタプリタ) | N/A | N/A |

### 実行速度

| 言語 | 実行時間 | 実行 RSS メモリ | CPU 使用率 |
|------|---------|---------------|-----------|
| Rust (ネイティブ) | 0.33 s | 1.4 MB | N/A |
| Go (ネイティブ) | 0.41 s | 3.5 MB | N/A |
| L# (wasmtime) | 0.03 s | 14.1 MB | N/A |
| JS (Node.js) | 0.07 s | 43.1 MB | N/A |

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
Benchmarking parse/fib: Collecting 100 samples in estimated 5.0060 s (1.8M iterations)
Benchmarking parse/fib: Analyzing
parse/fib               time:   [2.3633 µs 2.4438 µs 2.5622 µs]
                        change: [-20.151% -11.358% -3.5892%] (p = 0.01 < 0.05)
Benchmarking parse/factorial
Benchmarking parse/factorial: Warming up for 3.0000 s
Benchmarking parse/factorial: Collecting 100 samples in estimated 5.0067 s (1.7M iterations)
Benchmarking parse/factorial: Analyzing
parse/factorial         time:   [2.8838 µs 2.9710 µs 3.0669 µs]
                        change: [-35.884% -23.297% -10.452%] (p = 0.00 < 0.05)
Benchmarking parse/hello
Benchmarking parse/hello: Warming up for 3.0000 s
Benchmarking parse/hello: Collecting 100 samples in estimated 5.0021 s (7.6M iterations)
Benchmarking parse/hello: Analyzing
parse/hello             time:   [529.39 ns 556.00 ns 587.02 ns]
                        change: [+2.7012% +10.329% +20.940%] (p = 0.01 < 0.05)
Benchmarking infer/fib
Benchmarking infer/fib: Warming up for 3.0000 s
Benchmarking infer/fib: Collecting 100 samples in estimated 5.8583 s (15k iterations)
Benchmarking infer/fib: Analyzing
infer/fib               time:   [353.44 µs 413.24 µs 482.17 µs]
                        change: [+7.0128% +17.791% +29.322%] (p = 0.00 < 0.05)
Benchmarking infer/factorial
Benchmarking infer/factorial: Warming up for 3.0000 s
Benchmarking infer/factorial: Collecting 100 samples in estimated 5.9920 s (10k iterations)
Benchmarking infer/factorial: Analyzing
infer/factorial         time:   [422.53 µs 451.24 µs 485.99 µs]
                        change: [+11.986% +21.119% +30.648%] (p = 0.00 < 0.05)
Benchmarking infer/hello
Benchmarking infer/hello: Warming up for 3.0000 s
Benchmarking infer/hello: Collecting 100 samples in estimated 5.0870 s (66k iterations)
Benchmarking infer/hello: Analyzing
infer/hello             time:   [62.353 µs 67.996 µs 74.476 µs]
                        change: [-19.601% -6.7037% +7.0608%] (p = 0.37 > 0.05)
Benchmarking lower/fib
Benchmarking lower/fib: Warming up for 3.0000 s
Benchmarking lower/fib: Collecting 100 samples in estimated 5.0024 s (3.7M iterations)
Benchmarking lower/fib: Analyzing
lower/fib               time:   [1.3243 µs 1.3781 µs 1.4500 µs]
                        change: [-22.282% -14.643% -7.6991%] (p = 0.00 < 0.05)
Benchmarking lower/factorial
Benchmarking lower/factorial: Warming up for 3.0000 s
Benchmarking lower/factorial: Collecting 100 samples in estimated 5.0035 s (3.3M iterations)
Benchmarking lower/factorial: Analyzing
lower/factorial         time:   [1.4728 µs 1.6049 µs 1.7755 µs]
                        change: [-3.0938% +4.6511% +13.281%] (p = 0.31 > 0.05)
Benchmarking lower/hello
Benchmarking lower/hello: Warming up for 3.0000 s
Benchmarking lower/hello: Collecting 100 samples in estimated 5.0037 s (6.5M iterations)
Benchmarking lower/hello: Analyzing
lower/hello             time:   [796.29 ns 826.50 ns 860.33 ns]
                        change: [-9.1853% -4.7068% -0.0479%] (p = 0.05 < 0.05)
Benchmarking codegen/fib
Benchmarking codegen/fib: Warming up for 3.0000 s
Benchmarking codegen/fib: Collecting 100 samples in estimated 5.0033 s (682k iterations)
Benchmarking codegen/fib: Analyzing
codegen/fib             time:   [7.3538 µs 7.6892 µs 8.0740 µs]
                        change: [-27.150% -13.205% +1.3690%] (p = 0.13 > 0.05)
Benchmarking codegen/factorial
Benchmarking codegen/factorial: Warming up for 3.0000 s
Benchmarking codegen/factorial: Collecting 100 samples in estimated 5.0280 s (692k iterations)
Benchmarking codegen/factorial: Analyzing
codegen/factorial       time:   [6.7496 µs 6.9417 µs 7.2461 µs]
                        change: [-16.586% -8.7323% -1.1502%] (p = 0.04 < 0.05)
Benchmarking codegen/hello
Benchmarking codegen/hello: Warming up for 3.0000 s
Benchmarking codegen/hello: Collecting 100 samples in estimated 5.0051 s (722k iterations)
Benchmarking codegen/hello: Analyzing
codegen/hello           time:   [6.7731 µs 7.1050 µs 7.6045 µs]
                        change: [-10.100% -3.8603% +2.9214%] (p = 0.25 > 0.05)
Benchmarking full_pipeline/fib
Benchmarking full_pipeline/fib: Warming up for 3.0000 s
Benchmarking full_pipeline/fib: Collecting 100 samples in estimated 5.9631 s (20k iterations)
Benchmarking full_pipeline/fib: Analyzing
full_pipeline/fib       time:   [286.93 µs 296.09 µs 307.30 µs]
                        change: [-4.1539% +0.8129% +6.8870%] (p = 0.80 > 0.05)
Benchmarking full_pipeline/factorial
Benchmarking full_pipeline/factorial: Warming up for 3.0000 s
Benchmarking full_pipeline/factorial: Collecting 100 samples in estimated 5.5558 s (15k iterations)
Benchmarking full_pipeline/factorial: Analyzing
full_pipeline/factorial time:   [374.19 µs 391.20 µs 412.91 µs]
                        change: [-2.9542% +2.3217% +7.5608%] (p = 0.41 > 0.05)
Benchmarking full_pipeline/hello
Benchmarking full_pipeline/hello: Warming up for 3.0000 s
Benchmarking full_pipeline/hello: Collecting 100 samples in estimated 5.2057 s (86k iterations)
Benchmarking full_pipeline/hello: Analyzing
full_pipeline/hello     time:   [58.614 µs 59.866 µs 61.494 µs]
                        change: [-6.3238% -2.4765% +1.2136%] (p = 0.22 > 0.05)
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
    Finished `bench` profile [optimized] target(s) in 0.47s
     Running benches/compiler_pipeline.rs (target/release/deps/compiler_pipeline-1e05cb3c97e3076f)
Gnuplot not found, using plotters backend
Benchmarking parse/fib
Benchmarking parse/fib: Warming up for 3.0000 s
Benchmarking parse/fib: Collecting 100 samples in estimated 5.0060 s (1.8M iterations)
Benchmarking parse/fib: Analyzing
parse/fib               time:   [2.3633 µs 2.4438 µs 2.5622 µs]
                        change: [-20.151% -11.358% -3.5892%] (p = 0.01 < 0.05)
                        Performance has improved.
Found 8 outliers among 100 measurements (8.00%)
  5 (5.00%) high mild
  3 (3.00%) high severe
Benchmarking parse/factorial
Benchmarking parse/factorial: Warming up for 3.0000 s
Benchmarking parse/factorial: Collecting 100 samples in estimated 5.0067 s (1.7M iterations)
Benchmarking parse/factorial: Analyzing
parse/factorial         time:   [2.8838 µs 2.9710 µs 3.0669 µs]
                        change: [-35.884% -23.297% -10.452%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) high mild
  2 (2.00%) high severe
Benchmarking parse/hello
Benchmarking parse/hello: Warming up for 3.0000 s
Benchmarking parse/hello: Collecting 100 samples in estimated 5.0021 s (7.6M iterations)
Benchmarking parse/hello: Analyzing
parse/hello             time:   [529.39 ns 556.00 ns 587.02 ns]
                        change: [+2.7012% +10.329% +20.940%] (p = 0.01 < 0.05)
                        Performance has regressed.
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) high mild
  4 (4.00%) high severe

Benchmarking infer/fib
Benchmarking infer/fib: Warming up for 3.0000 s
Benchmarking infer/fib: Collecting 100 samples in estimated 5.8583 s (15k iterations)
Benchmarking infer/fib: Analyzing
infer/fib               time:   [353.44 µs 413.24 µs 482.17 µs]
                        change: [+7.0128% +17.791% +29.322%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 10 outliers among 100 measurements (10.00%)
  2 (2.00%) high mild
  8 (8.00%) high severe
Benchmarking infer/factorial
Benchmarking infer/factorial: Warming up for 3.0000 s
Benchmarking infer/factorial: Collecting 100 samples in estimated 5.9920 s (10k iterations)
Benchmarking infer/factorial: Analyzing
infer/factorial         time:   [422.53 µs 451.24 µs 485.99 µs]
                        change: [+11.986% +21.119% +30.648%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 5 outliers among 100 measurements (5.00%)
  2 (2.00%) high mild
  3 (3.00%) high severe
Benchmarking infer/hello
Benchmarking infer/hello: Warming up for 3.0000 s
Benchmarking infer/hello: Collecting 100 samples in estimated 5.0870 s (66k iterations)
Benchmarking infer/hello: Analyzing
infer/hello             time:   [62.353 µs 67.996 µs 74.476 µs]
                        change: [-19.601% -6.7037% +7.0608%] (p = 0.37 > 0.05)
                        No change in performance detected.
Found 10 outliers among 100 measurements (10.00%)
  6 (6.00%) high mild
  4 (4.00%) high severe

Benchmarking lower/fib
Benchmarking lower/fib: Warming up for 3.0000 s
Benchmarking lower/fib: Collecting 100 samples in estimated 5.0024 s (3.7M iterations)
Benchmarking lower/fib: Analyzing
lower/fib               time:   [1.3243 µs 1.3781 µs 1.4500 µs]
                        change: [-22.282% -14.643% -7.6991%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 11 outliers among 100 measurements (11.00%)
  5 (5.00%) high mild
  6 (6.00%) high severe
Benchmarking lower/factorial
Benchmarking lower/factorial: Warming up for 3.0000 s
Benchmarking lower/factorial: Collecting 100 samples in estimated 5.0035 s (3.3M iterations)
Benchmarking lower/factorial: Analyzing
lower/factorial         time:   [1.4728 µs 1.6049 µs 1.7755 µs]
                        change: [-3.0938% +4.6511% +13.281%] (p = 0.31 > 0.05)
                        No change in performance detected.
Found 14 outliers among 100 measurements (14.00%)
  3 (3.00%) high mild
  11 (11.00%) high severe
Benchmarking lower/hello
Benchmarking lower/hello: Warming up for 3.0000 s
Benchmarking lower/hello: Collecting 100 samples in estimated 5.0037 s (6.5M iterations)
Benchmarking lower/hello: Analyzing
lower/hello             time:   [796.29 ns 826.50 ns 860.33 ns]
                        change: [-9.1853% -4.7068% -0.0479%] (p = 0.05 < 0.05)
                        Change within noise threshold.
Found 13 outliers among 100 measurements (13.00%)
  8 (8.00%) high mild
  5 (5.00%) high severe

Benchmarking codegen/fib
Benchmarking codegen/fib: Warming up for 3.0000 s
Benchmarking codegen/fib: Collecting 100 samples in estimated 5.0033 s (682k iterations)
Benchmarking codegen/fib: Analyzing
codegen/fib             time:   [7.3538 µs 7.6892 µs 8.0740 µs]
                        change: [-27.150% -13.205% +1.3690%] (p = 0.13 > 0.05)
                        No change in performance detected.
Found 9 outliers among 100 measurements (9.00%)
  5 (5.00%) high mild
  4 (4.00%) high severe
Benchmarking codegen/factorial
Benchmarking codegen/factorial: Warming up for 3.0000 s
Benchmarking codegen/factorial: Collecting 100 samples in estimated 5.0280 s (692k iterations)
Benchmarking codegen/factorial: Analyzing
codegen/factorial       time:   [6.7496 µs 6.9417 µs 7.2461 µs]
                        change: [-16.586% -8.7323% -1.1502%] (p = 0.04 < 0.05)
                        Performance has improved.
Found 6 outliers among 100 measurements (6.00%)
  3 (3.00%) high mild
  3 (3.00%) high severe
Benchmarking codegen/hello
Benchmarking codegen/hello: Warming up for 3.0000 s
Benchmarking codegen/hello: Collecting 100 samples in estimated 5.0051 s (722k iterations)
Benchmarking codegen/hello: Analyzing
codegen/hello           time:   [6.7731 µs 7.1050 µs 7.6045 µs]
                        change: [-10.100% -3.8603% +2.9214%] (p = 0.25 > 0.05)
                        No change in performance detected.
Found 13 outliers among 100 measurements (13.00%)
  8 (8.00%) high mild
  5 (5.00%) high severe

Benchmarking full_pipeline/fib
Benchmarking full_pipeline/fib: Warming up for 3.0000 s
Benchmarking full_pipeline/fib: Collecting 100 samples in estimated 5.9631 s (20k iterations)
Benchmarking full_pipeline/fib: Analyzing
full_pipeline/fib       time:   [286.93 µs 296.09 µs 307.30 µs]
                        change: [-4.1539% +0.8129% +6.8870%] (p = 0.80 > 0.05)
                        No change in performance detected.
Found 10 outliers among 100 measurements (10.00%)
  3 (3.00%) high mild
  7 (7.00%) high severe
Benchmarking full_pipeline/factorial
Benchmarking full_pipeline/factorial: Warming up for 3.0000 s
Benchmarking full_pipeline/factorial: Collecting 100 samples in estimated 5.5558 s (15k iterations)
Benchmarking full_pipeline/factorial: Analyzing
full_pipeline/factorial time:   [374.19 µs 391.20 µs 412.91 µs]
                        change: [-2.9542% +2.3217% +7.5608%] (p = 0.41 > 0.05)
                        No change in performance detected.
Found 10 outliers among 100 measurements (10.00%)
  1 (1.00%) high mild
  9 (9.00%) high severe
Benchmarking full_pipeline/hello
Benchmarking full_pipeline/hello: Warming up for 3.0000 s
Benchmarking full_pipeline/hello: Collecting 100 samples in estimated 5.2057 s (86k iterations)
Benchmarking full_pipeline/hello: Analyzing
full_pipeline/hello     time:   [58.614 µs 59.866 µs 61.494 µs]
                        change: [-6.3238% -2.4765% +1.2136%] (p = 0.22 > 0.05)
                        No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
  3 (3.00%) high mild
  2 (2.00%) high severe
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

# L# パフォーマンスベンチマーク レポート

> 計測日時: 2026-03-24 01:01:18
> Git: `9e80781` (main)
> プラットフォーム: Darwin arm64 (25.3.0)

## サマリ

| 項目 | 値 |
|------|-----|
| コンパイル時間 (fib.ls) | 1.42 |
| コンパイル RSS メモリ | 11.1 MB |
| Wasm 実行時間 (fib 10) | 0.03 |
| Wasm 実行 RSS メモリ | 14.1 MB |
| 平均 Wasm サイズ | 1722 B (1.7 KB) |
| コンパイル成功数 | 14 / 14 |

---

## 詳細結果

### コンパイル速度 (criterion)

```
Benchmarking parse/fib
Benchmarking parse/fib: Warming up for 3.0000 s
Benchmarking parse/fib: Collecting 100 samples in estimated 5.0113 s (1.8M iterations)
Benchmarking parse/fib: Analyzing
parse/fib               time:   [2.6661 µs 3.0741 µs 3.6672 µs]
                        change: [-9.2302% -0.0151% +12.766%] (p = 1.00 > 0.05)
Benchmarking parse/factorial
Benchmarking parse/factorial: Warming up for 3.0000 s
Benchmarking parse/factorial: Collecting 100 samples in estimated 5.0067 s (1.5M iterations)
Benchmarking parse/factorial: Analyzing
parse/factorial         time:   [2.9782 µs 3.1596 µs 3.3981 µs]
                        change: [+8.1206% +26.055% +52.958%] (p = 0.01 < 0.05)
Benchmarking parse/hello
Benchmarking parse/hello: Warming up for 3.0000 s
Benchmarking parse/hello: Collecting 100 samples in estimated 5.0017 s (9.2M iterations)
Benchmarking parse/hello: Analyzing
parse/hello             time:   [479.17 ns 504.02 ns 535.04 ns]
                        change: [+1.0610% +6.6604% +12.797%] (p = 0.02 < 0.05)
Benchmarking infer/fib
Benchmarking infer/fib: Warming up for 3.0000 s
Benchmarking infer/fib: Collecting 100 samples in estimated 5.8791 s (20k iterations)
Benchmarking infer/fib: Analyzing
infer/fib               time:   [284.93 µs 303.17 µs 325.95 µs]
                        change: [-2.8595% +4.8536% +11.791%] (p = 0.21 > 0.05)
Benchmarking infer/factorial
Benchmarking infer/factorial: Warming up for 3.0000 s
Benchmarking infer/factorial: Collecting 100 samples in estimated 5.5563 s (15k iterations)
Benchmarking infer/factorial: Analyzing
infer/factorial         time:   [362.03 µs 400.06 µs 443.93 µs]
                        change: [-12.951% -4.4481% +3.6491%] (p = 0.34 > 0.05)
Benchmarking infer/hello
Benchmarking infer/hello: Warming up for 3.0000 s
Benchmarking infer/hello: Collecting 100 samples in estimated 5.3736 s (61k iterations)
Benchmarking infer/hello: Analyzing
infer/hello             time:   [63.354 µs 70.341 µs 79.433 µs]
                        change: [+18.015% +33.054% +47.150%] (p = 0.00 < 0.05)
Benchmarking lower/fib
Benchmarking lower/fib: Warming up for 3.0000 s
Benchmarking lower/fib: Collecting 100 samples in estimated 5.0077 s (2.4M iterations)
Benchmarking lower/fib: Analyzing
lower/fib               time:   [1.5443 µs 1.6856 µs 1.8720 µs]
                        change: [+16.105% +25.151% +37.348%] (p = 0.00 < 0.05)
Benchmarking lower/factorial
Benchmarking lower/factorial: Warming up for 3.0000 s
Benchmarking lower/factorial: Collecting 100 samples in estimated 5.0013 s (2.9M iterations)
Benchmarking lower/factorial: Analyzing
lower/factorial         time:   [1.4438 µs 1.4896 µs 1.5540 µs]
                        change: [-13.097% -6.6221% +0.5667%] (p = 0.06 > 0.05)
Benchmarking lower/hello
Benchmarking lower/hello: Warming up for 3.0000 s
Benchmarking lower/hello: Collecting 100 samples in estimated 5.0034 s (5.9M iterations)
Benchmarking lower/hello: Analyzing
lower/hello             time:   [849.85 ns 895.03 ns 945.26 ns]
                        change: [+6.1200% +10.294% +15.077%] (p = 0.00 < 0.05)
Benchmarking codegen/fib
Benchmarking codegen/fib: Warming up for 3.0000 s
Benchmarking codegen/fib: Collecting 100 samples in estimated 5.0006 s (636k iterations)
Benchmarking codegen/fib: Analyzing
codegen/fib             time:   [6.9525 µs 7.1855 µs 7.5306 µs]
                        change: [+0.7351% +18.751% +45.155%] (p = 0.08 > 0.05)
Benchmarking codegen/factorial
Benchmarking codegen/factorial: Warming up for 3.0000 s
Benchmarking codegen/factorial: Collecting 100 samples in estimated 5.0187 s (712k iterations)
Benchmarking codegen/factorial: Analyzing
codegen/factorial       time:   [6.9389 µs 7.1464 µs 7.4331 µs]
                        change: [-19.564% -6.6249% +5.9802%] (p = 0.41 > 0.05)
Benchmarking codegen/hello
Benchmarking codegen/hello: Warming up for 3.0000 s
Benchmarking codegen/hello: Collecting 100 samples in estimated 5.0215 s (768k iterations)
Benchmarking codegen/hello: Analyzing
codegen/hello           time:   [7.1149 µs 7.5172 µs 7.9893 µs]
                        change: [-12.650% -0.0073% +12.280%] (p = 1.00 > 0.05)
Benchmarking full_pipeline/fib
Benchmarking full_pipeline/fib: Warming up for 3.0000 s
Benchmarking full_pipeline/fib: Collecting 100 samples in estimated 6.4961 s (20k iterations)
Benchmarking full_pipeline/fib: Analyzing
full_pipeline/fib       time:   [284.64 µs 290.42 µs 298.40 µs]
                        change: [-18.740% -11.610% -5.4084%] (p = 0.00 < 0.05)
Benchmarking full_pipeline/factorial
Benchmarking full_pipeline/factorial: Warming up for 3.0000 s
Benchmarking full_pipeline/factorial: Collecting 100 samples in estimated 5.4848 s (15k iterations)
Benchmarking full_pipeline/factorial: Analyzing
full_pipeline/factorial time:   [365.97 µs 385.11 µs 414.24 µs]
                        change: [-26.812% -19.980% -13.851%] (p = 0.00 < 0.05)
Benchmarking full_pipeline/hello
Benchmarking full_pipeline/hello: Warming up for 3.0000 s
Benchmarking full_pipeline/hello: Collecting 100 samples in estimated 5.0980 s (86k iterations)
Benchmarking full_pipeline/hello: Analyzing
full_pipeline/hello     time:   [59.401 µs 62.696 µs 66.589 µs]
                        change: [-5.4330% -1.5041% +2.6927%] (p = 0.46 > 0.05)
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
    Finished `bench` profile [optimized] target(s) in 0.29s
     Running benches/compiler_pipeline.rs (target/release/deps/compiler_pipeline-1e05cb3c97e3076f)
Gnuplot not found, using plotters backend
Benchmarking parse/fib
Benchmarking parse/fib: Warming up for 3.0000 s
Benchmarking parse/fib: Collecting 100 samples in estimated 5.0113 s (1.8M iterations)
Benchmarking parse/fib: Analyzing
parse/fib               time:   [2.6661 µs 3.0741 µs 3.6672 µs]
                        change: [-9.2302% -0.0151% +12.766%] (p = 1.00 > 0.05)
                        No change in performance detected.
Found 18 outliers among 100 measurements (18.00%)
  6 (6.00%) high mild
  12 (12.00%) high severe
Benchmarking parse/factorial
Benchmarking parse/factorial: Warming up for 3.0000 s
Benchmarking parse/factorial: Collecting 100 samples in estimated 5.0067 s (1.5M iterations)
Benchmarking parse/factorial: Analyzing
parse/factorial         time:   [2.9782 µs 3.1596 µs 3.3981 µs]
                        change: [+8.1206% +26.055% +52.958%] (p = 0.01 < 0.05)
                        Performance has regressed.
Found 13 outliers among 100 measurements (13.00%)
  4 (4.00%) high mild
  9 (9.00%) high severe
Benchmarking parse/hello
Benchmarking parse/hello: Warming up for 3.0000 s
Benchmarking parse/hello: Collecting 100 samples in estimated 5.0017 s (9.2M iterations)
Benchmarking parse/hello: Analyzing
parse/hello             time:   [479.17 ns 504.02 ns 535.04 ns]
                        change: [+1.0610% +6.6604% +12.797%] (p = 0.02 < 0.05)
                        Performance has regressed.
Found 14 outliers among 100 measurements (14.00%)
  6 (6.00%) high mild
  8 (8.00%) high severe

Benchmarking infer/fib
Benchmarking infer/fib: Warming up for 3.0000 s
Benchmarking infer/fib: Collecting 100 samples in estimated 5.8791 s (20k iterations)
Benchmarking infer/fib: Analyzing
infer/fib               time:   [284.93 µs 303.17 µs 325.95 µs]
                        change: [-2.8595% +4.8536% +11.791%] (p = 0.21 > 0.05)
                        No change in performance detected.
Found 11 outliers among 100 measurements (11.00%)
  4 (4.00%) high mild
  7 (7.00%) high severe
Benchmarking infer/factorial
Benchmarking infer/factorial: Warming up for 3.0000 s
Benchmarking infer/factorial: Collecting 100 samples in estimated 5.5563 s (15k iterations)
Benchmarking infer/factorial: Analyzing
infer/factorial         time:   [362.03 µs 400.06 µs 443.93 µs]
                        change: [-12.951% -4.4481% +3.6491%] (p = 0.34 > 0.05)
                        No change in performance detected.
Found 15 outliers among 100 measurements (15.00%)
  4 (4.00%) high mild
  11 (11.00%) high severe
Benchmarking infer/hello
Benchmarking infer/hello: Warming up for 3.0000 s
Benchmarking infer/hello: Collecting 100 samples in estimated 5.3736 s (61k iterations)
Benchmarking infer/hello: Analyzing
infer/hello             time:   [63.354 µs 70.341 µs 79.433 µs]
                        change: [+18.015% +33.054% +47.150%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 12 outliers among 100 measurements (12.00%)
  6 (6.00%) high mild
  6 (6.00%) high severe

Benchmarking lower/fib
Benchmarking lower/fib: Warming up for 3.0000 s
Benchmarking lower/fib: Collecting 100 samples in estimated 5.0077 s (2.4M iterations)
Benchmarking lower/fib: Analyzing
lower/fib               time:   [1.5443 µs 1.6856 µs 1.8720 µs]
                        change: [+16.105% +25.151% +37.348%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 9 outliers among 100 measurements (9.00%)
  6 (6.00%) high mild
  3 (3.00%) high severe
Benchmarking lower/factorial
Benchmarking lower/factorial: Warming up for 3.0000 s
Benchmarking lower/factorial: Collecting 100 samples in estimated 5.0013 s (2.9M iterations)
Benchmarking lower/factorial: Analyzing
lower/factorial         time:   [1.4438 µs 1.4896 µs 1.5540 µs]
                        change: [-13.097% -6.6221% +0.5667%] (p = 0.06 > 0.05)
                        No change in performance detected.
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) high mild
  2 (2.00%) high severe
Benchmarking lower/hello
Benchmarking lower/hello: Warming up for 3.0000 s
Benchmarking lower/hello: Collecting 100 samples in estimated 5.0034 s (5.9M iterations)
Benchmarking lower/hello: Analyzing
lower/hello             time:   [849.85 ns 895.03 ns 945.26 ns]
                        change: [+6.1200% +10.294% +15.077%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 11 outliers among 100 measurements (11.00%)
  3 (3.00%) high mild
  8 (8.00%) high severe

Benchmarking codegen/fib
Benchmarking codegen/fib: Warming up for 3.0000 s
Benchmarking codegen/fib: Collecting 100 samples in estimated 5.0006 s (636k iterations)
Benchmarking codegen/fib: Analyzing
codegen/fib             time:   [6.9525 µs 7.1855 µs 7.5306 µs]
                        change: [+0.7351% +18.751% +45.155%] (p = 0.08 > 0.05)
                        No change in performance detected.
Found 12 outliers among 100 measurements (12.00%)
  12 (12.00%) high severe
Benchmarking codegen/factorial
Benchmarking codegen/factorial: Warming up for 3.0000 s
Benchmarking codegen/factorial: Collecting 100 samples in estimated 5.0187 s (712k iterations)
Benchmarking codegen/factorial: Analyzing
codegen/factorial       time:   [6.9389 µs 7.1464 µs 7.4331 µs]
                        change: [-19.564% -6.6249% +5.9802%] (p = 0.41 > 0.05)
                        No change in performance detected.
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) high mild
  6 (6.00%) high severe
Benchmarking codegen/hello
Benchmarking codegen/hello: Warming up for 3.0000 s
Benchmarking codegen/hello: Collecting 100 samples in estimated 5.0215 s (768k iterations)
Benchmarking codegen/hello: Analyzing
codegen/hello           time:   [7.1149 µs 7.5172 µs 7.9893 µs]
                        change: [-12.650% -0.0073% +12.280%] (p = 1.00 > 0.05)
                        No change in performance detected.
Found 12 outliers among 100 measurements (12.00%)
  6 (6.00%) high mild
  6 (6.00%) high severe

Benchmarking full_pipeline/fib
Benchmarking full_pipeline/fib: Warming up for 3.0000 s
Benchmarking full_pipeline/fib: Collecting 100 samples in estimated 6.4961 s (20k iterations)
Benchmarking full_pipeline/fib: Analyzing
full_pipeline/fib       time:   [284.64 µs 290.42 µs 298.40 µs]
                        change: [-18.740% -11.610% -5.4084%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 11 outliers among 100 measurements (11.00%)
  8 (8.00%) high mild
  3 (3.00%) high severe
Benchmarking full_pipeline/factorial
Benchmarking full_pipeline/factorial: Warming up for 3.0000 s
Benchmarking full_pipeline/factorial: Collecting 100 samples in estimated 5.4848 s (15k iterations)
Benchmarking full_pipeline/factorial: Analyzing
full_pipeline/factorial time:   [365.97 µs 385.11 µs 414.24 µs]
                        change: [-26.812% -19.980% -13.851%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 13 outliers among 100 measurements (13.00%)
  4 (4.00%) high mild
  9 (9.00%) high severe
Benchmarking full_pipeline/hello
Benchmarking full_pipeline/hello: Warming up for 3.0000 s
Benchmarking full_pipeline/hello: Collecting 100 samples in estimated 5.0980 s (86k iterations)
Benchmarking full_pipeline/hello: Analyzing
full_pipeline/hello     time:   [59.401 µs 62.696 µs 66.589 µs]
                        change: [-5.4330% -1.5041% +2.6927%] (p = 0.46 > 0.05)
                        No change in performance detected.
Found 14 outliers among 100 measurements (14.00%)
  2 (2.00%) high mild
  12 (12.00%) high severe
```

</details>


### Wasm バイナリサイズ

| ファイル | サイズ (bytes) | サイズ (KB) |
|---------|---------------|------------|
| `computation.ls` | 1665 B | 1.6 KB |
| `constrained.ls` | 1866 B | 1.8 KB |
| `factorial.ls` | 1694 B | 1.7 KB |
| `fib.ls` | 1681 B | 1.6 KB |
| `gadt.ls` | 1828 B | 1.8 KB |
| `hello.ls` | 1641 B | 1.6 KB |
| `hkt.ls` | 1755 B | 1.7 KB |
| `module.ls` | 1680 B | 1.6 KB |
| `nested-module.ls` | 1698 B | 1.7 KB |
| `record.ls` | 1711 B | 1.7 KB |
| `trait-where.ls` | 1661 B | 1.6 KB |
| `trait.ls` | 1641 B | 1.6 KB |
| `type-alias.ls` | 1661 B | 1.6 KB |
| `types.ls` | 1923 B | 1.9 KB |


### ランタイム計測 (fib.ls)

| 計測項目 | コンパイル | 実行 (wasmtime) |
|---------|-----------|----------------|
| 時間 | 1.42 | 0.03 |
| RSS メモリ | 11.1 MB | 14.1 MB |

### 言語比較 (fibonacci 35)

| 言語 | 実行時間 | RSS メモリ | バイナリサイズ |
|------|---------|-----------|-------------|
| Rust | 0.74 | 1.4 MB | 432.5 KB |
| Go | 0.47 | 3.7 MB | 2348.3 KB |
| JS (Node.js) | 0.15 | 43.4 MB | N/A |
| L# (Wasm) | 0.03 | 14.1 MB | 1.6 KB |

> **注**: 実行時間は `/usr/bin/time` の real 時間。L# は wasmtime 経由の実行のため、ランタイム起動オーバーヘッドを含む。

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
| CPU 使用率 | ✅ 計測済み | `/usr/bin/time` |
| メモリ使用量 (RSS) | ✅ 計測済み | `/usr/bin/time -l` |
| バイナリサイズ | ✅ 計測済み | `wc -c` |
| コンパイル速度 | ✅ 計測済み | criterion |
| GPU 使用率 | N/A | Wasm/WASI に GPU アクセスなし |
| GC 挙動 | 将来対応 | WasmGC 統計 API 利用時 |
| DOM 操作 | 将来対応 | wasm-bindgen 導入時 |

---

*このレポートは `scripts/bench-report.sh` で自動生成されました。*

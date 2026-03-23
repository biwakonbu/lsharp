# L# パフォーマンスベンチマーク レポート

> 計測日時: 2026-03-24 01:31:50
> Git: `24e5a98` (main)
> プラットフォーム: Darwin arm64 (25.3.0)

---

## サマリ

| 項目 | 値 |
|------|-----|
| L# コンパイル時間 (fib.ls) | 2.32 s |
| L# コンパイル RSS メモリ | 11.0 MB |
| L# Wasm 実行時間 (fib 35) | 0.03 s |
| L# Wasm 実行 RSS メモリ | 14.2 MB |
| L# Wasm 平均サイズ | 1.7 KB |
| コンパイル成功数 | 14 / 14 |

---

## 言語比較 (fibonacci 35)

### コンパイル速度

| 言語 | コンパイル時間 | コンパイル RSS メモリ | CPU 使用率 |
|------|-------------|-------------------|-----------|
| Rust (`rustc -O`) | 0.63 s | 87.7 MB | N/A |
| Go (`go build`) | 0.34 s | 64.9 MB | N/A |
| L# (`lsharp compile`) | 2.32 s | 11.0 MB | N/A |
| JS (Node.js) | N/A (インタプリタ) | N/A | N/A |

### 実行速度

| 言語 | 実行時間 | 実行 RSS メモリ | CPU 使用率 |
|------|---------|---------------|-----------|
| Rust (ネイティブ) | 0.01 s | 1.4 MB | N/A |
| Go (ネイティブ) | 0.02 s | 3.6 MB | N/A |
| L# (wasmtime) | 0.03 s | 14.2 MB | N/A |
| JS (Node.js) | 0.08 s | 43.1 MB | N/A |

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

> `--skip-bench` 指定のためスキップ。`scripts/bench-report.sh` を引数なしで実行してください。


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

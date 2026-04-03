# GC CI Gate Specification

> GC-06 の CI ゲート仕様。Phase 11 runtime stability gate (CP-05) の一部。  
> 正本の完了条件は `docs/development/planning/runtime-stability-spec.md` S14-S16 を参照し、本書はそれを CI artifact / required job に落とす運用規則を固定する。

## 目的と適用範囲

- 現在の required job は `.github/workflows/ci.yml` の `gc-metrics-artifact` である。
- 現在の artifact 生成は `scripts/ci/collect-gc-metrics.sh` が担い、`test_e2e_alloc_metrics_ci_artifact_payload` の成功と JSON 形状検証までを blocking にしている。
- 現状の payload は `allocator_mode = bump` 前提の proxy metrics だが、既存 GC-05 representative workload（light compile+run / REPL 50 eval / stateful REPL session / actual `lsp --stdio` repeated sequence）の実行結果を `proxy_workloads` として自己記述する。
- ただし collector 有効 GC の S14-S16 を直接判定する artifact ではない。
- したがって本書では **現在の blocking 条件** と **S14-S16 を閉じるための machine-readable 判定規則** を分けて定義する。

## 収集メトリクス

| # | メトリクス | 説明 | WARN 閾値 | FAIL 閾値 |
|---|-----------|------|-----------|-----------|
| 1 | Peak RSS | プロセスの最大常駐メモリ | ベースライン +20% | ベースライン +50% |
| 2 | Heap allocated bytes | `__alloc` による累計確保バイト数 | — | S14 用の collector 有効 series が最終 10% で最大値更新を継続 |
| 3 | Live object count | 未解放オブジェクト数 (bump allocator では total と同一) | — | — |
| 4 | GC pause time | GC 停止時間 (collector 導入後) | +30% | +100% |
| 5 | Full GC count | フル GC 実行回数 (collector 導入後) | — | — |

## テストレベル

| レベル | イテレーション数 | 実行タイミング | テスト名 |
|--------|-----------------|---------------|---------|
| Light | 48 回 compile+run | 全 PR | `test_e2e_gc_light_compile_run_loop` |
| Light soak | 50 回 eval (alloc 付き) | 全 PR | `test_e2e_gc_repl_soak_50_eval` |
| Medium soak | 500 回 eval (alloc 付き) | Nightly | `test_e2e_gc_repl_soak_500_eval` (`#[ignore]`) |
| Full soak | 1,000 回 compile+run | Release gate | `test_e2e_gc_compile_run_loop_1000` (`#[ignore]`) |

## CI 統合

### 全 PR (現在の required set)

```yaml
- cargo test test_e2e_gc_light_compile_run_loop
- cargo test test_e2e_gc_repl_soak_50_eval
- cargo test test_e2e_alloc_metrics
- bash scripts/ci/collect-gc-metrics.sh
```

### Nightly (拡張)

```yaml
- cargo test test_e2e_gc -- --include-ignored
```

### Release gate (全量)

```yaml
- cargo test -- --include-ignored
```

## 現在の artifact 仕様

### 生成元

- script: `scripts/ci/collect-gc-metrics.sh`
- targeted test: `test_e2e_alloc_metrics_ci_artifact_payload`
- artifact path: `ci-artifacts/gc-metrics/{commit_sha}/summary.json`
- proof sidecar path: `ci-artifacts/gc-metrics/{commit_sha}/collector-proof.json`
- artifact name: `gc-metrics-{commit_sha}`
- required job: `.github/workflows/ci.yml` の `gc-metrics-artifact`
- test-only validation path: `LSHARP_GC_METRICS_INPUT=/path/to/summary.json bash scripts/ci/collect-gc-metrics.sh`
- optional proof overlay path: `LSHARP_GC_PROOF_BUNDLE_INPUT=/path/to/collector-proof.json bash scripts/ci/collect-gc-metrics.sh`
- default proof overlay path: sibling `collector-proof.json` が存在すれば env 指定なしでも自動 merge
- normalized sidecar output: validator 通過後は sibling `collector-proof.json` を常に書き戻す

### payload schema

現行の `summary.json` は次の 18 キーを必須とする。
`gate_status` / `s14_status` / `s15_status` / `s16_status` は artifact が自己記述する gate 状態であり、
仕様の 4 値 (`pass` / `fail` / `blocked` / `n/a`) を直接保持する。

```json
{
  "allocator_mode": "bump",
  "ci_level": "simple",
  "gate_status": "accepted",
  "s14_status": "n/a",
  "s15_status": "n/a",
  "s16_status": "n/a",
  "s15_proof": null,
  "s16_proof": null,
  "heap_bytes_series": [],
    "proxy_workloads": {
      "compile_run_light_loop": { "status": "pass", "iterations": 48, "last_stdout": "1" },
      "repl_soak_50_eval": { "status": "pass", "iterations": 50, "eval_count": 50 },
      "repl_stateful_long_session": {
        "status": "pass",
        "iterations": 200,
        "eval_count": 200,
        "total_input_bytes": 4500,
        "last_type_tag": 100
      },
      "repl_stateful_single_session": {
        "status": "pass",
        "iterations": 50,
       "eval_count": 50,
       "total_input_bytes": 1125,
       "last_type_tag": 100
     },
     "lsp_actual_stdio_repeated_sequence": {
       "status": "pass",
       "iterations": 12,
       "response_frames": 61
     }
   },
   "peak_alloc_bytes": 0,
   "total_alloc_count": 0,
   "live_alloc_count": 0,
  "max_single_alloc": 0,
  "alloc_span": 0,
  "leak_growing_count": 0,
  "leak_total": 0,
  "leak_suspect": 0
}
```

`gate_status` は artifact が構造・proxy metrics の採取に成功した場合は `"accepted"` となる。
`s14_status` / `s15_status` / `s16_status` は bump allocator では常に `"n/a"` であり、
collector 有効になって初めて `"pass"` / `"fail"` / `"blocked"` に変わる。
現在の bump artifact でも schema は固定し、`heap_bytes_series` には空配列を入れておく。
`s15_proof` / `s16_proof` は S15 / S16 の machine-readable 証拠 slot であり、`blocked` / `n/a` では `null`、`pass` / `fail` では object を要求する。
`proxy_workloads` には既存 GC-05 representative workload の結果を格納し、各 entry の `status = "pass"` を required とする。required entry は light compile+run / REPL 50 eval / stateful long-session REPL / stateful single-session REPL / actual `lsp --stdio` repeated sequence の 5 つで固定する。
`LSHARP_GC_METRICS_INPUT` を指定した validate-only 実行では、既存 `summary.json` を再利用して Python validator だけを走らせられる。
`LSHARP_GC_PROOF_BUNDLE_INPUT` を指定した場合、または sibling `collector-proof.json` が存在する場合、
script はその `s15_status` / `s15_proof` / `s16_status` / `s16_proof` を `summary.json`
へ merge してから同じ validator を走らせる。受理した場合は merge 後 payload を
`summary.json` へ正規化して書き戻し、さらに sibling `collector-proof.json` も
現在の `s15_status` / `s15_proof` / `s16_status` / `s16_proof` を持つ normalized sidecar
として常に出力する。proof bundle 未指定の bump / blocked path でも sidecar は生成される。

proof bundle は次の 4 キーだけを許可する（部分指定可）。

```json
{
  "s15_status": "pass",
  "s15_proof": { "...": "..." },
  "s16_status": "blocked",
  "s16_proof": null
}
```

### artifact rejection criteria

`gc-metrics-artifact` job は、次のいずれかで **reject / fail** とみなす。

| ID | 条件 | 現在の実装根拠 | CI への影響 |
|---|---|---|---|
| AR-01 | `test_e2e_alloc_metrics_ci_artifact_payload` が失敗する / `gate_status != "accepted"` / `sXX_status = "fail"` | `scripts/ci/collect-gc-metrics.sh` の `cargo test` と Python 検証 | required job fail |
| AR-02 | `ci-artifacts/gc-metrics/{commit_sha}/summary.json` または明示指定した proof bundle を読めない | 同 script の Python 検証がファイルを開く | required job fail |
| AR-03 | artifact JSON または proof bundle JSON が parse できない | Python `json.loads(...)` | required job fail |
| AR-04 | 必須 18 キーのいずれかが欠落、`proxy_workloads` / その required entry が欠落、`sXX_status` が 4 値外、`s14_status` が evaluator と不一致、proof bundle に未知キーがある、または `s15_proof` / `s16_proof` が status と整合しない | Python の key/value 検証 | required job fail |

### artifact acceptance の意味

- 受理 (`accepted`) は **artifact の構造と proxy metrics / representative workload の採取に成功した** ことだけを意味する。
- 受理は S14-S16 の達成を意味しない。
- 現在の `gc-metrics-artifact` required check は **artifact rejection を blocking にする第 1 段** であり、collector 有効 gate の代替ではない。

## S14-S16 判定状態

machine-readable に扱うため、S14-S16 の状態は次の 4 値で固定する。

| 状態 | 意味 |
|---|---|
| `pass` | 判定に必要な証跡が揃い、規則を満たした |
| `fail` | 判定に必要な証跡が揃い、規則に違反した |
| `blocked` | 判定対象の gate が required だが、collector 有効証跡が未配線で閉じられない |
| `n/a` | 現在の artifact が bump allocator proxy metrics であり、その gate を直接判定できない |

現在の `summary.json` だけで読める状態は次の通り。

| Gate | bump artifact 単独の状態 | 理由 |
|---|---|---|
| S14 | `n/a` | bump payload では `heap_bytes_series = []` で、collector 有効証跡がない |
| S15 | `n/a` | bootstrap fixed-point の比較対象が artifact にない |
| S16 | `n/a` | GC 起因 crash / dangling pointer / workload 完走の collector 有効証跡がない |

`CP-05` / `GC-06` の完了判定では、S14-S16 が `pass` になるまで **論理上は `blocked`** とみなす。  
つまり、現在の PR CI は green でも runtime stability gate は未完了のままでよい。
その一方で `proxy_workloads` により、light compile+run / REPL / stateful long-session REPL / actual `lsp --stdio` repeated sequence が artifact 上で機械可読に追跡できる。

## monotonic trend evaluation rules (S14)

S14 は `docs/development/planning/runtime-stability-spec.md` の
「最終 10% 区間の heap bytes が最大値を更新し続けていないこと」
を次の規則で機械判定する。

### 必要入力

- 単一プロセス長寿命ワークロードの `heap_bytes_series`
- `allocator_mode != bump`
- サンプル数 `n >= 2`

### 判定手順

1. `tail_start = floor(n * 0.9)` とする。
2. `tail = heap_bytes_series[tail_start..n)` とする。
3. 各 tail 要素 `tail[i]` が、その時点までの全履歴 `heap_bytes_series[0..tail_start+i)` の最大値を **毎回** 更新しているかを調べる。
4. tail の全サンプルが最大値更新を継続した場合は `fail`、1 点でも更新が止まれば `pass` とする。

擬似コード:

```text
running_max = max(heap_bytes_series[0..tail_start])
status = fail
for sample in tail:
  if sample > running_max:
    running_max = sample
    continue
  status = pass
  break
```

### 補足

- 一時的な増加は許容する。失敗条件は「最終 10% 区間の全点が新しい最大値を作り続ける」場合だけである。
- 現行の `leak_growing_count` / `leak_total` / `leak_suspect` は bump allocator 上の proxy 指標であり、この S14 判定を代替しない。
- `allocator_mode = bump` の artifact は S14 を `n/a` とする。

## fixed-point gate definition (S15)

S15 は collector 有効時の selfhost bootstrap が fixed-point を壊さないことを指す。  
CI では次の比較単位を **すべて一致** と定義したときだけ `pass` にする。

| 比較対象 | pass 条件 |
|---|---|
| `stageN.wasm` と `stageN+1.wasm` | raw bytes が bit-identical |
| exported symbol list | 完全一致 |
| data section bytes | 完全一致 |
| diagnostics | 完全一致 |

artifact ではこれを `s15_proof` object に落とし込み、最低限次のキーを要求する。

```json
{
  "gc_mode": "mark-sweep",
  "stage_pair": ["stage1", "stage2"],
  "bytes_identical": true,
  "exports_identical": true,
  "data_sections_identical": true,
  "diagnostics_identical": true
}
```

- `gc_mode` は collector 有効 proof を表すため、`mark-sweep` または `generational` のみ許可する。`none` は無効。
- `stage_pair` は 2 要素の stage 名配列を維持し、比較結果 4 項目はすべて boolean で保持する。
- proof bundle merge を使う場合も、`s15_status` / `s15_proof` はこの schema を崩してはならない。

### blocking 条件

- GC 有効 bootstrap の stage 比較 artifact が存在しない間は S15 は `blocked` のまま。
- 比較 artifact が存在して上表のいずれかが不一致なら `fail`。
- GC 無効 (`--gc=none`) と GC 有効の両方で fixed-point が一致したときのみ `pass`。
- `s15_status = blocked` / `n/a` の間は `s15_proof = null` を維持し、未証明を machine-readable に固定する。

現時点では `gc-metrics-artifact` job 自体はこの比較をまだ実行しないため、PR CI の blocking 条件には未接続である。

## crash-free gate definition (S16)

S16 は collector 有効ワークロードが GC 由来 crash なしで完走することを指す。  
次の 3 条件をすべて満たした場合のみ `pass` とする。

1. collector 起動時の `SIGSEGV` / trap / `unreachable` が 0 件
2. P11-5b の全ワークロードを GC 有効で完走
3. root 漏れに起因する dangling pointer 検出が 0 件

artifact ではこれを `s16_proof` object に落とし込み、最低限次のキーを要求する。

```json
{
  "gc_mode": "mark-sweep",
  "completed_workloads": [
    "compile_run_light_loop",
    "repl_soak_50_eval",
    "repl_stateful_long_session",
    "repl_stateful_single_session",
    "lsp_actual_stdio_repeated_sequence"
  ],
  "all_workloads_completed": true,
  "sigsegv_count": 0,
  "trap_count": 0,
  "unreachable_count": 0,
  "dangling_pointer_count": 0
}
```

- `gc_mode` は collector 有効 proof を表すため、`mark-sweep` または `generational` のみ許可する。`none` は無効。
- `completed_workloads` は文字列配列とし、重複や未知の workload 名を許可しない。
- `s16_status = pass` の場合、`completed_workloads` は required workload set
  (`compile_run_light_loop` / `repl_soak_50_eval` / `repl_stateful_long_session` /
  `repl_stateful_single_session` / `lsp_actual_stdio_repeated_sequence`)
  と完全一致していなければならない。
- proof bundle merge を使う場合も、`s16_status` / `s16_proof` はこの schema を崩してはならない。

いずれか 1 つでも違反した場合は `fail`、collector 有効ジョブ自体が未配線なら `blocked`。
`s16_status = blocked` / `n/a` の間は `s16_proof = null` を維持する。

## いつ CI が block するか

### 現在

- GitHub Branch Protection 上の required check として CI を block するのは `gc-metrics-artifact` job のみ。
- したがって **現在 block される条件** は AR-01〜AR-04 の artifact rejection に限る。

### CP-05 / GC-06 の完了判定

- runtime stability gate としては、S14-S16 のいずれかが `blocked` または `fail` なら閉じない。
- これは「PR CI が赤になる」という意味ではなく、「TODO / phase plan 上で GC-06 を完了扱いにしない」という意味で使う。

## 5 メトリクス収集テスト

`test_e2e_alloc_metrics_five_metric_collection` で以下を検証する。

1. **peak_alloc_bytes**: 最初と最後の alloc アドレス差 ≥ 0
2. **total_alloc_count**: 実行した alloc 回数と一致
3. **live_alloc_count**: bump allocator では total と同一
4. **max_single_alloc**: 最大要求サイズと一致
5. **alloc_span**: 最初と最後のアドレス距離 ≥ 合計確保サイズ

## 今後のロードマップ

1. **Phase 0 (現在)**: bump allocator + アドレス追跡メトリクス + artifact rejection の blocking 化
2. **Phase 1**: collector 導入 → `heap_bytes_series` / GC pause / crash-free 証跡を artifact 化
3. **Phase 2**: selfhost bootstrap fixed-point artifact を GC 有効/無効両方で比較
4. **Phase 3**: S14-S16 を required checks へ昇格し、runtime stability gate と Branch Protection を一致させる

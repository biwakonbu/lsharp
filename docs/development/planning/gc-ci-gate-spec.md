# GC CI Gate Specification

> GC-06 の CI ゲート仕様。Phase 11 runtime stability gate (CP-05) の一部。

## 収集メトリクス

| # | メトリクス | 説明 | WARN 閾値 | FAIL 閾値 |
|---|-----------|------|-----------|-----------|
| 1 | Peak RSS | プロセスの最大常駐メモリ | ベースライン +20% | ベースライン +50% |
| 2 | Heap allocated bytes | `__alloc` による累計確保バイト数 | — | 単調増加が停止しない場合 |
| 3 | Live object count | 未解放オブジェクト数 (bump allocator では total と同一) | — | — |
| 4 | GC pause time | GC 停止時間 (将来の mark-sweep 導入後) | +30% | +100% |
| 5 | Full GC count | フル GC 実行回数 (将来の世代別 GC 導入後) | — | — |

## テストレベル

| レベル | イテレーション数 | 実行タイミング | テスト名 |
|--------|-----------------|---------------|---------|
| Light | 48 回 compile+run | 全 PR | `test_e2e_gc_light_compile_run_loop` |
| Light soak | 50 回 eval (alloc 付き) | 全 PR | `test_e2e_gc_repl_soak_50_eval` |
| Medium soak | 500 回 eval (alloc 付き) | Nightly | `test_e2e_gc_repl_soak_500_eval` (`#[ignore]`) |
| Full soak | 1,000 回 compile+run | Release gate | `test_e2e_gc_compile_run_loop_1000` (`#[ignore]`) |

## CI 統合

### 全 PR (必須)
```yaml
- cargo test test_e2e_gc_light_compile_run_loop
- cargo test test_e2e_gc_repl_soak_50_eval
- cargo test test_e2e_alloc_metrics
```

### Nightly (拡張)
```yaml
- cargo test test_e2e_gc -- --include-ignored
```

### Release gate (全量)
```yaml
- cargo test -- --include-ignored
```

## 閾値定義

### Peak RSS
- **WARN**: ベースラインから +20% 以上の増加
- **FAIL**: ベースラインから +50% 以上の増加
- ベースラインは `test_e2e_gc_light_compile_run_loop` の 48 回実行時の RSS

### Compile latency
- **WARN**: ベースラインから +30% 以上の増加
- **FAIL**: ベースラインから +100% 以上の増加

### Leak detection
- bump allocator 環境ではアドレス単調増加が正常動作
- 将来の GC 導入後、`leak_suspect = 0` (アドレス再利用) を期待

## 5 メトリクス収集テスト

`test_e2e_alloc_metrics_five_metric_collection` で以下を検証:

1. **peak_alloc_bytes**: 最初と最後の alloc アドレス差 ≥ 0
2. **total_alloc_count**: 実行した alloc 回数と一致
3. **live_alloc_count**: bump allocator では total と同一
4. **max_single_alloc**: 最大要求サイズと一致
5. **alloc_span**: 最初と最後のアドレス距離 ≥ 合計確保サイズ

## 今後のロードマップ

1. **Phase 0 (現在)**: bump allocator + アドレス追跡メトリクス
2. **Phase 1**: mark-sweep GC 導入 → GC pause time 計測開始
3. **Phase 2**: 世代別 GC → Full GC count 計測開始
4. **Phase 3**: RSS ベースラインの自動更新 + regression alert

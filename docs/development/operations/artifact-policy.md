# アーティファクトポリシー

CI / CD パイプラインで生成されるアーティファクトの命名規則と保持期間を定義する。

## 命名規則

| アーティファクト | 命名パターン | 説明 |
|---|---|---|
| CI Gate v2 結果 | `ci-gate-v2-results` | gate-v2 の全ジョブ結果サマリー |
| GC metrics | `gc-metrics-{commit_sha}` | runtime stability 用の GC/alloc metrics JSON |
| Bootstrap stages | `bootstrap-stages-{sha}` | selfhost ブートストラップの中間成果物 |
| Bootstrap diff | `bootstrap-diff-{sha}` | stage 間の差分レポート |
| Native binaries | `native-binaries-{os}-{arch}` | ネイティブビルド成果物 |
| Differential report | `differential-report-{sha}` | shadow-oracle 差分テストレポート |
| Release artifacts | `release-{version}-{os}-{arch}` | リリース配布物 |
| Benchmark results | `benchmark-{date}` | パフォーマンスベンチマーク結果 |

### 命名規則の詳細

- `{commit_sha}`: GitHub Actions の `github.sha` が指すフルコミット SHA
- `{sha}`: Git コミット SHA。短縮形を使う場合は各 workflow 側で明示する
- `{os}`: `linux` / `macos` / `windows`
- `{arch}`: `x86_64` / `aarch64`
- `{version}`: セマンティックバージョン（例: `0.2.0`）
- `{date}`: ISO 8601 日付（例: `2026-01-15`）

## 保持期間

| Git ref | 保持期間 | 根拠 |
|---|---|---|
| PR | 5 日 | マージ後は不要。レビュー期間をカバー |
| main ブランチ | 30 日 | 回帰調査に十分な期間 |
| リリースタグ | 永続 | ユーザー配布物として永続保持 |

### CI での設定

```yaml
# ci.yml での retention-days 設定例
- uses: actions/upload-artifact@v4
  with:
    name: ci-gate-v2-results
    retention-days: 30        # main ブランチ
    if-no-files-found: ignore
```

PR の場合は GitHub Actions のデフォルト保持期間（90 日）を上書きし、5 日に短縮する。

## チェックサム

リリースアーティファクトには SHA-256 チェックサムを付与する。

```bash
# scripts/checksum.sh でチェックサム生成
sha256sum release-*.tar.gz > SHA256SUMS
```

チェックサムファイルは対応するリリースアーティファクトと同じ保持期間とする。

## ストレージ管理

- GitHub Actions のアーティファクトストレージ上限に注意する
- 不要な中間成果物は `if-no-files-found: ignore` で欠落を許容する
- 大容量アーティファクト（Wasm バイナリ等）は圧縮して保存する
- `gc-metrics-{commit_sha}` は `ci-artifacts/gc-metrics/{commit_sha}/summary.json` を正本とし、PR では 5 日、main では 30 日保持する

## GC metrics artifact の受理 / 却下

`gc-metrics-artifact` は required job であり、他の「欠落を許容する中間成果物」と扱いを分ける。

### 正本

- workflow: `.github/workflows/ci.yml`
- collector script: `scripts/ci/collect-gc-metrics.sh`
- 正本パス: `ci-artifacts/gc-metrics/{commit_sha}/summary.json`
- upload 名: `gc-metrics-{commit_sha}`

### 却下条件

次のいずれかに当てはまる場合、artifact は運用上 **却下** とみなす。

1. `test_e2e_alloc_metrics_ci_artifact_payload` が失敗
2. `summary.json` が存在しない、または読めない
3. JSON parse に失敗
4. `allocator_mode`, `ci_level`, `gate_status`, `s14_status`, `s15_status`, `s16_status`, `heap_bytes_series`, `proxy_workloads`, `peak_alloc_bytes`, `total_alloc_count`, `live_alloc_count`, `max_single_alloc`, `alloc_span`, `leak_growing_count`, `leak_total`, `leak_suspect` のいずれかが欠落
5. `proxy_workloads.compile_run_light_loop`, `proxy_workloads.repl_soak_50_eval`, `proxy_workloads.repl_stateful_long_session`, `proxy_workloads.repl_stateful_single_session`, `proxy_workloads.lsp_actual_stdio_repeated_sequence` のいずれかが欠落、または `status != "pass"`

### 受理の意味

- 受理は「GC-06 の第 1 段 artifact が構造的に有効で、既存 GC-05 representative workload の proxy 証跡も回収できた」という意味であり、S14-S16 の full gate 達成を意味しない。
- bump allocator の proxy metrics は collector 有効 GC の単調増加判定 / fixed-point / crash-free を直接閉じない。
- そのため `gc-metrics-artifact` が green でも、`docs/development/planning/runtime-stability-spec.md` S14-S16 は別途 `blocked` のまま残りうる。

## 証跡

- `.github/workflows/ci.yml` (`retention-days` 設定)
- `scripts/ci/collect-gc-metrics.sh`
- `docs/development/planning/gc-ci-gate-spec.md`
- `scripts/checksum.sh`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops02_artifact_policy`)

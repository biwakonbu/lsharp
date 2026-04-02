# アーティファクトポリシー

CI / CD パイプラインで生成される **workflow-local artifact** と、タグ付き配布で公開する
**GitHub Release asset** の命名規則 / 保持期間を定義する。
この文書は **現在 workflow が実際に emit している名前だけ** を正本として扱う。
将来用の placeholder 名は、対応する workflow とテストが入るまで active contract にしない。

## 命名規則

### workflow-local artifact（`actions/upload-artifact`）

| アーティファクト | 命名パターン | ソース | 説明 |
|---|---|---|---|
| CI Gate v2 結果 | `ci-gate-v2-results` | `.github/workflows/ci.yml` | gate-v2 の全ジョブ結果サマリー |
| Bootstrap diff | `bootstrap-diff-{commit_sha}` | `.github/workflows/ci.yml` `bootstrap` | fixed-point / stage-chain 比較レポート |
| Fresh clone archive | `fresh-clone-archive-{commit_sha}` | `.github/workflows/ci.yml` `fresh-clone-artifact` | binary-only gate 用の release-style archive |
| GC metrics | `gc-metrics-{commit_sha}` | `.github/workflows/ci.yml` `gc-metrics-artifact` | runtime stability 用の GC/alloc metrics JSON |
| Shadow oracle 結果 | `shadow-oracle-results` | `.github/workflows/ci.yml` `shadow-oracle` | differential test の補助成果物 |
| Release build artifact | `lsharp-{version}-{target}` | `.github/workflows/release.yml` `build` | release workflow 内で download される論理名 |

### GitHub Release asset

| アセット | 命名パターン | ソース | 説明 |
|---|---|---|---|
| Release archive | `lsharp-{version}-{target}.{ext}` | `.github/workflows/release.yml` `release` | tag に添付されるユーザー向け配布ファイル |
| Guest component sidecar | `lsharp-{version}-{target}.component.wasm` | `.github/workflows/release.yml` `release` | host launcher archive と同じ tag に添付される検証/rollback 用 guest component |

### 命名規則の詳細

- `{commit_sha}`: GitHub Actions の `github.sha` が指すフルコミット SHA
- `{version}`: Git tag / release version（例: `v0.2.0`）
- `{target}`: Rust target triple（例: `x86_64-unknown-linux-gnu`）
- `{ext}`: archive 拡張子（`tar.gz` / `zip`）

`lsharp-{version}-{target}` は **workflow-local artifact name** であり、`actions/download-artifact`
が参照する論理名である。実際にユーザーが取得する **GitHub Release asset** は
`lsharp-{version}-{target}.{ext}` と `lsharp-{version}-{target}.component.wasm` であり、拡張子付きのファイル名を正本とする。

将来 `bootstrap-stages-*` / `native-binaries-*` / `benchmark-*` のような新しい artifact 名を
追加する場合も、この文書へ先に placeholder を書くのではなく、workflow とテストを追加した時点で
active contract として追記する。

## 保持期間

| アーティファクト | PR | `main` | tag / release | 根拠 |
|---|---:|---:|---:|---|
| `bootstrap-diff-{commit_sha}` | 5 日 | 30 日 | - | fixed-point 調査用 |
| `fresh-clone-archive-{commit_sha}` | 5 日 | 30 日 | - | binary-only gate の再検証用 |
| `gc-metrics-{commit_sha}` | 5 日 | 30 日 | - | runtime stability 調査用 |
| `ci-gate-v2-results` | 30 日 | 30 日 | - | 集約結果の調査用 |
| `shadow-oracle-results` | 14 日 | 14 日 | - | non-blocking differential 補助証跡 |
| `lsharp-{version}-{target}` | - | - | 30 日 | release workflow 中の workflow-local artifact |
| `lsharp-{version}-{target}.{ext}` | - | - | 永続 | GitHub Release asset として公開 |
| `lsharp-{version}-{target}.component.wasm` | - | - | 永続 | GitHub Release asset として公開 |

### CI での設定

```yaml
# ci.yml: PR は 5 日、main は 30 日
- uses: actions/upload-artifact@v4
  with:
    name: bootstrap-diff-${{ github.sha }}
    retention-days: ${{ github.event_name == 'pull_request' && 5 || 30 }}
```

```yaml
# release.yml: workflow-local artifact は 30 日保持し、別途 GitHub Release へ添付する
- uses: actions/upload-artifact@v4
  with:
    name: lsharp-${{ github.ref_name }}-${{ matrix.target }}
    retention-days: 30
```

PR の場合は GitHub Actions のデフォルト保持期間（90 日）を上書きし、5 日に短縮する。
GitHub Release asset の「永続」は `retention-days` ではなく、GitHub Release 上の配布物として
残す運用を指す。

## チェックサム

GitHub Release asset には top-level `dist/checksums.txt` を付与する。

```bash
# scripts/checksum.sh で release-level checksum asset を生成
bash scripts/checksum.sh dist > dist/checksums.txt
```

`dist/checksums.txt` は同じ release で公開する archive 群に対応する release-level checksum asset として扱う。

## ストレージ管理

- GitHub Actions のアーティファクトストレージ上限に注意する
- 不要な中間成果物は `if-no-files-found: ignore` で欠落を許容する
- 大容量アーティファクト（Wasm バイナリ等）は圧縮して保存する
- `gc-metrics-{commit_sha}` は `ci-artifacts/gc-metrics/{commit_sha}/summary.json` を正本とし、PR では 5 日、main では 30 日保持する
- release workflow の `lsharp-{version}-{target}` は **workflow-local artifact** であり、ユーザー向け名称は GitHub Release asset `lsharp-{version}-{target}.{ext}` と `lsharp-{version}-{target}.component.wasm` として別に扱う

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
4. `allocator_mode`, `ci_level`, `gate_status`, `s14_status`, `s15_status`, `s16_status`, `s15_proof`, `s16_proof`, `heap_bytes_series`, `proxy_workloads`, `peak_alloc_bytes`, `total_alloc_count`, `live_alloc_count`, `max_single_alloc`, `alloc_span`, `leak_growing_count`, `leak_total`, `leak_suspect` のいずれかが欠落
5. `proxy_workloads.compile_run_light_loop`, `proxy_workloads.repl_soak_50_eval`, `proxy_workloads.repl_stateful_long_session`, `proxy_workloads.repl_stateful_single_session`, `proxy_workloads.lsp_actual_stdio_repeated_sequence` のいずれかが欠落、または `status != "pass"`

### 受理の意味

- 受理は「GC-06 の第 1 段 artifact が構造的に有効で、既存 GC-05 representative workload の proxy 証跡も回収できた」という意味であり、S14-S16 の full gate 達成を意味しない。
- bump allocator の proxy metrics は collector 有効 GC の単調増加判定 / fixed-point / crash-free を直接閉じない。
- そのため `gc-metrics-artifact` が green でも、`docs/development/planning/runtime-stability-spec.md` S14-S16 は別途 `blocked` のまま残りうる。

## 証跡

- `.github/workflows/ci.yml` (`retention-days` 設定)
- `.github/workflows/release.yml`
- `scripts/ci/collect-gc-metrics.sh`
- `docs/development/planning/gc-ci-gate-spec.md`
- `scripts/checksum.sh`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops02_artifact_policy`)

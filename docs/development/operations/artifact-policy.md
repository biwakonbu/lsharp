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
| GC metrics | `gc-metrics-{commit_sha}` | `.github/workflows/ci.yml` `gc-metrics-artifact` | runtime stability 用の GC/alloc metrics directory (`summary.json` + `collector-proof.json`) |
| Native proxy artifact | `native-proxy-{commit_sha}` | `.github/workflows/ci.yml` `native-proxy-artifact` | Darwin arm64 host で materialize した `stage1-native` / `stage2-native` / `stage3-native` proxy bundle (`manifest.json` + canonical bundle files) と `actual-stage23-gap.json` |
| Native Linux x86 smoke | `native-linux-x86-{commit_sha}` | `.github/workflows/ci.yml` `native-linux-x86-smoke` | `x86_64-unknown-linux-gnu` の target descriptor / ELF emitter / x86_64 codegen smoke summary |
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
| `native-proxy-{commit_sha}` | 5 日 | 30 日 | - | Darwin arm64 native proxy artifact の比較 / 調査用 |
| `native-linux-x86-{commit_sha}` | 5 日 | 30 日 | - | Linux x86_64 native server target smoke の比較 / 調査用 |
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
- `gc-metrics-{commit_sha}` は `ci-artifacts/gc-metrics/{commit_sha}/` directory を正本とし、`collect-gc-metrics.sh` が `summary.json` と sibling `collector-proof.json` を常に揃えた上で PR では 5 日、main では 30 日保持する
- `native-proxy-{commit_sha}` は `ci-artifacts/native-proxy/{commit_sha}/` directory を正本とし、`scripts/ci/build-native.sh` が `stage1-native` / `stage2-native` / `stage3-native` canonical bundle、top-level `manifest.json`、および representative build entry の actual blocker report `actual-stage23-gap.json` を揃えた上で PR では 5 日、main では 30 日保持する
- `native-linux-x86-{commit_sha}` は `ci-artifacts/native-linux-x86/{commit_sha}/` directory を正本とし、`scripts/ci/native-linux-x86-smoke.sh` が Ubuntu x86_64 上で `x86_64-unknown-linux-gnu` の target descriptor / ELF emitter / x86_64 codegen smoke を実行し、`summary.json` を揃えた上で PR では 5 日、main では 30 日保持する
- ローカル VM 診断の `ci-artifacts/native-linux-x86-hostgen-vm/` は release artifact ではない。`scripts/ci/prune-native-linux-x86-hostgen-artifacts.sh` が current / stage1 reuse / stage2 reuse を保護し、既定で最新 8 世代だけを保持する。保持数は `LSHARP_NATIVE_LINUX_X86_ARTIFACT_RETENTION_COUNT`、削除候補の確認は `LSHARP_NATIVE_LINUX_X86_ARTIFACT_PRUNE_DRY_RUN=1` で上書きできる
- release workflow の `lsharp-{version}-{target}` は **workflow-local artifact** であり、ユーザー向け名称は GitHub Release asset `lsharp-{version}-{target}.{ext}` と `lsharp-{version}-{target}.component.wasm` として別に扱う

## GC metrics artifact の受理 / 却下

`gc-metrics-artifact` は required job であり、他の「欠落を許容する中間成果物」と扱いを分ける。

### 正本

- workflow: `.github/workflows/ci.yml`
- collector script: `scripts/ci/collect-gc-metrics.sh`
- 正本パス: `ci-artifacts/gc-metrics/{commit_sha}/summary.json`
- proof sidecar: `ci-artifacts/gc-metrics/{commit_sha}/collector-proof.json`
- upload 名: `gc-metrics-{commit_sha}`

### 却下条件

次のいずれかに当てはまる場合、artifact は運用上 **却下** とみなす。

1. `test_e2e_alloc_metrics_ci_artifact_payload` が失敗
2. `summary.json` が存在しない、または読めない
3. JSON parse に失敗
4. `allocator_mode`, `ci_level`, `gate_status`, `s14_status`, `s14_reason`, `s15_status`, `s16_status`, `s15_reason`, `s16_reason`, `s15_proof`, `s16_proof`, `heap_bytes_series`, `proxy_workloads`, `peak_alloc_bytes`, `total_alloc_count`, `live_alloc_count`, `max_single_alloc`, `alloc_span`, `leak_growing_count`, `leak_total`, `leak_suspect` のいずれかが欠落
5. `proxy_workloads.compile_run_light_loop`, `proxy_workloads.repl_soak_50_eval`, `proxy_workloads.repl_stateful_long_session`, `proxy_workloads.repl_stateful_single_session`, `proxy_workloads.lsp_actual_stdio_repeated_sequence` のいずれかが欠落、または `status != "pass"`

### 受理の意味

- current required path では、受理は「collector-backed `summary.json` / `collector-proof.json` が構造的に有効で、representative workload と S14/S15/S16 の machine-readable evidence が揃った」ことを意味する。
- `summary.json` は `s14_reason` / `s15_reason` / `s16_reason` を持ち、`blocked` / `n/a` の理由を machine-readable に保持する。
- `collect-gc-metrics.sh` は sibling `collector-proof.json` が存在する場合はそれを `summary.json` へ merge して同一 validator に通し、受理後は current `s15_*` / `s16_*` slot と `s15_reason` / `s16_reason` を持つ normalized sidecar として `collector-proof.json` を常に書き戻す。
- proof bundle 未指定でも `collector-proof.json` は emit され、fixture の bump / blocked path では `summary.json` 側の `s15_*` / `s16_*` slot と machine-readable reason をそのまま mirror する。
- validate-only fixture では bump allocator / `blocked` / `n/a` payload を引き続き扱うが、required PR artifact は collector-backed path を正本とする。
- そのため current `gc-metrics-artifact` が green で `s14_status = s15_status = s16_status = pass` を保持していれば、`docs/development/planning/runtime-stability-spec.md` S14-S16 の closure evidence として扱う。

## 証跡

- `.github/workflows/ci.yml` (`retention-days` 設定)
- `.github/workflows/release.yml`
- `scripts/ci/collect-gc-metrics.sh`
- `docs/development/planning/gc-ci-gate-spec.md`
- `scripts/checksum.sh`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops02_artifact_policy`)

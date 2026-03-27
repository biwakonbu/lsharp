# アーティファクトポリシー

CI / CD パイプラインで生成されるアーティファクトの命名規則と保持期間を定義する。

## 命名規則

| アーティファクト | 命名パターン | 説明 |
|---|---|---|
| CI Gate v2 結果 | `ci-gate-v2-results` | gate-v2 の全ジョブ結果サマリー |
| GC metrics | `gc-metrics-{sha}` | runtime stability 用の GC/alloc metrics JSON |
| Bootstrap stages | `bootstrap-stages-{sha}` | selfhost ブートストラップの中間成果物 |
| Bootstrap diff | `bootstrap-diff-{sha}` | stage 間の差分レポート |
| Native binaries | `native-binaries-{os}-{arch}` | ネイティブビルド成果物 |
| Differential report | `differential-report-{sha}` | shadow-oracle 差分テストレポート |
| Release artifacts | `release-{version}-{os}-{arch}` | リリース配布物 |
| Benchmark results | `benchmark-{date}` | パフォーマンスベンチマーク結果 |

### 命名規則の詳細

- `{sha}`: Git コミットの短縮 SHA（7 文字）
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
- `gc-metrics-{sha}` は `ci-artifacts/gc-metrics/{commit_sha}/summary.json` を正本とし、PR では 5 日、main では 30 日保持する

## 証跡

- `.github/workflows/ci.yml` (`retention-days` 設定)
- `scripts/ci/collect-gc-metrics.sh`
- `scripts/checksum.sh`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops02_artifact_policy`)

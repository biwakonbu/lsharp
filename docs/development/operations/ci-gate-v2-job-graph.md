# CI Gate v2 ジョブグラフ

CI パイプラインのジョブ依存関係と保護ルールの正本。release / distribution / signing / tier1/tier2 は [`release-distribution-signing.md`](./release-distribution-signing.md) に分離し、この文書では blocking CI graph に集中する。

## ジョブ依存関係

```
test ─────────────────┐
lint ─────────────────┤
format ───────────────┤
bootstrap ────────────┤
audit-docs ───────────┤
default-path-smoke ───┤
test-fresh-clone ─────┤
fresh-clone-smoke ────┤──→ ci-gate ──→ ci-gate-v2
gc-metrics-artifact ──┤
native-proxy-artifact ┤
editor-extension-build ┤
shadow-oracle ─────────┘────────────→ ci-gate-v2 (non-blocking)
```

### 依存の詳細

- `bootstrap` は `test` 完了後に開始する（`needs: [test]`）
- `default-path-smoke` は `test` 完了後に開始する（`needs: [test]`）
- `test-fresh-clone` は `fresh-clone-artifact` 完了後に開始し、download 済み release-style archive を Rust toolchain 無しで binary-only smoke する
- `fresh-clone-smoke` は `test` 完了後に開始する（`needs: [test]`）
- `gc-metrics-artifact` は `test` 完了後に開始し、targeted GC metrics JSON を回収する（`needs: [test]`）
- `native-proxy-artifact` は `test` 完了後に開始し、Darwin arm64 host で `build-native.sh` を実行して native proxy bundle artifact を回収する（`needs: [test]`）
- `editor-extension-build` は VS Code 拡張 compile と Claude Code plugin JSON を検証する
- `shadow-oracle` は `test` 完了後に開始する（`needs: [test]`、`continue-on-error: true`）
- `ci-gate` は上記 11 ジョブの全成功を要求する
- `ci-gate-v2` は `ci-gate` の required 11 ジョブ + `shadow-oracle` を集約する

## 必須ジョブ (ci-gate)

| # | ジョブ | コマンド | 説明 |
|---|--------|----------|------|
| 1 | **test** | `cargo test` | 全ユニット / E2E テスト |
| 2 | **lint** | `cargo clippy -- -D warnings` | Rust リント |
| 3 | **format** | `cargo fmt --check` | フォーマット検証 |
| 4 | **bootstrap** | `bash scripts/ci/compile-phase11-inputs.sh` | selfhost 入力セットのコンパイル |
| 5 | **audit-docs** | `bash scripts/audit_docs.sh` | ドキュメント整合性チェック |
| 6 | **default-path-smoke** | `bash scripts/ci/default-path-smoke.sh` | `lsharp` バイナリ経路検証 |
| 7 | **test-fresh-clone** | `bash scripts/ci/test-fresh-clone.sh <archive>` | workflow-local release-style archive を Rust toolchain 無しで binary-only smoke |
| 8 | **fresh-clone-smoke** | `bash scripts/ci/test-fresh-clone.sh` | clean checkout 相当での再ビルド / smoke |
| 9 | **gc-metrics-artifact** | `bash scripts/ci/collect-gc-metrics.sh` | runtime stability 用の GC metrics JSON を生成 |
| 10 | **native-proxy-artifact** | `bash scripts/ci/build-native.sh` | Darwin arm64 host で `stage1-native` / `stage2-native` / `stage3-native` proxy bundle artifact を生成 |
| 11 | **editor-extension-build** | `npm install && npm run compile` + `python3 -c ...plugin.json` | VS Code / Claude Code 拡張の配布面を検証 |

### ci-gate の判定ロジック

```yaml
if: always()
# 11 ジョブすべてが success でなければ exit 1
```

## オプショナルジョブ (ci-gate-v2)

| ジョブ | 説明 | ブロッキング |
|--------|------|--------------|
| **shadow-oracle** | selfhost vs Rust 差分テスト | No (`continue-on-error: true`) |

### ci-gate-v2 の判定ロジック

1. 必須 11 ジョブのいずれかが失敗 → **exit 1**
2. `shadow-oracle` が失敗 → **WARNING ログのみ**（非ブロッキング）
3. 全ジョブ成功 → **pass**

## ブランチ保護

| ブランチ | 必須ジョブ | 説明 |
|----------|-----------|------|
| `main` | `ci-gate-v2` | required 11 ジョブ + non-blocking `shadow-oracle` の集約 |
| PRs | `ci-gate` | 必須 11 ジョブの成功 |

GitHub の branch protection UI では required check として Actions 表示名 `CI Gate v2` を選択し、workflow source / docs 正本では job id `ci-gate-v2` を使う。

## アーティファクト

`ci-gate-v2` はジョブ結果サマリーを `ci-gate-v2-results` として保存する（`retention-days: 30`）。`gc-metrics-artifact` は `gc-metrics-{sha}` として `ci-artifacts/gc-metrics/{commit_sha}/` directory を保存し、`collect-gc-metrics.sh` が正規化した `summary.json` と `collector-proof.json` を同梱する。`native-proxy-artifact` は `native-proxy-{sha}` として `ci-artifacts/native-proxy/{commit_sha}/` directory を保存し、`build-native.sh` が `manifest.json` と `stage1-native` / `stage2-native` / `stage3-native` canonical bundle を同梱する。`test-fresh-clone` は upstream の `fresh-clone-artifact` が `fresh-clone-archive-${sha}` を publish し、それを download して binary-only smoke を行う。

## 同時実行制御

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

同一ブランチ / PR の重複実行は自動キャンセルされる。

## 証跡

- `.github/workflows/ci.yml`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops01_ci_gate_v2`)

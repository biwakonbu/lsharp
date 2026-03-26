# CI Gate v2 ジョブグラフ

CI パイプラインのジョブ依存関係と保護ルールを定義する。

## ジョブ依存関係

```
test ──────────────┐
lint ──────────────┤
format ────────────┤
bootstrap ─────────┤──→ ci-gate ──→ ci-gate-v2
audit-docs ────────┤
default-path-smoke ┘
shadow-oracle ─────────→ ci-gate-v2 (non-blocking)
```

### 依存の詳細

- `bootstrap` は `test` 完了後に開始する（`needs: [test]`）
- `default-path-smoke` は `test` 完了後に開始する（`needs: [test]`）
- `shadow-oracle` は `test` 完了後に開始する（`needs: [test]`、`continue-on-error: true`）
- `ci-gate` は上記 6 ジョブの全成功を要求する
- `ci-gate-v2` は `ci-gate` の 6 ジョブ + `shadow-oracle` を集約する

## 必須ジョブ (ci-gate)

| # | ジョブ | コマンド | 説明 |
|---|--------|----------|------|
| 1 | **test** | `cargo test` | 全ユニット / E2E テスト |
| 2 | **lint** | `cargo clippy -- -D warnings` | Rust リント |
| 3 | **format** | `cargo fmt --check` | フォーマット検証 |
| 4 | **bootstrap** | `bash scripts/ci/compile-phase11-inputs.sh` | selfhost 入力セットのコンパイル |
| 5 | **audit-docs** | `bash scripts/audit_docs.sh` | ドキュメント整合性チェック |
| 6 | **default-path-smoke** | `bash scripts/ci/default-path-smoke.sh` | `lsharp` バイナリ経路検証 |

### ci-gate の判定ロジック

```yaml
if: always()
# 6 ジョブすべてが success でなければ exit 1
```

## オプショナルジョブ (ci-gate-v2)

| ジョブ | 説明 | ブロッキング |
|--------|------|--------------|
| **shadow-oracle** | selfhost vs Rust 差分テスト | No (`continue-on-error: true`) |

### ci-gate-v2 の判定ロジック

1. 必須 6 ジョブのいずれかが失敗 → **exit 1**
2. `shadow-oracle` が失敗 → **WARNING ログのみ**（非ブロッキング）
3. 全ジョブ成功 → **pass**

## ブランチ保護

| ブランチ | 必須ジョブ | 説明 |
|----------|-----------|------|
| `main` | `ci-gate-v2` | shadow-oracle を含む全検証 |
| PRs | `ci-gate` | 必須 6 ジョブの成功 |

## アーティファクト

`ci-gate-v2` はジョブ結果サマリーを `ci-gate-v2-results` として保存する（`retention-days: 30`）。

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

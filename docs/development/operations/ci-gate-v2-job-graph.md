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
native-linux-x86-smoke ┤
native-linux-x86-selfregen ┤
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
- `native-linux-x86-smoke` は `test` 完了後に開始し、Ubuntu x86_64 上で Linux x86_64 native server target の descriptor / ELF emitter / x86_64 codegen smoke を回収する（`needs: [test]`）
- `native-linux-x86-selfregen` は `native-linux-x86-smoke` 完了後に専用 self-hosted Linux x64 runner (`lsharp-linux-x86-selfregen`) 上で `scripts/ci/native-linux-x86-selfregen.sh` を実行し、actual Linux native self-regeneration の stage2/stage3 byte-for-byte compare summary を回収する（`needs: [native-linux-x86-smoke]`）
- `editor-extension-build` は VS Code 拡張 compile と Claude Code plugin JSON を検証する
- `shadow-oracle` は `test` 完了後に開始する（`needs: [test]`、`continue-on-error: true`）
- `ci-gate` は上記 13 ジョブの全成功を要求する
- `ci-gate-v2` は `ci-gate` の required 13 ジョブ + `shadow-oracle` を集約する

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
| 11 | **native-linux-x86-smoke** | `bash scripts/ci/native-linux-x86-smoke.sh` | Linux x86_64 native server target の descriptor / ELF emitter / x86_64 codegen smoke を生成 |
| 12 | **native-linux-x86-selfregen** | `bash scripts/ci/native-linux-x86-selfregen.sh` | 専用 self-hosted Linux x64 runner 上で actual Linux native self-regeneration の stage2/stage3 compare 証跡を生成 |
| 13 | **editor-extension-build** | `npm install && npm run compile` + `python3 -c ...plugin.json` | VS Code / Claude Code 拡張の配布面を検証 |

### ci-gate の判定ロジック

```yaml
if: always()
# 13 ジョブすべてが success でなければ exit 1
```

## ignored gate 実行契約

`test` ジョブの `cargo test` は Rust の標準挙動どおり `#[ignore]` テストを実行しない。required CI の full cargo test は、通常の unit / E2E 回帰を広く確認するための bounded gate として維持し、長時間 bootstrap / LSP parity / legacy native 検証を `-- --ignored` 付きで混ぜない。これにより PR ごとの wall time・artifact 生成量・host resource 消費を予測可能に保ち、重い検証は明示 opt-in の証跡として分離する。

`bootstrap` ジョブは `scripts/ci/compile-phase11-inputs.sh` を正本にし、push / pull_request / manual dispatch の通常経路では以下の契約で実行する。

| gate | 既定 | 実行する ignored suite | 起動条件 |
|------|------|-------------------------|----------|
| Phase 11 fixed input fixed-point | CI で有効 | `test_e2e_bootstrap_fixed_point_stage2_stage3`, `test_e2e_bootstrap_stage2_self_feed_fixed_input_set`, `test_e2e_bootstrap_fixed_input_set_stage_chain_match_*` | `.github/workflows/ci.yml` の `bootstrap` job が `RUN_BOOTSTRAP_FIXED_POINT=1` を渡す |
| LSP parity ignored gates | 既定スキップ | `lsp_stateful_parity` の `test_e2e_lsp_actual_stdio_` / `test_e2e_lsp_stateful_`、`lsp_edge_case_parity` の `test_e2e_lsp_edge_` | `workflow_dispatch` input `run_lsp_parity_gates=true` → `RUN_LSP_PARITY_GATES=1`、またはローカルで同 env を明示 |
| legacy stage1 / bootstrap gates | 既定スキップ | stage1 pipeline / binary / section / module compile / stdlib / examples / determinism probe、selfhost CLI / DocTools / formatter / LSP runtime / macro compiler、legacy native differential / native stage-chain / typeinfer pipeline の `--ignored` suite 群 | `workflow_dispatch` input `run_legacy_stage1_gates=true` → `RUN_BOOTSTRAP_LEGACY_STAGE1=1`、またはローカルで同 env を明示 |

`RUN_LSP_PARITY_GATES` と `RUN_BOOTSTRAP_LEGACY_STAGE1` は script 内でも既定 `0` のため、push / pull_request の required gate では default-skipped のままにする。manual dispatch は同じ `bootstrap` job と artifact 経路を使うが、実行者が input を選んだ場合だけ該当 ignored suite を追加で実行する。

## オプショナルジョブ (ci-gate-v2)

| ジョブ | 説明 | ブロッキング |
|--------|------|--------------|
| **shadow-oracle** | selfhost vs Rust 差分テスト | No (`continue-on-error: true`) |

### ci-gate-v2 の判定ロジック

1. 必須 13 ジョブのいずれかが失敗 → **exit 1**
2. `shadow-oracle` が失敗 → **WARNING ログのみ**（非ブロッキング）
3. 全ジョブ成功 → **pass**

## ブランチ保護

| ブランチ | 必須ジョブ | 説明 |
|----------|-----------|------|
| `main` | `ci-gate-v2` | required 13 ジョブ + non-blocking `shadow-oracle` の集約 |
| PRs | `ci-gate` | 必須 13 ジョブの成功 |

GitHub の branch protection UI では required check として Actions 表示名 `CI Gate v2` を選択し、workflow source / docs 正本では job id `ci-gate-v2` を使う。

## アーティファクト

`ci-gate-v2` はジョブ結果サマリーを `ci-gate-v2-results` として保存する（`retention-days: 30`）。`gc-metrics-artifact` は `gc-metrics-{sha}` として `ci-artifacts/gc-metrics/{commit_sha}/` directory を保存し、`collect-gc-metrics.sh` が正規化した `summary.json` と `collector-proof.json` を同梱する。`native-proxy-artifact` は `native-proxy-{sha}` として `ci-artifacts/native-proxy/{commit_sha}/` directory を保存し、`build-native.sh` が `manifest.json`、`stage1-native` / `stage2-native` / `stage3-native` canonical bundle、`actual-stage23-gap.json` blocker report を同梱する。`native-linux-x86-smoke` は `native-linux-x86-{commit_sha}` として `ci-artifacts/native-linux-x86/{commit_sha}/summary.json` を保存し、`x86_64-unknown-linux-gnu` の target smoke 証跡を同梱する。`native-linux-x86-selfregen` は `native-linux-x86-selfregen-{commit_sha}` として `ci-artifacts/native-linux-x86-hostgen-vm/{commit_sha}/actual-selfregen-summary.json` と failure/debug artifact を保存し、actual Linux native self-regeneration の required evidence とする。`test-fresh-clone` は upstream の `fresh-clone-artifact` が `fresh-clone-archive-${sha}` を publish し、それを download して binary-only smoke を行う。

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

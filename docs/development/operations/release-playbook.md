# リリースプレイブック

L# のリリース手順を定義する。自動化スクリプト `scripts/release-playbook.sh` と連携して使用する。

## 概要

```
バージョンバンプ → CI 検証 → アーティファクト生成 → チェックサム → タグ作成 → GitHub Release
```

## 手順

### 1. バージョンバンプ

```bash
# Cargo.toml のバージョンを更新
# workspace 全体で統一バージョンを使用
vim Cargo.toml   # version = "0.x.y"
```

- `Cargo.toml` の `[workspace.package]` セクションで一元管理
- セマンティックバージョニングに従う

### 2. CI 検証

```bash
./scripts/release-playbook.sh <version>
```

スクリプトは以下を順に実行する:

| Step | コマンド | 説明 |
|------|----------|------|
| 1 | `cargo build --release` | リリースビルド |
| 2 | `cargo test` | 全テスト実行 |
| 3 | `cargo clippy -- -D warnings` | リント |
| 4 | `cargo fmt --check` | フォーマット検証 |
| 5 | `LSHARP_BIN=target/release/lsharp bash scripts/ci/compile-phase11-inputs.sh` | release バイナリで selfhost / stdlib / examples の固定入力セットを検証 |
| 6 | `LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh` + `scripts/smoke_test_readme.sh` | release バイナリ smoke + README smoke |
| 7 | チェックサム生成 | `scripts/checksum.sh` |

### 3. アーティファクト生成

リリースビルド成果物:

| アーティファクト | 説明 |
|---|---|
| `lsharp` バイナリ | `target/release/lsharp` |
| release playbook 検証成果物 | `target/release-playbook/` 以下の bootstrap / smoke 出力 |
| チェックサム | SHA-256 チェックサムファイル |

### 4. チェックサム生成

```bash
# scripts/checksum.sh が利用可能な場合
bash scripts/checksum.sh
```

全リリースアーティファクトに SHA-256 チェックサムを付与する。

### 5. タグ作成

```bash
git tag v<version>
git push origin v<version>
```

- タグ名は `v` プレフィックス付き（例: `v0.2.0`）
- タグはリリースコミットに対して作成する

### 6. GitHub Release 公開

1. GitHub Releases ページで新規リリースを作成
2. タグ `v<version>` を選択
3. リリースノートを記載（変更点、破壊的変更、移行手順）
4. アーティファクトをアップロード
5. チェックサムファイルを添付

## リリースチャネル

| チャネル | 頻度 | 説明 |
|----------|------|------|
| **stable** | 月次〜四半期 | 全テスト通過 + 2 週間の RC 期間 |
| **nightly** | 日次 | main ブランチの HEAD ビルド。安定性保証なし |

### Stable リリース基準

- 全 CI ジョブ（`ci-gate-v2`）が pass
- RC 期間中に致命的バグ報告なし
- パフォーマンス回帰なし（ベンチマーク比較）
- ドキュメント更新完了

### Nightly ビルド

- `main` への push 毎に自動生成
- タグ: `nightly-{date}` (例: `nightly-2026-01-15`)
- 保持期間: 7 日

## ロールバック

リリース後に致命的問題が発見された場合:

1. 該当リリースを GitHub Releases で `pre-release` に変更
2. 修正版を緊急リリース（パッチバージョン）
3. 必要に応じて `scripts/rollback.sh` で Rust 実装に切り戻し

詳細は `docs/development/operations/rollback-procedure.md` を参照。

## 証跡

- `scripts/release-playbook.sh`
- `scripts/checksum.sh`
- `scripts/smoke_test_readme.sh`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops06_release_playbook`)

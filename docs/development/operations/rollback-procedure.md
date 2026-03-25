# ロールバック手順

selfhost コンパイラに致命的な問題が発生した場合の、Rust 実装への緊急ロールバック手順。

## 前提条件

- `legacy-rust-bootstrap/` ディレクトリに Rust 実装のスナップショットが存在すること
- Git リポジトリがクリーンな状態であること

## ロールバック手順

### 1. 問題の特定と記録

```bash
# 現在の状態を記録
git log --oneline -5
git status
```

### 2. ロールバックブランチの作成

```bash
git checkout -b rollback/emergency-$(date +%Y%m%d)
```

### 3. Rust 実装の復元

```bash
# crates/ を legacy-rust-bootstrap/ から復元
cp -r legacy-rust-bootstrap/crates/ crates/
cp legacy-rust-bootstrap/Cargo.toml Cargo.toml
cp legacy-rust-bootstrap/Cargo.lock Cargo.lock
```

### 4. 検証

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

### 5. コミットとデプロイ

```bash
git add -A
git commit -m "emergency: rollback to Rust implementation"
git push origin rollback/emergency-$(date +%Y%m%d)
```

## ロールバック後の対応

1. selfhost コンパイラの問題を調査
2. 修正パッチを作成
3. 修正を検証後、再度 selfhost に切り替え

## スクリプトによる自動化

```bash
# ドライランで手順を確認
./scripts/rollback.sh --dry-run

# 実行 (手動確認が必要)
./scripts/rollback.sh
```

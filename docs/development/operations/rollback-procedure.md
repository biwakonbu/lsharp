# ロールバック手順

selfhost コンパイラに致命的な問題が発生した場合の、Rust 実装への緊急ロールバック手順。

> **関連ドキュメント**
> - ADR (撤去決定・前提条件): [`docs/development/operations/adr-rust-removal.md`](./adr-rust-removal.md)
> - 撤去前ゲート定義: [`docs/development/planning/completion-criteria.md` § P11-2e-3](../planning/completion-criteria.md)
> - 配布・署名: [`docs/development/operations/release-distribution-signing.md`](./release-distribution-signing.md)

## ロールバックの種類

| 種類 | タイミング | 前提 |
|------|-----------|------|
| **A: 撤去中の即時ロールバック** | 段階的削除の途中で問題が発見された場合 | `git revert` で復元可能。`v0.x.y-rust-final` タグ不要 |
| **B: 撤去完了後の完全ロールバック** | Rust 削除後に深刻な問題が発生した場合 | `v0.x.y-rust-final` タグが存在すること (**PENDING**) |
| **C: `legacy-rust-bootstrap/` からの復元** | A / B が使えない場合の最終手段 | `legacy-rust-bootstrap/` ディレクトリが存在すること |

> **注意**: タイプ B は `v0.x.y-rust-final` タグの作成 (**現在 PENDING**) が前提となる。
> タグが確定するまでは タイプ A / C のみ実施可能。タグ作成手順は `adr-rust-removal.md` の「撤去前提条件」を参照。

## 前提条件

- `legacy-rust-bootstrap/` ディレクトリに Rust 実装のスナップショットが存在すること
- Git リポジトリがクリーンな状態であること

## ロールバック手順

### A: 撤去中の即時ロールバック

撤去の段階的削除中に問題が発見された場合:

```bash
git revert <removal-commit>
cargo build   # Rust コンパイラが復元されることを確認
cargo test
```

### B: 撤去完了後の完全ロールバック（`v0.x.y-rust-final` タグ使用）

> **PENDING**: このロールバック経路は `v0.x.y-rust-final` タグが確定してから使用可能になる。

タグが作成された後の手順:

#### B-1. 問題の特定と記録

```bash
git log --oneline -5
git status
```

#### B-2. ロールバックブランチの作成

```bash
git checkout -b rollback/emergency-$(date +%Y%m%d)
```

#### B-3. Rust 実装の復元

```bash
# crates/ を legacy-rust-bootstrap/ から復元
cp -r legacy-rust-bootstrap/crates/ crates/
cp legacy-rust-bootstrap/Cargo.toml Cargo.toml
cp legacy-rust-bootstrap/Cargo.lock Cargo.lock
```

#### B-4. 検証

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

#### B-5. コミットとデプロイ

```bash
git add -A
git commit -m "emergency: rollback to Rust implementation"
git push origin rollback/emergency-$(date +%Y%m%d)
```

### C: `legacy-rust-bootstrap/` からの最終手段復元

タイプ A / B が使えない場合:

```bash
# rollback-procedure.md の手順に従う
bash scripts/rollback.sh --dry-run  # シミュレーション
bash scripts/rollback.sh            # 実行
```

## ロールバック後の対応

1. selfhost コンパイラの問題を調査
2. 修正パッチを作成
3. 修正を検証後、再度 selfhost に切り替え

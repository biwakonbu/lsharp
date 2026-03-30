# ロールバック手順

selfhost コンパイラまたは配布物に致命的な問題が発生した場合の、**host launcher + embedded guest component** を対象とした緊急ロールバック手順。

> **関連ドキュメント**
> - ADR (配布モデル転換・前提条件): [`docs/development/operations/adr-rust-removal.md`](./adr-rust-removal.md)
> - Phase 13 移行前ゲート定義: [`docs/development/planning/completion-criteria.md` § P11-2e-3](../planning/completion-criteria.md)
> - 配布・署名: [`docs/development/operations/release-distribution-signing.md`](./release-distribution-signing.md)

## ロールバックの種類

| 種類 | タイミング | 前提 |
|------|-----------|------|
| **A: リリース切替中の即時ロールバック** | host launcher / component の切替 PR 中に問題が発見された場合 | `git revert` で復元可能 |
| **B: 公開済み配布物の巻き戻し** | 公開済みリリースに深刻な問題が発生した場合 | last-known-good release tag または package が存在すること |
| **C: component 再埋め込み / 再パッケージ** | launcher は正常だが guest component のみ壊れている場合 | 正常な component artifact が存在すること |

> **注意**: 本文書のロールバック対象は「Rust 実装への回帰」ではない。Rust workspace は host launcher / component tooling context として残存するため、巻き戻し先は **前回正常な host launcher / guest component の組み合わせ** とする。

## last-known-good (LKG) anchor

stable release の rollback 先は、GitHub Release notes に記録した `Rollback anchor` を正本とする。

- `last-known-good release tag`: 前回正常な stable tag (`vX.Y.Z`)
- `host launcher assets`: その tag に添付した host launcher archive 群
- `guest component assets`: 同じ tag に添付した embedded guest component / sidecar package
- `checksum`: 同じ release の checksum file

package manager package は二次配布のため、LKG 判定や rollback 復元元には使わない。必ず GitHub Release 上の tag + asset set を使う。

## 前提条件

- GitHub Release notes の `Rollback anchor` から last-known-good release tag / asset 名 / checksum 名が特定できること
- Git リポジトリがクリーンな状態であること

## ロールバック手順

### A: リリース切替中の即時ロールバック

host launcher / component の切替中に問題が発見された場合:

```bash
git revert <cutover-commit>
cargo build   # host launcher が再ビルドできることを確認
cargo test
```

### B: 公開済み配布物の巻き戻し

前回正常な release tag / package が存在する前提で、公開済み配布物を巻き戻す。

#### B-1. 問題の特定と記録

```bash
git log --oneline -5
git status
```

同時に GitHub Release notes の `Rollback anchor` から以下を控える。

- `last-known-good release tag`
- host launcher asset 名
- guest component asset 名
- checksum file 名

#### B-2. ロールバックブランチの作成

```bash
git checkout -b rollback/emergency-$(date +%Y%m%d)
```

#### B-3. 前回正常な host launcher / component の復元

```bash
git checkout v<last-known-good> -- .
cargo build --release
```

- embedded guest component のみ壊れている場合は、同じ `Rollback anchor` に記録された guest component asset を復元する。
- sidecar package を併売している場合も、同一 tag の asset set だけを使う。

#### B-4. 検証

```bash
cargo test
cargo clippy -- -D warnings
LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh
```

#### B-5. コミットとデプロイ

```bash
git add -A
git commit -m "emergency: rollback host launcher package to last-known-good component"
git push origin rollback/emergency-$(date +%Y%m%d)
```

### C: component 再埋め込み / 再パッケージ

launcher 自体は正常で、embedded guest component または sidecar package のみ差し替える場合:

```bash
bash scripts/rollback.sh --dry-run v<last-known-good>             # シミュレーション
bash scripts/rollback.sh v<last-known-good> <guest-component-asset>  # 実行
```

実行後は以下を確認する。

```bash
LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh
cargo test
```

## ロールバック後の対応

1. host launcher / guest component のどちらに問題があったかを切り分ける
2. 修正パッチを作成
3. 修正を Wasm component smoke で検証後、再度配布経路へ戻す

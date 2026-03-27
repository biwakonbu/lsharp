# V2-03: パッケージマネージャー配布

## 概要

Homebrew, apt, scoop を通じた L# コンパイラの配布。公式アーカイブとの関係、checksum、release 後段の運用順序は [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md) を正本とし、このページでは package manager 固有の差分だけを保持する。

## 前提条件
- PKG-01 (公式アーカイブ) 完了

## 設計
### Homebrew
- Formula: `lsharp.rb`
- `brew install lsharp`
- バージョン/チェックサムの自動更新

### apt (Debian/Ubuntu)
- PPA またはカスタムリポジトリ
- `apt install lsharp`
- GPG 署名付きパッケージ

### scoop (Windows)
- Manifest: `lsharp.json`
- `scoop install lsharp`
- チェックサム検証

### バージョンパリティ
- 全パッケージマネージャーで同一バージョン
- リリースから 24 時間以内に全チャネル更新

## 正本参照

- 公式アーカイブ / checksum / signing 順序: [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md)
- 手元リリース手順: [`../../operations/release-playbook.md`](../../operations/release-playbook.md)
- artifact 命名 / retention: [`../../operations/artifact-policy.md`](../../operations/artifact-policy.md)

## ステータス
Phase 11 後に実装予定。

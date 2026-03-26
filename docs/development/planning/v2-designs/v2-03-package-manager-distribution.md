# V2-03: パッケージマネージャー配布

## 概要
Homebrew, apt, scoop を通じた L# コンパイラの配布。

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

## ステータス
Phase 11 後に実装予定。

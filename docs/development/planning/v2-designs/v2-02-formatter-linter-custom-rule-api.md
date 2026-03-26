# V2-02: Formatter/Linter カスタムルール API

## 概要
ビルトインルールに加えて、ユーザー定義のフォーマット・リントルールを追加可能にする。

## 前提条件
- FMT-01 (Formatter roundtrip) 完了
- LINT-01 (ビルトインルール) 完了

## 設計
### AST Walker API
- `LintContext` 構造体の公開
- ノード訪問コールバック (pre-visit / post-visit)
- ルール登録 API

### Config Loader
- TOML 設定ファイルからのルール読み込み
- ルール優先度と競合解決
- v1 では `custom-rules = []` 時にエラー返却

### ビルトイン順序の維持
- カスタムルールはビルトインルールの後に実行
- ビルトインルールの結果をオーバーライド不可

## ステータス
Phase 11 後に実装予定。

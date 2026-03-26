# V2-04: Linux aarch64 Tier2 配布

## 概要
Linux aarch64 (ARM64) プラットフォームを Tier2 サポートとして追加。

## 前提条件
- Tier1 プラットフォーム (macOS arm64, macOS x86_64, Linux x86_64, Windows x86_64) が安定

## 設計
### クロスビルド記述子
- `NativeTarget.ls` に `linux-aarch64` ターゲット追加
- クロスコンパイル CI ジョブ

### アーティファクト命名
- Tier1 と同一命名規則: `lsharp-{version}-linux-aarch64.tar.gz`
- チェックサム付き

### テスト戦略
- QEMU ベースの CI テスト
- Tier2 = テスト実行するが、リリースブロッカーにはしない

## ステータス
Phase 11 後に実装予定。

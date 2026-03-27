# V2-04: Linux aarch64 Tier2 配布

## 概要

Linux aarch64 (ARM64) プラットフォームを Tier2 サポートとして追加。tier1/tier2 の定義、artifact 命名、cross-build の運用位置は [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md) を正本とし、このページでは linux-aarch64 固有の追加要求だけを保持する。

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

## 正本参照

- 配布階層 / checksum / cross-build 方針: [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md)
- artifact 命名 / retention: [`../../operations/artifact-policy.md`](../../operations/artifact-policy.md)
- 手元リリース手順: [`../../operations/release-playbook.md`](../../operations/release-playbook.md)

## ステータス
Phase 11 後に実装予定。

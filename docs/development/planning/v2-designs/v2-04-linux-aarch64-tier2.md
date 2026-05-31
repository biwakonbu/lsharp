# V2-04: Linux aarch64 archived design

## 概要

Linux aarch64 (ARM64) プラットフォームを将来再導入する場合の archived design。現行 supported product/release targets は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) の 2 つであり、Linux aarch64 は out of support scope として release blocker から外す。配布対象、artifact 命名、cross-build の運用位置は [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md) を正本とし、このページでは linux-aarch64 固有の追加要求だけを保持する。

## 前提条件
- support scope を変更して Linux aarch64 を再導入する判断があること
- supported product/release targets (Mac Apple Silicon, Linux x86_64) が安定

## 設計
### クロスビルド記述子
- `NativeTarget.ls` に `linux-aarch64` ターゲット追加
- クロスコンパイル CI ジョブ

### アーティファクト命名
- supported target と同一命名規則: `lsharp-{version}-aarch64-unknown-linux-gnu.tar.gz`
- チェックサム付き

### テスト戦略
- QEMU ベースの CI テスト
- support scope 再導入前は product/release blocker にしない

## 正本参照

- 配布階層 / checksum / cross-build 方針: [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md)
- artifact 命名 / retention: [`../../operations/artifact-policy.md`](../../operations/artifact-policy.md)
- 手元リリース手順: [`../../operations/release-playbook.md`](../../operations/release-playbook.md)

## ステータス
Archived / out of support scope。

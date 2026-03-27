# V2-05: Windows Authenticode 署名

## 概要

Windows バイナリに Authenticode コード署名を追加。release 順序、checksum、Windows 以外を含む配布全体の正本は [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md) を参照し、このページでは Windows 固有の署名要件だけを保持する。

## 前提条件
- PKG-01 (公式アーカイブ) 完了
- Windows ビルドが安定

## 設計
### 署名プロセス
1. EV コードサイニング証明書取得
2. CI シークレットとして証明書を保存
3. リリースパイプラインで `signtool.exe` による署名
4. タイムスタンプサーバー使用 (証明書失効後も有効)

### 検証ステップ
- `signtool verify /pa lsharp.exe`
- SmartScreen 警告が表示されないことを確認

### CI 統合
- Windows リリースジョブに署名ステップ追加
- 署名検証を smoke test に含める

## 正本参照

- 配布フロー / signing 順序: [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md)
- 手元リリース手順: [`../../operations/release-playbook.md`](../../operations/release-playbook.md)
- artifact 命名 / retention: [`../../operations/artifact-policy.md`](../../operations/artifact-policy.md)

## ステータス
Phase 11 後に実装予定。

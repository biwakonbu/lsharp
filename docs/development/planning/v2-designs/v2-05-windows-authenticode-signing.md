# V2-05: Windows Authenticode 署名

## 概要
Windows バイナリに Authenticode コード署名を追加。

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

## ステータス
Phase 11 後に実装予定。

# V2-01: LSP インクリメンタル同期

## 概要
LSP の Full Sync (TextDocumentSyncKind.Full) から Incremental Sync への移行。

## 前提条件
- LSP-01 (Full Sync スケルトン) 完了
- LSP-02 (10 メソッドパリティ) 完了

## 設計
### テキスト編集差分適用レイヤー
- `TextDocumentContentChangeEvent` の range ベース適用
- ドキュメントバッファの差分更新
- JSON スナップショット互換性の維持

### 実装方針
1. ドキュメントバッファにロープ (rope) データ構造を導入
2. 差分イベントのバリデーション (range 範囲チェック)
3. パース対象を変更範囲に限定する増分パーサー統合
4. Full Sync フォールバック (差分適用失敗時)

### テスト戦略
- 既存 Full Sync テストが引き続きパス
- 差分適用の正確性テスト
- パフォーマンステスト (1000行ファイルの部分編集 < 50ms)

## ステータス
Phase 11 後に実装予定。

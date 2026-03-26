# V2-07: WasmGC オプショナルバックエンド

## 概要
WasmGC (Garbage Collection proposal) を使用するオプショナルバックエンドの追加。

## 前提条件
- GC-03 (世代別パス) 完了
- 現行 Wasm backend が安定

## 設計
### 共有レイヤー
- AST / Type / IR は共有
- Codegen / Runtime ABI はバックエンド固有

### WasmGC 固有
- `struct` / `array` GC 型の使用
- `ref.cast` / `ref.test` による型検査
- Records → `struct` 型マッピング
- ADT → tagged `struct` (タグフィールド + ペイロード)
- Strings → `array i8` (UTF-8)

### 優先順位
1. Records / ADT → struct 型
2. Strings → array 型
3. Closures → funcref + env struct
4. GC 自動管理 (手動 mark-sweep 不要)

### 切り替え方法
- `--backend=wasmgc` コンパイラフラグ
- デフォルトは既存のリニアメモリバックエンド

## ステータス
Phase 11 後に実装予定。

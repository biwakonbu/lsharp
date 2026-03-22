; computation.ls - Computation Expression のサンプル
; 注: MVP 段階ではビルダー登録のみ。let!/return の Wasm 実行は GC 型の wasmtime サポート後に完全対応予定。

(defn identity [x] x)
(defn mb [m x] m)

(computation-builder maybe-builder mb identity)

(defn main []
  (print 42))

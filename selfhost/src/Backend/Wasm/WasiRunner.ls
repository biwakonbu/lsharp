(module Backend.Wasm.WasiRunner)

;; WasiRunner.ls - L# セルフホスティング: WASI 実行ランナー
;;
;; コンパイル済み Wasm バイナリを WASI ランタイムで実行する。

;; === 実行状態 ===

;; 実行結果: [exit-code, stdout-bytes, stderr-bytes]
(defn make-run-result [exit-code stdout-bytes stderr-bytes]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) exit-code)
      stdout-bytes)
    stderr-bytes))

;; === WASI ランナー ===

;; run-wasi: Wasm バイナリを WASI 環境で実行
;; wasm-bytes: Wasm バイナリの Vector
;; 戻り値: run-result
(defn run-wasi [wasm-bytes]
  ;; WASI ランタイムの初期化と実行
  ;; 実際の実行はホストランタイム (wasmtime) が担当
  ;; ここではインターフェースの定義のみ
  (let [exit-code 0
        stdout (vector-new 256)
        stderr (vector-new 256)]
    (make-run-result exit-code stdout stderr)))

;; エントリポイント (テスト用)
(defn main []
  (let [result (run-wasi (vector-new 0))]
    (do
      (print (vector-get result 0))   ;; exit-code: 0
      0)))

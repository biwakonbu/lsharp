(module Backend.Wasm.Emit)

;; Emit.ls - L# セルフホスティング: Wasm バイナリ section builders
;;
;; Wasm バイナリのセクション構築とバイナリエンコーディング。
;; LEB128 エンコーダ、セクションビルダー等を提供。

;; === LEB128 エンコーディング ===

;; 符号なし LEB128 エンコード: 値 -> バイト列 (Vector)
(defn emit-leb128 [bytes value]
  (let [result (ref-new bytes)
    v (ref-new value)]
    (do
      (let [byte (% (ref-get v) 128)
        rest (/ (ref-get v) 128)]
        (if (= rest 0)
          (do (ref-set result (vector-push (ref-get result) byte)) 0)
          (do
            (ref-set result (vector-push (ref-get result) (+ byte 128)))
            (ref-set v rest)
            (let [byte2 (% (ref-get v) 128)
              rest2 (/ (ref-get v) 128)]
              (if (= rest2 0)
                (do (ref-set result (vector-push (ref-get result) byte2)) 0)
                (do
                  (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                  (ref-set v rest2)
                  (do (ref-set result (vector-push (ref-get result) (% (ref-get v) 128))) 0)))))))
      (ref-get result))))

;; 符号なし LEB128 エンコード (単体): 値 -> バイト列 Vector
(defn encode-leb128 [value]
  (emit-leb128 (vector-new 4) value))

;; === バイト列操作 ===

;; バイト列にバイトを追加
(defn emit-byte [bytes b]
  (vector-push bytes b))

;; === セクションビルダー ===

;; Type セクション (ID=1): 関数シグネチャ定義
;; 簡易版: () -> i64 の1型のみ
(defn emit-type-section []
  (let [bytes (vector-new 16)]
    (let [b1 (emit-byte bytes 1) ;; Section ID = 1 (Type)
      b2 (emit-byte b1 5) ;; セクションサイズ
      b3 (emit-byte b2 1) ;; 型数
      b4 (emit-byte b3 96) ;; 0x60 = func type
      b5 (emit-byte b4 0) ;; パラメータ数
      b6 (emit-byte b5 1) ;; 戻り値数
      b7 (emit-byte b6 126)] ;; i64 = 0x7E
      b7)))

;; Function セクション (ID=3): funcidx -> typeidx マッピング
(defn emit-function-section []
  (let [bytes (vector-new 8)]
    (let [b1 (emit-byte bytes 3) ;; Section ID = 3
      b2 (emit-byte b1 2) ;; セクションサイズ
      b3 (emit-byte b2 1) ;; 関数数
      b4 (emit-byte b3 0)] ;; type index 0
      b4)))

;; Memory セクション (ID=5): linear memory 定義
(defn emit-memory-section []
  (let [bytes (vector-new 8)]
    (let [b1 (emit-byte bytes 5) ;; Section ID = 5
      b2 (emit-byte b1 3) ;; セクションサイズ
      b3 (emit-byte b2 1) ;; メモリ数
      b4 (emit-byte b3 0) ;; limits: no max
      b5 (emit-byte b4 1)] ;; initial pages = 1
      b5)))

;; Export セクション (ID=7): _start エクスポート
(defn emit-export-section []
  (let [bytes (vector-new 16)]
    (let [b1 (emit-byte bytes 7) ;; Section ID = 7
      b2 (emit-byte b1 10) ;; セクションサイズ
      b3 (emit-byte b2 1) ;; エクスポート数
      b4 (emit-byte b3 6) ;; 名前長 "_start"
      b5 (emit-byte b4 95) ;; '_'
      b6 (emit-byte b5 115) ;; 's'
      b7 (emit-byte b6 116) ;; 't'
      b8 (emit-byte b7 97) ;; 'a'
      b9 (emit-byte b8 114) ;; 'r'
      b10 (emit-byte b9 116) ;; 't'
      b11 (emit-byte b10 0) ;; kind = function
      b12 (emit-byte b11 0)] ;; func index 0
      b12)))

;; Wasm ヘッダー生成 (magic + version)
(defn emit-header []
  (let [h (vector-new 8)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push h 0) ;; \0
                  97) ;; a
                115) ;; s
              109) ;; m
            1) ;; version 1
          0) 0) 0)))

;; エントリポイント (テスト用)
(defn main []
  (let [leb (encode-leb128 300)
    header (emit-header)]
    (do
      ;; LEB128(300) = [172, 2]
      (print (vector-get leb 0)) ;; 172
      (print (vector-get leb 1)) ;; 2
      ;; ヘッダー検証
      (print (vector-length header)) ;; 8
      0)))

;; WasmEmit.ls - L# セルフホスティング: Wasm バイナリ生成
;;
;; IR 命令列から Wasm バイナリを生成する。
;; LEB128 エンコーディング + Wasm セクション構造。

;; === Wasm バイナリ構造 ===

;; マジックナンバー: \0asm
(defn wasm-magic-0 [] 0)    ;; 0x00
(defn wasm-magic-1 [] 97)   ;; 'a'
(defn wasm-magic-2 [] 115)  ;; 's'
(defn wasm-magic-3 [] 109)  ;; 'm'

;; バージョン: 1.0
(defn wasm-version-0 [] 1)
(defn wasm-version-1 [] 0)
(defn wasm-version-2 [] 0)
(defn wasm-version-3 [] 0)

;; セクション ID
(defn section-type [] 1)
(defn section-function [] 3)
(defn section-export [] 7)
(defn section-code [] 10)

;; Wasm 値型
(defn wasm-i32 [] 127)     ;; 0x7F
(defn wasm-i64 [] 126)     ;; 0x7E
(defn wasm-funcref [] 112) ;; 0x70

;; Wasm 命令
(defn wasm-end [] 11)       ;; 0x0B
(defn wasm-i64-const [] 66) ;; 0x42
(defn wasm-local-get [] 32) ;; 0x20
(defn wasm-local-set [] 33) ;; 0x21
(defn wasm-i64-add [] 124)  ;; 0x7C (i64.add)
(defn wasm-i64-sub [] 125)  ;; 0x7D (i64.sub)
(defn wasm-i64-mul [] 126)  ;; 0x7E (i64.mul)
(defn wasm-call [] 16)      ;; 0x10
(defn wasm-return [] 15)    ;; 0x0F

;; === LEB128 エンコーディング ===

;; 符号なし LEB128: 値 → バイト列 (Vector)
(defn leb128-u [value]
  (let [result (ref-new (vector-new 4))
        v (ref-new value)]
    (do
      (let [byte (% (ref-get v) 128)
            rest (/ (ref-get v) 128)]
        (if (= rest 0)
          (ref-set result (vector-push (ref-get result) byte))
          (do
            (ref-set result (vector-push (ref-get result) (+ byte 128)))
            (ref-set v rest)
            (let [byte2 (% (ref-get v) 128)
                  rest2 (/ (ref-get v) 128)]
              (if (= rest2 0)
                (ref-set result (vector-push (ref-get result) byte2))
                (do
                  (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                  (ref-set v rest2)
                  (ref-set result (vector-push (ref-get result) (% (ref-get v) 128)))))))))
      (ref-get result))))

;; 符号付き LEB128 (簡易版: 正の値のみ)
(defn leb128-s [value]
  (leb128-u value))

;; === バイト列操作 ===

;; バイト列に LEB128 値を追加
(defn emit-leb128 [bytes value]
  (let [leb (leb128-u value)
        result (ref-new bytes)
        i (ref-new 0)
        n (vector-length leb)]
    (do
      (if (< (ref-get i) n)
        (do
          (ref-set result (vector-push (ref-get result) (vector-get leb (ref-get i))))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (ref-set result (vector-push (ref-get result) (vector-get leb (ref-get i))))
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) n)
                (do
                  (ref-set result (vector-push (ref-get result) (vector-get leb (ref-get i))))
                  0)
                0))
            0))
        0)
      (ref-get result))))

;; バイト列にバイトを追加
(defn emit-byte [bytes b]
  (vector-push bytes b))

;; === Wasm ヘッダー生成 ===

;; Wasm バイナリの先頭 8 バイト
(defn emit-header []
  (let [h (vector-new 8)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push h 0)  ;; \0
                  97)   ;; a
                115)  ;; s
              109)  ;; m
            1)    ;; version 1
          0)
        0)
      0)))

;; === Type セクション生成 ===

;; 関数型 () -> i64 を生成
(defn emit-type-section-main []
  (let [bytes (vector-new 16)]
    ;; Section ID = 1 (Type)
    (let [b1 (emit-byte bytes 1)
          ;; セクションサイズ (5バイト)
          b2 (emit-byte b1 5)
          ;; 型の数 (1個)
          b3 (emit-byte b2 1)
          ;; 関数型マーカー (0x60)
          b4 (emit-byte b3 96)
          ;; パラメータ数 (0)
          b5 (emit-byte b4 0)
          ;; 戻り値数 (1)
          b6 (emit-byte b5 1)
          ;; 戻り値型 (i64 = 0x7E)
          b7 (emit-byte b6 126)]
      b7)))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [header (emit-header)
        type-sec (emit-type-section-main)
        leb5 (leb128-u 5)
        leb300 (leb128-u 300)]
    (do
      ;; ヘッダー検証
      (print (vector-length header))    ;; 8
      (print (vector-get header 0))     ;; 0 (\0)
      (print (vector-get header 1))     ;; 97 (a)
      (print (vector-get header 2))     ;; 115 (s)
      (print (vector-get header 3))     ;; 109 (m)
      (print (vector-get header 4))     ;; 1 (version)

      ;; Type セクション検証
      (print (vector-length type-sec))  ;; 7
      (print (vector-get type-sec 0))   ;; 1 (section id)
      (print (vector-get type-sec 1))   ;; 5 (size)
      (print (vector-get type-sec 2))   ;; 1 (count)
      (print (vector-get type-sec 3))   ;; 96 (0x60 = func type)

      ;; LEB128 検証
      (print (vector-get leb5 0))       ;; 5
      (print (vector-get leb300 0))     ;; 172
      (print (vector-get leb300 1))     ;; 2

      0)))

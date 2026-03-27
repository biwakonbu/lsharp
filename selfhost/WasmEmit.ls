(module WasmEmit)
(import IR)

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
(defn section-import [] 2)
(defn section-function [] 3)
(defn section-memory [] 5)
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
(defn wasm-i64-eq [] 81)    ;; 0x51 (i64.eq)
(defn wasm-i64-div-s [] 127) ;; 0x7F (i64.div_s)
(defn wasm-if [] 4)         ;; 0x04
(defn wasm-else [] 5)       ;; 0x05
(defn wasm-drop [] 26)      ;; 0x1A

;; === LEB128 エンコーディング ===

;; 符号なし LEB128: 値 → バイト列 (Vector)
(defn leb128-u [value]
  (let [result (ref-new (vector-new 4))
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

;; 符号付き LEB128: 値 → バイト列 (Vector)
;; 正の値と負の値の両方をサポート
;; 負の値は 2 の補数表現を使用
(defn leb128-s [value]
  (if (< value 0)
    ;; 負の値の処理
    ;; -1 → [0x7F], -128 → [0x00, 0x7F], etc.
    (let [result (ref-new (vector-new 4))
          v (ref-new value)
          done (ref-new 0)]
      (do
        ;; バイト 1: value & 0x7F
        ;; 負数の場合: (value + 128) % 128 でマスク、上位ビット判定
        (let [byte1 (% (+ (% (ref-get v) 128) 128) 128)
              rest1 (if (< (ref-get v) -64) 1 0)]
          (if (= rest1 0)
            ;; 1バイトで収まる (-64 <= value < 0)
            (do (ref-set result (vector-push (ref-get result) byte1)) 0)
            ;; 2バイト以上必要
            (do
              (ref-set result (vector-push (ref-get result) (+ byte1 128)))
              ;; 算術右シフト: (value - byte1) / 128
              ;; ただし L# では負数の除算が切り捨てなので注意
              (let [shifted (/ (- (ref-get v) byte1) 128)
                    byte2 (% (+ (% shifted 128) 128) 128)
                    rest2 (if (< shifted -64) 1 0)]
                (if (= rest2 0)
                  (do (ref-set result (vector-push (ref-get result) byte2)) 0)
                  (do
                    (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                    (let [shifted2 (/ (- shifted byte2) 128)
                          byte3 (% (+ (% shifted2 128) 128) 128)]
                      (do (ref-set result (vector-push (ref-get result) byte3)) 0))))))))
        (ref-get result)))
    ;; 正の値: 符号ビットを考慮 (最上位ビットが 0 であることを保証)
    (let [result (ref-new (vector-new 4))
          v (ref-new value)]
      (do
        (let [byte (% (ref-get v) 128)
              rest (/ (ref-get v) 128)]
          (if (= rest 0)
            ;; 1バイト: 0 <= value < 64 の場合はそのまま
            ;; 64 <= value < 128 の場合は 2バイト必要 (符号ビット)
            (if (< byte 64)
              (do (ref-set result (vector-push (ref-get result) byte)) 0)
              (do
                (ref-set result (vector-push (ref-get result) (+ byte 128)))
                (do (ref-set result (vector-push (ref-get result) 0)) 0)))
            (do
              (ref-set result (vector-push (ref-get result) (+ byte 128)))
              (ref-set v rest)
              (let [byte2 (% (ref-get v) 128)
                    rest2 (/ (ref-get v) 128)]
                (if (= rest2 0)
                  (if (< byte2 64)
                    (do (ref-set result (vector-push (ref-get result) byte2)) 0)
                    (do
                      (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                      (do (ref-set result (vector-push (ref-get result) 0)) 0)))
                  (do
                    (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                    (ref-set v rest2)
                    (let [byte3 (% (ref-get v) 128)]
                      (if (< byte3 64)
                        (do (ref-set result (vector-push (ref-get result) byte3)) 0)
                        (do
                          (ref-set result (vector-push (ref-get result) (+ byte3 128)))
                          (do (ref-set result (vector-push (ref-get result) 0)) 0))))))))))
        (ref-get result)))))

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

;; バイト列に符号付き LEB128 値を追加
(defn emit-leb128-s [bytes value]
  (let [leb (leb128-s value)
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

;; i64 パラメータ型を param-count 個並べる
(defn append-i64-param-types [dst idx param-count]
  (if (>= idx param-count)
    dst
    (append-i64-param-types
      (emit-byte dst 126)
      (+ idx 1)
      param-count)))

;; 関数 metadata list から Type section body を構築する
(defn append-function-types [dst functions idx func-count]
  (if (>= idx func-count)
    dst
    (let [func-meta (vector-get functions idx)
          param-count (vector-get func-meta 0)
          body0 (emit-byte dst 96)
          body1 (emit-leb128 body0 param-count)
          body2 (append-i64-param-types body1 0 param-count)
          body3 (emit-byte body2 1)
          body4 (emit-byte body3 126)]
      (append-function-types body4 functions (+ idx 1) func-count))))

;; 関数 metadata list から Type section を生成する
(defn emit-type-section-functions [functions]
  (let [func-count (vector-length functions)
        body0 (emit-leb128 (vector-new 32) func-count)
        body1 (append-function-types body0 functions 0 func-count)
        body-size (vector-length body1)
        result0 (emit-byte (vector-new 32) 1)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body1 0 body-size)))

;; `env.__alloc : (i64) -> i64` と `main : () -> i64` を持つ narrow bootstrap 用 Type section
(defn emit-type-section-alloc-main []
  (let [body0 (emit-leb128 (vector-new 24) 2)
        ;; type 0: (i64) -> i64
        body1 (emit-byte body0 96)
        body2 (emit-leb128 body1 1)
        body3 (emit-byte body2 126)
        body4 (emit-byte body3 1)
        body5 (emit-byte body4 126)
        ;; type 1: () -> i64
        body6 (emit-byte body5 96)
        body7 (emit-leb128 body6 0)
        body8 (emit-byte body7 1)
        body9 (emit-byte body8 126)
        body-size (vector-length body9)
        result0 (emit-byte (vector-new 24) 1)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body9 0 body-size)))

;; `env.__alloc : (i64) -> i64`, `env.print : (i64) -> ()`, `main : () -> i64` を持つ narrow bootstrap 用 Type section
(defn emit-type-section-alloc-print-main []
  (let [body0 (emit-leb128 (vector-new 32) 3)
        ;; type 0: (i64) -> i64
        body1 (emit-byte body0 96)
        body2 (emit-leb128 body1 1)
        body3 (emit-byte body2 126)
        body4 (emit-byte body3 1)
        body5 (emit-byte body4 126)
        ;; type 1: (i64) -> ()
        body6 (emit-byte body5 96)
        body7 (emit-leb128 body6 1)
        body8 (emit-byte body7 126)
        body9 (emit-leb128 body8 0)
        ;; type 2: () -> i64
        body10 (emit-byte body9 96)
        body11 (emit-leb128 body10 0)
        body12 (emit-byte body11 1)
        body13 (emit-byte body12 126)
        body-size (vector-length body13)
        result0 (emit-byte (vector-new 32) 1)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body13 0 body-size)))

;; === Function セクション生成 ===

;; type index 0 を func-count 個並べる
(defn append-type-index-zeros [dst idx func-count]
  (if (>= idx func-count)
    dst
    (append-type-index-zeros
      (emit-byte dst 0)
      (+ idx 1)
      func-count)))

;; type index 0..n-1 を順に並べる
(defn append-type-index-sequence [dst idx func-count]
  (if (>= idx func-count)
    dst
    (append-type-index-sequence
      (emit-leb128 dst idx)
      (+ idx 1)
      func-count)))

;; Function セクション (ID=3): funcidx -> typeidx マッピング
;; すべて type index 0 を使う
(defn emit-function-section-count [func-count]
  (let [body0 (emit-leb128 (vector-new 16) func-count)
        body1 (append-type-index-zeros body0 0 func-count)
        body-size (vector-length body1)
        result0 (emit-byte (vector-new 16) 3)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body1 0 body-size)))

;; 簡易版: 1関数 (main) のみ、type index 0
(defn emit-function-section []
  (emit-function-section-count 1))

;; 1関数 (main) のみだが type index を明示指定する helper
(defn emit-function-section-main-type-index [type-idx]
  (let [body0 (emit-leb128 (vector-new 16) 1)
        body1 (emit-leb128 body0 type-idx)
        body-size (vector-length body1)
        result0 (emit-byte (vector-new 16) 3)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body1 0 body-size)))

;; 関数 metadata list に対し、type index 0..n-1 を割り当てる
(defn emit-function-section-functions [functions]
  (let [func-count (vector-length functions)
        body0 (emit-leb128 (vector-new 32) func-count)
        body1 (append-type-index-sequence body0 0 func-count)
        body-size (vector-length body1)
        result0 (emit-byte (vector-new 32) 3)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body1 0 body-size)))

;; === Export セクション生成 ===

;; Export セクション (ID=7): _start をエクスポート
(defn emit-export-section-main-index [func-idx]
  (let [body0 (emit-leb128 (vector-new 16) 1)
        body1 (emit-byte body0 6)
        body2 (emit-byte body1 95)
        body3 (emit-byte body2 115)
        body4 (emit-byte body3 116)
        body5 (emit-byte body4 97)
        body6 (emit-byte body5 114)
        body7 (emit-byte body6 116)
        body8 (emit-byte body7 0)
        body9 (emit-leb128 body8 func-idx)
        body-size (vector-length body9)
        result0 (emit-byte (vector-new 16) 7)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body9 0 body-size)))

(defn emit-export-section []
  (emit-export-section-main-index 0))

;; Export セクション: _start と memory をエクスポート
(defn emit-export-section-main-memory-index [func-idx mem-idx]
  (let [body0 (emit-leb128 (vector-new 24) 2)
        ;; export "_start" function
        body1 (emit-byte body0 6)
        body2 (emit-byte body1 95)
        body3 (emit-byte body2 115)
        body4 (emit-byte body3 116)
        body5 (emit-byte body4 97)
        body6 (emit-byte body5 114)
        body7 (emit-byte body6 116)
        body8 (emit-byte body7 0)
        body9 (emit-leb128 body8 func-idx)
        ;; export "memory" memory
        body10 (emit-byte body9 6)
        body11 (emit-byte body10 109)
        body12 (emit-byte body11 101)
        body13 (emit-byte body12 109)
        body14 (emit-byte body13 111)
        body15 (emit-byte body14 114)
        body16 (emit-byte body15 121)
        body17 (emit-byte body16 2)
        body18 (emit-leb128 body17 mem-idx)
        body-size (vector-length body18)
        result0 (emit-byte (vector-new 24) 7)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body18 0 body-size)))

;; === Memory セクション生成 ===

;; Memory セクション (ID=5): 1ページ (64KB) の linear memory
(defn emit-memory-section []
  (let [bytes (vector-new 8)]
    (let [b1 (emit-byte bytes 5)     ;; Section ID = 5 (Memory)
          b2 (emit-byte b1 3)        ;; セクションサイズ (3バイト)
          b3 (emit-byte b2 1)        ;; メモリ数 (1個)
          b4 (emit-byte b3 0)        ;; limits: no max (0x00)
          b5 (emit-byte b4 1)]       ;; initial pages (1)
      b5)))

;; === Import セクション生成 ===

;; Import セクション (ID=2): WASI fd_write をインポート
;; wasi_snapshot_preview1.fd_write : (i32, i32, i32, i32) -> i32
(defn emit-import-section []
  (let [bytes (vector-new 64)]
    (let [b1 (emit-byte bytes 2)     ;; Section ID = 2 (Import)
          b2 (emit-byte b1 36)       ;; セクションサイズ (36バイト)
          b3 (emit-byte b2 1)        ;; インポート数 (1個)
          ;; モジュール名 "wasi_snapshot_preview1" (21バイト)
          b4 (emit-byte b3 21)       ;; 名前長
          b5 (emit-byte b4 119)      ;; 'w'
          b6 (emit-byte b5 97)       ;; 'a'
          b7 (emit-byte b6 115)      ;; 's'
          b8 (emit-byte b7 105)      ;; 'i'
          b9 (emit-byte b8 95)       ;; '_'
          b10 (emit-byte b9 115)     ;; 's'
          b11 (emit-byte b10 110)    ;; 'n'
          b12 (emit-byte b11 97)     ;; 'a'
          b13 (emit-byte b12 112)    ;; 'p'
          b14 (emit-byte b13 115)    ;; 's'
          b15 (emit-byte b14 104)    ;; 'h'
          b16 (emit-byte b15 111)    ;; 'o'
          b17 (emit-byte b16 116)    ;; 't'
          b18 (emit-byte b17 95)     ;; '_'
          b19 (emit-byte b18 112)    ;; 'p'
          b20 (emit-byte b19 114)    ;; 'r'
          b21 (emit-byte b20 101)    ;; 'e'
          b22 (emit-byte b21 118)    ;; 'v'
          b23 (emit-byte b22 105)    ;; 'i'
          b24 (emit-byte b23 101)    ;; 'e'
          b25 (emit-byte b24 119)    ;; 'w'
          b26 (emit-byte b25 49)     ;; '1'
          ;; 関数名 "fd_write" (8バイト)
          b27 (emit-byte b26 8)      ;; 名前長
          b28 (emit-byte b27 102)    ;; 'f'
          b29 (emit-byte b28 100)    ;; 'd'
          b30 (emit-byte b29 95)     ;; '_'
          b31 (emit-byte b30 119)    ;; 'w'
          b32 (emit-byte b31 114)    ;; 'r'
          b33 (emit-byte b32 105)    ;; 'i'
          b34 (emit-byte b33 116)    ;; 't'
          b35 (emit-byte b34 101)    ;; 'e'
          ;; インポート種別: 関数 (0x00) + 型インデックス
          b36 (emit-byte b35 0)      ;; kind = function
          b37 (emit-byte b36 0)]     ;; type index 0 (暫定)
      b37)))

;; `env.__alloc : (i64) -> i64` を 1 件だけ import する narrow bootstrap 用 Import section
(defn emit-import-section-alloc []
  (let [body0 (emit-leb128 (vector-new 24) 1)
        ;; module "env"
        body1 (emit-leb128 body0 3)
        body2 (emit-byte body1 101)
        body3 (emit-byte body2 110)
        body4 (emit-byte body3 118)
        ;; field "__alloc"
        body5 (emit-leb128 body4 7)
        body6 (emit-byte body5 95)
        body7 (emit-byte body6 95)
        body8 (emit-byte body7 97)
        body9 (emit-byte body8 108)
        body10 (emit-byte body9 108)
        body11 (emit-byte body10 111)
        body12 (emit-byte body11 99)
        body13 (emit-byte body12 0)
        body14 (emit-leb128 body13 0)
        body-size (vector-length body14)
        result0 (emit-byte (vector-new 24) 2)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body14 0 body-size)))

;; `env.__alloc : (i64) -> i64` と `env.print : (i64) -> ()` を import する narrow bootstrap 用 Import section
(defn emit-import-section-alloc-print []
  (let [body0 (emit-leb128 (vector-new 32) 2)
        ;; import 0: env.__alloc (type 0)
        body1 (emit-leb128 body0 3)
        body2 (emit-byte body1 101)
        body3 (emit-byte body2 110)
        body4 (emit-byte body3 118)
        body5 (emit-leb128 body4 7)
        body6 (emit-byte body5 95)
        body7 (emit-byte body6 95)
        body8 (emit-byte body7 97)
        body9 (emit-byte body8 108)
        body10 (emit-byte body9 108)
        body11 (emit-byte body10 111)
        body12 (emit-byte body11 99)
        body13 (emit-byte body12 0)
        body14 (emit-leb128 body13 0)
        ;; import 1: env.print (type 1)
        body15 (emit-leb128 body14 3)
        body16 (emit-byte body15 101)
        body17 (emit-byte body16 110)
        body18 (emit-byte body17 118)
        body19 (emit-leb128 body18 5)
        body20 (emit-byte body19 112)
        body21 (emit-byte body20 114)
        body22 (emit-byte body21 105)
        body23 (emit-byte body22 110)
        body24 (emit-byte body23 116)
        body25 (emit-byte body24 0)
        body26 (emit-leb128 body25 1)
        body-size (vector-length body26)
        result0 (emit-byte (vector-new 32) 2)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body26 0 body-size)))

;; `env.__alloc`, `env.print`, `env.read-file` を import する narrow bootstrap 用 Import section
;; import index は alloc=0, print=1, read-file=2 を固定する
(defn emit-import-section-alloc-print-read []
  (let [body0 (emit-leb128 (vector-new 48) 3)
        ;; import 0: env.__alloc (type 0)
        body1 (emit-leb128 body0 3)
        body2 (emit-byte body1 101)
        body3 (emit-byte body2 110)
        body4 (emit-byte body3 118)
        body5 (emit-leb128 body4 7)
        body6 (emit-byte body5 95)
        body7 (emit-byte body6 95)
        body8 (emit-byte body7 97)
        body9 (emit-byte body8 108)
        body10 (emit-byte body9 108)
        body11 (emit-byte body10 111)
        body12 (emit-byte body11 99)
        body13 (emit-byte body12 0)
        body14 (emit-leb128 body13 0)
        ;; import 1: env.print (type 1)
        body15 (emit-leb128 body14 3)
        body16 (emit-byte body15 101)
        body17 (emit-byte body16 110)
        body18 (emit-byte body17 118)
        body19 (emit-leb128 body18 5)
        body20 (emit-byte body19 112)
        body21 (emit-byte body20 114)
        body22 (emit-byte body21 105)
        body23 (emit-byte body22 110)
        body24 (emit-byte body23 116)
        body25 (emit-byte body24 0)
        body26 (emit-leb128 body25 1)
        ;; import 2: env.read-file (type 0)
        body27 (emit-leb128 body26 3)
        body28 (emit-byte body27 101)
        body29 (emit-byte body28 110)
        body30 (emit-byte body29 118)
        body31 (emit-leb128 body30 9)
        body32 (emit-byte body31 114)
        body33 (emit-byte body32 101)
        body34 (emit-byte body33 97)
        body35 (emit-byte body34 100)
        body36 (emit-byte body35 45)
        body37 (emit-byte body36 102)
        body38 (emit-byte body37 105)
        body39 (emit-byte body38 108)
        body40 (emit-byte body39 101)
        body41 (emit-byte body40 0)
        body42 (emit-leb128 body41 0)
        body-size (vector-length body42)
        result0 (emit-byte (vector-new 48) 2)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body42 0 body-size)))

;; === Code セクション生成 ===

;; バイト列 Vector をすべて出力先へコピーする
(defn append-byte-vector [dst src idx count]
  (if (>= idx count)
    dst
    (append-byte-vector
      (emit-byte dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

;; IR 命令列をすべて Wasm bytes へ変換する
(defn append-ir-instrs [body ir-instrs idx count]
  (if (>= idx count)
    body
    (let [instr (vector-get ir-instrs idx)
          opcode (vector-get instr 0)
          operand (vector-get instr 1)]
      (append-ir-instrs
        (emit-ir-instr body opcode operand)
        ir-instrs
        (+ idx 1)
        count))))

;; 単一関数本体を構築する
(defn build-function-body [ir-instrs]
  (let [body0 (emit-byte (vector-new 64) 0)
        body1 (append-ir-instrs body0 ir-instrs 0 (vector-length ir-instrs))]
    (emit-byte body1 11)))

;; 関数 metadata から関数本体を構築する
(defn build-function-body-function [func-meta]
  (let [local-count (vector-get func-meta 1)
        ir-instrs (vector-get func-meta 2)
        body0 (if (= local-count 0)
                (emit-byte (vector-new 64) 0)
                (emit-byte
                  (emit-leb128
                    (emit-leb128 (vector-new 64) 1)
                    local-count)
                  126))
        body1 (append-ir-instrs body0 ir-instrs 0 (vector-length ir-instrs))]
    (emit-byte body1 11)))

;; 関数本体列を Code section body へ積む
(defn append-code-bodies [body ir-list idx func-count]
  (if (>= idx func-count)
    body
    (let [func-body (build-function-body (vector-get ir-list idx))
          with-size (emit-leb128 body (vector-length func-body))
          with-body (append-byte-vector with-size func-body 0 (vector-length func-body))]
      (append-code-bodies with-body ir-list (+ idx 1) func-count))))

;; IR 命令列のリストを Wasm Code セクションに変換
(defn emit-code-section-list [ir-list]
  (let [func-count (vector-length ir-list)
        body0 (emit-leb128 (vector-new 64) func-count)
        body1 (append-code-bodies body0 ir-list 0 func-count)
        body-size (vector-length body1)
        result0 (emit-byte (vector-new 64) 10)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body1 0 body-size)))

;; 関数 metadata list を Wasm Code セクションに変換
(defn append-code-bodies-functions [body functions idx func-count]
  (if (>= idx func-count)
    body
    (let [func-body (build-function-body-function (vector-get functions idx))
          with-size (emit-leb128 body (vector-length func-body))
          with-body (append-byte-vector with-size func-body 0 (vector-length func-body))]
      (append-code-bodies-functions with-body functions (+ idx 1) func-count))))

(defn emit-code-section-functions [functions]
  (let [func-count (vector-length functions)
        body0 (emit-leb128 (vector-new 64) func-count)
        body1 (append-code-bodies-functions body0 functions 0 func-count)
        body-size (vector-length body1)
        result0 (emit-byte (vector-new 64) 10)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body1 0 body-size)))

;; IR 命令列を Wasm Code セクションに変換
;; 簡易版: 1関数のみ、ローカル変数なし
(defn emit-code-section [ir-instrs]
  (emit-code-section-list (vector-push (vector-new 2) ir-instrs)))

;; Main.ls 用: header + type + code の合計バイト長 (簡易モジュール)
(defn emit-wasm [ir-instrs]
  (let [h (emit-header)
        t (emit-type-section-main)
        c (emit-code-section ir-instrs)]
    (+ (+ (vector-length h) (vector-length t)) (vector-length c))))

;; タグ付きポインタ用の最上位ビット定数 (-2^63) を追加する
(defn emit-tagged-pointer-high-bit [bytes]
  (let [b1 (emit-byte bytes 66)
        b2 (emit-byte b1 128)
        b3 (emit-byte b2 128)
        b4 (emit-byte b3 128)
        b5 (emit-byte b4 128)
        b6 (emit-byte b5 128)
        b7 (emit-byte b6 128)
        b8 (emit-byte b7 128)
        b9 (emit-byte b8 128)
        b10 (emit-byte b9 128)
        b11 (emit-byte b10 127)]
    b11))

;; print: [value:i64] -> Unit(i64=0)
;; narrow bootstrap slice では import index 1 に env.print を置く
(defn emit-print-instr [bytes]
  (let [b1 (emit-leb128 (emit-byte bytes 16) 1)]
    (emit-leb128-s (emit-byte b1 66) 0)))

;; read-file: [path:i64] -> content:i64
;; narrow bootstrap slice では import index 2 に env.read-file を置く
(defn emit-read-file-instr [bytes]
  (emit-leb128 (emit-byte bytes 16) 2))

;; vector-push: [Vector ptr:i64, value:i64] -> Vector ptr:i64
;; narrow bootstrap slice では __alloc import index 0 を使う
(defn emit-vector-push-instr [bytes operand]
  (let [tagged-idx (- operand 1)
        val-idx operand
        len-idx (+ operand 1)
        cap-idx (+ operand 2)
        newcap-idx (+ operand 3)
        newaddr-idx (+ operand 4)
        ;; stack top = value, next = tagged vector
        b1 (emit-leb128 (emit-byte bytes 33) val-idx)
        b2 (emit-leb128 (emit-byte b1 33) tagged-idx)
        ;; len = load addr[8]
        b3 (emit-leb128 (emit-byte b2 32) tagged-idx)
        b4 (emit-byte b3 167)
        b5 (emit-byte b4 40)
        b6 (emit-byte b5 0)
        b7 (emit-byte b6 8)
        b8 (emit-byte b7 173)
        b9 (emit-leb128 (emit-byte b8 33) len-idx)
        ;; cap = load addr[4]
        b10 (emit-leb128 (emit-byte b9 32) tagged-idx)
        b11 (emit-byte b10 167)
        b12 (emit-byte b11 40)
        b13 (emit-byte b12 0)
        b14 (emit-byte b13 4)
        b15 (emit-byte b14 173)
        b16 (emit-leb128 (emit-byte b15 33) cap-idx)
        ;; if (len >= cap) -> i64
        b17 (emit-leb128 (emit-byte b16 32) len-idx)
        b18 (emit-leb128 (emit-byte b17 32) cap-idx)
        b19 (emit-byte b18 89)
        b20 (emit-byte b19 4)
        b21 (emit-byte b20 126)
        ;; newcap = cap * 2
        b22 (emit-leb128 (emit-byte b21 32) cap-idx)
        b23 (emit-leb128-s (emit-byte b22 66) 2)
        b24 (emit-byte b23 126)
        b25 (emit-leb128 (emit-byte b24 33) newcap-idx)
        ;; newcap = max(newcap, 4)
        b26 (emit-leb128 (emit-byte b25 32) newcap-idx)
        b27 (emit-leb128-s (emit-byte b26 66) 4)
        b28 (emit-byte b27 85)
        b29 (emit-byte b28 4)
        b30 (emit-byte b29 126)
        b31 (emit-leb128 (emit-byte b30 32) newcap-idx)
        b32 (emit-byte b31 5)
        b33 (emit-leb128-s (emit-byte b32 66) 4)
        b34 (emit-byte b33 11)
        b35 (emit-leb128 (emit-byte b34 33) newcap-idx)
        ;; alloc_size = 16 + newcap * 8
        b36 (emit-leb128-s (emit-byte b35 66) 16)
        b37 (emit-leb128 (emit-byte b36 32) newcap-idx)
        b38 (emit-leb128-s (emit-byte b37 66) 8)
        b39 (emit-byte b38 126)
        b40 (emit-byte b39 124)
        b41 (emit-leb128 (emit-byte b40 16) 0)
        b42 (emit-leb128 (emit-byte b41 33) newaddr-idx)
        ;; header.tag = 5
        b43 (emit-leb128 (emit-byte b42 32) newaddr-idx)
        b44 (emit-byte b43 167)
        b45 (emit-leb128 (emit-byte b44 65) 5)
        b46 (emit-byte b45 54)
        b47 (emit-byte b46 0)
        b48 (emit-byte b47 0)
        ;; header.capacity = newcap
        b49 (emit-leb128 (emit-byte b48 32) newaddr-idx)
        b50 (emit-byte b49 167)
        b51 (emit-leb128 (emit-byte b50 32) newcap-idx)
        b52 (emit-byte b51 167)
        b53 (emit-byte b52 54)
        b54 (emit-byte b53 0)
        b55 (emit-byte b54 4)
        ;; header.length = len + 1
        b56 (emit-leb128 (emit-byte b55 32) newaddr-idx)
        b57 (emit-byte b56 167)
        b58 (emit-leb128 (emit-byte b57 32) len-idx)
        b59 (emit-byte b58 167)
        b60 (emit-leb128 (emit-byte b59 65) 1)
        b61 (emit-byte b60 106)
        b62 (emit-byte b61 54)
        b63 (emit-byte b62 0)
        b64 (emit-byte b63 8)
        ;; header.padding = 0
        b65 (emit-leb128 (emit-byte b64 32) newaddr-idx)
        b66 (emit-byte b65 167)
        b67 (emit-leb128 (emit-byte b66 65) 0)
        b68 (emit-byte b67 54)
        b69 (emit-byte b68 0)
        b70 (emit-byte b69 12)
        ;; memory.copy(new_addr + 16, old_addr + 16, len * 8)
        b71 (emit-leb128 (emit-byte b70 32) newaddr-idx)
        b72 (emit-byte b71 167)
        b73 (emit-leb128 (emit-byte b72 65) 16)
        b74 (emit-byte b73 106)
        b75 (emit-leb128 (emit-byte b74 32) tagged-idx)
        b76 (emit-byte b75 167)
        b77 (emit-leb128 (emit-byte b76 65) 16)
        b78 (emit-byte b77 106)
        b79 (emit-leb128 (emit-byte b78 32) len-idx)
        b80 (emit-byte b79 167)
        b81 (emit-leb128 (emit-byte b80 65) 8)
        b82 (emit-byte b81 108)
        b83 (emit-byte b82 252)
        b84 (emit-byte b83 10)
        b85 (emit-byte b84 0)
        b86 (emit-byte b85 0)
        ;; mem[new_addr + 16 + len * 8] = value
        b87 (emit-leb128 (emit-byte b86 32) newaddr-idx)
        b88 (emit-byte b87 167)
        b89 (emit-leb128 (emit-byte b88 32) len-idx)
        b90 (emit-byte b89 167)
        b91 (emit-leb128 (emit-byte b90 65) 8)
        b92 (emit-byte b91 108)
        b93 (emit-leb128 (emit-byte b92 65) 16)
        b94 (emit-byte b93 106)
        b95 (emit-byte b94 106)
        b96 (emit-leb128 (emit-byte b95 32) val-idx)
        b97 (emit-byte b96 55)
        b98 (emit-byte b97 0)
        b99 (emit-byte b98 0)
        ;; return tagged new addr
        b100 (emit-leb128 (emit-byte b99 32) newaddr-idx)
        b101 (emit-tagged-pointer-high-bit b100)
        b102 (emit-byte b101 124)
        ;; else
        b103 (emit-byte b102 5)
        ;; mem[tagged + 16 + len * 8] = value
        b104 (emit-leb128 (emit-byte b103 32) tagged-idx)
        b105 (emit-byte b104 167)
        b106 (emit-leb128 (emit-byte b105 32) len-idx)
        b107 (emit-byte b106 167)
        b108 (emit-leb128 (emit-byte b107 65) 8)
        b109 (emit-byte b108 108)
        b110 (emit-leb128 (emit-byte b109 65) 16)
        b111 (emit-byte b110 106)
        b112 (emit-byte b111 106)
        b113 (emit-leb128 (emit-byte b112 32) val-idx)
        b114 (emit-byte b113 55)
        b115 (emit-byte b114 0)
        b116 (emit-byte b115 0)
        ;; mem[tagged + 8] = len + 1
        b117 (emit-leb128 (emit-byte b116 32) tagged-idx)
        b118 (emit-byte b117 167)
        b119 (emit-leb128 (emit-byte b118 32) len-idx)
        b120 (emit-byte b119 167)
        b121 (emit-leb128 (emit-byte b120 65) 1)
        b122 (emit-byte b121 106)
        b123 (emit-byte b122 54)
        b124 (emit-byte b123 0)
        b125 (emit-byte b124 8)
        ;; return same tagged ptr
        b126 (emit-leb128 (emit-byte b125 32) tagged-idx)
        b127 (emit-byte b126 11)]
    b127))

;; ref-new: [value:i64] -> tagged ref ptr:i64
(defn emit-ref-new-instr [bytes operand]
  (let [val-idx (- operand 1)
        addr-idx operand
        b1 (emit-leb128 (emit-byte bytes 33) val-idx)
        b2 (emit-leb128-s (emit-byte b1 66) 16)
        b3 (emit-leb128 (emit-byte b2 16) 0)
        b4 (emit-leb128 (emit-byte b3 33) addr-idx)
        ;; tag = 7
        b5 (emit-leb128 (emit-byte b4 32) addr-idx)
        b6 (emit-byte b5 167)
        b7 (emit-leb128 (emit-byte b6 65) 7)
        b8 (emit-byte b7 54)
        b9 (emit-byte b8 0)
        b10 (emit-byte b9 0)
        ;; size = 16
        b11 (emit-leb128 (emit-byte b10 32) addr-idx)
        b12 (emit-byte b11 167)
        b13 (emit-leb128 (emit-byte b12 65) 16)
        b14 (emit-byte b13 54)
        b15 (emit-byte b14 0)
        b16 (emit-byte b15 4)
        ;; value
        b17 (emit-leb128 (emit-byte b16 32) addr-idx)
        b18 (emit-byte b17 167)
        b19 (emit-leb128 (emit-byte b18 32) val-idx)
        b20 (emit-byte b19 55)
        b21 (emit-byte b20 0)
        b22 (emit-byte b21 8)
        ;; return tagged ptr
        b23 (emit-leb128 (emit-byte b22 32) addr-idx)
        b24 (emit-tagged-pointer-high-bit b23)]
    (emit-byte b24 124)))

;; ref-set: [tagged ref ptr:i64, value:i64] -> Unit(i64=0)
(defn emit-ref-set-instr [bytes operand]
  (let [val-idx (- operand 1)
        b1 (emit-leb128 (emit-byte bytes 33) val-idx)
        b2 (emit-byte b1 167)
        b3 (emit-leb128 (emit-byte b2 32) val-idx)
        b4 (emit-byte b3 55)
        b5 (emit-byte b4 0)
        b6 (emit-byte b5 8)]
    (emit-leb128-s (emit-byte b6 66) 0)))

(defn emit-block-empty [bytes]
  (emit-byte (emit-byte bytes 2) 64))

(defn emit-loop-empty [bytes]
  (emit-byte (emit-byte bytes 3) 64))

(defn emit-br [bytes depth]
  (emit-leb128 (emit-byte bytes 12) depth))

(defn emit-br-if [bytes depth]
  (emit-leb128 (emit-byte bytes 13) depth))

(defn emit-memory-fill [bytes]
  (let [b1 (emit-byte bytes 252)
        b2 (emit-byte b1 11)
        b3 (emit-byte b2 0)]
    b3))

;; map-new: [] -> tagged map ptr:i64
(defn emit-map-new-instr [bytes operand]
  (let [addr-idx (- operand 1)
        b1 (emit-leb128-s (emit-byte bytes 66) 272)
        b2 (emit-leb128 (emit-byte b1 16) 0)
        b3 (emit-leb128 (emit-byte b2 33) addr-idx)
        ;; tag = 6
        b4 (emit-leb128 (emit-byte b3 32) addr-idx)
        b5 (emit-byte b4 167)
        b6 (emit-leb128 (emit-byte b5 65) 6)
        b7 (emit-byte b6 54)
        b8 (emit-byte b7 0)
        b9 (emit-byte b8 0)
        ;; capacity = 16
        b10 (emit-leb128 (emit-byte b9 32) addr-idx)
        b11 (emit-byte b10 167)
        b12 (emit-leb128 (emit-byte b11 65) 16)
        b13 (emit-byte b12 54)
        b14 (emit-byte b13 0)
        b15 (emit-byte b14 4)
        ;; size = 0
        b16 (emit-leb128 (emit-byte b15 32) addr-idx)
        b17 (emit-byte b16 167)
        b18 (emit-leb128 (emit-byte b17 65) 0)
        b19 (emit-byte b18 54)
        b20 (emit-byte b19 0)
        b21 (emit-byte b20 8)
        ;; zero fill entry region
        b22 (emit-leb128 (emit-byte b21 32) addr-idx)
        b23 (emit-byte b22 167)
        b24 (emit-leb128 (emit-byte b23 65) 16)
        b25 (emit-byte b24 106)
        b26 (emit-leb128 (emit-byte b25 65) 0)
        b27 (emit-leb128 (emit-byte b26 65) 256)
        b28 (emit-memory-fill b27)
        ;; return tagged ptr
        b29 (emit-leb128 (emit-byte b28 32) addr-idx)
        b30 (emit-tagged-pointer-high-bit b29)]
    (emit-byte b30 124)))

;; map-insert: [tagged map ptr:i64, key:i64, value:i64] -> tagged map ptr:i64
(defn emit-map-insert-instr [bytes operand]
  (let [tagged-idx (- operand 1)
        key-idx operand
        val-idx (+ operand 1)
        cap-idx (+ operand 2)
        i-idx (+ operand 3)
        ea-idx (+ operand 4)
        b1 (emit-leb128 (emit-byte bytes 33) val-idx)
        b2 (emit-leb128 (emit-byte b1 33) key-idx)
        b3 (emit-leb128 (emit-byte b2 33) tagged-idx)
        ;; cap = load tagged[4]
        b4 (emit-leb128 (emit-byte b3 32) tagged-idx)
        b5 (emit-byte b4 167)
        b6 (emit-byte b5 40)
        b7 (emit-byte b6 0)
        b8 (emit-byte b7 4)
        b9 (emit-byte b8 173)
        b10 (emit-leb128 (emit-byte b9 33) cap-idx)
        ;; i = 0
        b11 (emit-leb128-s (emit-byte b10 66) 0)
        b12 (emit-leb128 (emit-byte b11 33) i-idx)
        b13 (emit-block-empty b12)
        b14 (emit-loop-empty b13)
        ;; if i >= cap break
        b15 (emit-leb128 (emit-byte b14 32) i-idx)
        b16 (emit-leb128 (emit-byte b15 32) cap-idx)
        b17 (emit-byte b16 89)
        b18 (emit-br-if b17 1)
        ;; ea = untag(tagged) + 16 + i * 16
        b19 (emit-leb128 (emit-byte b18 32) tagged-idx)
        b20 (emit-byte b19 167)
        b21 (emit-byte b20 173)
        b22 (emit-leb128-s (emit-byte b21 66) 16)
        b23 (emit-byte b22 124)
        b24 (emit-leb128 (emit-byte b23 32) i-idx)
        b25 (emit-leb128-s (emit-byte b24 66) 16)
        b26 (emit-byte b25 126)
        b27 (emit-byte b26 124)
        b28 (emit-leb128 (emit-byte b27 33) ea-idx)
        ;; if entry key == 0
        b29 (emit-leb128 (emit-byte b28 32) ea-idx)
        b30 (emit-byte b29 167)
        b31 (emit-byte b30 41)
        b32 (emit-byte b31 0)
        b33 (emit-byte b32 0)
        b34 (emit-leb128-s (emit-byte b33 66) 0)
        b35 (emit-byte b34 81)
        b36 (emit-byte (emit-byte b35 4) 64)
        ;; store key
        b37 (emit-leb128 (emit-byte b36 32) ea-idx)
        b38 (emit-byte b37 167)
        b39 (emit-leb128 (emit-byte b38 32) key-idx)
        b40 (emit-byte b39 55)
        b41 (emit-byte b40 0)
        b42 (emit-byte b41 0)
        ;; store value
        b43 (emit-leb128 (emit-byte b42 32) ea-idx)
        b44 (emit-byte b43 167)
        b45 (emit-leb128 (emit-byte b44 32) val-idx)
        b46 (emit-byte b45 55)
        b47 (emit-byte b46 0)
        b48 (emit-byte b47 8)
        ;; size++
        b49 (emit-leb128 (emit-byte b48 32) tagged-idx)
        b50 (emit-byte b49 167)
        b51 (emit-leb128 (emit-byte b50 32) tagged-idx)
        b52 (emit-byte b51 167)
        b53 (emit-byte b52 40)
        b54 (emit-byte b53 0)
        b55 (emit-byte b54 8)
        b56 (emit-leb128 (emit-byte b55 65) 1)
        b57 (emit-byte b56 106)
        b58 (emit-byte b57 54)
        b59 (emit-byte b58 0)
        b60 (emit-byte b59 8)
        b61 (emit-br b60 2)
        b62 (emit-byte b61 11)
        ;; if entry key == key
        b63 (emit-leb128 (emit-byte b62 32) ea-idx)
        b64 (emit-byte b63 167)
        b65 (emit-byte b64 41)
        b66 (emit-byte b65 0)
        b67 (emit-byte b66 0)
        b68 (emit-leb128 (emit-byte b67 32) key-idx)
        b69 (emit-byte b68 81)
        b70 (emit-byte (emit-byte b69 4) 64)
        ;; overwrite value
        b71 (emit-leb128 (emit-byte b70 32) ea-idx)
        b72 (emit-byte b71 167)
        b73 (emit-leb128 (emit-byte b72 32) val-idx)
        b74 (emit-byte b73 55)
        b75 (emit-byte b74 0)
        b76 (emit-byte b75 8)
        b77 (emit-br b76 2)
        b78 (emit-byte b77 11)
        ;; i++
        b79 (emit-leb128 (emit-byte b78 32) i-idx)
        b80 (emit-leb128-s (emit-byte b79 66) 1)
        b81 (emit-byte b80 124)
        b82 (emit-leb128 (emit-byte b81 33) i-idx)
        b83 (emit-br b82 0)
        b84 (emit-byte b83 11)
        b85 (emit-byte b84 11)
        b86 (emit-leb128 (emit-byte b85 32) tagged-idx)]
    b86))

;; map-get: [tagged map ptr:i64, key:i64] -> value:i64
(defn emit-map-get-instr [bytes operand]
  (let [tagged-idx (- operand 1)
        key-idx operand
        cap-idx (+ operand 1)
        result-idx (+ operand 2)
        i-idx (+ operand 3)
        ea-idx (+ operand 4)
        b1 (emit-leb128 (emit-byte bytes 33) key-idx)
        b2 (emit-leb128 (emit-byte b1 33) tagged-idx)
        ;; cap
        b3 (emit-leb128 (emit-byte b2 32) tagged-idx)
        b4 (emit-byte b3 167)
        b5 (emit-byte b4 40)
        b6 (emit-byte b5 0)
        b7 (emit-byte b6 4)
        b8 (emit-byte b7 173)
        b9 (emit-leb128 (emit-byte b8 33) cap-idx)
        ;; result = 0
        b10 (emit-leb128-s (emit-byte b9 66) 0)
        b11 (emit-leb128 (emit-byte b10 33) result-idx)
        ;; i = 0
        b12 (emit-leb128-s (emit-byte b11 66) 0)
        b13 (emit-leb128 (emit-byte b12 33) i-idx)
        b14 (emit-block-empty b13)
        b15 (emit-loop-empty b14)
        ;; if i >= cap break
        b16 (emit-leb128 (emit-byte b15 32) i-idx)
        b17 (emit-leb128 (emit-byte b16 32) cap-idx)
        b18 (emit-byte b17 89)
        b19 (emit-br-if b18 1)
        ;; ea = untag(tagged) + 16 + i * 16
        b20 (emit-leb128 (emit-byte b19 32) tagged-idx)
        b21 (emit-byte b20 167)
        b22 (emit-byte b21 173)
        b23 (emit-leb128-s (emit-byte b22 66) 16)
        b24 (emit-byte b23 124)
        b25 (emit-leb128 (emit-byte b24 32) i-idx)
        b26 (emit-leb128-s (emit-byte b25 66) 16)
        b27 (emit-byte b26 126)
        b28 (emit-byte b27 124)
        b29 (emit-leb128 (emit-byte b28 33) ea-idx)
        ;; if entry key == key
        b30 (emit-leb128 (emit-byte b29 32) ea-idx)
        b31 (emit-byte b30 167)
        b32 (emit-byte b31 41)
        b33 (emit-byte b32 0)
        b34 (emit-byte b33 0)
        b35 (emit-leb128 (emit-byte b34 32) key-idx)
        b36 (emit-byte b35 81)
        b37 (emit-byte (emit-byte b36 4) 64)
        b38 (emit-leb128 (emit-byte b37 32) ea-idx)
        b39 (emit-byte b38 167)
        b40 (emit-byte b39 41)
        b41 (emit-byte b40 0)
        b42 (emit-byte b41 8)
        b43 (emit-leb128 (emit-byte b42 33) result-idx)
        b44 (emit-br b43 2)
        b45 (emit-byte b44 11)
        ;; i++
        b46 (emit-leb128 (emit-byte b45 32) i-idx)
        b47 (emit-leb128-s (emit-byte b46 66) 1)
        b48 (emit-byte b47 124)
        b49 (emit-leb128 (emit-byte b48 33) i-idx)
        b50 (emit-br b49 0)
        b51 (emit-byte b50 11)
        b52 (emit-byte b51 11)
        b53 (emit-leb128 (emit-byte b52 32) result-idx)]
    b53))

;; map-contains?: [tagged map ptr:i64, key:i64] -> 1/0
(defn emit-map-contains-instr [bytes operand]
  (let [tagged-idx (- operand 1)
        key-idx operand
        cap-idx (+ operand 1)
        result-idx (+ operand 2)
        i-idx (+ operand 3)
        ea-idx (+ operand 4)
        b1 (emit-leb128 (emit-byte bytes 33) key-idx)
        b2 (emit-leb128 (emit-byte b1 33) tagged-idx)
        b3 (emit-leb128 (emit-byte b2 32) tagged-idx)
        b4 (emit-byte b3 167)
        b5 (emit-byte b4 40)
        b6 (emit-byte b5 0)
        b7 (emit-byte b6 4)
        b8 (emit-byte b7 173)
        b9 (emit-leb128 (emit-byte b8 33) cap-idx)
        b10 (emit-leb128-s (emit-byte b9 66) 0)
        b11 (emit-leb128 (emit-byte b10 33) result-idx)
        b12 (emit-leb128-s (emit-byte b11 66) 0)
        b13 (emit-leb128 (emit-byte b12 33) i-idx)
        b14 (emit-block-empty b13)
        b15 (emit-loop-empty b14)
        b16 (emit-leb128 (emit-byte b15 32) i-idx)
        b17 (emit-leb128 (emit-byte b16 32) cap-idx)
        b18 (emit-byte b17 89)
        b19 (emit-br-if b18 1)
        b20 (emit-leb128 (emit-byte b19 32) tagged-idx)
        b21 (emit-byte b20 167)
        b22 (emit-byte b21 173)
        b23 (emit-leb128-s (emit-byte b22 66) 16)
        b24 (emit-byte b23 124)
        b25 (emit-leb128 (emit-byte b24 32) i-idx)
        b26 (emit-leb128-s (emit-byte b25 66) 16)
        b27 (emit-byte b26 126)
        b28 (emit-byte b27 124)
        b29 (emit-leb128 (emit-byte b28 33) ea-idx)
        b30 (emit-leb128 (emit-byte b29 32) ea-idx)
        b31 (emit-byte b30 167)
        b32 (emit-byte b31 41)
        b33 (emit-byte b32 0)
        b34 (emit-byte b33 0)
        b35 (emit-leb128 (emit-byte b34 32) key-idx)
        b36 (emit-byte b35 81)
        b37 (emit-byte (emit-byte b36 4) 64)
        b38 (emit-leb128-s (emit-byte b37 66) 1)
        b39 (emit-leb128 (emit-byte b38 33) result-idx)
        b40 (emit-br b39 2)
        b41 (emit-byte b40 11)
        b42 (emit-leb128 (emit-byte b41 32) i-idx)
        b43 (emit-leb128-s (emit-byte b42 66) 1)
        b44 (emit-byte b43 124)
        b45 (emit-leb128 (emit-byte b44 33) i-idx)
        b46 (emit-br b45 0)
        b47 (emit-byte b46 11)
        b48 (emit-byte b47 11)
        b49 (emit-leb128 (emit-byte b48 32) result-idx)]
    b49))

;; map-remove: [tagged map ptr:i64, key:i64] -> tagged map ptr:i64
(defn emit-map-remove-instr [bytes operand]
  (let [tagged-idx (- operand 1)
        key-idx operand
        cap-idx (+ operand 1)
        i-idx (+ operand 2)
        ea-idx (+ operand 3)
        ek-idx (+ operand 4)
        b1 (emit-leb128 (emit-byte bytes 33) key-idx)
        b2 (emit-leb128 (emit-byte b1 33) tagged-idx)
        b3 (emit-leb128 (emit-byte b2 32) tagged-idx)
        b4 (emit-byte b3 167)
        b5 (emit-byte b4 40)
        b6 (emit-byte b5 0)
        b7 (emit-byte b6 4)
        b8 (emit-byte b7 173)
        b9 (emit-leb128 (emit-byte b8 33) cap-idx)
        b10 (emit-leb128-s (emit-byte b9 66) 0)
        b11 (emit-leb128 (emit-byte b10 33) i-idx)
        b12 (emit-block-empty b11)
        b13 (emit-loop-empty b12)
        b14 (emit-leb128 (emit-byte b13 32) i-idx)
        b15 (emit-leb128 (emit-byte b14 32) cap-idx)
        b16 (emit-byte b15 89)
        b17 (emit-br-if b16 1)
        b18 (emit-leb128 (emit-byte b17 32) tagged-idx)
        b19 (emit-byte b18 167)
        b20 (emit-byte b19 173)
        b21 (emit-leb128-s (emit-byte b20 66) 16)
        b22 (emit-byte b21 124)
        b23 (emit-leb128 (emit-byte b22 32) i-idx)
        b24 (emit-leb128-s (emit-byte b23 66) 16)
        b25 (emit-byte b24 126)
        b26 (emit-byte b25 124)
        b27 (emit-leb128 (emit-byte b26 33) ea-idx)
        b28 (emit-leb128 (emit-byte b27 32) ea-idx)
        b29 (emit-byte b28 167)
        b30 (emit-byte b29 41)
        b31 (emit-byte b30 0)
        b32 (emit-byte b31 0)
        b33 (emit-leb128 (emit-byte b32 33) ek-idx)
        b34 (emit-leb128 (emit-byte b33 32) ek-idx)
        b35 (emit-leb128 (emit-byte b34 32) key-idx)
        b36 (emit-byte b35 81)
        b37 (emit-byte (emit-byte b36 4) 64)
        b38 (emit-leb128 (emit-byte b37 32) ea-idx)
        b39 (emit-byte b38 167)
        b40 (emit-leb128-s (emit-byte b39 66) 0)
        b41 (emit-byte b40 55)
        b42 (emit-byte b41 0)
        b43 (emit-byte b42 0)
        b44 (emit-leb128 (emit-byte b43 32) ea-idx)
        b45 (emit-byte b44 167)
        b46 (emit-leb128-s (emit-byte b45 66) 0)
        b47 (emit-byte b46 55)
        b48 (emit-byte b47 0)
        b49 (emit-byte b48 8)
        b50 (emit-leb128 (emit-byte b49 32) tagged-idx)
        b51 (emit-byte b50 167)
        b52 (emit-leb128 (emit-byte b51 32) tagged-idx)
        b53 (emit-byte b52 167)
        b54 (emit-byte b53 40)
        b55 (emit-byte b54 0)
        b56 (emit-byte b55 8)
        b57 (emit-leb128 (emit-byte b56 65) 1)
        b58 (emit-byte b57 107)
        b59 (emit-byte b58 54)
        b60 (emit-byte b59 0)
        b61 (emit-byte b60 8)
        b62 (emit-br b61 2)
        b63 (emit-byte b62 11)
        b64 (emit-leb128 (emit-byte b63 32) i-idx)
        b65 (emit-leb128-s (emit-byte b64 66) 1)
        b66 (emit-byte b65 124)
        b67 (emit-leb128 (emit-byte b66 33) i-idx)
        b68 (emit-br b67 0)
        b69 (emit-byte b68 11)
        b70 (emit-byte b69 11)
        b71 (emit-leb128 (emit-byte b70 32) tagged-idx)]
    b71))

;; IR opcode を Wasm opcode に変換して bytes に追加
;; T3-6: ビルトインヘルパー -- 比較演算子 (i64.gt_s, i64.lt_s, i64.ge_s, i64.le_s) 追加
(defn emit-ir-instr [bytes opcode operand]
  (if (= opcode 1)
    ;; i64.const (符号付き LEB128 を使用)
    (emit-leb128-s (emit-byte bytes 66) operand)
    (if (= opcode 10)
      ;; local.get
      (emit-leb128 (emit-byte bytes 32) (- operand 1))
      (if (= opcode 11)
        ;; local.set
        (emit-leb128 (emit-byte bytes 33) (- operand 1))
        (if (= opcode 20)
          ;; i64.add
          (emit-byte bytes 124)
          (if (= opcode 21)
            ;; i64.sub
            (emit-byte bytes 125)
            (if (= opcode 22)
              ;; i64.mul
              (emit-byte bytes 126)
              (if (= opcode 23)
                ;; i64.div_s
                (emit-byte bytes 127)
                (if (= opcode 30)
                  ;; i64.eq (0x51)
                              (emit-byte bytes 81)
                              (if (= opcode 31)
                                ;; T3-6: i64.gt_s (0x55)
                                (emit-byte bytes 85)
                                (if (= opcode 32)
                      ;; T3-6: i64.lt_s (0x53)
                      (emit-byte bytes 83)
                      (if (= opcode 33)
                        ;; T3-6: i64.ge_s (0x59)
                        (emit-byte bytes 89)
                        (if (= opcode 34)
                          ;; T3-6: i64.le_s (0x57)
                          (emit-byte bytes 87)
                          (if (= opcode 40)
                            ;; call
                                      (emit-leb128 (emit-byte bytes 16) operand)
                                      (if (= opcode 41)
                                        ;; if (i64 -> i32 変換が必要だが簡易版では省略)
                                        (emit-byte (emit-byte bytes 4) 64)  ;; if + void block type
                                        (if (= opcode 43)
                                          ;; end
                                          (emit-byte bytes 11)
                                          (if (= opcode 44)
                                            ;; drop
                                            (emit-byte bytes 26)
                                            (if (= opcode 50)
                                              ;; string-char-at: [String ptr:i64, index:i64] -> char code:i64
                                            (let [temp-idx (- operand 1)
                                                  b1 (emit-leb128 (emit-byte bytes 33) temp-idx)
                                                  b2 (emit-byte b1 167)
                                                  b3 (emit-leb128 (emit-byte b2 65) 8)
                                                  b4 (emit-byte b3 106)
                                                  b5 (emit-leb128 (emit-byte b4 32) temp-idx)
                                                  b6 (emit-byte b5 167)
                                                  b7 (emit-byte b6 106)
                                                  b8 (emit-byte b7 45)
                                                  b9 (emit-byte b8 0)
                                                  b10 (emit-byte b9 0)]
                                                (emit-byte b10 173))
                                            (if (= opcode 51)
                                              ;; string-length: [String ptr:i64] -> len:i64
                                              (let [b1 (emit-byte bytes 167)
                                                    b2 (emit-byte b1 40)
                                                    b3 (emit-byte b2 0)
                                                    b4 (emit-byte b3 4)]
                                                (emit-byte b4 173))
                                            (if (= opcode 52)
                                              ;; vector-length: [Vector ptr:i64] -> len:i64
                                              (let [b1 (emit-byte bytes 167)
                                                    b2 (emit-byte b1 40)
                                                    b3 (emit-byte b2 0)
                                                    b4 (emit-byte b3 8)]
                                                (emit-byte b4 173))
                                            (if (= opcode 53)
                                              ;; vector-get: [Vector ptr:i64, index:i64] -> elem:i64
                                              (let [temp-idx (- operand 1)
                                                    b1 (emit-leb128 (emit-byte bytes 33) temp-idx)
                                                    b2 (emit-byte b1 167)
                                                    b3 (emit-leb128 (emit-byte b2 32) temp-idx)
                                                    b4 (emit-byte b3 167)
                                                    b5 (emit-leb128 (emit-byte b4 65) 8)
                                                    b6 (emit-byte b5 108)
                                                    b7 (emit-leb128 (emit-byte b6 65) 16)
                                                    b8 (emit-byte b7 106)
                                                    b9 (emit-byte b8 106)
                                                    b10 (emit-byte b9 41)
                                                    b11 (emit-byte b10 0)
                                                    b12 (emit-byte b11 0)]
                                                b12)
                                            (if (= opcode 54)
                                              ;; vector-new: [capacity:i64] -> tagged vector ptr:i64
                                              (let [cap-idx (- operand 1)
                                                    addr-idx operand
                                                    ;; cap を temp local に保存
                                                    b1 (emit-leb128 (emit-byte bytes 33) cap-idx)
                                                    ;; 16 + cap * 8 を __alloc(0) へ渡す
                                                    b2 (emit-leb128-s (emit-byte b1 66) 16)
                                                    b3 (emit-leb128 (emit-byte b2 32) cap-idx)
                                                    b4 (emit-leb128-s (emit-byte b3 66) 8)
                                                    b5 (emit-byte b4 126)
                                                    b6 (emit-byte b5 124)
                                                    b7 (emit-leb128 (emit-byte b6 16) 0)
                                                    b8 (emit-leb128 (emit-byte b7 33) addr-idx)
                                                    ;; header.tag = 5
                                                    b9 (emit-leb128 (emit-byte b8 32) addr-idx)
                                                    b10 (emit-byte b9 167)
                                                    b11 (emit-leb128 (emit-byte b10 65) 5)
                                                    b12 (emit-byte b11 54)
                                                    b13 (emit-byte b12 0)
                                                    b14 (emit-byte b13 0)
                                                    ;; header.capacity = cap
                                                    b15 (emit-leb128 (emit-byte b14 32) addr-idx)
                                                    b16 (emit-byte b15 167)
                                                    b17 (emit-leb128 (emit-byte b16 32) cap-idx)
                                                    b18 (emit-byte b17 167)
                                                    b19 (emit-byte b18 54)
                                                    b20 (emit-byte b19 0)
                                                    b21 (emit-byte b20 4)
                                                    ;; header.length = 0
                                                    b22 (emit-leb128 (emit-byte b21 32) addr-idx)
                                                    b23 (emit-byte b22 167)
                                                    b24 (emit-leb128 (emit-byte b23 65) 0)
                                                    b25 (emit-byte b24 54)
                                                    b26 (emit-byte b25 0)
                                                    b27 (emit-byte b26 8)
                                                    ;; header.reserved = 0
                                                    b28 (emit-leb128 (emit-byte b27 32) addr-idx)
                                                    b29 (emit-byte b28 167)
                                                    b30 (emit-leb128 (emit-byte b29 65) 0)
                                                    b31 (emit-byte b30 54)
                                                    b32 (emit-byte b31 0)
                                                    b33 (emit-byte b32 12)
                                                    ;; tagged pointer を返す
                                                    b34 (emit-leb128 (emit-byte b33 32) addr-idx)
                                                    b35 (emit-byte b34 66)
                                                    b36 (emit-byte b35 128)
                                                    b37 (emit-byte b36 128)
                                                    b38 (emit-byte b37 128)
                                                    b39 (emit-byte b38 128)
                                                    b40 (emit-byte b39 128)
                                                    b41 (emit-byte b40 128)
                                                    b42 (emit-byte b41 128)
                                                    b43 (emit-byte b42 128)
                                                    b44 (emit-byte b43 128)
                                                    b45 (emit-byte b44 127)]
                                                (emit-byte b45 124))
                                            (if (= opcode 55)
                                              (emit-vector-push-instr bytes operand)
                                            (if (= opcode 56)
                                              (emit-ref-new-instr bytes operand)
                                            (if (= opcode 57)
                                              (let [b1 (emit-byte bytes 167)
                                                    b2 (emit-byte b1 41)
                                                    b3 (emit-byte b2 0)
                                                    b4 (emit-byte b3 8)]
                                                b4)
                                            (if (= opcode 58)
                                              (emit-ref-set-instr bytes operand)
                                            (if (= opcode 59)
                                              (emit-print-instr bytes)
                                            (if (= opcode 60)
                                              (emit-map-new-instr bytes operand)
                                            (if (= opcode 61)
                                              (let [b1 (emit-byte bytes 167)
                                                    b2 (emit-byte b1 40)
                                                    b3 (emit-byte b2 0)
                                                    b4 (emit-byte b3 8)]
                                                (emit-byte b4 173))
                                            (if (= opcode 62)
                                              (emit-map-insert-instr bytes operand)
                                            (if (= opcode 63)
                                              (emit-map-get-instr bytes operand)
                                            (if (= opcode 65)
                                              (emit-map-contains-instr bytes operand)
                                            (if (= opcode 66)
                                              (emit-map-remove-instr bytes operand)
                                            (if (= opcode 64)
                                              (emit-read-file-instr bytes)
                                            ;; 未知のopcode: スキップ
                                              bytes))))))))))))))))))))))))))))))))))

;; === Data セクション生成 ===

;; Data セクション (ID=11): 文字列定数をリニアメモリに配置
;; data-bytes: バイト値の Vector (文字列の中身)
;; offset: メモリ上の配置オフセット
(defn emit-data-section [data-bytes offset]
  (let [data-len (vector-length data-bytes)
        body0 (emit-byte (vector-new 64) 1)
        body1 (emit-byte body0 0)
        body2 (emit-byte body1 65)
        body3 (emit-leb128 body2 offset)
        body4 (emit-byte body3 11)
        body5 (emit-leb128 body4 data-len)
        body-vec (append-byte-vector body5 data-bytes 0 data-len)
        body-size (vector-length body-vec)
        result0 (emit-byte (vector-new 64) 11)
        result1 (emit-leb128 result0 body-size)]
    (append-byte-vector result1 body-vec 0 body-size)))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [header (emit-header)
        type-sec (emit-type-section-main)
        leb5 (leb128-u 5)
        leb300 (leb128-u 300)
        ;; 符号付き LEB128 テスト
        sleb-pos (leb128-s 5)
        sleb-neg1 (leb128-s -1)
        sleb-neg128 (leb128-s -128)]
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

      ;; 符号付き LEB128 検証
      (print (vector-get sleb-pos 0))     ;; 5
      (print (vector-length sleb-neg1))   ;; 1
      (print (vector-get sleb-neg1 0))    ;; 127 (0x7F)

      0)))

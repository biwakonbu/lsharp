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

;; === Function セクション生成 ===

;; Function セクション (ID=3): funcidx -> typeidx マッピング
;; 簡易版: 1関数 (main) のみ、type index 0
(defn emit-function-section []
  (let [bytes (vector-new 8)]
    (let [b1 (emit-byte bytes 3)     ;; Section ID = 3 (Function)
          b2 (emit-byte b1 2)        ;; セクションサイズ (2バイト)
          b3 (emit-byte b2 1)        ;; 関数数 (1個)
          b4 (emit-byte b3 0)]       ;; type index 0
      b4)))

;; === Export セクション生成 ===

;; Export セクション (ID=7): _start をエクスポート
(defn emit-export-section []
  (let [bytes (vector-new 16)]
    (let [b1 (emit-byte bytes 7)     ;; Section ID = 7 (Export)
          b2 (emit-byte b1 10)       ;; セクションサイズ (10バイト)
          b3 (emit-byte b2 1)        ;; エクスポート数 (1個)
          ;; エクスポート名 "_start" (6バイト)
          b4 (emit-byte b3 6)        ;; 名前長
          b5 (emit-byte b4 95)       ;; '_'
          b6 (emit-byte b5 115)      ;; 's'
          b7 (emit-byte b6 116)      ;; 't'
          b8 (emit-byte b7 97)       ;; 'a'
          b9 (emit-byte b8 114)      ;; 'r'
          b10 (emit-byte b9 116)     ;; 't'
          ;; エクスポート種別: 関数 (0x00)
          b11 (emit-byte b10 0)
          ;; 関数インデックス (0)
          b12 (emit-byte b11 0)]
      b12)))

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

;; === Code セクション生成 ===

;; IR 命令列を Wasm Code セクションに変換
;; 簡易版: 1関数のみ、ローカル変数なし
(defn emit-code-section [ir-instrs]
  (let [;; まず関数本体のバイト列を生成
        body (ref-new (vector-new 64))
        i (ref-new 0)
        n (vector-length ir-instrs)]
    (do
      ;; ローカル変数宣言 (0個)
      (ref-set body (emit-byte (ref-get body) 0))
      ;; IR 命令を Wasm opcodes に変換 (最大8命令の展開)
      (if (< (ref-get i) n)
        (do
          (let [instr (vector-get ir-instrs (ref-get i))
                opcode (vector-get instr 0)
                operand (vector-get instr 1)]
            (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (let [instr (vector-get ir-instrs (ref-get i))
                    opcode (vector-get instr 0)
                    operand (vector-get instr 1)]
                (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) n)
                (do
                  (let [instr (vector-get ir-instrs (ref-get i))
                        opcode (vector-get instr 0)
                        operand (vector-get instr 1)]
                    (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) n)
                    (do
                      (let [instr (vector-get ir-instrs (ref-get i))
                            opcode (vector-get instr 0)
                            operand (vector-get instr 1)]
                        (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
                      (ref-set i (+ (ref-get i) 1))
                      (if (< (ref-get i) n)
                        (do
                          (let [instr (vector-get ir-instrs (ref-get i))
                                opcode (vector-get instr 0)
                                operand (vector-get instr 1)]
                            (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
                          (ref-set i (+ (ref-get i) 1))
                          (if (< (ref-get i) n)
                            (do
                              (let [instr (vector-get ir-instrs (ref-get i))
                                    opcode (vector-get instr 0)
                                    operand (vector-get instr 1)]
                                (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
                              (ref-set i (+ (ref-get i) 1))
                              (if (< (ref-get i) n)
                                (do
                                  (let [instr (vector-get ir-instrs (ref-get i))
                                        opcode (vector-get instr 0)
                                        operand (vector-get instr 1)]
                                    (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
                                  (ref-set i (+ (ref-get i) 1))
                                  (if (< (ref-get i) n)
                                    (do
                                      (let [instr (vector-get ir-instrs (ref-get i))
                                            opcode (vector-get instr 0)
                                            operand (vector-get instr 1)]
                                        (ref-set body (emit-ir-instr (ref-get body) opcode operand)))
                                      0)
                                    0))
                                0))
                            0))
                        0))
                    0))
                0))
            0))
        0)
      ;; end 命令
      (ref-set body (emit-byte (ref-get body) 11))
      ;; Code セクションを構築
      (let [func-body (ref-get body)
            func-body-size (vector-length func-body)
            body-size-len (vector-length (leb128-u func-body-size))
            section-size (+ (+ func-body-size body-size-len) 1)
            ;; セクション: [id=10, section-size, func-count=1, func-body-size, ...func-body]
            result (vector-new 64)
            r1 (emit-byte result 10)       ;; Section ID = 10 (Code)
            r2 (emit-leb128 r1 section-size)  ;; セクションサイズ (count + body-size + func-body)
            r3 (emit-byte r2 1)            ;; 関数数 (1個)
            r4 (emit-leb128 r3 func-body-size)]  ;; 関数本体サイズ
        ;; func-body のバイト列を追加 (最大16バイトの展開)
        (let [j (ref-new 0)
              out (ref-new r4)]
          (do
            (if (< (ref-get j) func-body-size)
              (do
                (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                (ref-set j (+ (ref-get j) 1))
                (if (< (ref-get j) func-body-size)
                  (do
                    (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                    (ref-set j (+ (ref-get j) 1))
                    (if (< (ref-get j) func-body-size)
                      (do
                        (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                        (ref-set j (+ (ref-get j) 1))
                        (if (< (ref-get j) func-body-size)
                          (do
                            (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                            (ref-set j (+ (ref-get j) 1))
                            (if (< (ref-get j) func-body-size)
                              (do
                                (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                (ref-set j (+ (ref-get j) 1))
                                (if (< (ref-get j) func-body-size)
                                  (do
                                    (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                    (ref-set j (+ (ref-get j) 1))
                                    (if (< (ref-get j) func-body-size)
                                      (do
                                        (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                        (ref-set j (+ (ref-get j) 1))
                                        (if (< (ref-get j) func-body-size)
                                          (do
                                            (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                            (ref-set j (+ (ref-get j) 1))
                                            (if (< (ref-get j) func-body-size)
                                              (do
                                                (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                                (ref-set j (+ (ref-get j) 1))
                                                (if (< (ref-get j) func-body-size)
                                                  (do
                                                    (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                                    (ref-set j (+ (ref-get j) 1))
                                                    (if (< (ref-get j) func-body-size)
                                                      (do
                                                        (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                                        (ref-set j (+ (ref-get j) 1))
                                                        (if (< (ref-get j) func-body-size)
                                                          (do
                                                            (ref-set out (emit-byte (ref-get out) (vector-get func-body (ref-get j))))
                                                            0)
                                                          0))
                                                      0))
                                                  0))
                                              0))
                                          0))
                                      0))
                                  0))
                              0))
                          0))
                      0))
                  0))
              0)
            (ref-get out)))))))

;; Main.ls 用: header + type + code の合計バイト長 (簡易モジュール)
(defn emit-wasm [ir-instrs]
  (let [h (emit-header)
        t (emit-type-section-main)
        c (emit-code-section ir-instrs)]
    (+ (+ (vector-length h) (vector-length t)) (vector-length c))))

;; IR opcode を Wasm opcode に変換して bytes に追加
;; T3-6: ビルトインヘルパー -- 比較演算子 (i64.gt_s, i64.lt_s, i64.ge_s, i64.le_s) 追加
(defn emit-ir-instr [bytes opcode operand]
  (if (= opcode 1)
    ;; i64.const (符号付き LEB128 を使用)
    (emit-leb128-s (emit-byte bytes 66) operand)
    (if (= opcode 10)
      ;; local.get
      (emit-leb128 (emit-byte bytes 32) operand)
      (if (= opcode 11)
        ;; local.set
        (emit-leb128 (emit-byte bytes 33) operand)
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
                                ;; 未知のopcode: スキップ
                                bytes))))))))))))))))

;; === Data セクション生成 ===

;; Data セクション (ID=11): 文字列定数をリニアメモリに配置
;; data-bytes: バイト値の Vector (文字列の中身)
;; offset: メモリ上の配置オフセット
(defn emit-data-section [data-bytes offset]
  (let [data-len (vector-length data-bytes)
        ;; セクション本体を構築
        body (ref-new (vector-new 64))
        ;; データセグメント数 (1個)
        _ (ref-set body (emit-byte (ref-get body) 1))
        ;; メモリインデックス (0) + active データ
        _ (ref-set body (emit-byte (ref-get body) 0))
        ;; i32.const offset + end でオフセット式
        _ (ref-set body (emit-byte (ref-get body) 65))  ;; i32.const
        _ (ref-set body (emit-leb128 (ref-get body) offset))
        _ (ref-set body (emit-byte (ref-get body) 11))  ;; end
        ;; データバイト数
        _ (ref-set body (emit-leb128 (ref-get body) data-len))
        ;; データバイト列のコピー (最大16バイト)
        j (ref-new 0)]
    (do
      (if (< (ref-get j) data-len)
        (do
          (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
          (ref-set j (+ (ref-get j) 1))
          (if (< (ref-get j) data-len)
            (do
              (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
              (ref-set j (+ (ref-get j) 1))
              (if (< (ref-get j) data-len)
                (do
                  (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
                  (ref-set j (+ (ref-get j) 1))
                  (if (< (ref-get j) data-len)
                    (do
                      (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
                      (ref-set j (+ (ref-get j) 1))
                      (if (< (ref-get j) data-len)
                        (do
                          (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
                          (ref-set j (+ (ref-get j) 1))
                          (if (< (ref-get j) data-len)
                            (do
                              (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
                              (ref-set j (+ (ref-get j) 1))
                              (if (< (ref-get j) data-len)
                                (do
                                  (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
                                  (ref-set j (+ (ref-get j) 1))
                                  (if (< (ref-get j) data-len)
                                    (do
                                      (ref-set body (emit-byte (ref-get body) (vector-get data-bytes (ref-get j))))
                                      0)
                                    0))
                                0))
                            0))
                        0))
                    0))
                0))
            0))
        0)
      ;; セクションヘッダーを追加
      (let [body-vec (ref-get body)
            body-size (vector-length body-vec)
            result (vector-new 64)
            r1 (emit-byte result 11)            ;; Section ID = 11 (Data)
            r2 (emit-leb128 r1 body-size)       ;; セクションサイズ
            ;; body のバイトをコピー
            k (ref-new 0)
            out (ref-new r2)]
        (do
          (if (< (ref-get k) body-size)
            (do
              (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
              (ref-set k (+ (ref-get k) 1))
              (if (< (ref-get k) body-size)
                (do
                  (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                  (ref-set k (+ (ref-get k) 1))
                  (if (< (ref-get k) body-size)
                    (do
                      (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                      (ref-set k (+ (ref-get k) 1))
                      (if (< (ref-get k) body-size)
                        (do
                          (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                          (ref-set k (+ (ref-get k) 1))
                          (if (< (ref-get k) body-size)
                            (do
                              (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                              (ref-set k (+ (ref-get k) 1))
                              (if (< (ref-get k) body-size)
                                (do
                                  (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                                  (ref-set k (+ (ref-get k) 1))
                                  (if (< (ref-get k) body-size)
                                    (do
                                      (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                                      (ref-set k (+ (ref-get k) 1))
                                      (if (< (ref-get k) body-size)
                                        (do
                                          (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                                          (ref-set k (+ (ref-get k) 1))
                                          (if (< (ref-get k) body-size)
                                            (do
                                              (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                                              (ref-set k (+ (ref-get k) 1))
                                              (if (< (ref-get k) body-size)
                                                (do
                                                  (ref-set out (emit-byte (ref-get out) (vector-get body-vec (ref-get k))))
                                                  0)
                                                0))
                                            0))
                                        0))
                                    0))
                                0))
                            0))
                        0))
                    0))
                0))
            0)
          (ref-get out))))))

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

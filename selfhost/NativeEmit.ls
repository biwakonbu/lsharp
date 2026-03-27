(module NativeEmit)
(import NativeTarget)

;; NativeEmit.ls - L# セルフホスティング: ネイティブバイナリ出力
;;
;; ネイティブ機械語バイト列から Mach-O / ELF オブジェクトファイルを生成する。
;; ターゲット記述子に応じて適切なオブジェクト形式を選択する。

;; === Mach-O ヘッダー定数 ===
(defn macho-magic-64 [] 4277009103)   ;; 0xFEEDFACF (64bit Mach-O)
(defn macho-cpu-x86-64 [] 16777223)   ;; CPU_TYPE_X86_64
(defn macho-cpu-arm64 [] 16777228)    ;; CPU_TYPE_ARM64
(defn macho-filetype-object [] 1)     ;; MH_OBJECT

;; === ELF ヘッダー定数 ===
(defn elf-magic-0 [] 127)   ;; 0x7F
(defn elf-magic-1 [] 69)    ;; 'E'
(defn elf-magic-2 [] 76)    ;; 'L'
(defn elf-magic-3 [] 70)    ;; 'F'
(defn elf-class-64 [] 2)    ;; ELFCLASS64
(defn elf-data-lsb [] 1)    ;; ELFDATA2LSB (リトルエンディアン)
(defn elf-type-rel [] 1)    ;; ET_REL (再配置可能オブジェクト)
(defn elf-machine-x86-64 [] 62)  ;; EM_X86_64

;; === Mach-O オブジェクト生成 ===

;; Mach-O ヘッダーを生成 (簡易版)
;; target: ターゲット記述子
;; 戻り値: ヘッダーバイト列
(defn emit-macho-header [target]
  (let [bytes (vector-new 32)
        ;; マジックナンバー (リトルエンディアン: CF FA ED FE)
        b1 (vector-push bytes 207)     ;; 0xCF
        b2 (vector-push b1 250)        ;; 0xFA
        b3 (vector-push b2 237)        ;; 0xED
        b4 (vector-push b3 254)        ;; 0xFE
        ;; CPU タイプ (4バイト、リトルエンディアン)
        arch (target-arch target)
        cpu-byte (if (= arch 2) 12 7)  ;; ARM64=0x0C, X86_64=0x07
        b5 (vector-push b4 cpu-byte)
        b6 (vector-push b5 0)
        b7 (vector-push b6 0)
        b8 (vector-push b7 1)          ;; 0x01000000 (CPU_ARCH_ABI64)
        ;; CPU サブタイプ (4バイト)
        b9 (vector-push b8 0)
        b10 (vector-push b9 0)
        b11 (vector-push b10 0)
        b12 (vector-push b11 0)
        ;; ファイルタイプ: MH_OBJECT = 1
        b13 (vector-push b12 1)
        b14 (vector-push b13 0)
        b15 (vector-push b14 0)
        b16 (vector-push b15 0)]
    b16))

;; === ELF オブジェクト生成 ===

;; ELF ヘッダーを生成 (簡易版)
;; 戻り値: ヘッダーバイト列
(defn emit-elf-header []
  (let [bytes (vector-new 16)
        ;; ELF マジック: 7F 45 4C 46
        b1 (vector-push bytes 127)     ;; 0x7F
        b2 (vector-push b1 69)         ;; 'E'
        b3 (vector-push b2 76)         ;; 'L'
        b4 (vector-push b3 70)         ;; 'F'
        ;; EI_CLASS: ELFCLASS64 = 2
        b5 (vector-push b4 2)
        ;; EI_DATA: ELFDATA2LSB = 1
        b6 (vector-push b5 1)
        ;; EI_VERSION: EV_CURRENT = 1
        b7 (vector-push b6 1)
        ;; EI_OSABI: ELFOSABI_NONE = 0
        b8 (vector-push b7 0)]
    b8))

;; === オブジェクトファイル出力 ===

;; native code 全体を object result へ追記
(defn append-native-object-bytes [result native-code idx len]
  (if (>= idx len)
    (ref-get result)
    (do
      (ref-set result (vector-push (ref-get result) (vector-get native-code idx)))
      (append-native-object-bytes result native-code (+ idx 1) len))))

;; ネイティブ機械語からオブジェクトファイルを生成
;; native-code: 機械語バイト列
;; target: ターゲット記述子
;; 戻り値: オブジェクトファイルのバイト列
(defn emit-object [native-code target]
  (let [obj-format (target-obj-format target)]
    (if (= obj-format 1)
      ;; Mach-O
      (emit-macho native-code target)
      ;; ELF
      (emit-elf native-code))))

;; Mach-O オブジェクトファイルを生成
;; native-code: 機械語バイト列
;; target: ターゲット記述子
;; 戻り値: Mach-O バイト列 (ヘッダー + コード)
(defn emit-macho [native-code target]
  (let [header (emit-macho-header target)
        result (ref-new header)
        n (vector-length native-code)]
    (append-native-object-bytes result native-code 0 n)))

;; ELF オブジェクトファイルを生成
;; native-code: 機械語バイト列
;; 戻り値: ELF バイト列 (ヘッダー + コード)
(defn emit-elf [native-code]
  (let [header (emit-elf-header)
        result (ref-new header)
        n (vector-length native-code)]
    (append-native-object-bytes result native-code 0 n)))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; テスト用コードバイト列
        code (vector-push (vector-push (vector-new 4) 195) 144)  ;; ret, nop
        target-mac (make-target 1)   ;; x86_64-apple-darwin
        target-linux (make-target 3) ;; x86_64-unknown-linux-gnu
        obj-mac (emit-object code target-mac)
        obj-linux (emit-object code target-linux)]
    (do
      ;; Mach-O: ヘッダー 16バイト + コード 2バイト = 18
      (print (vector-length obj-mac))
      ;; ELF: ヘッダー 8バイト + コード 2バイト = 10
      (print (vector-length obj-linux))
      0)))

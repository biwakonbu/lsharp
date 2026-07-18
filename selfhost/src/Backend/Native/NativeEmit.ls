(module Backend.Native.NativeEmit)
(import Backend.Native.NativeTarget)

;; NativeEmit.ls - L# セルフホスティング: ネイティブバイナリ出力
;;
;; ネイティブ機械語バイト列から Mach-O / ELF オブジェクトファイルを生成する。
;; ターゲット記述子に応じて適切なオブジェクト形式を選択する。

;; === Mach-O ヘッダー定数 ===
(defn macho-magic-64 [] 4277009103) ;; 0xFEEDFACF (64bit Mach-O)
(defn macho-cpu-x86-64 [] 16777223) ;; CPU_TYPE_X86_64
(defn macho-cpu-arm64 [] 16777228) ;; CPU_TYPE_ARM64
(defn macho-filetype-object [] 1) ;; MH_OBJECT

;; === ELF ヘッダー定数 ===
(defn elf-magic-0 [] 127) ;; 0x7F
(defn elf-magic-1 [] 69) ;; 'E'
(defn elf-magic-2 [] 76) ;; 'L'
(defn elf-magic-3 [] 70) ;; 'F'
(defn elf-class-64 [] 2) ;; ELFCLASS64
(defn elf-data-lsb [] 1) ;; ELFDATA2LSB (リトルエンディアン)
(defn elf-type-rel [] 1) ;; ET_REL (再配置可能オブジェクト)
(defn elf-machine-x86-64 [] 62) ;; EM_X86_64

;; === Mach-O オブジェクト生成 ===

;; Mach-O ヘッダーを生成 (簡易版)
;; target: ターゲット記述子
;; 戻り値: ヘッダーバイト列
(defn emit-macho-header [target]
  (let [bytes (vector-new 32)
    ;; マジックナンバー (リトルエンディアン: CF FA ED FE)
    b1 (vector-push bytes 207) ;; 0xCF
    b2 (vector-push b1 250) ;; 0xFA
    b3 (vector-push b2 237) ;; 0xED
    b4 (vector-push b3 254) ;; 0xFE
    ;; CPU タイプ (4バイト、リトルエンディアン)
    arch (target-arch target)
    cpu-byte (if (= arch 2) 12 7) ;; ARM64=0x0C, X86_64=0x07
    b5 (vector-push b4 cpu-byte)
    b6 (vector-push b5 0)
    b7 (vector-push b6 0)
    b8 (vector-push b7 1) ;; 0x01000000 (CPU_ARCH_ABI64)
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

(defn align-to [value alignment]
  (let [rem (% value alignment)]
    (if (= rem 0)
      value
      (+ value (- alignment rem)))))

(defn append-u16-le [bytes value]
  (let [b0 (% value 256)
    b1 (% (/ value 256) 256)]
    (vector-push (vector-push bytes b0) b1)))

(defn append-u32-le [bytes value]
  (let [b0 (% value 256)
    b1 (% (/ value 256) 256)
    b2 (% (/ value 65536) 256)
    b3 (% (/ value 16777216) 256)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push bytes b0)
          b1)
        b2)
      b3)))

(defn append-u64-le [bytes value]
  (let [lo (append-u32-le bytes (% value 4294967296))
    hi (append-u32-le lo (/ value 4294967296))]
    hi))

(defn append-zeroes [bytes count]
  (if (<= count 0)
    bytes
    (append-zeroes (vector-push bytes 0) (- count 1))))

(defn append-zeroes-until [bytes target-len]
  (append-zeroes bytes (- target-len (vector-length bytes))))

(defn append-macho-sectname-text [bytes]
  (append-zeroes
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push bytes 95) ;; _
              95) ;; _
            116) ;; t
          101) ;; e
        120) ;; x
      116) ;; t
    10))

(defn append-macho-segname-text [bytes]
  (append-zeroes
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push bytes 95) ;; _
              95) ;; _
            84) ;; T
          69) ;; E
        88) ;; X
      84) ;; T
    10))

(defn append-macho-strtab-generated [bytes]
  (let [b0 (vector-push bytes 0)
    b1 (vector-push b0 95) ;; _
    b2 (vector-push b1 103) ;; g
    b3 (vector-push b2 101) ;; e
    b4 (vector-push b3 110) ;; n
    b5 (vector-push b4 101) ;; e
    b6 (vector-push b5 114) ;; r
    b7 (vector-push b6 97) ;; a
    b8 (vector-push b7 116) ;; t
    b9 (vector-push b8 101) ;; e
    b10 (vector-push b9 100)] ;; d
    (vector-push b10 0)))

(defn append-macho-header-x86-64 [bytes sizeofcmds]
  (let [with-magic (append-u32-le bytes 4277009103)
    with-cpu (append-u32-le with-magic 16777223)
    with-subtype (append-u32-le with-cpu 3)
    with-filetype (append-u32-le with-subtype 1)
    with-ncmds (append-u32-le with-filetype 3)
    with-sizeofcmds (append-u32-le with-ncmds sizeofcmds)
    with-flags (append-u32-le with-sizeofcmds 0)]
    (append-u32-le with-flags 0)))

(defn append-macho-section-text-x86-64 [bytes text-offset code-len]
  (let [with-sectname (append-macho-sectname-text bytes)
    with-segname (append-macho-segname-text with-sectname)
    with-addr (append-u64-le with-segname 0)
    with-size (append-u64-le with-addr code-len)
    with-offset (append-u32-le with-size text-offset)
    with-align (append-u32-le with-offset 3)
    with-reloff (append-u32-le with-align 0)
    with-nreloc (append-u32-le with-reloff 0)
    with-flags (append-u32-le with-nreloc 2147484672)
    with-reserved1 (append-u32-le with-flags 0)
    with-reserved2 (append-u32-le with-reserved1 0)]
    (append-u32-le with-reserved2 0)))

(defn append-macho-segment-command-x86-64 [bytes text-offset code-len]
  (let [with-cmd (append-u32-le bytes 25)
    with-cmdsize (append-u32-le with-cmd 152)
    with-segname (append-zeroes with-cmdsize 16)
    with-vmaddr (append-u64-le with-segname 0)
    with-vmsize (append-u64-le with-vmaddr code-len)
    with-fileoff (append-u64-le with-vmsize text-offset)
    with-filesize (append-u64-le with-fileoff code-len)
    with-maxprot (append-u32-le with-filesize 7)
    with-initprot (append-u32-le with-maxprot 5)
    with-nsects (append-u32-le with-initprot 1)
    with-flags (append-u32-le with-nsects 0)]
    (append-macho-section-text-x86-64 with-flags text-offset code-len)))

(defn append-macho-symtab-command [bytes symtab-offset strtab-offset strtab-size]
  (let [with-cmd (append-u32-le bytes 2)
    with-cmdsize (append-u32-le with-cmd 24)
    with-symoff (append-u32-le with-cmdsize symtab-offset)
    with-nsyms (append-u32-le with-symoff 1)
    with-stroff (append-u32-le with-nsyms strtab-offset)]
    (append-u32-le with-stroff strtab-size)))

(defn append-macho-build-version-command [bytes]
  (let [with-cmd (append-u32-le bytes 50)
    with-cmdsize (append-u32-le with-cmd 24)
    with-platform (append-u32-le with-cmdsize 1)
    with-minos (append-u32-le with-platform 720896)
    with-sdk (append-u32-le with-minos 0)]
    (append-u32-le with-sdk 0)))

(defn append-macho-symbol-generated [bytes]
  (let [with-name (append-u32-le bytes 1)
    with-type (vector-push with-name 15)
    with-sect (vector-push with-type 1)
    with-desc (append-u16-le with-sect 0)]
    (append-u64-le with-desc 0)))

(defn append-elf-ident [bytes]
  (append-zeroes
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push bytes 127) ;; 0x7F
                  69) ;; E
                76) ;; L
              70) ;; F
            2) ;; ELFCLASS64
          1) ;; little endian
        1) ;; EV_CURRENT
      0) ;; SYSV ABI
    8))

(defn append-elf-header [bytes section-header-offset section-count]
  (let [with-ident (append-elf-ident bytes)
    with-type (append-u16-le with-ident 1)
    with-machine (append-u16-le with-type 62)
    with-version (append-u32-le with-machine 1)
    with-entry (append-u64-le with-version 0)
    with-phoff (append-u64-le with-entry 0)
    with-shoff (append-u64-le with-phoff section-header-offset)
    with-flags (append-u32-le with-shoff 0)
    with-ehsize (append-u16-le with-flags 64)
    with-phentsize (append-u16-le with-ehsize 0)
    with-phnum (append-u16-le with-phentsize 0)
    with-shentsize (append-u16-le with-phnum 64)
    with-shnum (append-u16-le with-shentsize section-count)]
    (append-u16-le with-shnum 4)))

(defn emit-elf-header [section-header-offset section-count]
  (append-elf-header (vector-new 64) section-header-offset section-count))

(defn append-elf-symbol [bytes name info other shndx value size]
  (let [with-name (append-u32-le bytes name)
    with-info (vector-push with-name info)
    with-other (vector-push with-info other)
    with-shndx (append-u16-le with-other shndx)
    with-value (append-u64-le with-shndx value)]
    (append-u64-le with-value size)))

(defn append-elf-section-header [bytes name section-type flags addr offset size link info addralign entsize]
  (let [with-name (append-u32-le bytes name)
    with-type (append-u32-le with-name section-type)
    with-flags (append-u64-le with-type flags)
    with-addr (append-u64-le with-flags addr)
    with-offset (append-u64-le with-addr offset)
    with-size (append-u64-le with-offset size)
    with-link (append-u32-le with-size link)
    with-info (append-u32-le with-link info)
    with-align (append-u64-le with-info addralign)]
    (append-u64-le with-align entsize)))

(defn append-elf-strtab [bytes]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push bytes 0)
                      103) ;; g
                    101) ;; e
                  110) ;; n
                101) ;; e
              114) ;; r
            97) ;; a
          116) ;; t
        101) ;; e
      100) ;; d
    0))

(defn append-elf-shstrtab [bytes]
  (let [b0 (vector-push bytes 0)
    ;; .text
    b1 (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push b0 46) 116) 101) 120) 116) 0)
    ;; .symtab
    b2 (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push b1 46) 115) 121) 109) 116) 97) 98) 0)
    ;; .strtab
    b3 (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push b2 46) 115) 116) 114) 116) 97) 98) 0)
    ;; .shstrtab
    b4 (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push b3 46) 115) 104) 115) 116) 114) 116) 97) 98) 0)]
    ;; .note.GNU-stack
    (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push b4 46) 110) 111) 116) 101) 46) 71) 78) 85) 45) 115) 116) 97) 99) 107) 0)))

;; === オブジェクトファイル出力 ===

;; native code を小区間ごとに object result へ追記する。
;; 1 byte ごとの非末尾再帰は Wasm call stack と root stack を同時に膨らませるため、
;; bounded step を self-TCO で回し、各区間の一時 root を次の区間へ持ち越さない。
(defn append-native-object-bytes-bounded [result native-code idx len remaining]
  (if (if (>= idx len) true (<= remaining 0))
    idx
    (let [current (ref-get result)]
      (do
        (root_push native-code)
        (root_push current)
        (let [next (vector-push current (vector-get native-code idx))]
          (do
            (root_push next)
            (ref-set result next)
            (root_pop)
            (root_pop)
            (root_pop)
            (append-native-object-bytes-bounded result native-code (+ idx 1) len (- remaining 1))))))))

(defn continue-append-native-object-bytes-step-64 [result native-code len idx]
  (if (>= idx len)
    0
    (do
      (root_push result)
      (root_push native-code)
      (let [next-idx (append-native-object-bytes-bounded result native-code idx len 64)]
        (do
          (root_pop)
          (root_pop)
          (continue-append-native-object-bytes-step-64 result native-code len next-idx))))))

(defn append-native-object-bytes [result native-code idx len]
  (do
    (continue-append-native-object-bytes-step-64 result native-code len idx)
    (ref-get result)))

;; ネイティブ機械語からオブジェクトファイルを生成
;; native-code: 機械語バイト列
;; target: ターゲット記述子
;; 戻り値: オブジェクトファイルのバイト列
(defn emit-object [native-code target]
  (do
    (root_push native-code)
    (root_push target)
    (let [obj-format (target-obj-format target)
      object
        (if (= obj-format 1)
          ;; Mach-O
          (emit-macho native-code target)
          ;; ELF
          (emit-elf native-code))]
      (do
        (root_pop)
        (root_pop)
        object))))

;; Mach-O オブジェクトファイルを生成
;; native-code: 機械語バイト列
;; target: ターゲット記述子
;; 戻り値: Mach-O バイト列 (x86_64 は linkable object、aarch64 は簡易ヘッダー + コード)
(defn emit-macho [native-code target]
  (if (= (target-arch target) (arch-x86-64))
    (emit-macho-x86-64 native-code)
    (let [header (emit-macho-header target)
      result (ref-new header)
      n (vector-length native-code)]
      (append-native-object-bytes result native-code 0 n))))

;; x86_64 Darwin 向け linkable Mach-O 64-bit relocatable object を生成する。
(defn emit-macho-x86-64 [native-code]
  (do
    (root_push native-code)
    (let [code-len (vector-length native-code)
      sizeofcmds 200
      text-offset (+ 32 sizeofcmds)
      symtab-offset (align-to (+ text-offset code-len) 8)
      strtab-offset (+ symtab-offset 16)
      strtab-size 12
      header (append-macho-build-version-command
        (append-macho-symtab-command
          (append-macho-segment-command-x86-64
            (append-macho-header-x86-64 (vector-new 32) sizeofcmds)
            text-offset
            code-len)
          symtab-offset
          strtab-offset
          strtab-size))]
      (do
        (root_push header)
        (let [result (ref-new header)]
          (do
            (ref-set result (append-native-object-bytes result native-code 0 code-len))
            (ref-set result (append-zeroes-until (ref-get result) symtab-offset))
            (ref-set result (append-macho-symbol-generated (ref-get result)))
            (ref-set result (append-macho-strtab-generated (ref-get result)))
            (root_pop)
            (root_pop)
            (ref-get result)))))))

;; ELF オブジェクトファイルを生成
;; native-code: 機械語バイト列
;; 戻り値: linkable ELF64 relocatable object
(defn emit-elf [native-code]
  (do
    (root_push native-code)
    (let [code-len (vector-length native-code)
      text-offset 64
      symtab-offset (align-to (+ text-offset code-len) 8)
      symtab-size 72
      strtab-offset (+ symtab-offset symtab-size)
      strtab-size 11
      shstrtab-offset (+ strtab-offset strtab-size)
      shstrtab-size 49
      section-header-offset (align-to (+ shstrtab-offset shstrtab-size) 8)
      header (emit-elf-header section-header-offset 6)]
      (do
        (root_push header)
        (let [result (ref-new header)]
          (do
            (ref-set result (append-native-object-bytes result native-code 0 code-len))
            (ref-set result (append-zeroes-until (ref-get result) symtab-offset))
            (ref-set result (append-elf-symbol (ref-get result) 0 0 0 0 0 0))
            (ref-set result (append-elf-symbol (ref-get result) 0 3 0 1 0 0))
            (ref-set result (append-elf-symbol (ref-get result) 1 18 0 1 0 code-len))
            (ref-set result (append-elf-strtab (ref-get result)))
            (ref-set result (append-elf-shstrtab (ref-get result)))
            (ref-set result (append-zeroes-until (ref-get result) section-header-offset))
            (ref-set result (append-elf-section-header (ref-get result) 0 0 0 0 0 0 0 0 0 0))
            (ref-set result (append-elf-section-header (ref-get result) 1 1 6 0 text-offset code-len 0 0 16 0))
            (ref-set result (append-elf-section-header (ref-get result) 7 2 0 0 symtab-offset symtab-size 3 2 8 24))
            (ref-set result (append-elf-section-header (ref-get result) 15 3 0 0 strtab-offset strtab-size 0 0 1 0))
            (ref-set result (append-elf-section-header (ref-get result) 23 3 0 0 shstrtab-offset shstrtab-size 0 0 1 0))
            (ref-set result (append-elf-section-header (ref-get result) 33 1 0 0 0 0 0 0 1 0))
            (root_pop)
            (root_pop)
            (ref-get result)))))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; テスト用コードバイト列
    code (vector-push (vector-push (vector-new 4) 195) 144) ;; ret, nop
    target-mac (make-target 1) ;; x86_64-apple-darwin
    target-linux (make-target 3) ;; x86_64-unknown-linux-gnu
    obj-mac (emit-object code target-mac)
    obj-linux (emit-object code target-linux)]
    (do
      ;; x86_64 Mach-O: linkable relocatable object + コード 2バイト
      (print (vector-length obj-mac))
      ;; ELF: linkable ELF64 relocatable object
      (print (vector-length obj-linux))
      0)))

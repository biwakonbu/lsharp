(module Backend.Native.NativeTarget)

;; NativeTarget.ls - L# セルフホスティング: ネイティブターゲット記述子
;;
;; x86_64-apple-darwin, aarch64-apple-darwin, x86_64-unknown-linux-gnu
;; の3つのターゲットトリプルをサポートする。
;; 各ターゲットはアーキテクチャ、OS、オブジェクト形式の情報を持つ。

;; === アーキテクチャ定数 ===
(defn arch-x86-64 [] 1)
(defn arch-aarch64 [] 2)

;; === OS 定数 ===
(defn os-darwin [] 1)
(defn os-linux [] 2)

;; === オブジェクト形式定数 ===
(defn obj-macho [] 1)
(defn obj-elf [] 2)

;; === リンカー種別定数 ===
(defn linker-ld64 [] 1) ;; macOS ld64
(defn linker-lld [] 2) ;; LLVM lld
(defn linker-gnu-ld [] 3) ;; GNU ld

;; === calling convention 定数 ===
(defn cc-sysv64 [] 1) ;; x86_64 System V / Darwin x86_64 共有 slice
(defn cc-aapcs64 [] 2) ;; aarch64 AAPCS64

;; === section policy 定数 ===
(defn section-policy-macho [] 1)
(defn section-policy-elf [] 2)

;; === relocation call policy 定数 ===
(defn reloc-call-x86-pcrel32 [] 1)
(defn reloc-call-aarch64-branch26 [] 2)

;; === response file style 定数 ===
(defn response-file-lines [] 1)

;; === runtime artifact policy 定数 ===
(defn runtime-bundled [] 1) ;; 配布物/CI artifact から供給
(defn runtime-generated [] 2) ;; 補助 build step で生成

;; === runtime object kind 定数 ===
(defn runtime-object-runtime-o [] 1)

;; === ABI 定数 ===
(defn stack-align-16 [] 16)

;; === ターゲット記述子 ===
;; 現在の実装は次の 12-field descriptor を持つ。
;; [arch, os, obj-format, triple-id, calling-convention,
;;  stack-alignment, section-policy, reloc-call,
;;  linker-flavor, response-file-style, runtime-policy, runtime-object-kind]
;;
;; triple-id: 1 = x86_64-apple-darwin
;;            2 = aarch64-apple-darwin
;;            3 = x86_64-unknown-linux-gnu

(defn build-target [arch os obj-format triple-id calling-convention stack-alignment section-policy reloc-call linker-flavor response-file-style runtime-policy runtime-object-kind]
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
                      (vector-push
                        (vector-push
                          (vector-new 12)
                          arch)
                        os)
                      obj-format)
                    triple-id)
                  calling-convention)
                stack-alignment)
              section-policy)
            reloc-call)
          linker-flavor)
        response-file-style)
      runtime-policy)
    runtime-object-kind))

;; x86_64-apple-darwin ターゲット記述子
(defn target-x86-64-darwin []
  (build-target
    (arch-x86-64)
    (os-darwin)
    (obj-macho)
    1
    (cc-sysv64)
    (stack-align-16)
    (section-policy-macho)
    (reloc-call-x86-pcrel32)
    (linker-ld64)
    (response-file-lines)
    (runtime-bundled)
    (runtime-object-runtime-o)))

;; aarch64-apple-darwin ターゲット記述子
(defn target-aarch64-darwin []
  (build-target
    (arch-aarch64)
    (os-darwin)
    (obj-macho)
    2
    (cc-aapcs64)
    (stack-align-16)
    (section-policy-macho)
    (reloc-call-aarch64-branch26)
    (linker-ld64)
    (response-file-lines)
    (runtime-bundled)
    (runtime-object-runtime-o)))

;; x86_64-unknown-linux-gnu ターゲット記述子
(defn target-x86-64-linux []
  (build-target
    (arch-x86-64)
    (os-linux)
    (obj-elf)
    3
    (cc-sysv64)
    (stack-align-16)
    (section-policy-elf)
    (reloc-call-x86-pcrel32)
    (linker-lld)
    (response-file-lines)
    (runtime-bundled)
    (runtime-object-runtime-o)))

;; === ターゲット取得関数 ===

;; ターゲットトリプル ID からターゲット記述子を取得
;; triple-id: 1, 2, 3 のいずれか
(defn make-target [triple-id]
  (if (= triple-id 1)
    (target-x86-64-darwin)
    (if (= triple-id 2)
      (target-aarch64-darwin)
      (if (= triple-id 3)
        (target-x86-64-linux)
        ;; デフォルト: x86_64-apple-darwin
        (target-x86-64-darwin)))))

;; ターゲットからアーキテクチャを取得
(defn target-arch [target]
  (vector-get target 0))

;; ターゲットから OS を取得
(defn target-os [target]
  (vector-get target 1))

;; ターゲットからオブジェクト形式を取得
(defn target-obj-format [target]
  (vector-get target 2))

;; ターゲットトリプル ID を取得
(defn target-triple [target]
  (vector-get target 3))

;; ターゲットから calling convention を取得
(defn target-calling-convention [target]
  (vector-get target 4))

;; ターゲットから stack alignment を取得
(defn target-stack-alignment [target]
  (vector-get target 5))

;; ターゲットから section policy を取得
(defn target-section-policy [target]
  (vector-get target 6))

;; ターゲットから call relocation policy を取得
(defn target-reloc-call [target]
  (vector-get target 7))

;; ターゲットからリンカー種別を取得
(defn target-linker-flavor [target]
  (vector-get target 8))

;; ターゲットから response file style を取得
(defn target-response-file-style [target]
  (vector-get target 9))

;; ターゲットから runtime policy を取得
(defn target-runtime-policy [target]
  (vector-get target 10))

;; ターゲットから runtime object kind を取得
(defn target-runtime-object-kind [target]
  (vector-get target 11))

;; === ホストターゲット検出 ===

;; 現在のホストターゲットを返す (簡易版: aarch64-apple-darwin を仮定)
(defn host-target []
  (target-aarch64-darwin))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [t1 (make-target 1)
    t2 (make-target 2)
    t3 (make-target 3)]
    (do
      ;; x86_64-apple-darwin
      (print (target-arch t1)) ;; 1 (x86_64)
      (print (target-os t1)) ;; 1 (darwin)
      (print (target-triple t1)) ;; 1

      ;; aarch64-apple-darwin
      (print (target-arch t2)) ;; 2 (aarch64)
      (print (target-os t2)) ;; 1 (darwin)
      (print (target-triple t2)) ;; 2

      ;; x86_64-unknown-linux-gnu
      (print (target-arch t3)) ;; 1 (x86_64)
      (print (target-os t3)) ;; 2 (linux)
      (print (target-triple t3)) ;; 3

      0)))

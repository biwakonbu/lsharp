(module NativeTarget)

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

;; === ターゲット記述子 ===
;; ターゲットは [arch, os, obj-format, triple-id] の Vector で表現
;; triple-id: 1 = x86_64-apple-darwin
;;            2 = aarch64-apple-darwin
;;            3 = x86_64-unknown-linux-gnu

;; x86_64-apple-darwin ターゲット記述子
(defn target-x86-64-darwin []
  (vector-push (vector-push (vector-push (vector-push
    (vector-new 4)
    (arch-x86-64))    ;; arch = x86_64
    (os-darwin))       ;; os = darwin
    (obj-macho))       ;; format = Mach-O
    1))                ;; triple-id = 1

;; aarch64-apple-darwin ターゲット記述子
(defn target-aarch64-darwin []
  (vector-push (vector-push (vector-push (vector-push
    (vector-new 4)
    (arch-aarch64))    ;; arch = aarch64
    (os-darwin))       ;; os = darwin
    (obj-macho))       ;; format = Mach-O
    2))                ;; triple-id = 2

;; x86_64-unknown-linux-gnu ターゲット記述子
(defn target-x86-64-linux []
  (vector-push (vector-push (vector-push (vector-push
    (vector-new 4)
    (arch-x86-64))    ;; arch = x86_64
    (os-linux))        ;; os = linux
    (obj-elf))         ;; format = ELF
    3))                ;; triple-id = 3

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
      (print (target-arch t1))     ;; 1 (x86_64)
      (print (target-os t1))       ;; 1 (darwin)
      (print (target-triple t1))   ;; 1

      ;; aarch64-apple-darwin
      (print (target-arch t2))     ;; 2 (aarch64)
      (print (target-os t2))       ;; 1 (darwin)
      (print (target-triple t2))   ;; 2

      ;; x86_64-unknown-linux-gnu
      (print (target-arch t3))     ;; 1 (x86_64)
      (print (target-os t3))       ;; 2 (linux)
      (print (target-triple t3))   ;; 3

      0)))

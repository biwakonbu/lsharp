(module Tools.Validation.Whitespace)

;; Rust の str::trim が扱う Unicode White_Space を UTF-8 byte 列で判定する。
;; selfhost の string-char-at は code point ではなく byte を返すため、
;; ASCII は 1 byte、Unicode whitespace は対応する UTF-8 sequence 幅で進める。
(defn validation-ascii-whitespace? [byte]
  (or (= byte 32) (and (>= byte 9) (<= byte 13))))

(defn validation-byte-at [value idx len]
  (if (< idx len)
    (string-char-at value idx)
    -1))

(defn validation-unicode-whitespace-width [value idx len]
  (let [first (validation-byte-at value idx len)
    second (validation-byte-at value (+ idx 1) len)
    third (validation-byte-at value (+ idx 2) len)]
    (if (= first 194)
      (if (or (= second 133) (= second 160)) 2 0)
      (if (and (= first 225) (and (= second 154) (= third 128)))
        3
        (if (and (= first 226) (= second 128))
          (if (or
                (and (>= third 128) (<= third 138))
                (or (= third 168) (or (= third 169) (= third 175))))
            3
            0)
          (if (and (= first 226) (and (= second 129) (= third 159)))
            3
            (if (and (= first 227) (and (= second 128) (= third 128))) 3 0)))))))

(defn validation-whitespace-width [value idx len]
  (let [byte (validation-byte-at value idx len)]
    (if (validation-ascii-whitespace? byte)
      1
      (validation-unicode-whitespace-width value idx len))))

(defn validation-nonblank-loop [value idx len]
  (if (>= idx len)
    0
    (let [width (validation-whitespace-width value idx len)]
      (if (> width 0)
        (validation-nonblank-loop value (+ idx width) len)
        1))))

(defn validation-nonblank? [value]
  (validation-nonblank-loop value 0 (string-length value)))

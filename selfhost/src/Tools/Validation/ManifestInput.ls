(module Tools.Validation.ManifestInput)

;; Native source-file smoke の positional version 1 manifest input 境界。
;;
;; selfhost の JSON decoder がまだ typed manifest graph へ接続されていない間も、
;; source が emit した canonical manifest を同じ report/exit boundary へ戻せるよう、
;; manifest の固定 wire fields を fail-closed に確認する。generic JSON を黙って無視せず、
;; graph の trace を再構築できない形は呼び出し側で拒否する。

(defn manifest-input-ws? [ch]
  (if
    (or
      (or (= ch 32) (= ch 9))
      (or (= ch 10) (= ch 13)))
    1
    0))

(defn manifest-input-first-nonspace-loop [src idx len]
  (if (>= idx len)
    -1
    (if (= (manifest-input-ws? (string-char-at src idx)) 1)
      (manifest-input-first-nonspace-loop src (+ idx 1) len)
      idx)))

(defn manifest-input-pattern-at? [src pattern idx]
  (let [pattern-len (string-length pattern)
    end (+ idx pattern-len)]
    (if (> end (string-length src))
      0
      (if (string-eq (substring src idx end) pattern) 1 0))))

(defn manifest-input-find-pattern-loop [src pattern idx len]
  (if (>= idx len)
    -1
    (if (= (manifest-input-pattern-at? src pattern idx) 1)
      idx
      (manifest-input-find-pattern-loop src pattern (+ idx 1) len))))

(defn manifest-input-find-pattern [src pattern]
  (manifest-input-find-pattern-loop src pattern 0 (string-length src)))

(defn manifest-input-count-pattern-loop [src pattern idx len count]
  (let [found (manifest-input-find-pattern-loop src pattern idx len)]
    (if (< found 0)
      count
      (manifest-input-count-pattern-loop
        src
        pattern
        (+ found (string-length pattern))
        len
        (+ count 1)))))

(defn manifest-input-count-pattern [src pattern]
  (manifest-input-count-pattern-loop src pattern 0 (string-length src) 0))

(defn validation-manifest-json-input? [src]
  (let [start (manifest-input-first-nonspace-loop src 0 (string-length src))]
      (if (and (>= start 0) (= (string-char-at src start) 123))
      (if (and
            (>= (manifest-input-find-pattern src "\"schema_version\":1") 0)
            (and
              (>= (manifest-input-find-pattern src "\"nodes\":[") 0)
              (and
                (>= (manifest-input-find-pattern src "\"evidence\":[") 0)
                (>= (manifest-input-find-pattern src "\"edges\":[") 0))))
        1
        0)
      0)))

(defn validation-manifest-positional-path? [value]
  (if (and
        (> (string-length value) 0)
        (!= (string-char-at value 0) 45))
    1
    0))

(defn validation-manifest-required-trace? [src]
  (if (and
        (>= (manifest-input-find-pattern src "\"relation\":\"motivates\"") 0)
        (>= (manifest-input-find-pattern src "\"relation\":\"tested-by\"") 0))
    1
    0))

(defn validation-manifest-open-question-count [src]
  (manifest-input-count-pattern src "\"kind\":\"open-question\""))

(defn validation-manifest-independent-review-count [src]
  (let [method-count (manifest-input-count-pattern src "\"method\":\"review\"")
    independence-count
      (manifest-input-count-pattern src "\"independence\":\"independent-review\"")]
    (if (< method-count independence-count) method-count independence-count)))

(defn validation-manifest-contradiction-count [src]
  (+
    (manifest-input-count-pattern src "\"outcome\":\"contradicted\"")
    (manifest-input-count-pattern src "\"relation\":\"contradicts\"")))

(defn validation-manifest-stale-review-count [src]
  (manifest-input-count-pattern src "\"relation\":\"invalidates\""))

(defn validation-manifest-stale-evidence-count [src]
  (manifest-input-count-pattern src "\"outcome\":\"stale\""))

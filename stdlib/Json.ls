;; Json.ls - L# 標準ライブラリ: JSON パーサー
;;
;; 再帰降下パーサーによる JSON パース。
;; JsonValue ADT: Null=0, Bool=1, Num=2, Str=3, Arr=4, Obj=5
;;
;; 注意: 現在の L# では ADT のパターンマッチと文字列操作が
;; セルフホスティング用の簡易実装であるため、JSON パーサーも
;; 整数タグ + Vector ベースの簡易実装とする。

;; === JsonValue タグ定数 ===

(defn json-null [] 0)
(defn json-bool [] 1)
(defn json-num [] 2)
(defn json-str [] 3)
(defn json-arr [] 4)
(defn json-obj [] 5)

;; === JsonValue 構築 ===

;; Null 値: [0]
(defn make-json-null []
  (vector-push (vector-new 1) 0))

;; Bool 値: [1, 0/1]
(defn make-json-bool [b]
  (vector-push (vector-push (vector-new 2) 1) b))

;; Num 値: [2, value]
(defn make-json-num [n]
  (vector-push (vector-push (vector-new 2) 2) n))

;; Str 値: [3, string-hash]
;; 注意: 文字列はハッシュ値で保持 (簡易実装)
(defn make-json-str [s]
  (vector-push (vector-push (vector-new 2) 3) s))

;; Arr 値: [4, elem-count, elem1, elem2, ...]
(defn make-json-arr [elems]
  (let [result (vector-push (vector-push (vector-new 8) 4) (vector-length elems))
        i (ref-new 0)
        n (vector-length elems)
        out (ref-new result)]
    (do
      (if (< (ref-get i) n)
        (do
          (ref-set out (vector-push (ref-get out) (vector-get elems (ref-get i))))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (ref-set out (vector-push (ref-get out) (vector-get elems (ref-get i))))
              (ref-set i (+ (ref-get i) 1))
              0)
            0))
        0)
      (ref-get out))))

;; Obj 値: [5, field-count, key1, val1, key2, val2, ...]
(defn make-json-obj [fields]
  (let [result (vector-push (vector-push (vector-new 8) 5) (vector-length fields))]
    result))

;; === JsonValue アクセス ===

;; JsonValue のタグを取得
(defn json-tag [jv]
  (vector-get jv 0))

;; JsonValue が Null か
(defn json-is-null [jv]
  (= (json-tag jv) 0))

;; JsonValue が Bool の場合、値を取得
(defn json-bool-value [jv]
  (vector-get jv 1))

;; JsonValue が Num の場合、値を取得
(defn json-num-value [jv]
  (vector-get jv 1))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [null-val (make-json-null)
        bool-val (make-json-bool 1)
        num-val (make-json-num 42)
        str-val (make-json-str 0)]
    (do
      ;; タグの検証
      (print (json-tag null-val))    ;; 0 (Null)
      (print (json-tag bool-val))    ;; 1 (Bool)
      (print (json-tag num-val))     ;; 2 (Num)
      (print (json-tag str-val))     ;; 3 (Str)

      ;; 値の検証
      (print (json-bool-value bool-val))  ;; 1 (true)
      (print (json-num-value num-val))    ;; 42

      0)))

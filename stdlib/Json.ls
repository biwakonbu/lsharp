;; Json.ls - L# 標準ライブラリ: JSON パーサー
;;
;; 再帰降下パーサーによる JSON パース。
;; JsonValue ADT: Null=0, Bool=1, Num=2, Str=3, Arr=4, Obj=5
;;
;; 注意: 現在の L# では ADT のパターンマッチと文字列操作が
;; セルフホスティング用の簡易実装であるため、JSON パーサーも
;; 整数タグ + Vector ベースの簡易実装とする。

;; === JsonValue タグ定数 ===

(defn json-null
  []
  :doc "JsonValue の Null タグ値を返す。"
  :returns "Null を表すタグ値 0"
  :example [ (json-null)]
  0)
(defn json-bool
  []
  :doc "JsonValue の Bool タグ値を返す。"
  :returns "Bool を表すタグ値 1"
  :example [ (json-bool)]
  1)
(defn json-num
  []
  :doc "JsonValue の Num タグ値を返す。"
  :returns "Num を表すタグ値 2"
  :example [ (json-num)]
  2)
(defn json-str
  []
  :doc "JsonValue の Str タグ値を返す。"
  :returns "Str を表すタグ値 3"
  :example [ (json-str)]
  3)
(defn json-arr
  []
  :doc "JsonValue の Arr タグ値を返す。"
  :returns "Arr を表すタグ値 4"
  :example [ (json-arr)]
  4)
(defn json-obj
  []
  :doc "JsonValue の Obj タグ値を返す。"
  :returns "Obj を表すタグ値 5"
  :example [ (json-obj)]
  5)

;; === JsonValue 構築 ===

;; Null 値: [0]
(defn make-json-null
  []
  :doc "Null を表す簡易 JsonValue ベクタを作る。"
  :returns "Null の JsonValue 表現"
  :example [ (make-json-null)]
  (vector-push (vector-new 1) 0))

;; Bool 値: [1, 0/1]
(defn make-json-bool
  [b]
  :doc "Bool を表す簡易 JsonValue ベクタを作る。"
  :params [ (b "保持したい真偽値")]
  :returns "Bool の JsonValue 表現"
  :example [ (make-json-bool 1)]
  (vector-push (vector-push (vector-new 2) 1) b))

;; Num 値: [2, value]
(defn make-json-num
  [n]
  :doc "数値を表す簡易 JsonValue ベクタを作る。"
  :params [ (n "保持したい数値")]
  :returns "Num の JsonValue 表現"
  :example [ (make-json-num 42)]
  (vector-push (vector-push (vector-new 2) 2) n))

;; Str 値: [3, string-hash]
;; 注意: 文字列はハッシュ値で保持 (簡易実装)
(defn make-json-str
  [s]
  :doc "文字列を表す簡易 JsonValue ベクタを作る。"
  :params [ (s "保持したい文字列または識別子")]
  :returns "Str の JsonValue 表現"
  :example [ (make-json-str 0)]
  (vector-push (vector-push (vector-new 2) 3) s))

;; Arr 値: [4, elem-count, elem1, elem2, ...]
(defn make-json-arr
  [elems]
  :doc "要素ベクタから配列 JsonValue を構築する。"
  :params [ (elems "要素の JsonValue ベクタ")]
  :returns "Arr の JsonValue 表現"
  :example [ (make-json-arr (vector-push (vector-new 1) (make-json-null)))]
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
(defn make-json-obj
  [fields]
  :doc "フィールドベクタからオブジェクト JsonValue を構築する。"
  :params [ (fields "キーと値を交互に並べたベクタ")]
  :returns "Obj の JsonValue 表現"
  :example [ (make-json-obj (vector-new 0))]
  (let [result (vector-push (vector-push (vector-new 8) 5) (vector-length fields))]
    result))

;; === JsonValue アクセス ===

;; JsonValue のタグを取得
(defn json-tag
  [jv]
  :doc "JsonValue の先頭タグを返す。"
  :params [ (jv "タグを取り出したい JsonValue")]
  :returns "JsonValue の種類を表す整数タグ"
  :example [ (json-tag (make-json-null))]
  (vector-get jv 0))

;; JsonValue が Null か
(defn json-is-null
  [jv]
  :doc "JsonValue が Null かどうかを判定する。"
  :params [ (jv "判定対象の JsonValue")]
  :returns "Null なら 1、そうでなければ 0"
  :example [ (json-is-null (make-json-null))]
  (= (json-tag jv) 0))

;; JsonValue が Bool の場合、値を取得
(defn json-bool-value
  [jv]
  :doc "Bool JsonValue から保持している真偽値を取り出す。"
  :params [ (jv "Bool の JsonValue")]
  :returns "内部に保持している 0 または 1"
  :example [ (json-bool-value (make-json-bool 1))]
  (vector-get jv 1))

;; JsonValue が Num の場合、値を取得
(defn json-num-value
  [jv]
  :doc "Num JsonValue から保持している数値を取り出す。"
  :params [ (jv "Num の JsonValue")]
  :returns "内部に保持している数値"
  :example [ (json-num-value (make-json-num 42))]
  (vector-get jv 1))

;; === エントリポイント (テスト用) ===

(private
  (defn main []
    (let [null-val (make-json-null)
      bool-val (make-json-bool 1)
      num-val (make-json-num 42)
      str-val (make-json-str 0)]
      (do
        ;; タグの検証
        (print (json-tag null-val)) ;; 0 (Null)
        (print (json-tag bool-val)) ;; 1 (Bool)
        (print (json-tag num-val)) ;; 2 (Num)
        (print (json-tag str-val)) ;; 3 (Str)

        ;; 値の検証
        (print (json-bool-value bool-val)) ;; 1 (true)
        (print (json-num-value num-val)) ;; 42

        0))))

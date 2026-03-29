(module Syntax.Span)

;; Span.ls - L# セルフホスティング: ソース位置情報
;;
;; [start, end] 形式でソース位置を保持する。
;; AST ノードのエラー報告やデバッグに使用。

;; === Span 構築 ===

;; 新しい Span を作成: [start, end]
(defn span-new [start end]
  (vector-push (vector-push (vector-new 2) start) end))

;; 別名: make-span
(defn make-span [start end]
  (span-new start end))

;; === Span アクセサ ===

;; Span の開始位置を取得
(defn span-start [span]
  (vector-get span 0))

;; Span の終了位置を取得
(defn span-end [span]
  (vector-get span 1))

;; === Span 操作 ===

;; 二つの Span をマージ: 最小の start と最大の end を取る
(defn span-merge [s1 s2]
  (let [start1 (span-start s1)
        start2 (span-start s2)
        end1 (span-end s1)
        end2 (span-end s2)
        min-start (if (< start1 start2) start1 start2)
        max-end (if (> end1 end2) end1 end2)]
    (span-new min-start max-end)))

;; ダミー Span (位置情報なし): [0, 0]
(defn span-dummy []
  (span-new 0 0))

;; === 行・列の導出 ===

;; ソース文字列と位置から行番号を計算 (1-indexed)
;; 改行文字 (ASCII 10) をカウント
(defn span-line [src pos]
  (let [line (ref-new 1)
        i (ref-new 0)]
    (do
      ;; 最大 256 文字分の改行をスキャン (展開ループ)
      (if (< (ref-get i) pos)
        (do
          (if (= (string-char-at src (ref-get i)) 10)
            (do (ref-set line (+ (ref-get line) 1)) 0)
            0)
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) pos)
            (do
              (if (= (string-char-at src (ref-get i)) 10)
                (do (ref-set line (+ (ref-get line) 1)) 0)
                0)
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) pos)
                (do
                  (if (= (string-char-at src (ref-get i)) 10)
                    (do (ref-set line (+ (ref-get line) 1)) 0)
                    0)
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) pos)
                    (do
                      (if (= (string-char-at src (ref-get i)) 10)
                        (do (ref-set line (+ (ref-get line) 1)) 0)
                        0)
                      (ref-set i (+ (ref-get i) 1))
                      0)
                    0))
                0))
            0))
        0)
      (ref-get line))))

;; ソース文字列と位置から列番号を計算 (1-indexed)
;; 直前の改行からの距離
(defn span-column [src pos]
  (let [col (ref-new 1)
        i (ref-new (- pos 1))]
    (do
      ;; 後方に改行を探す
      (if (>= (ref-get i) 0)
        (if (= (string-char-at src (ref-get i)) 10)
          0
          (do
            (ref-set col (+ (ref-get col) 1))
            (ref-set i (- (ref-get i) 1))
            (if (>= (ref-get i) 0)
              (if (= (string-char-at src (ref-get i)) 10)
                0
                (do
                  (ref-set col (+ (ref-get col) 1))
                  (ref-set i (- (ref-get i) 1))
                  (if (>= (ref-get i) 0)
                    (if (= (string-char-at src (ref-get i)) 10)
                      0
                      (do
                        (ref-set col (+ (ref-get col) 1))
                        0))
                    0)))
              0)))
        0)
      (ref-get col))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [s1 (span-new 0 10)
        s2 (span-new 5 20)
        merged (span-merge s1 s2)
        dummy (span-dummy)]
    (do
      ;; 基本アクセサ
      (print (span-start s1))    ;; 0
      (print (span-end s1))      ;; 10
      ;; マージ
      (print (span-start merged)) ;; 0
      (print (span-end merged))   ;; 20
      ;; ダミー
      (print (span-start dummy))  ;; 0
      (print (span-end dummy))    ;; 0
      0)))

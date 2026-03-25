(module Derive)
(import AST)
(import Parser)

;; Derive.ls - L# セルフホスティング: derive マクロ展開
;;
;; `:derive` アノテーションからヘルパー宣言 (decl) を自動生成する。
;; パイプライン順: Parser -> Derive -> MacroExpand -> TypeInfer
;;
;; === サポートする derive ===
;; - :derive Eq     -> 構造的等価性比較関数を生成
;; - :derive Show   -> 文字列表現関数を生成
;; - :derive Clone  -> 値のクローン関数を生成
;;
;; === 設計 ===
;; expand-derives は AST の宣言リストを走査し、
;; :derive メタデータを持つ型宣言に対してヘルパー関数を挿入する。
;; 生成される関数は型定義の直後に配置される。

;; === derive 展開メイン ===

;; AST の宣言リストを走査して :derive を展開
;; decls: 宣言の vector (parse 結果のトップレベル)
;; 戻り値: 展開後の宣言 vector (元の宣言 + 生成されたヘルパー)
(defn expand-derives [decls]
  (let [result (vector-new 16)]
    (expand-derives-loop decls result 0)))

;; expand-derive: 単一宣言に対する derive 展開 (expand-derives のエイリアス的ヘルパー)
(defn expand-derive [decl]
  (let [decls (vector-push (vector-new 2) decl)]
    (expand-derives decls)))

;; derive 展開ループ
(defn expand-derives-loop [decls result idx]
  (if (>= idx (vector-length decls))
    result
    (let [decl (vector-get decls idx)
          tag (vector-get decl 0)]
      ;; ast-type-decl は型宣言タグ
      ;; type 宣言に :derive メタデータがあるか検査
      (if (has-derive-metadata decl)
        ;; derive 対象: 元宣言を追加 + ヘルパー関数を生成して追加
        (let [result1 (vector-push result decl)
              helpers (generate-derive-helpers decl)
              result2 (append-all result1 helpers 0)]
          (expand-derives-loop decls result2 (+ idx 1)))
        ;; derive 対象外: そのまま追加
        (expand-derives-loop decls (vector-push result decl) (+ idx 1))))))

;; === メタデータ検査 ===

;; 宣言に :derive メタデータがあるかチェック
;; 簡易実装: vector の要素に derive マーカー (tag=200) があるか走査
(defn has-derive-metadata [decl]
  (has-derive-metadata-loop decl 0))

(defn has-derive-metadata-loop [decl idx]
  (if (>= idx (vector-length decl))
    0  ;; false: derive なし
    (let [elem (vector-get decl idx)]
      (if (== elem 200)  ;; derive マーカータグ
        1  ;; true: derive あり
        (has-derive-metadata-loop decl (+ idx 1))))))

;; === ヘルパー関数生成 ===

;; derive 対象の型宣言からヘルパー関数群を生成
;; 戻り値: 生成された宣言の vector
(defn generate-derive-helpers [decl]
  (let [helpers (vector-new 4)
        ;; 型名ハッシュを取得 (vector の index 1 が名前ハッシュの想定)
        type-name-hash (if (>= (vector-length decl) 2)
                         (vector-get decl 1)
                         0)]
    ;; Eq ヘルパー: 等価性比較 (簡易スタブ)
    (let [eq-fn (make-eq-helper type-name-hash)]
      (vector-push helpers eq-fn))))

;; Eq ヘルパー関数ノードを生成
;; tag=20 (defn), name-hash=eq_{type}, params=2, body=比較
(defn make-eq-helper [type-name-hash]
  (let [;; 関数名: type-name-hash + "eq" のハッシュ (簡易結合)
        fn-name-hash (+ (* type-name-hash 31) 101)  ;; 'e'=101 に基づく簡易ハッシュ
        n (vector-new 8)]
    ;; [20, fn-name-hash, param-count=2, param1-hash, param2-hash, body]
    (let [n1 (vector-push n 20)
          n2 (vector-push n1 fn-name-hash)
          n3 (vector-push n2 2)          ;; param-count
          n4 (vector-push n3 97)         ;; param 'a' のハッシュ
          n5 (vector-push n4 98)]        ;; param 'b' のハッシュ
      ;; body: (== a b) -> 簡易的に 0 を返す (スタブ)
      (vector-push n5 0))))

;; === ユーティリティ ===

;; vector の全要素を別の vector に追加
(defn append-all [target source idx]
  (if (>= idx (vector-length source))
    target
    (append-all (vector-push target (vector-get source idx)) source (+ idx 1))))

;; エントリポイント (テスト用)
(defn main []
  (let [;; 空の宣言リストに対して derive 展開
        empty-decls (vector-new 2)
        result1 (expand-derives empty-decls)
        ;; derive マーカー付き type 宣言で展開テスト
        type-decl (let [d (vector-new 4)]
                    (vector-push (vector-push (vector-push d (ast-type-decl)) 12345) 200))
        decls2 (vector-push (vector-new 2) type-decl)
        result2 (expand-derives decls2)]
    (do
      (print (vector-length result1))  ;; 0 (空入力)
      (print (vector-length result2))  ;; 2 (元宣言 + Eq ヘルパー)
      0)))

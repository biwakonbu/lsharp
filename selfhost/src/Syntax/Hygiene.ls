(module Syntax.Hygiene)

;; Hygiene.ls - L# セルフホスティング: 衛生的マクロ支援
;;
;; マクロ展開時の名前衝突を防ぐための仕組みを提供する。
;; - gensym: ユニークシンボル生成
;; - scope-id: スコープID管理
;; - expansion-trace: マクロ展開のトレース記録
;;
;; === 設計方針 ===
;; 各 gensym シンボルはグローバルカウンタベースのユニークIDを持つ。
;; scope-id はマクロ展開のネスト深度とスコープ番号の組で管理。
;; expansion-trace はマクロ展開の履歴を vector に記録し、
;; デバッグやエラー報告に利用する。

;; === グローバルカウンタ ===
;; gensym 用のユニーク ID カウンタ (ref で可変状態管理)

;; gensym: ユニークシンボルを生成する
;; 戻り値: [tag=100, unique-id] の vector (tag=100 は gensym シンボルを表す)
;; counter-ref: ref-new で作成したカウンタ参照
(defn gensym [counter-ref]
  (let [id (ref-get counter-ref)]
    (do (ref-set counter-ref (+ id 1))
        (let [sym (vector-new 2)]
          (vector-push (vector-push sym 100) id)))))

;; gensym シンボルかどうかを判定
(defn gensym? [node]
  (if (== (vector-get node 0) 100) 1 0))

;; gensym の ID を取得
(defn gensym-id [node]
  (vector-get node 1))

;; === スコープ ID 管理 ===
;; スコープ ID: [depth, scope-number] の組
;; depth: マクロ展開のネスト深度
;; scope-number: 同一深度内のスコープ番号

;; 新しいスコープ ID を作成
;; depth: マクロ展開のネスト深度
;; scope-counter-ref: スコープ番号カウンタ参照
(defn scope-id [depth scope-counter-ref]
  (let [num (ref-get scope-counter-ref)]
    (do (ref-set scope-counter-ref (+ num 1))
        (let [sid (vector-new 2)]
          (vector-push (vector-push sid depth) num)))))

;; make-scope-id: scope-id のエイリアス (後方互換)
(defn make-scope-id [depth scope-counter-ref]
  (scope-id depth scope-counter-ref))

;; スコープ ID の depth を取得
(defn scope-depth [sid]
  (vector-get sid 0))

;; スコープ ID の番号を取得
(defn scope-number [sid]
  (vector-get sid 1))

;; === 展開トレース ===
;; マクロ展開の履歴を記録する。
;; 各エントリ: [macro-name-hash, scope-id, source-span]

;; 新しい展開トレースを作成
(defn expansion-trace []
  (vector-new 8))

;; make-expansion-trace: expansion-trace のエイリアス (後方互換)
(defn make-expansion-trace []
  (expansion-trace))

;; 展開ステップをトレースに追加
;; trace: 展開トレース vector
;; macro-name-hash: マクロ名のハッシュ
;; sid: スコープ ID
;; source-span: ソース位置
(defn trace-expansion [trace macro-name-hash sid source-span]
  (let [entry (vector-new 4)]
    (let [e (vector-push (vector-push (vector-push entry macro-name-hash) sid) source-span)]
      (vector-push trace e))))

;; トレースのエントリ数を取得
(defn trace-length [trace]
  (vector-length trace))

;; トレースの N 番目のエントリを取得
(defn trace-entry [trace n]
  (vector-get trace n))

;; === 衛生的名前解決 ===
;; シンボルにスコープ情報を付与して名前解決の曖昧さを排除する

;; スコープ付きシンボル: [tag=101, name-hash, scope-id]
(defn scoped-symbol [name-hash sid]
  (let [sym (vector-new 4)]
    (vector-push (vector-push (vector-push sym 101) name-hash) sid)))

;; スコープ付きシンボルの名前ハッシュを取得
(defn scoped-name [sym]
  (vector-get sym 1))

;; スコープ付きシンボルのスコープ ID を取得
(defn scoped-scope [sym]
  (vector-get sym 2))

;; エントリポイント (テスト用)
(defn main []
  (let [counter (ref-new 0)
        scope-counter (ref-new 0)
        ;; gensym テスト
        g1 (gensym counter)
        g2 (gensym counter)
        ;; scope-id テスト
        s1 (scope-id 0 scope-counter)
        s2 (scope-id 1 scope-counter)
        ;; expansion-trace テスト
        trace (expansion-trace)]
    (do
      (print (gensym-id g1))      ;; 0
      (print (gensym-id g2))      ;; 1
      (print (scope-depth s1))    ;; 0
      (print (scope-number s1))   ;; 0
      (print (scope-depth s2))    ;; 1
      (print (scope-number s2))   ;; 1
      (print (trace-length trace)) ;; 0
      0)))

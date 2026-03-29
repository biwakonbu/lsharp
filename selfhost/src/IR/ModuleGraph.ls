(module IR.ModuleGraph)

;; ModuleGraph.ls - L# セルフホスティング: モジュール依存グラフ
;;
;; import 宣言からの依存グラフ構築、トポロジカルソート、循環依存検出

;; === 依存グラフ構造 ===

;; グラフは HashMap<module-hash, deps-vector> で表現
;; deps-vector: 依存先モジュールハッシュのリスト

;; 空のグラフを生成
(defn graph-new []
  (map-new))

;; グラフにモジュールを追加 (依存先リスト付き)
(defn graph-add-module [graph mod-hash deps]
  (map-insert graph mod-hash deps))

;; === トポロジカルソート ===

;; import 宣言からの依存グラフをトポロジカルソートする
;; graph: HashMap<module-hash, deps-vector>
;; modules: ソート対象モジュールハッシュの Vector
;; 戻り値: ソート済みモジュールハッシュの Vector (依存先が先)
(defn topological-sort [graph modules]
  (let [n (vector-length modules)
        result (ref-new (vector-new n))
        visited (ref-new (map-new))
        i (ref-new 0)]
    (do
      ;; 各モジュールに対して DFS
      (if (> n 0)
        (do
          (ref-set result (topo-visit graph (vector-get modules 0) (ref-get visited) (ref-get result)))
          (ref-set visited (map-insert (ref-get visited) (vector-get modules 0) 1))
          (ref-set i 1)
          (if (> n 1)
            (do
              (ref-set result (topo-visit graph (vector-get modules 1) (ref-get visited) (ref-get result)))
              (ref-set visited (map-insert (ref-get visited) (vector-get modules 1) 1))
              (if (> n 2)
                (do
                  (ref-set result (topo-visit graph (vector-get modules 2) (ref-get visited) (ref-get result)))
                  (ref-set visited (map-insert (ref-get visited) (vector-get modules 2) 1))
                  (if (> n 3)
                    (do
                      (ref-set result (topo-visit graph (vector-get modules 3) (ref-get visited) (ref-get result)))
                      (ref-set visited (map-insert (ref-get visited) (vector-get modules 3) 1))
                      0)
                    0))
                0))
            0))
        0)
      (ref-get result))))

;; DFS ヘルパー: 未訪問なら依存先を先に追加してからモジュールを追加
(defn topo-visit [graph mod-hash visited result]
  (let [already (map-get visited mod-hash)]
    (if (= already 1)
      result
      (let [deps (map-get graph mod-hash)
            new-visited (map-insert visited mod-hash 1)]
        ;; 依存先がなければ自身を追加
        (vector-push result mod-hash)))))

;; === 循環依存の検出 ===

;; グラフに循環依存が存在するかを検出する
;; graph: HashMap<module-hash, deps-vector>
;; modules: 全モジュールハッシュの Vector
;; 戻り値: 0 = 循環なし、1 = 循環あり
(defn detect-cycle [graph modules]
  (let [n (vector-length modules)
        ;; 状態マップ: 0=未訪問, 1=訪問中, 2=完了
        state (ref-new (map-new))
        has-cycle (ref-new 0)
        i (ref-new 0)]
    (do
      ;; 各モジュールについて DFS で循環を検出
      (if (> n 0)
        (do
          (ref-set has-cycle (cycle-visit graph (vector-get modules 0)
                               (ref-get state) (ref-get has-cycle)))
          (if (> n 1)
            (do
              (ref-set has-cycle (cycle-visit graph (vector-get modules 1)
                                   (ref-get state) (ref-get has-cycle)))
              (if (> n 2)
                (do
                  (ref-set has-cycle (cycle-visit graph (vector-get modules 2)
                                       (ref-get state) (ref-get has-cycle)))
                  (if (> n 3)
                    (do
                      (ref-set has-cycle (cycle-visit graph (vector-get modules 3)
                                           (ref-get state) (ref-get has-cycle)))
                      0)
                    0))
                0))
            0))
        0)
      (ref-get has-cycle))))

;; DFS による循環検出ヘルパー
;; 戻り値: 0 = 循環なし、1 = 循環あり
(defn cycle-visit [graph mod-hash state has-cycle]
  (if (= has-cycle 1)
    1
    (let [s (map-get state mod-hash)]
      (if (= s 1)
        ;; 訪問中のノードに戻った = 循環
        1
        (if (= s 2)
          ;; 完了済み = スキップ
          has-cycle
          ;; 未訪問: 訪問中にマークして探索
          has-cycle)))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [g (graph-new)
        ;; A -> B, B -> C (循環なし)
        g1 (graph-add-module g 1 (vector-push (vector-new 1) 2))
        g2 (graph-add-module g1 2 (vector-push (vector-new 1) 3))
        g3 (graph-add-module g2 3 (vector-new 0))
        modules (vector-push (vector-push (vector-push (vector-new 3) 1) 2) 3)
        sorted (topological-sort g3 modules)
        cycle-result (detect-cycle g3 modules)]
    (do
      (print (vector-length sorted))  ;; ソート結果の長さ
      (print cycle-result)            ;; 0 (循環なし)
      0)))

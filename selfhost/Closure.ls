(module Closure)
(import AST)

;; Closure.ls - L# セルフホスティング: クロージャ変換
;;
;; ラムダ式の自由変数を解析し、環境キャプチャを行う。
;; クロージャを関数 + 環境構造体のペアに変換する。

;; === 自由変数の解析 ===

;; AST 式ノード内の自由変数 (束縛されていない変数参照) を収集
;; expr: AST 式ノード
;; bound: 束縛済み変数ハッシュの集合 (HashMap<hash, 1>)
;; 戻り値: 自由変数ハッシュの Vector
(defn free-vars [expr bound]
  (let [tag (vector-get expr 0)
        result (ref-new (vector-new 4))]
    (do
      (if (= tag 4)
        ;; 変数参照: 束縛されていなければ自由変数
        (let [name-hash (vector-get expr 1)
              is-bound (map-get bound name-hash)]
          (if (= is-bound 0)
            (do (ref-set result (vector-push (ref-get result) name-hash)) 0)
            0))
        (if (= tag 8)
          ;; ラムダ式: [8, param-count, param1-hash, ..., body]
          ;; パラメータを束縛に追加して body の自由変数を収集
          (let [param-count (vector-get expr 1)
                new-bound (ref-new bound)]
            (do
              (if (> param-count 0)
                (do
                  (do (ref-set new-bound (map-insert (ref-get new-bound) (vector-get expr 2) 1)) 0)
                  (if (> param-count 1)
                    (do (ref-set new-bound (map-insert (ref-get new-bound) (vector-get expr 3) 1)) 0)
                    0))
                0)
              (let [body-idx (+ 2 param-count)
                    body-fvs (free-vars (vector-get expr body-idx) (ref-get new-bound))
                    fv-len (vector-length body-fvs)
                    j (ref-new 0)]
                (do
                  (if (> fv-len 0)
                    (do
                      (ref-set result (vector-push (ref-get result) (vector-get body-fvs 0)))
                      (if (> fv-len 1)
                        (do (ref-set result (vector-push (ref-get result) (vector-get body-fvs 1))) 0)
                        0))
                    0)
                  0))))
          (if (= tag 6)
            ;; if 式: cond, then, else の自由変数を合併
            (let [fv1 (free-vars (vector-get expr 1) bound)
                  fv2 (free-vars (vector-get expr 2) bound)
                  fv3 (free-vars (vector-get expr 3) bound)]
              (do
                ;; fv1 を結果に追加
                (if (> (vector-length fv1) 0)
                  (do (ref-set result (vector-push (ref-get result) (vector-get fv1 0))) 0)
                  0)
                ;; fv2 を結果に追加
                (if (> (vector-length fv2) 0)
                  (do (ref-set result (vector-push (ref-get result) (vector-get fv2 0))) 0)
                  0)
                ;; fv3 を結果に追加
                (if (> (vector-length fv3) 0)
                  (do (ref-set result (vector-push (ref-get result) (vector-get fv3 0))) 0)
                  0)
                0))
            (if (= tag 7)
              ;; let 束縛: [7, name-hash, init, body]
              ;; init の自由変数 + (name を束縛に追加した body の自由変数)
              (let [name-hash (vector-get expr 1)
                    fv-init (free-vars (vector-get expr 2) bound)
                    new-bound (map-insert bound name-hash 1)
                    fv-body (free-vars (vector-get expr 3) new-bound)]
                (do
                  (if (> (vector-length fv-init) 0)
                    (do (ref-set result (vector-push (ref-get result) (vector-get fv-init 0))) 0)
                    0)
                  (if (> (vector-length fv-body) 0)
                    (do (ref-set result (vector-push (ref-get result) (vector-get fv-body 0))) 0)
                    0)
                  0))
              0))))
      (ref-get result))))

;; === 環境キャプチャ ===

;; 自由変数リストから環境キャプチャ構造体を生成
;; free-var-hashes: 自由変数ハッシュの Vector
;; env: 現在の環境 (HashMap<name-hash, local-idx>)
;; 戻り値: キャプチャ環境 [var-count, [hash1, idx1], [hash2, idx2], ...]
(defn capture-env [free-var-hashes env]
  (let [n (vector-length free-var-hashes)
        captures (ref-new (vector-new (+ 1 n)))]
    (do
      ;; var-count を先頭に格納
      (ref-set captures (vector-push (ref-get captures) n))
      ;; 各自由変数のハッシュとローカルインデックスをペアで格納
      (if (> n 0)
        (do
          (let [h0 (vector-get free-var-hashes 0)
                idx0 (map-get env h0)
                pair0 (vector-push (vector-push (vector-new 2) h0) idx0)]
            (ref-set captures (vector-push (ref-get captures) pair0)))
          (if (> n 1)
            (do
              (let [h1 (vector-get free-var-hashes 1)
                    idx1 (map-get env h1)
                    pair1 (vector-push (vector-push (vector-new 2) h1) idx1)]
                (ref-set captures (vector-push (ref-get captures) pair1)))
              (if (> n 2)
                (do
                  (let [h2 (vector-get free-var-hashes 2)
                        idx2 (map-get env h2)
                        pair2 (vector-push (vector-push (vector-new 2) h2) idx2)]
                    (ref-set captures (vector-push (ref-get captures) pair2)))
                  0)
                0))
            0))
        0)
      (ref-get captures))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; (fn [x] (+ x y)) の y が自由変数
        ;; var y: [4, 200]
        var-y (vector-push (vector-push (vector-new 2) 4) 200)
        bound (map-new)
        fvs (free-vars var-y bound)

        ;; 環境キャプチャテスト
        env (map-insert (map-new) 200 3)
        captured (capture-env fvs env)]
    (do
      (print (vector-length fvs))       ;; 1 (y が自由変数)
      (print (vector-get fvs 0))        ;; 200 (y のハッシュ)
      (print (vector-get captured 0))   ;; 1 (キャプチャ数)
      0)))

(module Types.Constraints)
(import Types.Type)
(import Types.TypeScheme)

;; Constraints.ls - L# セルフホスティング: トレイト制約システム
;;
;; トレイト定義の登録、実装の登録、制約の解決を行う。
;; Rust 版 constraints.rs に対応する。
;;
;; 依存: Type.ls, TypeScheme.ls

;; ============================================================
;; Trait Registry: トレイト定義の管理
;; ============================================================
;; トレイト = [name-hash, method-count, method1-hash, method1-type, ...]
;; レジストリ = HashMap<trait-name-hash, trait-def>

;; 新しいトレイトレジストリを作成
(defn make-trait-registry []
  (map-new))

;; トレイトレジストリにトレイト定義を登録
;; trait-hash: トレイト名のハッシュ
;; trait-def: トレイト定義 [method-count, method1-hash, method1-type, ...]
(defn register-trait [registry trait-hash trait-def]
  (map-insert registry trait-hash trait-def))

;; トレイト定義を参照
(defn lookup-trait [registry trait-hash]
  (map-get registry trait-hash))

;; ============================================================
;; Impl Registry: トレイト実装の管理
;; ============================================================
;; 実装 = [trait-hash, type-hash, method-count, method1-impl, ...]
;; レジストリ = HashMap<(trait-hash * 10000 + type-hash), impl-def>

;; 新しい実装レジストリを作成
(defn make-impl-registry []
  (map-new))

;; 実装レジストリにトレイト実装を登録
;; trait-hash: トレイト名のハッシュ
;; type-hash: 実装対象の型のハッシュ
;; impl-def: 実装定義 [method-count, method1-impl, ...]
(defn register-impl [registry trait-hash type-hash impl-def]
  (let [key (+ (* trait-hash 10000) type-hash)]
    (map-insert registry key impl-def)))

;; 実装を参照
(defn lookup-impl [registry trait-hash type-hash]
  (let [key (+ (* trait-hash 10000) type-hash)]
    (map-get registry key)))

;; ============================================================
;; Constraint: 型制約
;; ============================================================
;; 制約 = [trait-hash, type] (型が指定トレイトを実装していること)
;; 制約リスト = Vector<constraint>

;; 新しい制約リストを作成
(defn constraints-new []
  (vector-new 8))

;; 制約を追加
(defn add-constraint [clist trait-hash ty]
  (let [c (vector-push (vector-push (vector-new 2) trait-hash) ty)]
    (vector-push clist c)))

;; 制約の数を取得
(defn constraints-count [clist]
  (vector-length clist))

;; i 番目の制約を取得
(defn constraint-at [clist i]
  (vector-get clist i))

;; 制約のトレイトハッシュ
(defn constraint-trait [c]
  (vector-get c 0))

;; 制約の型
(defn constraint-type [c]
  (vector-get c 1))

;; ============================================================
;; solve-constraints: 制約解決
;; ============================================================
;; 制約リスト内の全制約を検証し、満たされない制約があればエラーを返す
;;
;; 引数:
;;   constraints  - 制約リスト
;;   trait-reg    - トレイトレジストリ
;;   impl-reg     - 実装レジストリ
;;   subst        - 現在の置換 (型変数を解決するため)
;; 戻り値:
;;   0 = 全制約満足
;;   制約のインデックス + 1 = 満たされない制約のインデックス (1-indexed)

(defn solve-constraints [clist trait-reg impl-reg subst]
  (let [count (constraints-count clist)]
    (solve-constraints-loop clist trait-reg impl-reg subst 0 count)))

;; 制約解決ループ
(defn solve-constraints-loop [clist trait-reg impl-reg subst i count]
  (if (>= i count)
    ;; 全制約チェック済み: 成功
    0
    (let [c (constraint-at clist i)
          t-hash (constraint-trait c)
          ty (constraint-type c)
          ;; 置換を適用して具体型を取得
          resolved-ty (apply-subst subst ty)]
      ;; トレイトが登録されているか
      (if (= (lookup-trait trait-reg t-hash) 0)
        ;; 未登録トレイト: エラー (制約インデックス + 1)
        (+ i 1)
        ;; 解決済み型が Con (具体型) か確認
        (let [ty-tag-val (type-tag resolved-ty)]
          (if (= ty-tag-val 1)
            ;; 具体型: 実装があるか確認
            (let [ty-hash (type-name resolved-ty)
                  impl-found (lookup-impl impl-reg t-hash ty-hash)]
              (if (= impl-found 0)
                ;; 実装なし: エラー
                (+ i 1)
                ;; 実装あり: 次の制約へ
                (solve-constraints-loop clist trait-reg impl-reg subst (+ i 1) count)))
            ;; 型変数: 解決を先送り (成功扱い)
            (solve-constraints-loop clist trait-reg impl-reg subst (+ i 1) count)))))))

;; ============================================================
;; エントリポイント (テスト用)
;; ============================================================

(defn main []
  (let [t-reg (make-trait-registry)
        i-reg (make-impl-registry)
        ;; Show トレイトを登録 (hash=500)
        t-reg2 (register-trait t-reg 500 (vector-push (vector-new 1) 1))
        ;; Int に Show を実装 (Int hash=100)
        i-reg2 (register-impl i-reg 500 100 (vector-push (vector-new 1) 1))
        ;; 制約: Show Int
        cs (add-constraint (constraints-new) 500 (make-type-int))
        ;; 制約解決
        result (solve-constraints cs t-reg2 i-reg2 (map-new))]
    (do
      (print result)  ;; 0 (成功)
      0)))

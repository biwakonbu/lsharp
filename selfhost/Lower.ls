(module Lower)
(import IR)
(import LowerExpr)
(import LowerDecl)
(import LowerPattern)

;; Lower.ls - L# セルフホスティング: AST → IR 変換オーケストレーション
;;
;; AST の Program を受け取り、各宣言を IR モジュールに変換する。
;; 式・宣言・パターンの lowering は各サブモジュールに委譲。

;; === lowering オーケストレーション ===

;; AST プログラム (宣言リスト) を IR モジュールに変換
;; program: 宣言ノードの Vector
;; 戻り値: IR モジュール (関数 IR のリスト)
(defn lower-module [program]
  (let [n (vector-length program)
        ir-funcs (ref-new (vector-new n))
        i (ref-new 0)]
    (do
      ;; 各宣言を lowering
      (if (> n 0)
        (do
          (ref-set ir-funcs (vector-push (ref-get ir-funcs)
            (lower-decl-dispatch (vector-get program 0))))
          (if (> n 1)
            (do
              (ref-set ir-funcs (vector-push (ref-get ir-funcs)
                (lower-decl-dispatch (vector-get program 1))))
              (if (> n 2)
                (do
                  (ref-set ir-funcs (vector-push (ref-get ir-funcs)
                    (lower-decl-dispatch (vector-get program 2))))
                  (if (> n 3)
                    (do
                      (ref-set ir-funcs (vector-push (ref-get ir-funcs)
                        (lower-decl-dispatch (vector-get program 3))))
                      0)
                    0))
                0))
            0))
        0)
      (ref-get ir-funcs))))

;; 宣言の種類に応じて適切な lowering 関数にディスパッチ
(defn lower-decl-dispatch [decl]
  (let [tag (vector-get decl 0)]
    (if (= tag 20)
      ;; defn 宣言
      (lower-defn-to-ir decl)
      ;; その他: そのまま返す (未対応)
      decl)))

;; defn 宣言を IR 関数に変換
(defn lower-defn-to-ir [decl]
  (let [name-hash (vector-get decl 1)
        param-count (vector-get decl 2)
        body-idx (+ 3 param-count)
        body (vector-get decl body-idx)
        ir-body (lower-expr-to-ir body)]
    ;; IR 関数: [name-hash, param-count, ir-body]
    (vector-push (vector-push (vector-push (vector-new 3) name-hash) param-count) ir-body)))

;; 式を IR に変換 (LowerExpr に委譲)
(defn lower-expr-to-ir [expr]
  (let [tag (vector-get expr 0)]
    (if (= tag 1)
      ;; 整数リテラル: そのまま IR 定数命令
      (make-instr 1 (vector-get expr 1))
      expr)))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; テスト: 空プログラム
        empty-prog (vector-new 0)
        result (lower-module empty-prog)]
    (do
      (print (vector-length result))  ;; 0
      0)))

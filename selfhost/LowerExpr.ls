(module LowerExpr)
(import IR)

;; LowerExpr.ls - L# セルフホスティング: 式の lowering
;;
;; AST の式ノードを IR 命令列に変換する。

;; === 式の lowering ===

;; AST 式ノードを IR 命令列に変換
;; expr: AST 式ノード (Vector [tag, ...])
;; env: 変数環境 (HashMap<name-hash, local-idx>)
;; instrs: 追記先の命令列 (Vector)
;; 戻り値: 更新された instrs
(defn lower-expr [expr env instrs]
  (let [tag (vector-get expr 0)]
    (if (= tag 1)
      ;; 整数リテラル: i64.const value
      (vector-push instrs (make-instr 1 (vector-get expr 1)))
      (if (= tag 2)
        ;; 真偽値リテラル: i64.const 0/1
        (vector-push instrs (make-instr 1 (vector-get expr 1)))
        (if (= tag 4)
          ;; 変数参照: local.get idx
          (let [name-hash (vector-get expr 1)
                idx (map-get env name-hash)]
            (vector-push instrs (make-instr 10 idx)))
          (if (= tag 6)
            ;; if 式: [6, cond, then, else]
            (let [i1 (lower-expr (vector-get expr 1) env instrs)
                  i2 (vector-push i1 (make-instr 41 0))
                  i3 (lower-expr (vector-get expr 2) env i2)
                  i4 (vector-push i3 (make-instr 43 0))
                  i5 (lower-expr (vector-get expr 3) env i4)]
              (vector-push i5 (make-instr 43 0)))
            (if (= tag 7)
              ;; let 束縛: [7, name-hash, init, body]
              (let [name-hash (vector-get expr 1)
                    new-idx (+ 1 (map-size env))
                    i1 (lower-expr (vector-get expr 2) env instrs)
                    i2 (vector-push i1 (make-instr 11 new-idx))
                    new-env (map-insert env name-hash new-idx)]
                (lower-expr (vector-get expr 3) new-env i2))
              ;; その他: 未対応 → 0 をプッシュ
              (vector-push instrs (make-instr 1 0)))))))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; 整数リテラルの lowering テスト
        lit (vector-push (vector-push (vector-new 2) 1) 42)
        env (map-new)
        result (lower-expr lit env (vector-new 4))]
    (do
      (print (vector-length result))  ;; 1
      0)))

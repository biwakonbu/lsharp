(module Types.TypeInferBuiltins)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; TypeInfer builtins: infer の初期環境だけを分離する
;; 推論本体 (infer-expr / infer-defn) は TypeInfer.ls に残す

(defn typeinfer-builtin-int-binop [int-ty]
  (mk-fun int-ty (mk-fun int-ty int-ty)))

(defn typeinfer-builtin-int-cmp [int-ty bool-ty]
  (mk-fun int-ty (mk-fun int-ty bool-ty)))

;; ビルトイン演算子の型を登録
;; + : Int -> Int -> Int (カリー化)
;; = : Int -> Int -> Bool
;; print : Int -> Int
(defn typeinfer-init-builtin-env [counter]
  (let [env (type-env-new)
    int-ty (mk-int)
    bool-ty (mk-bool)
    add-ty (typeinfer-builtin-int-binop int-ty)
    sub-ty (typeinfer-builtin-int-binop int-ty)
    mul-ty (typeinfer-builtin-int-binop int-ty)
    div-ty (typeinfer-builtin-int-binop int-ty)
    eq-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    gt-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    lt-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    print-ty (mk-fun int-ty int-ty)
    ;; 名前ハッシュ (ASCII コード)
    env1 (type-env-insert env 43 (mono add-ty))
    env2 (type-env-insert env1 45 (mono sub-ty))
    env3 (type-env-insert env2 42 (mono mul-ty))
    env4 (type-env-insert env3 47 (mono div-ty))
    env5 (type-env-insert env4 61 (mono eq-ty))
    env6 (type-env-insert env5 62 (mono gt-ty))
    env7 (type-env-insert env6 60 (mono lt-ty))
    env8 (type-env-insert env7 112 (mono print-ty))]
    env8))

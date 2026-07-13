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
;; + / - / * / % : Int -> Int -> Int (カリー化)
;; 比較演算子: Int -> Int -> Bool
;; print : Int -> Int
(defn typeinfer-init-builtin-env [counter]
  (let [env (type-env-new)
    int-ty (mk-int)
    bool-ty (mk-bool)
    add-ty (typeinfer-builtin-int-binop int-ty)
    sub-ty (typeinfer-builtin-int-binop int-ty)
    mul-ty (typeinfer-builtin-int-binop int-ty)
    div-ty (typeinfer-builtin-int-binop int-ty)
    mod-ty (typeinfer-builtin-int-binop int-ty)
    eq-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    gt-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    lt-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    lte-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    gte-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    eqeq-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    neq-ty (typeinfer-builtin-int-cmp int-ty bool-ty)
    print-ty (mk-fun int-ty int-ty)
    ;; 名前ハッシュ (ASCII コード)
    env1 (type-env-insert env 43 (mono add-ty))
    env2 (type-env-insert env1 45 (mono sub-ty))
    env3 (type-env-insert env2 42 (mono mul-ty))
    env4 (type-env-insert env3 47 (mono div-ty))
    env5 (type-env-insert env4 37 (mono mod-ty))
    env6 (type-env-insert env5 61 (mono eq-ty))
    env7 (type-env-insert env6 62 (mono gt-ty))
    env8 (type-env-insert env7 60 (mono lt-ty))
    env9 (type-env-insert env8 1921 (mono lte-ty))
    env10 (type-env-insert env9 1983 (mono gte-ty))
    env11 (type-env-insert env10 1952 (mono eqeq-ty))
    env12 (type-env-insert env11 1084 (mono neq-ty))
    env13 (type-env-insert env12 112 (mono print-ty))]
    env13))

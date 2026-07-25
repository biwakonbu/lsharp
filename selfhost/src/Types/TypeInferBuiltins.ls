(module Types.TypeInferBuiltins)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; TypeInfer builtins: infer の初期環境だけを分離する
;; 推論本体 (infer-expr / infer-defn) は TypeInfer.ls に残す

(defn typeinfer-builtin-int-binop [int-ty]
  (mk-fun int-ty (mk-fun int-ty int-ty)))

(defn typeinfer-builtin-int-cmp [int-ty bool-ty]
  (mk-fun int-ty (mk-fun int-ty bool-ty)))

(defn typeinfer-builtin-unary [param-ty result-ty]
  (mk-fun param-ty result-ty))

(defn typeinfer-builtin-binary [left-ty right-ty result-ty]
  (mk-fun left-ty (mk-fun right-ty result-ty)))

(defn typeinfer-builtin-ternary [first-ty second-ty third-ty result-ty]
  (mk-fun first-ty (mk-fun second-ty (mk-fun third-ty result-ty))))

;; 型変数を 1 個または 2 個束縛する builtin 型スキームを構築する。
(defn typeinfer-builtin-poly1 [ty var-ty]
  (poly ty (push-int-vector-local (vector-new 1) (ty-name var-ty))))

(defn typeinfer-builtin-poly2 [ty first-var second-var]
  (poly
    ty
    (push-int-vector-local
      (push-int-vector-local (vector-new 2) (ty-name first-var))
      (ty-name second-var))))

;; builtin env は一度に多数の型 object と map snapshot を構築するため、
;; 次の allocation で native GC に回収されないよう root を保持する。
(defn typeinfer-builtin-root-value [value]
  (do
    (root_push value)
    value))

(defn typeinfer-builtin-release-roots [count]
  (if (= count 0)
    0
    (do
      (root_pop)
      (typeinfer-builtin-release-roots (- count 1)))))

;; ビルトイン演算子の型を登録
;; + / - / * / % : Int -> Int -> Int (カリー化)
;; 比較演算子: Int -> Int -> Bool
;; selfhost の関数型はカリー化して保持する。
(defn typeinfer-init-builtin-env [counter]
  (let [env (typeinfer-builtin-root-value (type-env-new))
    int-ty (typeinfer-builtin-root-value (mk-int))
    bool-ty (typeinfer-builtin-root-value (mk-bool))
    string-ty (typeinfer-builtin-root-value (mk-string))
    float-ty (typeinfer-builtin-root-value (mk-float))
    unit-ty (typeinfer-builtin-root-value (mk-unit))
    ;; Rust host の Vector / Map は element type を持たない nominal type である。
    vector-ty (typeinfer-builtin-root-value (mk-vector))
    map-ty (typeinfer-builtin-root-value (mk-map))
    add-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-binop int-ty))
    sub-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-binop int-ty))
    mul-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-binop int-ty))
    div-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-binop int-ty))
    mod-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-binop int-ty))
    eq-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    gt-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    lt-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    lte-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    gte-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    eqeq-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    neq-ty (typeinfer-builtin-root-value (typeinfer-builtin-int-cmp int-ty bool-ty))
    float-binop-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary float-ty float-ty float-ty))
    ;; builtin の束縛型変数は user inference counter を消費しない固定 ID にする。
    ;; instantiate 時にだけ current counter から fresh variable が割り当てられる。
    print-var (typeinfer-builtin-root-value (mk-var 901))
    print-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary print-var unit-ty))
    alloc-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary int-ty int-ty))
    string-length-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary string-ty int-ty))
    string-concat-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary string-ty string-ty string-ty))
    string-eq-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary string-ty string-ty bool-ty))
    print-string-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary string-ty unit-ty))
    string-char-at-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary string-ty int-ty int-ty))
    substring-ty (typeinfer-builtin-root-value (typeinfer-builtin-ternary string-ty int-ty int-ty string-ty))
    int-to-string-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary int-ty string-ty))
    proc-exit-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary int-ty unit-ty))
    vector-new-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary int-ty vector-ty))
    vector-length-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary vector-ty int-ty))
    vector-get-var (typeinfer-builtin-root-value (mk-var 902))
    vector-get-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary vector-ty int-ty vector-get-var))
    vector-set-var (typeinfer-builtin-root-value (mk-var 903))
    vector-set-ty (typeinfer-builtin-root-value (typeinfer-builtin-ternary vector-ty int-ty vector-set-var vector-ty))
    vector-push-var (typeinfer-builtin-root-value (mk-var 904))
    vector-push-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary vector-ty vector-push-var vector-ty))
    map-new-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary unit-ty map-ty))
    map-size-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary map-ty int-ty))
    map-insert-key-var (typeinfer-builtin-root-value (mk-var 905))
    map-insert-value-var (typeinfer-builtin-root-value (mk-var 906))
    map-insert-ty
      (typeinfer-builtin-root-value (typeinfer-builtin-ternary map-ty map-insert-key-var map-insert-value-var map-ty))
    map-get-key-var (typeinfer-builtin-root-value (mk-var 907))
    map-get-value-var (typeinfer-builtin-root-value (mk-var 908))
    map-get-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary map-ty map-get-key-var map-get-value-var))
    map-contains-key-var (typeinfer-builtin-root-value (mk-var 909))
    ;; Rust host は historical comment と異なり Int を返す。互換性を正本にする。
    map-contains-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary map-ty map-contains-key-var int-ty))
    map-remove-key-var (typeinfer-builtin-root-value (mk-var 910))
    map-remove-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary map-ty map-remove-key-var map-ty))
    read-file-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary string-ty string-ty))
    write-file-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary string-ty string-ty int-ty))
    write-file-bytes-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary string-ty vector-ty int-ty))
    file-exists-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary string-ty bool-ty))
    command-line-args-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary unit-ty int-ty))
    command-line-arg-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary int-ty string-ty))
    read-stdin-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary unit-ty string-ty))
    root-push-var (typeinfer-builtin-root-value (mk-var 911))
    root-push-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary root-push-var int-ty))
    root-pop-var (typeinfer-builtin-root-value (mk-var 912))
    root-pop-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary unit-ty root-pop-var))
    root-set-var (typeinfer-builtin-root-value (mk-var 913))
    root-set-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary int-ty root-set-var int-ty))
    ref-new-var (typeinfer-builtin-root-value (mk-var 914))
    ref-new-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary ref-new-var (mk-ref ref-new-var)))
    ref-get-var (typeinfer-builtin-root-value (mk-var 915))
    ref-get-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary (mk-ref ref-get-var) ref-get-var))
    ref-set-var (typeinfer-builtin-root-value (mk-var 916))
    ref-set-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary (mk-ref ref-set-var) ref-set-var unit-ty))
    not-ty (typeinfer-builtin-root-value (typeinfer-builtin-unary bool-ty bool-ty))
    bool-binop-ty (typeinfer-builtin-root-value (typeinfer-builtin-binary bool-ty bool-ty bool-ty))
    ;; 名前ハッシュ (ASCII コード)
    env1 (typeinfer-builtin-root-value (type-env-insert env 43 (mono add-ty)))
    env2 (typeinfer-builtin-root-value (type-env-insert env1 45 (mono sub-ty)))
    env3 (typeinfer-builtin-root-value (type-env-insert env2 42 (mono mul-ty)))
    env4 (typeinfer-builtin-root-value (type-env-insert env3 47 (mono div-ty)))
    env5 (typeinfer-builtin-root-value (type-env-insert env4 37 (mono mod-ty)))
    env6 (typeinfer-builtin-root-value (type-env-insert env5 61 (mono eq-ty)))
    env7 (typeinfer-builtin-root-value (type-env-insert env6 62 (mono gt-ty)))
    env8 (typeinfer-builtin-root-value (type-env-insert env7 60 (mono lt-ty)))
    env9 (typeinfer-builtin-root-value (type-env-insert env8 1921 (mono lte-ty)))
    env10 (typeinfer-builtin-root-value (type-env-insert env9 1983 (mono gte-ty)))
    env11 (typeinfer-builtin-root-value (type-env-insert env10 1952 (mono eqeq-ty)))
    env12 (typeinfer-builtin-root-value (type-env-insert env11 1084 (mono neq-ty)))
    env13 (typeinfer-builtin-root-value (type-env-insert env12 106934957 (typeinfer-builtin-poly1 print-ty print-var)))
    env14 (typeinfer-builtin-root-value (type-env-insert env13 1379 (mono float-binop-ty)))
    env15 (typeinfer-builtin-root-value (type-env-insert env14 1441 (mono float-binop-ty)))
    env16 (typeinfer-builtin-root-value (type-env-insert env15 1348 (mono float-binop-ty)))
    env17 (typeinfer-builtin-root-value (type-env-insert env16 1503 (mono float-binop-ty)))
    env18 (typeinfer-builtin-root-value (type-env-insert env17 87125525333 (mono alloc-ty)))
    env19 (typeinfer-builtin-root-value (type-env-insert env18 1391193567100747810 (mono string-length-ty)))
    env20 (typeinfer-builtin-root-value (type-env-insert env19 1391193566852316240 (mono string-concat-ty)))
    env21 (typeinfer-builtin-root-value (type-env-insert env20 101378218725352 (mono string-eq-ty)))
    env22 (typeinfer-builtin-root-value (type-env-insert env21 2942060250258025265 (mono print-string-ty)))
    env23 (typeinfer-builtin-root-value (type-env-insert env22 6233512424790686798 (mono string-char-at-ty)))
    env24 (typeinfer-builtin-root-value (type-env-insert env23 101391823498833 (mono substring-ty)))
    env25 (typeinfer-builtin-root-value (type-env-insert env24 -6637826915257342139 (mono int-to-string-ty)))
    env26 (typeinfer-builtin-root-value (type-env-insert env25 98761626082613 (mono proc-exit-ty)))
    env27 (typeinfer-builtin-root-value (type-env-insert env26 3208847393531414 (mono vector-new-ty)))
    env28 (typeinfer-builtin-root-value (type-env-insert env27 3361052332089172656 (mono vector-length-ty)))
    env29 (typeinfer-builtin-root-value (type-env-insert env28 3208847393524684 (typeinfer-builtin-poly1 vector-get-ty vector-get-var)))
    env30 (typeinfer-builtin-root-value (type-env-insert env29 3208847393536216 (typeinfer-builtin-poly1 vector-set-ty vector-set-var)))
    env31 (typeinfer-builtin-root-value (type-env-insert env30 99474269199548772 (typeinfer-builtin-poly1 vector-push-ty vector-push-var)))
    env32 (typeinfer-builtin-root-value (type-env-insert env31 99619812783 (mono map-new-ty)))
    env33 (typeinfer-builtin-root-value (type-env-insert env32 3088214349266 (mono map-size-ty)))
    env34 (typeinfer-builtin-root-value (type-env-insert env33 2967773707765834 (typeinfer-builtin-poly2 map-insert-ty map-insert-key-var map-insert-value-var)))
    env35 (typeinfer-builtin-root-value (type-env-insert env34 99619806053 (typeinfer-builtin-poly2 map-get-ty map-get-key-var map-get-value-var)))
    env36 (typeinfer-builtin-root-value (type-env-insert env35 -3820778934353407281 (typeinfer-builtin-poly1 map-contains-ty map-contains-key-var)))
    env37 (typeinfer-builtin-root-value (type-env-insert env36 2967773956947477 (typeinfer-builtin-poly1 map-remove-ty map-remove-key-var)))
    env38 (typeinfer-builtin-root-value (type-env-insert env37 100097347767123 (mono read-file-ty)))
    env39 (typeinfer-builtin-root-value (type-env-insert env38 3246539326542506 (mono write-file-ty)))
    env40 (typeinfer-builtin-root-value (type-env-insert env39 7965480599336288136 (mono write-file-bytes-ty)))
    env41 (typeinfer-builtin-root-value (type-env-insert env40 2680668565995926546 (mono file-exists-ty)))
    env42 (typeinfer-builtin-root-value (type-env-insert env41 5217540237477903124 (mono command-line-args-ty)))
    env43 (typeinfer-builtin-root-value (type-env-insert env42 4333701572691766591 (mono command-line-arg-ty)))
    env44 (typeinfer-builtin-root-value (type-env-insert env43 3103017793106833 (mono read-stdin-ty)))
    env45 (typeinfer-builtin-root-value (type-env-insert env44 100385403511895 (typeinfer-builtin-poly1 root-push-ty root-push-var)))
    env46 (typeinfer-builtin-root-value (type-env-insert env45 3238238822772 (typeinfer-builtin-poly1 root-pop-ty root-pop-var)))
    env47 (typeinfer-builtin-root-value (type-env-insert env46 3238238825349 (typeinfer-builtin-poly1 root-set-ty root-set-var)))
    env48 (typeinfer-builtin-root-value (type-env-insert env47 104162612582 (typeinfer-builtin-poly1 ref-new-ty ref-new-var)))
    env49 (typeinfer-builtin-root-value (type-env-insert env48 104162605852 (typeinfer-builtin-poly1 ref-get-ty ref-get-var)))
    env50 (typeinfer-builtin-root-value (type-env-insert env49 104162617384 (typeinfer-builtin-poly1 ref-set-ty ref-set-var)))
    env51 (typeinfer-builtin-root-value (type-env-insert env50 109267 (mono not-ty)))
    env52 (typeinfer-builtin-root-value (type-env-insert env51 96727 (mono bool-binop-ty)))
    env53 (typeinfer-builtin-root-value (type-env-insert env52 3555 (mono bool-binop-ty)))]
    (do
      (typeinfer-builtin-release-roots 126)
      env53)))

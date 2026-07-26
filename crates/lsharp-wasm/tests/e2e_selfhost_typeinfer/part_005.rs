/// selfhost TypeInfer.ls テスト: 3 引数 defn は 3 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_three_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
        x-hash 120
        y-hash 121
        z-hash 122
        inner-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            (make-var z-hash))
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            inner-plus)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 7) 20)
                    name-hash)
                  3)
                x-hash)
              y-hash)
            z-hash)
        node (vector-push defn-node body-node)
        result (infer-defn node env counter)
        outer (result-type result)
        mid (ty-fr outer)
        inner (ty-fr mid)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag mid))
      (print (ty-tag (ty-fp mid)))
      (print (ty-name (ty-fp mid)))
      (print (ty-tag inner))
      (print (ty-tag (ty-fp inner)))
      (print (ty-name (ty-fp inner)))
      (print (ty-tag (ty-fr inner)))
      (print (ty-name (ty-fr inner)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 12,
        "three-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "three-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "mid type は Fun であるべき");
    assert_eq!(lines[5], "1", "mid param type は Con であるべき");
    assert_eq!(lines[6], "100", "mid param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "inner type は Fun であるべき");
    assert_eq!(lines[8], "1", "inner param type は Con であるべき");
    assert_eq!(
        lines[9], "100",
        "inner param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[10], "1", "inner return type は Con であるべき");
    assert_eq!(
        lines[11], "100",
        "inner return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 4 引数 defn は 4 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_four_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
        w-hash 123
        x-hash 120
        y-hash 121
        z-hash 122
        inner-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var z-hash))
            (make-var w-hash))
        mid-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            inner-plus)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            mid-plus)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 8) 20)
                      name-hash)
                    4)
                  x-hash)
                y-hash)
              z-hash)
            w-hash)
        node (vector-push defn-node body-node)
        result (infer-defn node env counter)
        outer (result-type result)
        level2 (ty-fr outer)
        level3 (ty-fr level2)
        level4 (ty-fr level3)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag level2))
      (print (ty-tag (ty-fp level2)))
      (print (ty-name (ty-fp level2)))
      (print (ty-tag level3))
      (print (ty-tag (ty-fp level3)))
      (print (ty-name (ty-fp level3)))
      (print (ty-tag level4))
      (print (ty-tag (ty-fp level4)))
      (print (ty-name (ty-fp level4)))
      (print (ty-tag (ty-fr level4)))
      (print (ty-name (ty-fr level4)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 15,
        "four-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "four-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(
        lines[6], "100",
        "level2 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(
        lines[9], "100",
        "level3 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(
        lines[12], "100",
        "level4 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[13], "1", "level4 return type は Con であるべき");
    assert_eq!(
        lines[14], "100",
        "level4 return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 5 引数 defn は 5 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_five_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
        v-hash 124
        w-hash 123
        x-hash 120
        y-hash 121
        z-hash 122
        inner-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var w-hash))
            (make-var v-hash))
        level3-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var z-hash))
            inner-plus)
        level2-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            level3-plus)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            level2-plus)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 9) 20)
                      name-hash)
                    5)
                  x-hash)
                y-hash)
              z-hash)
            w-hash)
        node (vector-push defn-node v-hash)
        node (vector-push node body-node)
        result (infer-defn node env counter)
        outer (result-type result)
        level2 (ty-fr outer)
        level3 (ty-fr level2)
        level4 (ty-fr level3)
        level5 (ty-fr level4)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag level2))
      (print (ty-tag (ty-fp level2)))
      (print (ty-name (ty-fp level2)))
      (print (ty-tag level3))
      (print (ty-tag (ty-fp level3)))
      (print (ty-name (ty-fp level3)))
      (print (ty-tag level4))
      (print (ty-tag (ty-fp level4)))
      (print (ty-name (ty-fp level4)))
      (print (ty-tag level5))
      (print (ty-tag (ty-fp level5)))
      (print (ty-name (ty-fp level5)))
      (print (ty-tag (ty-fr level5)))
      (print (ty-name (ty-fr level5)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 18,
        "five-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "five-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(
        lines[6], "100",
        "level2 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(
        lines[9], "100",
        "level3 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(
        lines[12], "100",
        "level4 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(
        lines[15], "100",
        "level5 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[16], "1", "level5 return type は Con であるべき");
    assert_eq!(
        lines[17], "100",
        "level5 return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 6 引数 defn は 6 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_six_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
        u-hash 125
        v-hash 124
        w-hash 123
        x-hash 120
        y-hash 121
        z-hash 122
        inner-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var v-hash))
            (make-var u-hash))
        level4-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var w-hash))
            inner-plus)
        level3-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var z-hash))
            level4-plus)
        level2-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            level3-plus)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            level2-plus)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 10) 20)
                        name-hash)
                      6)
                    x-hash)
                  y-hash)
                z-hash)
              w-hash)
            v-hash)
        node (vector-push defn-node u-hash)
        node (vector-push node body-node)
        result (infer-defn node env counter)
        outer (result-type result)
        level2 (ty-fr outer)
        level3 (ty-fr level2)
        level4 (ty-fr level3)
        level5 (ty-fr level4)
        level6 (ty-fr level5)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag level2))
      (print (ty-tag (ty-fp level2)))
      (print (ty-name (ty-fp level2)))
      (print (ty-tag level3))
      (print (ty-tag (ty-fp level3)))
      (print (ty-name (ty-fp level3)))
      (print (ty-tag level4))
      (print (ty-tag (ty-fp level4)))
      (print (ty-name (ty-fp level4)))
      (print (ty-tag level5))
      (print (ty-tag (ty-fp level5)))
      (print (ty-name (ty-fp level5)))
      (print (ty-tag level6))
      (print (ty-tag (ty-fp level6)))
      (print (ty-name (ty-fp level6)))
      (print (ty-tag (ty-fr level6)))
      (print (ty-name (ty-fr level6)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 21,
        "six-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "six-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(
        lines[6], "100",
        "level2 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(
        lines[9], "100",
        "level3 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(
        lines[12], "100",
        "level4 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(
        lines[15], "100",
        "level5 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[16], "3", "level6 type は Fun であるべき");
    assert_eq!(lines[17], "1", "level6 param type は Con であるべき");
    assert_eq!(
        lines[18], "100",
        "level6 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[19], "1", "level6 return type は Con であるべき");
    assert_eq!(
        lines[20], "100",
        "level6 return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 7 引数 defn は 7 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_seven_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    // 7 引数 defn: (defn f [a b c d e f g] (+ a b)) → Fun(Int, Fun(Int, ...))
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 99
        a-hash 101
        b-hash 102
        c-hash 103
        d-hash 104
        e-hash 105
        f-hash 106
        g-hash 107
        ;; body: (+ a b)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var a-hash))
            (make-var b-hash))
        ;; defn: [20, name-hash, param-count=7, a, b, c, d, e, f, g, body]
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push (vector-new 11) 20)
                            name-hash)
                          7)
                        a-hash)
                      b-hash)
                    c-hash)
                  d-hash)
                e-hash)
              f-hash)
            g-hash)
        node (vector-push defn-node body-node)
        result (infer-defn node env counter)
        outer (result-type result)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "seven-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "7 引数 defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
}

/// selfhost AST.ls テスト: field access constructor / traversal
#[test]
fn test_e2e_selfhost_ast_fieldaccess_helpers() {
    let ast_ls = ast_runtime_module();

    let harness = r#"
(defn main []
  (let [p-hash 99
        field-hash 120
        node (make-fieldaccess (make-var p-hash) field-hash)]
    (do
      (print (if (= (vector-get node 0) (ast-fieldaccess)) 1 0))
      (print (if (= (vector-get (vector-get node 1) 0) (ast-var)) 1 0))
      (print (if (= (vector-get node 2) field-hash) 1 0))
      (print (ast-contains-var node p-hash))
      (print (ast-count-nodes node))
      0)))
"#;

    let combined = format!("{}\n{}", ast_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "fieldaccess AST helper 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "fieldaccess は ast-fieldaccess であるべき");
    assert_eq!(lines[1], "1", "fieldaccess inner は var であるべき");
    assert_eq!(lines[2], "1", "fieldaccess field hash が保持されるべき");
    assert_eq!(lines[3], "1", "fieldaccess inner var が探索できるべき");
    assert_eq!(lines[4], "2", "fieldaccess の node count は 2 であるべき");
}

/// selfhost Parser.ls テスト: field access expression を最小 payload でパースできる
#[test]
fn test_e2e_selfhost_parser_field_access_expr() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(. p x)") 0)
        inner (vector-get node 1)]
    (do
      (print (if (= (vector-get node 0) (ast-fieldaccess)) 1 0))
      (print (if (= (vector-get inner 0) (ast-var)) 1 0))
      (print (if (= (vector-get inner 1) (name-hash "p" 0 1)) 1 0))
      (print (if (= (vector-get node 2) (name-hash "x" 0 1)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "fieldaccess parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "fieldaccess は ast-fieldaccess であるべき");
    assert_eq!(lines[1], "1", "fieldaccess inner は var であるべき");
    assert_eq!(lines[2], "1", "fieldaccess inner hash が一致すべき");
    assert_eq!(lines[3], "1", "fieldaccess field hash が一致すべき");
}

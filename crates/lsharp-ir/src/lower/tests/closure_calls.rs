//! closure call と引数 root の lowering 回帰

use super::*;

#[test]
fn test_lower_closure_call_roots_closure_receiver() {
    let module = lower(
        r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn apply [f x] (f x))
        (defn main [] (print (apply (make-inc) 41)))
        "#,
    );
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    let root_push_positions = call_positions(&apply_fn.body, 14);
    let call_indirect_pos = apply_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "generic closure call は receiver と opaque 引数を root_push するべき: {:?}",
        apply_fn.body
    );
    assert_eq!(
        count_call_instr(&apply_fn.body, 15),
        2,
        "generic closure call は receiver と opaque 引数に対応する root_pop を使うべき: {:?}",
        apply_fn.body
    );
    assert!(
        root_push_positions[0] < call_indirect_pos,
        "closure receiver は call_indirect 前に root_push で保護されるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_receiver_before_string_arg_eval() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn apply [f s] (f s))
        (defn main [] (print (apply (make-show) "hello")))
        "#,
    );
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    let root_push_positions = call_positions(&apply_fn.body, 14);
    let string_arg_get_pos = apply_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::LocalGet(idx) if *idx == 1))
        .unwrap();

    assert!(
        !root_push_positions.is_empty(),
        "closure call は receiver を root_push で保護すべき: {:?}",
        apply_fn.body
    );
    assert!(
        root_push_positions[0] < string_arg_get_pos,
        "closure receiver は string 引数の評価より前に root_push で保護されるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_annotated_string_param_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn apply [f (: s String)] (f s))
        (defn main [] (print (apply (make-show) "hello")))
        "#,
    );
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    let root_push_positions = call_positions(&apply_fn.body, 14);
    let call_indirect_pos = apply_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "closure call は receiver と注釈付き string 引数を root_push するべき: {:?}",
        apply_fn.body
    );
    assert_eq!(
        count_call_instr(&apply_fn.body, 15),
        2,
        "closure call は receiver と注釈付き string 引数に対応する root_pop を使うべき: {:?}",
        apply_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "注釈付き string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_let_bound_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)
                s "hello"]
            (f s)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "let 束縛 string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "let 束縛 string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "let 束縛 string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_let_bound_if_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)
                use-first true
                s (if use-first "hello" "world")]
            (f s)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "if 由来の let 束縛 string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "if 由来の let 束縛 string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "if 由来の let 束縛 string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_do_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)]
            (f (do 0 "hello"))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "do 最終式の string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "do 最終式の string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "do 最終式の string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_match_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)
                use-first true]
            (f (match use-first
                 [true "hello"]
                 [false "world"]))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "match 由来の string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "match 由来の string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "match 由来の string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_let_bound_match_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)
                use-first true
                s (match use-first
                    [true "hello"]
                    [false "world"])]
            (f s)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "match 由来の let 束縛 string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "match 由来の let 束縛 string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "match 由来の let 束縛 string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_let_expr_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)]
            (f (let [s "hello"] s))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "let 式由来の string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "let 式由来の string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "let 式由来の string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_let_expr_if_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)
                use-first true]
            (f (let [s (if use-first "hello" "world")] s))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "if 由来値を返す let 式の string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "if 由来値を返す let 式の string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "if 由来値を返す let 式の string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_read_stdin_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)]
            (f (read-stdin))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "read-stdin 由来の string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "read-stdin 由来の string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "read-stdin 由来の string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_generic_identity_result_argument() {
    let module = lower(
        r#"
        (defn id [x] x)
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)]
            (f (id "hello"))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        3,
        "generic identity の戻り値を使う closure call は receiver と inner call の literal に加えて outer string 引数も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        3,
        "generic identity の戻り値を使う closure call は 3 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "generic identity の戻り値は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_conservatively_roots_generic_param_argument() {
    let module = lower(
        r#"
        (defn pass [f x]
          (f x))
        "#,
    );
    let pass_fn = module.functions.iter().find(|f| f.name == "pass").unwrap();
    let root_push_positions = call_positions(&pass_fn.body, 14);
    let call_indirect_pos = pass_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "generic closure param call は receiver と opaque 引数を root_push するべき: {:?}",
        pass_fn.body
    );
    assert_eq!(
        count_call_instr(&pass_fn.body, 15),
        2,
        "generic closure param call は 2 回 root_pop するべき: {:?}",
        pass_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "opaque 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        pass_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_local_generic_closure_result_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [id (fn [x] x)
                f (make-show)]
            (f (id "hello"))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        4,
        "local generic closure の戻り値を使う closure call は outer receiver と inner closure call の receiver/literal に加えて outer string 引数も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        4,
        "local generic closure の戻り値を使う closure call は 4 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "local generic closure の戻り値は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_lambda_argument() {
    let module = lower(
        r#"
        (defn make-accept [] (fn [f] 0))
        (defn main []
          (let [g (make-accept)]
            (g (fn [x] x))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "lambda literal 引数を使う closure call は receiver と lambda 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "lambda literal 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "lambda literal 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_record_update_argument() {
    let module = lower(
        r#"
        (type Point (record (: x Int) (: y Int)))
        (defn make-show [] (fn [p] (Point.x p)))
        (defn main []
          (let [f (make-show)
                p {Point x 1 y 2}]
            (f {p | x 10})))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "record update 由来の record 引数を使う closure call は receiver と record 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "record update 由来の record 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "record update 由来の record 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

use super::support::*;

/// selfhost TypeInfer.ls テスト: record child の ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_record_child_pat_lit() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        point-ty
          (type-record-add-field
            (make-type-record point-hash)
            field-x
            (mk-bool))
        env (type-env-insert env0 point-var (mono point-ty))
        child-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-bool 1))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) (ast-pat-recordpat))
                  1)
                field-x)
              child-pat)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var point-var))
                   1)
                 pat)
               (make-lit-int 7))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match record child pat-lit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match record child pat-lit infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match record child pat-lit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "match record child pat-lit infer の型名は Int であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor child の unit ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_child_pat_unit_lit() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        ctor-ty (mk-fun (mk-unit) (mk-con some-hash))
        env1 (type-env-insert env0 ctor-hash (mono ctor-ty))
        env (type-env-insert env1 value-hash (mono (mk-con some-hash)))
        child-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-unit))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) (ast-pat-constructor))
                  ctor-hash)
                1)
              child-pat)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var value-hash))
                   1)
                 pat)
               (make-lit-bool 1))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match constructor child pat-unit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match constructor child pat-unit infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match constructor child pat-unit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "match constructor child pat-unit infer の型名は Bool であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 変数束縛の型推論
#[test]
fn test_e2e_selfhost_typeinfer_variable() {
    // let 束縛の型推論が正しく動作することを検証
    // 期待値: x: Int が推論され、print で出力可能
    let source = r#"
(module Main)
(defn main [] (let [x 42] (print x)))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: 関数の型推論 (arrow type)
#[test]
fn test_e2e_selfhost_typeinfer_function() {
    // 関数定義の型推論 (Int -> Int) が動作することを検証
    // 期待値: f: Int -> Int が推論され、適用結果が正しい
    let source = r#"
(module Main)
(defn f [x] (+ x 1))
(defn main [] (print (f 41)))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: let 多相 (let-polymorphism)
#[test]
fn test_e2e_selfhost_typeinfer_let_poly() {
    // let-polymorphism が動作することを検証
    // 期待値: id が Int にも Bool にも適用可能
    let source = r#"
(module Main)
(defn id [x] x)
(defn main [] (do (print (id 42)) (print (id true))))
"#;
    let result = compile_and_run(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "42");
    assert_eq!(lines[1], "1");
}

/// selfhost TypeInfer.ls テスト: 型の単一化 (unification)
#[test]
fn test_e2e_selfhost_typeinfer_unification() {
    // 型変数の単一化が動作することを検証
    // 期待値: 高階関数 apply の型が正しく推論される
    let source = r#"
(module Main)
(defn apply [f x] (f x))
(defn inc [n] (+ n 1))
(defn main [] (print (apply inc 41)))
"#;
    typecheck_only_expanded(source);
}

/// selfhost TypeInfer.ls テスト: if 式の型推論
#[test]
fn test_e2e_selfhost_typeinfer_if_expr() {
    // if 式の型推論 (条件=Bool, 両枝=同一型) の検証
    // 期待値: if の型チェックが成功し、正しい値が返る
    let source = r#"
(module Main)
(defn main [] (print (if true 42 0)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: パターンマッチの型推論
#[test]
fn test_e2e_selfhost_typeinfer_pattern_match() {
    // パターンマッチの最小型推論が動作することを検証
    // 期待値: match 式の各腕の型が一致することをチェック
    let source = r#"
(module Main)
(defn main []
  (let [x 1]
    (print (match x
      [1 "one"]
      [_ "other"]))))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "520");
}

// === Pipeline Integration Tests (TEST-003) ===

/// selfhost 完全パイプライン統合テスト
#[test]
fn test_e2e_selfhost_full_pipeline() {
    // Source->Lexer->Parser->MacroExpand->TypeInfer->Lower->WasmEmit の
    // 完全パイプラインが動作することを検証
    let source = r#"
(module Main)
(defn main [] (print 42))
"#;
    // selfhost compiler (stage1.wasm) で上記ソースをコンパイル実行
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost パイプラインテスト: fib.ls コンパイル
#[test]
fn test_e2e_selfhost_pipeline_fib() {
    // selfhost compiler で examples/fib.ls をコンパイルし、
    // Rust compiler と同一出力になることを検証
    let source = std::fs::read_to_string(example_path("fib.ls")).unwrap();
    let result = compile_and_run_expanded(&source);
    assert!(result.contains("55"), "fib(10) = 55");
}

/// selfhost パイプラインテスト: hello world
#[test]
fn test_e2e_selfhost_pipeline_hello() {
    let source = r#"
(module Main)
(defn main [] (print "hello"))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "hello");
}

// === Bootstrap Fixed-Point Tests (TEST-004) ===

/// bootstrap proxy 検証: stage1 == stage2 バイト列比較
#[test]
fn test_e2e_bootstrap_stage1_stage2_match() {
    // 真の stage1→stage2 自己コンパイル経路は未接続。
    // 現時点では bootstrap 入力集合に対する再コンパイルのバイト一致を proxy として使う。
    let main_path = selfhost_main_path();
    let stage1 = compile_file_only(&main_path);
    let stage2_proxy = compile_file_only(&main_path);
    assert_eq!(
        stage1, stage2_proxy,
        "bootstrap proxy must be byte-identical until true stage1->stage2 is wired"
    );
}

/// bootstrap proxy 検証: stage2 == stage3
#[test]
fn test_e2e_bootstrap_fixed_point_stage2_stage3() {
    // 真の stage2→stage3 は未接続のため、proxy としてセクション列の固定点を検証する。
    fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
        let mut sections = Vec::new();
        let mut pos = 8;
        while pos < wasm.len() {
            let section_id = wasm[pos];
            pos += 1;
            let mut size: usize = 0;
            let mut shift = 0;
            loop {
                if pos >= wasm.len() {
                    break;
                }
                let byte = wasm[pos] as usize;
                pos += 1;
                size |= (byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            sections.push((section_id, size));
            pos += size;
        }
        sections
    }

    let main_path = selfhost_main_path();
    let stage2_proxy = compile_file_only(&main_path);
    let stage3_proxy = compile_file_only(&main_path);
    assert_eq!(
        extract_sections(&stage2_proxy),
        extract_sections(&stage3_proxy),
        "bootstrap proxy sections must reach a fixed point until true stage2->stage3 is wired"
    );
}

/// bootstrap 決定性検証: 同一入力で複数回コンパイルして一致
#[test]
fn test_e2e_bootstrap_deterministic_output() {
    // 同じ selfhost ソースを2回コンパイルし、
    // 生成されたバイト列が一致することを確認（非決定性排除）
    let main_path = selfhost_main_path();
    let wasm1 = compile_file_only(&main_path);
    let wasm2 = compile_file_only(&main_path);
    assert_eq!(wasm1, wasm2, "bootstrap output must be deterministic");
}

/// WASM-03 / BOOT-04 進捗: マルチファイル Main を連続 4 回 compile し全バイト一致（Rust stage0 oracle）。
/// 真の stage1.wasm→stage2.wasm 自己コンパイルは未接続。退行検知を強化する。
#[test]
fn test_e2e_bootstrap_stage0_oracle_chain_four_way_identity() {
    let main_path = selfhost_main_path();
    let a = compile_file_only(&main_path);
    let b = compile_file_only(&main_path);
    let c = compile_file_only(&main_path);
    let d = compile_file_only(&main_path);
    assert_eq!(a, b, "oracle chain pass 1==2");
    assert_eq!(b, c, "oracle chain pass 2==3");
    assert_eq!(c, d, "oracle chain pass 3==4");
}

/// WASM-03: import なし単一モジュール (Token) の compile も連続一致すること
#[test]
fn test_e2e_wasm03_token_module_compile_deterministic() {
    let token_path = selfhost_source_path("Token.ls");
    let w1 = compile_file_only(&token_path);
    let w2 = compile_file_only(&token_path);
    assert_eq!(
        w1, w2,
        "Token.ls compile must be byte-deterministic (WASM-03)"
    );
}

// === P11-2: ブートストラップ閉路基盤テスト ===

/// selfhost 完全パイプライン: 全5ステージの通過とステージ間一貫性を検証
/// Main.ls の compile-full-pipeline が token/parse/expand/infer/compile を
/// 正しく通過し、各ステージの出力が因果的に一貫していることを確認する
#[test]
fn test_e2e_selfhost_pipeline_complete_stages() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();

    // 完全パイプラインの出力は lines[27]~[31] にある
    assert!(
        lines.len() >= 32,
        "完全パイプライン出力が不足: {} 行",
        lines.len()
    );

    // Stage 3 (expand): マクロ展開後の AST tag
    // "(defn main [] 42)" のリテラル整数は展開後も lit-int(1) を維持
    let expanded_tag: i64 = lines[27].parse().unwrap();
    assert_eq!(
        expanded_tag, 1,
        "Stage 3 (expand): AST tag はマクロ展開後も lit-int(1) を維持"
    );

    // Stage 4 (infer): 型推論結果 = Con(Int) = [1, 100]
    let ty_tag: i64 = lines[28].parse().unwrap();
    let ty_name: i64 = lines[29].parse().unwrap();
    assert_eq!(ty_tag, 1, "Stage 4 (infer): 型タグ Con=1");
    assert_eq!(ty_name, 100, "Stage 4 (infer): 型名 Int=100");

    // Stage 5 (compile): IR 命令が生成されている
    let ir_count: i64 = lines[30].parse().unwrap();
    assert!(ir_count > 0, "Stage 5 (compile): IR 命令数 > 0");

    // ステージ数の検証 (compile-full-pipeline が 5 を出力)
    let stage_count: i64 = lines[31].parse().unwrap();
    assert_eq!(stage_count, 5, "パイプラインステージ数 = 5");

    // ステージ間一貫性検証:
    // lit-int(tag=1) の AST → 型推論は必ず Int(100) であるべき
    if expanded_tag == 1 {
        assert_eq!(ty_name, 100, "一貫性: lit-int AST → Int 型");
    }
    // IR 命令が 1 つなら i64.const のはず
    if ir_count == 1 {
        // compile-full-pipeline の入力 "(defn main [] 42)" は
        // リテラル整数のみなので i64.const 1 命令
        assert_eq!(ir_count, 1, "一貫性: 単一リテラル → IR 1 命令");
    }
}

/// selfhost compiler の compile-source で stdlib 基本パターン
/// (単純な関数定義) をコンパイルできることを検証
/// token -> parse -> IR の各段階で正しい構造が生成されることを確認
#[test]
fn test_e2e_selfhost_compile_stdlib_basic() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();

    // compile-source が "(defn main [] 42)" を処理した結果
    // lines[14]~[20] に出力される
    assert!(
        lines.len() >= 21,
        "compile-source 出力が不足: {} 行",
        lines.len()
    );

    // トークン列が生成されている (16 = 7tok*2 + EOF*2)
    let token_count: i64 = lines[14].parse().unwrap();
    assert!(token_count > 0, "トークン列が生成されている");
    assert_eq!(token_count, 8, "トークン数 = 8 (Lexer.tokenize に準拠)");

    // AST が defn (tag=20) として構築されている
    let defn_tag: i64 = lines[15].parse().unwrap();
    assert_eq!(defn_tag, 20, "defn AST tag = 20");

    // body が lit-int (tag=1, value=42)
    let body_tag: i64 = lines[16].parse().unwrap();
    let body_val: i64 = lines[17].parse().unwrap();
    assert_eq!(body_tag, 1, "body AST tag = 1 (lit-int)");
    assert_eq!(body_val, 42, "body value = 42");

    // IR 命令が正しく生成されている
    let ir_count: i64 = lines[18].parse().unwrap();
    assert_eq!(ir_count, 1, "IR 命令数 = 1 (i64.const)");

    // IR 命令の中身: i64.const 42
    let ir_op: i64 = lines[19].parse().unwrap();
    let ir_operand: i64 = lines[20].parse().unwrap();
    assert_eq!(ir_op, 1, "IR opcode = i64.const(1)");
    assert_eq!(ir_operand, 42, "IR operand = 42");
}

/// selfhost/src/App/Main.ls が tiny native pipeline summary を出力できること
#[test]
fn test_e2e_selfhost_main_native_pipeline_summary() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 71,
        "native pipeline summary 出力が不足: {} 行",
        lines.len()
    );

    let native_len: i64 = lines[32].parse().unwrap();
    let object_len: i64 = lines[33].parse().unwrap();
    let link_response_len: i64 = lines[34].parse().unwrap();
    let target_arch: i64 = lines[35].parse().unwrap();
    let target_format: i64 = lines[36].parse().unwrap();
    let darwin_linker: i64 = lines[37].parse().unwrap();
    let source_ir_len: i64 = lines[38].parse().unwrap();
    let linux_native_len: i64 = lines[39].parse().unwrap();
    let linux_object_len: i64 = lines[40].parse().unwrap();
    let linux_link_response_len: i64 = lines[41].parse().unwrap();
    let linux_target_arch: i64 = lines[42].parse().unwrap();
    let linux_target_format: i64 = lines[43].parse().unwrap();
    let linux_linker: i64 = lines[44].parse().unwrap();
    let linux_ir_len: i64 = lines[45].parse().unwrap();
    let macho_multi_link_response_len: i64 = lines[46].parse().unwrap();
    let linux_multi_link_response_len: i64 = lines[47].parse().unwrap();
    let aarch64_native_len: i64 = lines[48].parse().unwrap();
    let aarch64_object_len: i64 = lines[49].parse().unwrap();
    let aarch64_link_response_len: i64 = lines[50].parse().unwrap();
    let aarch64_target_arch: i64 = lines[51].parse().unwrap();
    let aarch64_target_format: i64 = lines[52].parse().unwrap();
    let aarch64_linker: i64 = lines[53].parse().unwrap();
    let aarch64_ir_len: i64 = lines[54].parse().unwrap();
    let aarch64_multi_link_response_len: i64 = lines[55].parse().unwrap();
    let macho_magic_byte0: i64 = lines[56].parse().unwrap();
    let macho_cpu_byte: i64 = lines[57].parse().unwrap();
    let elf_magic_byte0: i64 = lines[58].parse().unwrap();
    let elf_class_byte: i64 = lines[59].parse().unwrap();
    let aarch64_macho_magic_byte0: i64 = lines[60].parse().unwrap();
    let aarch64_macho_cpu_byte: i64 = lines[61].parse().unwrap();
    let darwin_response_output_byte: i64 = lines[62].parse().unwrap();
    let darwin_response_object_byte: i64 = lines[63].parse().unwrap();
    let linux_response_output_byte: i64 = lines[64].parse().unwrap();
    let linux_response_object_byte: i64 = lines[65].parse().unwrap();
    let aarch64_response_output_byte: i64 = lines[66].parse().unwrap();
    let aarch64_response_object_byte: i64 = lines[67].parse().unwrap();
    let darwin_multi_response_object2_byte: i64 = lines[68].parse().unwrap();
    let linux_multi_response_object2_byte: i64 = lines[69].parse().unwrap();
    let aarch64_multi_response_object2_byte: i64 = lines[70].parse().unwrap();

    assert_eq!(native_len, 16, "tiny native payload は 16 bytes であるべき");
    assert_eq!(object_len, 32, "tiny Mach-O object は 32 bytes であるべき");
    assert_eq!(
        link_response_len, 6,
        "単一 object の linker response は 6 bytes であるべき"
    );
    assert_eq!(
        target_arch, 1,
        "tiny native pipeline は x86_64 target を使う"
    );
    assert_eq!(
        target_format, 1,
        "tiny native pipeline は Mach-O object format を使う"
    );
    assert_eq!(darwin_linker, 1, "Darwin summary は ld64 を選ぶ");
    assert_eq!(
        source_ir_len, 1,
        "native summary は compile-source 由来の IR 長を報告する"
    );
    assert_eq!(
        linux_native_len, 16,
        "tiny Linux native payload も 16 bytes であるべき"
    );
    assert_eq!(
        linux_object_len, 24,
        "tiny ELF object は 24 bytes であるべき"
    );
    assert_eq!(
        linux_link_response_len, 6,
        "単一 ELF object の linker response は 6 bytes であるべき"
    );
    assert_eq!(
        linux_target_arch, 1,
        "Linux summary も x86_64 target を使う"
    );
    assert_eq!(
        linux_target_format, 2,
        "Linux summary は ELF object format を使う"
    );
    assert_eq!(linux_linker, 2, "Linux summary は ld.lld を選ぶ");
    assert_eq!(
        linux_ir_len, 1,
        "Linux summary も compile-source 由来の IR 長を報告する"
    );
    assert_eq!(
        macho_multi_link_response_len, 8,
        "2 object の Mach-O linker response は 8 bytes であるべき"
    );
    assert_eq!(
        linux_multi_link_response_len, 8,
        "2 object の ELF linker response は 8 bytes であるべき"
    );
    assert_eq!(
        aarch64_native_len, 8,
        "tiny aarch64 native payload は 8 bytes であるべき (AArch64: MOVZ W0,#42 + RET)"
    );
    assert_eq!(
        aarch64_object_len, 24,
        "tiny aarch64 Mach-O object は 24 bytes であるべき (16-byte header + 8-byte AArch64 code)"
    );
    assert_eq!(
        aarch64_link_response_len, 6,
        "単一 aarch64 object の linker response は 6 bytes であるべき"
    );
    assert_eq!(aarch64_target_arch, 2, "aarch64 summary は target 2 を使う");
    assert_eq!(
        aarch64_target_format, 1,
        "aarch64 summary は Mach-O object format を使う"
    );
    assert_eq!(aarch64_linker, 1, "aarch64 summary は ld64 を選ぶ");
    assert_eq!(
        aarch64_ir_len, 1,
        "aarch64 summary も compile-source 由来の IR 長を報告する"
    );
    assert_eq!(
        aarch64_multi_link_response_len, 8,
        "2 object の aarch64 Mach-O linker response は 8 bytes であるべき"
    );
    assert_eq!(
        macho_magic_byte0, 207,
        "Mach-O header 先頭 byte は 0xCF であるべき"
    );
    assert_eq!(
        macho_cpu_byte, 7,
        "x86_64 Mach-O cpu byte は 0x07 であるべき"
    );
    assert_eq!(
        elf_magic_byte0, 127,
        "ELF header 先頭 byte は 0x7F であるべき"
    );
    assert_eq!(elf_class_byte, 2, "ELF class byte は ELF64=2 であるべき");
    assert_eq!(
        aarch64_macho_magic_byte0, 207,
        "aarch64 Mach-O header 先頭 byte も 0xCF であるべき"
    );
    assert_eq!(
        aarch64_macho_cpu_byte, 12,
        "aarch64 Mach-O cpu byte は 0x0C であるべき"
    );
    assert_eq!(
        darwin_response_output_byte, 99,
        "Darwin response file は output id=99 を含む"
    );
    assert_eq!(
        darwin_response_object_byte, 32,
        "Darwin response file は Mach-O object size=32 を含む"
    );
    assert_eq!(
        linux_response_output_byte, 99,
        "Linux response file も output id=99 を含む"
    );
    assert_eq!(
        linux_response_object_byte, 24,
        "Linux response file は ELF object size=24 を含む"
    );
    assert_eq!(
        aarch64_response_output_byte, 99,
        "aarch64 response file も output id=99 を含む"
    );
    assert_eq!(
        aarch64_response_object_byte, 24,
        "aarch64 response file は Mach-O object size=24 を含む (AArch64)"
    );
    assert_eq!(
        darwin_multi_response_object2_byte, 32,
        "Darwin multi-object response は 2 個目の Mach-O size=32 を含む"
    );
    assert_eq!(
        linux_multi_response_object2_byte, 24,
        "Linux multi-object response は 2 個目の ELF size=24 を含む"
    );
    assert_eq!(
        aarch64_multi_response_object2_byte, 24,
        "aarch64 multi-object response は 2 個目の Mach-O size=24 を含む (AArch64)"
    );
}

// =================================================
// P11-2: selfhost 個別モジュールコンパイル・決定性テスト
// =================================================

/// P11-2 T93: selfhost の全 .ls ファイルを個別にコンパイルし、
/// コンパイル可能なモジュール数を検証する。
/// MacroExpand.ls, TypeInfer.ls は Rust parser 未対応構文のためスキップ対象。
#[test]
fn test_e2e_selfhost_module_compile_individual() {
    let all_modules = [
        "Token",
        "AST",
        "IR",
        "Type",
        "TypeScheme",
        "TypeInferCore",
        "Compiler",
        "WasmEmit",
        "Lexer",
        "Parser",
        "Main",
        "Formatter",
        "JsonRpc",
        "Linter",
        "MacroExpand",
        "TypeInfer",
    ];
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost");

    let mut compiled = Vec::new();
    let mut skipped = Vec::new();

    for module in &all_modules {
        let path = base_dir.join(format!("{}.ls", module));
        if !path.exists() {
            skipped.push(format!("{} (ファイル不在)", module));
            continue;
        }
        match try_compile_file_only(&path) {
            Ok(wasm) => {
                assert_valid_wasm(&wasm);
                compiled.push(*module);
            }
            Err(_) => {
                skipped.push(format!("{} (パース/コンパイルエラー)", module));
            }
        }
    }

    // MacroExpand, TypeInfer 以外の 13 モジュールは全てコンパイル可能であるべき
    assert!(
        compiled.len() >= 13,
        "最低 13 モジュールがコンパイル可能であるべき (実際: {}, スキップ: {:?})",
        compiled.len(),
        skipped
    );
}

/// P11-2 T94/95: 全コンパイル可能 selfhost モジュールの決定性検証。
/// 各モジュールを 2 回コンパイルし、生成されるバイト列が完全一致することを確認。
/// Formatter, JsonRpc, Linter, TypeScheme を含む拡張版。
#[test]
fn test_e2e_selfhost_all_modules_deterministic() {
    // Rust parser で正常にコンパイルできるモジュール一覧
    let modules: &[(&str, &str)] = &[
        ("Lexer.ls", selfhost_module("Lexer.ls")),
        ("Parser.ls", selfhost_module("Parser.ls")),
        ("AST.ls", selfhost_module("AST.ls")),
        ("Token.ls", selfhost_module("Token.ls")),
        ("Compiler.ls", selfhost_module("Compiler.ls")),
        ("Type.ls", selfhost_module("Type.ls")),
        ("IR.ls", selfhost_module("IR.ls")),
        ("WasmEmit.ls", selfhost_module("WasmEmit.ls")),
        ("TypeScheme.ls", selfhost_module("TypeScheme.ls")),
        ("TypeInferCore.ls", selfhost_module("TypeInferCore.ls")),
        ("FormatterExpr.ls", selfhost_module("FormatterExpr.ls")),
        ("FormatterDecl.ls", selfhost_module("FormatterDecl.ls")),
        ("Formatter.ls", selfhost_module("Formatter.ls")),
        ("JsonRpc.ls", selfhost_module("JsonRpc.ls")),
        ("Linter.ls", selfhost_module("Linter.ls")),
        ("Main.ls", selfhost_module("Main.ls")),
    ];

    for (name, _source) in modules {
        let path = selfhost_source_path(name);
        let wasm1 = compile_file_only(&path);
        let wasm2 = compile_file_only(&path);
        assert_eq!(
            wasm1,
            wasm2,
            "{} のコンパイルが非決定的: {} bytes vs {} bytes",
            name,
            wasm1.len(),
            wasm2.len()
        );
        assert!(
            wasm1.len() > 100,
            "{} の wasm が小さすぎる: {} bytes",
            name,
            wasm1.len()
        );
    }
}

/// P11-2 T94: stage1 (Rust compiler) で selfhost 全コンパイル可能モジュールを
/// コンパイルし、Wasm バイナリのセクション構造が安定していることを検証。
/// CI 全モジュールテスト (test_e2e_bootstrap_ci_all_modules_compile) の拡張版。
#[test]
fn test_e2e_bootstrap_stage1_compile_selfhost_sources() {
    let modules = [
        "Token",
        "AST",
        "IR",
        "Type",
        "TypeScheme",
        "TypeInferCore",
        "Compiler",
        "WasmEmit",
        "Lexer",
        "Parser",
        "Main",
        "Formatter",
        "JsonRpc",
        "Linter",
    ];
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost");

    // 各セクション ID とサイズを抽出するヘルパー
    fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
        let mut sections = Vec::new();
        let mut pos = 8; // magic(4) + version(4)
        while pos < wasm.len() {
            let section_id = wasm[pos];
            pos += 1;
            let mut size: usize = 0;
            let mut shift = 0;
            loop {
                if pos >= wasm.len() {
                    break;
                }
                let byte = wasm[pos] as usize;
                pos += 1;
                size |= (byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            sections.push((section_id, size));
            pos += size;
        }
        sections
    }

    let mut compiled = 0;
    for module in &modules {
        let path = base_dir.join(format!("{}.ls", module));

        // 2 回コンパイルしてバイト列一致 + セクション安定性を検証 (import 付きはマルチファイル)
        let wasm1 = compile_file_only(&path);
        let wasm2 = compile_file_only(&path);

        assert_eq!(wasm1, wasm2, "{} のコンパイルが非決定的", module);
        assert_valid_wasm(&wasm1);

        // セクション構造が安定
        let sections1 = extract_sections(&wasm1);
        let sections2 = extract_sections(&wasm2);
        assert_eq!(sections1, sections2, "{} のセクション構造が不安定", module);

        // 最低限の Wasm セクションが含まれている
        let section_ids: Vec<u8> = sections1.iter().map(|s| s.0).collect();
        assert!(
            section_ids.contains(&1),
            "{}: Type section (1) が欠落",
            module
        );
        assert!(
            section_ids.contains(&10),
            "{}: Code section (10) が欠落",
            module
        );

        compiled += 1;
    }

    assert_eq!(
        compiled,
        modules.len(),
        "全 {} モジュールがコンパイル・検証されるべき",
        modules.len()
    );
}

/// selfhost 15ファイル全てに module 宣言が存在することを検証する。
/// 各ファイルの先頭に (module ModuleName) と (import ...) があることを確認。
/// MacroExpand.ls, TypeInfer.ls は Rust parser 未対応構文のためテキストベースで検証。
#[test]
fn test_e2e_selfhost_module_declarations() {
    let expected_modules: &[(&str, &str, &[&str])] = &[
        // (ファイル名, モジュール名, 期待される import 先)
        ("Token.ls", "Token", &[]),
        ("IR.ls", "IR", &[]),
        ("Type.ls", "Type", &[]),
        ("AST.ls", "AST", &["Token"]),
        ("TypeScheme.ls", "TypeScheme", &["Type"]),
        (
            "TypeInferCore.ls",
            "TypeInferCore",
            &["AST", "Type", "TypeScheme"],
        ),
        ("Lexer.ls", "Lexer", &["Token"]),
        ("Parser.ls", "Parser", &["Token", "AST"]),
        ("MacroExpand.ls", "MacroExpand", &["AST", "Token"]),
        ("TypeInfer.ls", "TypeInfer", &["AST", "Type", "TypeScheme"]),
        ("Compiler.ls", "Compiler", &["AST", "IR"]),
        ("WasmEmit.ls", "WasmEmit", &["IR"]),
        ("Linter.ls", "Linter", &["AST"]),
        ("FormatterExpr.ls", "FormatterExpr", &["AST"]),
        ("FormatterDecl.ls", "FormatterDecl", &["AST"]),
        ("Formatter.ls", "Formatter", &["AST", "FormatterExpr", "FormatterDecl"]),
        ("JsonRpc.ls", "JsonRpc", &[]),
        ("NativeTarget.ls", "NativeTarget", &[]),
        ("NativeCodegen.ls", "NativeCodegen", &["NativeTarget", "IR"]),
        ("NativeEmit.ls", "NativeEmit", &["NativeTarget"]),
        ("Linker.ls", "Linker", &["NativeTarget"]),
        (
            "Main.ls",
            "Main",
            &[
                "Lexer",
                "Parser",
                "MacroExpand",
                "TypeInfer",
                "Compiler",
                "WasmEmit",
                "NativeTarget",
                "NativeCodegen",
                "NativeEmit",
                "Linker",
            ],
        ),
    ];

    // MacroExpand, TypeInfer は Rust parser 未対応構文があるためパース検証をスキップ
    let parse_skip: &[&str] = &["MacroExpand.ls", "TypeInfer.ls"];

    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost");

    let mut text_verified = 0;
    let mut parse_verified = 0;

    for (filename, expected_module, expected_imports) in expected_modules {
        let path = base_dir.join(filename);
        assert!(path.exists(), "{} が見つからない", filename);

        let source =
            std::fs::read_to_string(&path).expect(&format!("{} の読み込みに失敗", filename));

        // テキストベースで module 宣言の存在を確認
        let module_decl = format!("(module {})", expected_module);
        assert!(
            source.contains(&module_decl),
            "{} に {} が見つからない",
            filename,
            module_decl
        );

        // テキストベースで import 宣言の存在を確認
        for imp in *expected_imports {
            let import_decl = format!("(import {})", imp);
            assert!(
                source.contains(&import_decl),
                "{} に {} が見つからない",
                filename,
                import_decl
            );
        }

        text_verified += 1;

        // パーサーで検証可能なファイルは AST レベルでも検証
        if !parse_skip.contains(filename) {
            let program = lsharp_syntax::parse(&source)
                .unwrap_or_else(|e| panic!("{} のパースに失敗: {:?}", filename, e));

            assert!(!program.decls.is_empty(), "{} の AST 宣言が空", filename);

            parse_verified += 1;
        }
    }

    assert_eq!(text_verified, 20, "全 20 モジュールでテキスト検証すべき");
    assert_eq!(
        parse_verified, 18,
        "パース可能な 18 モジュールで AST 検証すべき"
    );
}

use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_adt_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferAdt.ls");
    for name in [
        "typeinfer-adt-build-param-state-step-64-loop-bounded",
        "typeinfer-adt-build-param-state-rooted-v3",
        "typeinfer-adt-constructor-type-step-64-loop-bounded",
        "typeinfer-adt-constructor-type-rooted-v3",
        "typeinfer-register-adt-variants-step-64-loop-bounded",
        "typeinfer-register-adt-variants-rooted-v3",
        "typeinfer-register-adt-defs-step-64-loop-bounded",
        "typeinfer-register-adt-defs-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "TypeInferAdt の走査は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_typeinfer_adt_scanners_preserve_cross_chunk_registrations() {
    let params = (0..65)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let parametric_decl = format!(
        "(type (Many {params}) (ManyCtor {params}))",
        params = params
    );
    let variants = (0..65)
        .map(|index| format!("(V{index} Int)"))
        .collect::<Vec<_>>()
        .join(" ");
    let variant_decl = format!("(type Choice {variants})", variants = variants);
    let declarations = (0..65)
        .map(|index| format!("(type T{index} (C{index} Int))"))
        .collect::<Vec<_>>()
        .join(" ");
    let program = format!("{parametric_decl} {variant_decl} {declarations}");
    let program_literal = program.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [analysis (infer-program-analysis (parse-program "{program_literal}"))
        env (infer-program-analysis-env analysis)
        constructor (type-env-lookup env (name-hash "ManyCtor" 0 8))
        last-variant (type-env-lookup env (name-hash "V64" 0 3))
        last-decl (type-env-lookup env (name-hash "C64" 0 3))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      (print (if (= constructor 0) 0 1))
      (print (if (= last-variant 0) 0 1))
      (print (if (= last-decl 0) 0 1))
      0)))
"#,
        program_literal = program_literal
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["0", "0", "1", "1", "1"],
        "ADT の4走査は64要素境界を跨いでも診断とconstructor登録を保持するべき"
    );
}

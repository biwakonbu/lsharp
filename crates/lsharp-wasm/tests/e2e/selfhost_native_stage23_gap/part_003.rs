/// V2-08: representative build entry の IR opcode gap を actual stage23 blocker report として固定する。
#[test]
#[ignore]
fn test_e2e_native_actual_stage23_gap_report_for_representative_entry() {
    let entry_path = selfhost_main_path();
    let report = collect_selfhost_native_stage23_gap_report(&entry_path);

    maybe_write_native_stage23_gap_report(&report)
        .expect("actual-stage23 gap report の書き出しに失敗");

    assert!(
        report.function_count > 0,
        "representative build entry の lowered function が 0"
    );
    assert!(
        report.instruction_count > 0,
        "representative build entry の lowered instruction が 0"
    );
    assert!(
        report.selfhost_function_count > 0,
        "representative selfhost function-meta payload の関数数が 0"
    );
    assert!(
        report.selfhost_instruction_count > 0,
        "representative selfhost function-meta payload の命令数が 0"
    );
    // 制御フロー対応後: gap は空になるため "not empty" アサーションは削除済み
    // (supported set に Call/If/Else/End/Block/Loop/Br/BrIf を追加)

    // 制御フロー opcodes が gap から消えていることを確認
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "Call"
                    | "If"
                    | "IfEmpty"
                    | "Else"
                    | "End"
                    | "Block"
                    | "BlockEmpty"
                    | "Loop"
                    | "LoopEmpty"
                    | "Br"
                    | "BrIf"
            )
        }),
        "x86_64 gap report から制御フロー opcodes は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "Call"
                    | "If"
                    | "IfEmpty"
                    | "Else"
                    | "End"
                    | "Block"
                    | "BlockEmpty"
                    | "Loop"
                    | "LoopEmpty"
                    | "Br"
                    | "BrIf"
            )
        }),
        "aarch64 gap report から制御フロー opcodes は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report
            .unsupported_x86_64
            .iter()
            .any(|name| name == "LocalGet" || name == "LocalSet" || name == "Drop"),
        "x86_64 gap report から LocalGet/LocalSet/Drop は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report
            .unsupported_aarch64
            .iter()
            .any(|name| name == "LocalGet" || name == "LocalSet" || name == "Drop"),
        "aarch64 gap report から LocalGet/LocalSet/Drop は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Const"
                    | "I32Add"
                    | "I32Mul"
                    | "I32And"
                    | "I32Or"
                    | "I32WrapI64"
                    | "I64ExtendI32S"
                    | "I64ExtendI32U"
            )
        }),
        "x86_64 gap report から i32 core opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Const"
                    | "I32Add"
                    | "I32Mul"
                    | "I32And"
                    | "I32Or"
                    | "I32WrapI64"
                    | "I64ExtendI32S"
                    | "I64ExtendI32U"
            )
        }),
        "aarch64 gap report から i32 core opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Load"
                    | "I32Store"
                    | "I32Load8U"
                    | "I64Load"
                    | "I64Store"
                    | "MemoryCopy"
                    | "MemoryFill"
            )
        }),
        "x86_64 gap report から memory opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Load"
                    | "I32Store"
                    | "I32Load8U"
                    | "I64Load"
                    | "I64Store"
                    | "MemoryCopy"
                    | "MemoryFill"
            )
        }),
        "aarch64 gap report から memory opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| matches!(
            name.as_str(),
            "I64Add" | "I64Sub" | "I64Mul" | "I64Div" | "I64Rem"
        )),
        "x86_64 gap report から I64Add/I64Sub/I64Mul/I64Div/I64Rem は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| matches!(
            name.as_str(),
            "I64Add" | "I64Sub" | "I64Mul" | "I64Div" | "I64Rem"
        )),
        "aarch64 gap report から I64Add/I64Sub/I64Mul/I64Div/I64Rem は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I64Eq" | "I64Ne" | "I64LtS" | "I64GtS" | "I64LeS" | "I64GeS"
            )
        }),
        "x86_64 gap report から主要 i64 compare opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I64Eq" | "I64Ne" | "I64LtS" | "I64GtS" | "I64LeS" | "I64GeS"
            )
        }),
        "aarch64 gap report から主要 i64 compare opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_x86_64
            .iter()
            .any(|name| matches!(name.as_str(), "And" | "Or")),
        "selfhost x86_64 gap report から logical and/or は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(name.as_str(), "And" | "Or")),
        "selfhost aarch64 gap report から logical and/or は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(name.as_str(), "RootPush" | "RootPop" | "RootSet")),
        "selfhost aarch64 gap report から root ops は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(
                name.as_str(),
                "CommandLineArg" | "ReadFile" | "StringCharAt" | "StringLength"
            )),
        "selfhost aarch64 gap report から command-line-arg/read-file/string-char-at/string-length は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| name == "Print"),
        "selfhost aarch64 gap report から Print は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(
                name.as_str(),
                "VectorGet" | "VectorLength" | "VectorNew" | "VectorPush"
            )),
        "selfhost aarch64 gap report から vector-get/vector-length/vector-new/vector-push は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        report.unsupported_aarch64.is_empty(),
        "aarch64 lowered IR の native unsupported blocker は 0 であるべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        report.selfhost_unsupported_aarch64.is_empty(),
        "aarch64 selfhost function-meta の native unsupported blocker は 0 であるべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_x86_64
            .iter()
            .any(|name| matches!(name.as_str(), "RootPush" | "RootPop" | "RootSet")),
        "selfhost x86_64 gap report から root ops は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
}

/// V2-09: representative actual stage23 gap report は aarch64 selfhost parity の残 blocker を 0 にする。
#[test]
#[ignore]
fn test_e2e_native_actual_stage23_gap_report_has_zero_aarch64_selfhost_blockers() {
    let entry_path = selfhost_main_path();
    let report = collect_selfhost_native_stage23_gap_report(&entry_path);

    assert!(
        report.unsupported_aarch64.is_empty(),
        "aarch64 lowered IR の native unsupported blocker が残っている: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        report.selfhost_unsupported_aarch64.is_empty(),
        "aarch64 selfhost function-meta の native unsupported blocker が残っている: {:?}",
        report.selfhost_unsupported_aarch64
    );
}

#[test]
#[ignore]
fn test_e2e_native_actual_stage23_gap_report_includes_selfhost_runtime_blockers() {
    let entry_path = selfhost_main_path();
    let report = collect_selfhost_native_stage23_gap_report(&entry_path);

    assert!(
        !report
            .selfhost_unsupported_x86_64
            .iter()
            .any(|name| matches!(
                name.as_str(),
                "CommandLineArg" | "StringCharAt" | "StringLength"
            )),
        "selfhost x86_64 gap report から CommandLineArg/StringCharAt/StringLength は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
    assert!(
        report
            .selfhost_unsupported_x86_64
            .iter()
            .all(|name| name != "ReadFile"),
        "selfhost x86_64 gap report から ReadFile は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
    assert!(
        !report.selfhost_unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "CommandLineArg" | "ReadFile" | "StringCharAt" | "StringLength"
            )
        }),
        "selfhost aarch64 gap report から CommandLineArg/ReadFile/StringCharAt/StringLength は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
}

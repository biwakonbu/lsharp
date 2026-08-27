
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reaches_main_again_build_compile_progress_markers() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-build-compile-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect(
        "BOOT-04 main-build-compile-progress: stage2_self_compiler の build compile progress 実行に失敗した",
    );
    let values: Vec<i64> = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-compile-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 8,
        "BOOT-04 main-build-compile-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 111,
        "BOOT-04 main-build-compile-progress: 最初の marker は 111 であるべき"
    );
    assert_eq!(
        values[1], 112,
        "BOOT-04 main-build-compile-progress: register 後 marker 112 が続くべき"
    );
    assert!(
        values[2] > 0,
        "BOOT-04 main-build-compile-progress: register pair 数が正であるべき: {:?}",
        values
    );
    assert!(
        values.contains(&29),
        "BOOT-04 main-build-compile-progress: pair progress marker 29 が必要: {:?}",
        values
    );
    assert!(
        values.contains(&40),
        "BOOT-04 main-build-compile-progress: defn progress marker 40 が必要: {:?}",
        values
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 113)
        .expect("BOOT-04 main-build-compile-progress: final marker 113 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-compile-progress: final marker の後には function count だけが続くべき"
    );
    assert!(
        values[last_marker_index + 1] > 1000,
        "BOOT-04 main-build-compile-progress: function count が小さすぎる: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_warm_target_defn_parity_reaches_ast_make_type_constrained() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 warm-target-defn: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let parity_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/Syntax/AST.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "warm-target-defn",
        ],
    )
    .expect("BOOT-04 warm-target-defn: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 warm-target-defn: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 10,
        "BOOT-04 warm-target-defn: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 141,
        "BOOT-04 warm-target-defn: warm-up 完了 marker 141 から始まるべき"
    );
    assert_eq!(
        values[2], 142,
        "BOOT-04 warm-target-defn: data length marker 142 が続くべき"
    );
    assert_eq!(
        values[4], 124,
        "BOOT-04 warm-target-defn: target decl tag marker 124 が必要"
    );
    assert_eq!(
        values[5], 20,
        "BOOT-04 warm-target-defn: target decl は defn であるべき"
    );
    assert_eq!(
        values[6], 123,
        "BOOT-04 warm-target-defn: ftable IR marker 123 が必要"
    );
    assert!(
        values[7] > 0,
        "BOOT-04 warm-target-defn: ftable IR は空であってはいけない: {:?}",
        values
    );
    assert_eq!(
        values[8], 144,
        "BOOT-04 warm-target-defn: source-aware function-meta marker 144 が必要"
    );
    assert!(
        values[9] > 0,
        "BOOT-04 warm-target-defn: source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_target_defn_parity_reaches_ast_make_type_constrained() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // App/Main.ls の dispatch は「どの arg スロットが非空か」で probe を選ぶ。
    // target-defn は arg16 (arg17 は warm-target-defn で別物)。
    let mut probe_args = vec!["compiler", "src/Syntax/AST.ls"];
    while probe_args.len() < 16 {
        probe_args.push("");
    }
    probe_args.push("target-defn");

    let stage1_probe_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &probe_args,
    )
    .expect("BOOT-04 target-defn: stage1 の parity probe 実行に失敗した");

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 target-defn: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let parity_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("BOOT-04 target-defn: stage2_self_compiler の parity probe 実行に失敗した");

    // 本 test の主題は parity である — 同じ probe を同じ入力で stage1 の binary と
    // stage2 の binary に走らせ、同じものが見えるか。AST の形の pin は主題ではないので
    // 期待値リテラルは置かない (形の pin は stage1 側の `..._lengths` が引き受ける)。
    // 裁定は `docs/adr/decisions-target-defn-probe-shape-drift.md` の裁定 1。
    let stage1_pairs = parse_target_defn_pairs(&stage1_probe_output, "BOOT-04 target-defn stage1");
    let stage2_pairs = parse_target_defn_pairs(&parity_output, "BOOT-04 target-defn stage2");
    eprintln!("BOOT-04 target-defn stage1 = {stage1_pairs:?}");
    eprintln!("BOOT-04 target-defn stage2 = {stage2_pairs:?}");

    assert_eq!(
        stage1_pairs.iter().map(|&(m, _)| m).collect::<Vec<_>>(),
        stage2_pairs.iter().map(|&(m, _)| m).collect::<Vec<_>>(),
        "BOOT-04 target-defn: stage1 と stage2 で marker 列そのものが食い違う"
    );

    // marker 127 / 128 だけは比較から外す。probe の body 内ナビゲーションが
    // 旧 `let` shape 前提のまま AST の外を読んでおり (`I-80`)、範囲外読み出しの値は
    // binary 依存で stage1 と stage2 で実際に食い違うためである。
    // **これは stage 間の意味論の差ではなく、ゴミを読んでいることの帰結である。**
    // probe 本体を直す (ADR 却下案 B) までは比較対象にしない。
    // 除外した marker が消えていないことは上の marker 列一致が保証する。
    for (&(marker, stage1_value), &(_, stage2_value)) in stage1_pairs.iter().zip(stage2_pairs.iter())
    {
        if TARGET_DEFN_OUT_OF_RANGE_MARKERS.contains(&marker) {
            continue;
        }
        assert_eq!(
            stage1_value, stage2_value,
            "BOOT-04 target-defn: marker {marker} が stage1 と stage2 で食い違う: stage1={stage1_pairs:?} stage2={stage2_pairs:?}"
        );
    }

    // parity が空回りしていないことの下限。除外した 2 件を差し引いても
    // 25 ペアが実際に突き合わされている (実測 2026-08-27: 27 ペア)。
    assert!(
        stage1_pairs.len() >= 27,
        "BOOT-04 target-defn: 突き合わせたペアが少なすぎる: {stage1_pairs:?}"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_stage1_target_defn_parity_reports_ast_make_type_constrained_lengths() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let mut probe_args = vec!["compiler", "src/Syntax/AST.ls"];
    while probe_args.len() < 16 {
        probe_args.push("");
    }
    probe_args.push("target-defn");

    let parity_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &probe_args,
    )
    .expect("BOOT-04 stage1 target-defn: stage1 parity probe 実行に失敗した");
    let label = "BOOT-04 stage1 target-defn";
    let pairs = parse_target_defn_pairs(&parity_output, label);
    eprintln!("BOOT-04 stage1 target-defn = {pairs:?}");
    let marker = |m: i64| target_defn_marker(&pairs, m, label);

    // ---- 何を pin しているのか (`I-80` / decisions-target-defn-probe-shape-drift.md 裁定 2) ----
    //
    // (1) marker 124/125/126 は `selfhost/src/Syntax/AST.ls:260` の `make-type-constrained` の
    //     **形そのもの**である。AST.ls を refactor するとここが赤くなる。**それは正しい挙動**で、
    //     赤くなったら AST.ls の diff を見て値を更新すればよい。今回の問題は赤くなったことではなく、
    //     4 ヶ月間 `I-72` に隠れて赤くならなかったことである。
    // (2) marker 131..139 は probe が自分で仕込んだ sentinel (777 / 778 / 555 / 444 / 333 / 222) の
    //     ftable / env 往復であり、AST の形とは無関係。登録経路の一致を見ている。
    // (3) marker 127/128/129/130/133/135 は旧 `let` shape 前提のナビゲーションの出力で、
    //     現在の body は `ast-apply` なので AST の外を読んでいる。127/128 は binary 依存なので
    //     pin しない。129/130/133/135 はどちらの stage でも 0 になる (hash 0 は ftable に無く
    //     lookup が全部外れる)。**probe 本体を直すとここは非 0 になり赤くなる。それも正しい。**
    assert_eq!(
        marker(124),
        20,
        "decl tag が ast-defn (20) でない: {pairs:?}"
    );
    assert_eq!(
        marker(125),
        1,
        "make-type-constrained の param は name-hash の 1 個のはず: {pairs:?}"
    );
    assert_eq!(
        marker(126),
        5,
        "body の式タグが ast-apply (5) でない。AST.ls:260 の make-type-constrained は \
         vector-push-pair-rooted の単一呼び出しである: {pairs:?}"
    );

    // def-site 側 (`decls[31]`) は body ナビゲーションを経由しないので生きている。
    assert_ne!(marker(131), 0, "def-site の name hash が 0: {pairs:?}");
    assert!(
        marker(132) > 0,
        "register-all-pairs の ftable に def-site が登録されていない: {pairs:?}"
    );
    assert!(
        marker(134) > 0,
        "register-defns-chunked の ftable に def-site が登録されていない: {pairs:?}"
    );
    assert_eq!(
        marker(134),
        marker(136),
        "chunked 登録と再帰登録で同じ関数の index が違う: {pairs:?}"
    );

    // sentinel の往復。登録経路が 3 つあり、どれも decls[31] を 777 へ写すはず。
    assert_eq!(marker(137), 777, "register-defns-step の登録結果: {pairs:?}");
    assert_eq!(marker(138), 777, "register-defns の登録結果: {pairs:?}");
    assert_eq!(marker(139), 777, "ftable-register 直接の登録結果: {pairs:?}");
    assert_eq!(
        marker(140),
        32,
        "register-defns-step は decls[31] を 1 件処理して次の decl index 32 を返すはず: {pairs:?}"
    );
    assert_eq!(
        marker(141),
        778,
        "register-defns-step は次の関数 index として 778 を返すはず: {pairs:?}"
    );
    assert_eq!(marker(142), 555, "早期 ftable の往復: {pairs:?}");
    assert_eq!(marker(143), 444, "正リテラル hash の往復: {pairs:?}");
    assert_eq!(marker(144), 333, "負リテラル hash の往復: {pairs:?}");
    assert_eq!(marker(145), 222, "env の往復: {pairs:?}");
    assert_eq!(marker(146), 1, "ftable-size: {pairs:?}");
    assert_eq!(marker(147), 1, "map-size: {pairs:?}");

    // 壊れたナビゲーションの下流。現在の値を明示的に固定しておく。
    for stale in [129, 130, 133, 135] {
        assert_eq!(
            marker(stale),
            0,
            "marker {stale} は壊れた body ナビゲーションの下流なので 0 のはず。\
             非 0 になったなら probe 本体が直された可能性がある (I-80 を見よ): {pairs:?}"
        );
    }

    // IR 長。実測 2026-08-27 は ftable 版 / source-aware 版とも 21 命令。
    // 命令数そのものは codegen の改良で動くので下限だけを見る。
    assert!(
        marker(123) > 0,
        "ftable IR は空であってはいけない: {pairs:?}"
    );
    assert!(
        marker(122) > 0,
        "source-aware IR は空であってはいけない: {pairs:?}"
    );
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_probe_on_minimal_make_type_constrained_shape() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    // fixture は selfhost ルート配下に置く。stage1 は WASI の preopen が
    // selfhost ルートを "." に張るだけなので、std::env::temp_dir() の絶対パスは
    // 原理的に読めない。実測 (2026-08-27) では read-file が空文字列を返し、
    // stage1 側の probe が 301,-1 (= defn が 1 つも無い) になっていた。
    // つまり stage1 と stage2 を並べる本 test の主題が stage1 側で成立していなかった。
    let temp_root = selfhost_root
        .join("target/test-artifacts")
        .join(format!("lsharp_target_defn_minimal_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_ast_shape.ls");
    std::fs::write(
        &source_path,
        // fixture は `selfhost/src/Syntax/AST.ls:260` の現在の shape を鏡写しにする。
        // `vector-push-pair-rooted` は builtin ではなく AST.ls:67 の module-local defn (14 行) なので、
        // 定義ごと持ち込む。`root_push` / `root_set` / `root_pop` は runtime import であり、
        // flat file を CompilerMode へ食わせる本 fixture では既存 minimal fixture と同じく stub を置く。
        concat!(
            "(defn make-type-constrained [name-hash]",
            " (vector-push-pair-rooted (vector-new 2) (ast-typeconstrained) name-hash))\n",
            "(defn vector-push-pair-rooted [base first second]",
            " (do (root_push first) (root_push second)",
            " (let [base-slot (root_push base) with-first (vector-push base first)]",
            " (do (root_set base-slot with-first)",
            " (let [result (vector-push with-first second)]",
            " (do (root_pop) (root_pop) (root_pop) result))))))\n",
            "(defn ast-typeconstrained [] 24)\n",
            "(defn root_push [x] 0)\n",
            "(defn root_set [slot value] 0)\n",
            "(defn root_pop [] 0)\n",
            "(defn main [] 0)\n",
        ),
    )
    .expect("mini source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);
    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("fixture は selfhost ルート配下に置くこと")
        .to_str()
        .expect("utf-8 path");
    let mut source_step_args = vec!["compiler", source_path_str];
    while source_step_args.len() < 21 {
        source_step_args.push("");
    }
    source_step_args.push("first-defn-source-step");

    let stage1_probe_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &source_step_args,
    )
    .expect("stage1 first-defn probe on minimal source should run");
    eprintln!(
        "BOOT-04 minimal first-defn stage1 = {:?}",
        stage1_probe_output
    );

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let probe_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &source_step_args,
    )
    .expect("stage2 first-defn probe on minimal source should run");
    eprintln!("BOOT-04 minimal first-defn values = {:?}", probe_output);

    // 本 test の主題は stage1 と stage2 が同じ defn を同じ形で見ることである。
    // 301 = 先頭 defn の index、302 = その body の式タグ。fixture の先頭 defn は
    // make-type-constrained で、body は vector-push-pair-rooted の単一呼び出しなので
    // タグ 5 (ast-apply)。**旧 fixture は `let` 形 (タグ 7) を埋め込んでいたが、
    // それは 2026-04-22 の `901c10d8` で AST.ls から消えた形の残骸だった** (`I-80`)。
    let stage1_values = parse_progress_values(&stage1_probe_output, "stage1 first-defn");
    let stage2_values = parse_progress_values(&probe_output, "stage2 first-defn");
    assert_eq!(
        stage1_values,
        vec![301, 0, 302, 5],
        "stage1 の first-defn probe が期待形と違う"
    );
    assert_eq!(
        stage2_values, stage1_values,
        "stage1 と stage2 で先頭 defn の見え方が食い違っている"
    );
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_ir_parity_on_minimal_demo_main_shape() {
    let temp_root =
        std::env::temp_dir().join(format!("lsharp_demo_main_minimal_{}", std::process::id()));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_demo_main_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.Token)\n(defn demo-main [] (do (print (tok-lparen)) (print (tok-rparen)) (print (tok-eof)) 0))\n(defn tok-lparen [] 40)\n(defn tok-rparen [] 41)\n(defn tok-eof [] 99)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    // App/Main.ls の dispatch は「どの arg スロットが非空か」で probe を選ぶ (probe 名の
    // 文字列自体は見ていない)。first-defn-ir-parity は arg13。以前はここが arg18 で、
    // 実際には cache-compile-phase-probe が走っていた (実測 2026-08-27 で 150/151/152/153/154 =
    // cache-compile-phase-probe のマーカー列が出ていた)。test 名の probe に到達していなかった。
    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 13 {
        probe_args.push("");
    }
    probe_args.push("first-defn-ir-parity");
    let probe_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn-ir-parity probe on minimal demo-main source should run");
    eprintln!(
        "BOOT-04 minimal demo-main first-defn-ir-parity = {:?}",
        probe_output
    );
    // 91..99 は first-defn-ir-parity probe のマーカー。source 経路と ftable 経路が
    // 同じ IR 長 (96/97 の直後の値) を出すことが parity の主題である。
    // 実測 2026-08-27: [91,1,92,10,93,94,10,95,1,96,10,97,10,98,0,99,10]。
    let values = parse_progress_values(&probe_output, "first-defn-ir-parity");
    assert_eq!(
        values,
        vec![91, 1, 92, 10, 93, 94, 10, 95, 1, 96, 10, 97, 10, 98, 0, 99, 10],
        "first-defn-ir-parity の出力形が変わった"
    );
    assert_eq!(
        values[10], values[12],
        "source 経路と ftable 経路の IR 長が食い違っている: {values:?}"
    );
    assert_eq!(
        values[1], values[8],
        "defn index の replay が一致していない: {values:?}"
    );
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_source_probe_on_minimal_text_eq_loop_shape() {
    let temp_root = std::env::temp_dir().join(format!(
        "lsharp_text_eq_loop_minimal_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_text_eq_loop_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 22 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source");

    let probe_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source probe on minimal text-eq-loop source should run");
    eprintln!(
        "BOOT-04 minimal text-eq-loop source probe = {:?}",
        probe_output
    );
    let values = parse_progress_values(&probe_output, "BOOT-04 minimal text-eq-loop source probe");

    // 実測 (2026-08-27): 301 <first defn の index> 302 <body tag> 303 <cond tag> 304 <IR 命令数>
    // に続き、命令ごとに 206 <序数> 209 <?> 207 <opcode> 208 <引数> が並ぶ。
    // fixture は本 test 内のリテラルなので、すべて exact に固定できる。
    assert_eq!(
        &values[..8],
        &[301, 1, 302, 6, 303, 5, 304, 3],
        "BOOT-04 minimal text-eq-loop source probe: 先頭 8 値が実測と違う: {values:?}"
    );
    let instr_count = values[7];
    let ordinals: Vec<i64> = values
        .windows(2)
        .filter(|pair| pair[0] == 206)
        .map(|pair| pair[1])
        .collect();
    assert_eq!(
        ordinals,
        (0..instr_count).collect::<Vec<_>>(),
        "BOOT-04 minimal text-eq-loop source probe: 命令の序数が 0..{instr_count} の連番でない: {values:?}"
    );
    assert_eq!(
        values.len() as i64,
        8 + 8 * instr_count,
        "BOOT-04 minimal text-eq-loop source probe: 長さが命令数から決まる形になっていない: {values:?}"
    );
    // 命令列そのもの。ここが動いたら cond 式のコード生成が変わったということ。
    assert_eq!(
        &values[8..],
        &[
            206, 0, 209, 2, 207, 10, 208, 3, 206, 1, 209, 2, 207, 1, 208, 1, 206, 2, 209, 2, 207,
            20, 208, 0
        ],
        "BOOT-04 minimal text-eq-loop source probe: cond1 の IR が実測と違う: {values:?}"
    );
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_source_step_probe_on_minimal_path_parent_shape() {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_path_parent_minimal_step_probe_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_path_parent_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 21 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source-step");

    let probe_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source step probe on minimal path-parent source should run");
    eprintln!(
        "BOOT-04 minimal path-parent source step probe = {:?}",
        probe_output
    );
    let values = parse_progress_values(&probe_output, "BOOT-04 minimal path-parent source step probe");

    // 実測 (2026-08-27): 301 <first defn の index> 302 <body tag>。
    // path-parent の body は tag 7 (let) で tag-if ではないため、probe は 303/304 を出さずに終わる。
    // fixture は本 test 内のリテラルなので exact に固定できる。
    assert_eq!(
        values,
        vec![301, 1, 302, 7],
        "BOOT-04 minimal path-parent source step probe: 出力が実測と違う"
    );
}

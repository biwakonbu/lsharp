// =============================================================================
// I-85: debug probe の出力を検査するための共通 helper
//
// `WEAK-SUBJECT-ASSERT-01` で扱う 12 件の probe test は、いずれも
// `assert!(!output.trim().is_empty())` だけを持っていた。これは「何か出た」しか
// 見ておらず、出力の中身が壊れても緑のままになる。
//
// 個々の test に同じ構造検査を写経すると差分が発散するので、共通部分をここへ置く。
// helper 側は marker だけを exact に見て、バイト長・decl 数は下限と関係式で扱う。
// =============================================================================

/// I-85: `build-compile-progress` debug 出力の共通構造を固定する。
///
/// 実測 (2026-08-27) では 7 test すべてが次の形を取る:
///
/// ```text
/// 111 112 <pair-count> 29 0 <先頭 src バイト数> <先頭 decl 数>
///   ... pair ごとの 40/20/41/42/43 ループ ...
/// 30 <import 数> <末尾 src バイト数> <末尾 decl 数> 113 <生成関数の総数>
/// ```
///
/// marker は exact で固定し、バイト長・decl 数は下限と関係式だけを見る。
/// これらは `.ls` を 1 行編集するだけで動くので、exact に pin すると
/// 「ソースを触るたびに落ちる test」になり主題 (progress の構造) から外れる。
///
/// 戻り値は (先頭 src バイト数, 先頭 decl 数, 末尾 src バイト数, 末尾 decl 数, 生成関数の総数)。
fn assert_build_compile_progress_shape(values: &[i64], label: &str) -> (i64, i64, i64, i64, i64) {
    assert!(
        values.len() >= 18,
        "{label}: build-compile-progress の出力が短すぎる: {values:?}"
    );
    assert_eq!(
        values[0], 111,
        "{label}: 先頭 marker は 111 であるべき: {:?}",
        &values[..8.min(values.len())]
    );
    assert_eq!(
        values[1], 112,
        "{label}: 2 番目の marker は 112 であるべき: {:?}",
        &values[..8.min(values.len())]
    );
    assert_eq!(
        values[3], 29,
        "{label}: pair ループ開始 marker 29 が無い: {:?}",
        &values[..8.min(values.len())]
    );
    assert_eq!(
        values[4], 0,
        "{label}: 最初の pair index は 0 であるべき: {:?}",
        &values[..8.min(values.len())]
    );

    let tail = values.len() - 6;
    assert_eq!(
        values[tail], 30,
        "{label}: 終端 marker 30 が末尾から 6 番目に無い: {:?}",
        &values[tail..]
    );
    assert_eq!(
        values[values.len() - 2],
        113,
        "{label}: 終端 marker 113 が末尾から 2 番目に無い: {:?}",
        &values[tail..]
    );

    let pair_count = values[2];
    assert!(
        pair_count >= 1,
        "{label}: pair 数が 1 未満: {:?}",
        &values[..8.min(values.len())]
    );
    assert_eq!(
        values[tail + 1],
        pair_count - 1,
        "{label}: 終端の import 数は pair 数 - 1 であるべき: pair_count={pair_count} tail={:?}",
        &values[tail..]
    );

    let first_bytes = values[5];
    let first_decls = values[6];
    let last_bytes = values[tail + 2];
    let last_decls = values[tail + 3];
    let total_functions = values[values.len() - 1];
    for (name, value) in [
        ("先頭 src バイト数", first_bytes),
        ("先頭 decl 数", first_decls),
        ("末尾 src バイト数", last_bytes),
        ("末尾 decl 数", last_decls),
        ("生成関数の総数", total_functions),
    ] {
        assert!(value > 0, "{label}: {name} が正でない: {value}");
    }

    (
        first_bytes,
        first_decls,
        last_bytes,
        last_decls,
        total_functions,
    )
}

/// I-82 裁定 3: `debug progress` 系 probe の共通構造を固定する。
///
/// 実測 (2026-08-27) では `..._reports_*_progress` 3 件が次の形を取る:
///
/// ```text
/// 1 <末尾 decl 数> 2 <import 数> 3 <生成関数の総数>
/// 29 0 <先頭 src バイト数> <先頭 decl 数>
///   ... pair ごとの 40/20/41/42/43 ループ ...
/// 30 <import 数> <末尾 src バイト数> <末尾 decl 数> 4 <生成関数の総数>
/// ```
///
/// 先頭の 3 値は末尾ブロックの再掲であり、**その一致こそがこの probe の主題**である
/// (progress の冒頭で宣言した数と、最後まで走った結果が食い違わないこと)。
/// marker と再掲の一致だけを見て、バイト長・decl 数そのものは呼び出し側に委ねる。
///
/// 戻り値は (import 数, 末尾 decl 数, 生成関数の総数)。
fn assert_debug_progress_shape(values: &[i64], label: &str) -> (i64, i64, i64) {
    assert!(
        values.len() >= 21,
        "{label}: debug progress の出力が短すぎる: {values:?}"
    );
    for (index, expected) in [(0usize, 1i64), (2, 2), (4, 3), (6, 29), (7, 0)] {
        assert_eq!(
            values[index], expected,
            "{label}: 位置 {index} の marker が {expected} でない: {:?}",
            &values[..10.min(values.len())]
        );
    }
    assert!(
        values[8] > 0,
        "{label}: 先頭 src バイト数が正でない: {:?}",
        &values[..10.min(values.len())]
    );

    let tail = values.len() - 6;
    assert_eq!(
        values[tail], 30,
        "{label}: 終端 marker 30 が末尾から 6 番目に無い: {:?}",
        &values[tail..]
    );
    assert_eq!(
        values[tail + 4],
        4,
        "{label}: 終端 marker 4 が末尾から 2 番目に無い: {:?}",
        &values[tail..]
    );
    assert!(
        values[tail + 2] > 0,
        "{label}: 末尾 src バイト数が正でない: {:?}",
        &values[tail..]
    );

    assert_eq!(
        values[1],
        values[tail + 3],
        "{label}: 冒頭で宣言した decl 数と末尾の decl 数が食い違う: {:?} / {:?}",
        &values[..6],
        &values[tail..]
    );
    assert_eq!(
        values[3],
        values[tail + 1],
        "{label}: 冒頭で宣言した import 数と末尾の import 数が食い違う: {:?} / {:?}",
        &values[..6],
        &values[tail..]
    );
    assert_eq!(
        values[5],
        values[tail + 5],
        "{label}: 冒頭で宣言した関数総数と末尾の関数総数が食い違う: {:?} / {:?}",
        &values[..6],
        &values[tail..]
    );

    (values[3], values[1], values[5])
}

/// I-85: debug probe の stdout を数値列へ変換する。数値以外が混ざったら落とす。
fn parse_progress_values(output: &str, label: &str) -> Vec<i64> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("{label}: 数値でない debug 出力: {line:?}"))
        })
        .collect()
}

/// target-defn probe (`selfhost/src/App/CompilerMode.ls` の
/// `compile-file-mode-target-defn-parity-probe`) の出力を marker/value ペアへ分解する。
///
/// 出力は `121 59 124 20 ...` の交互列で、marker は昇順ではない (末尾に 123 / 122 が来る)。
/// 先頭 2 値は probe の身元 (121 = probe 識別子 / 59 = 見ている decl の index) なので、
/// **別の probe に到達していたらここで落ちる**。arg スロットの取り違えを黙って通さないための門である。
fn parse_target_defn_pairs(output: &str, label: &str) -> Vec<(i64, i64)> {
    let values: Vec<i64> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("{label}: 数値でない debug 出力: {line:?}"))
        })
        .collect();
    assert_eq!(
        values.len() % 2,
        0,
        "{label}: marker/value ペア数が崩れている: {values:?}"
    );
    assert!(
        values.len() >= 42,
        "{label}: target-defn probe の出力が短すぎる (実測 2026-08-27 は 54 値 / 27 ペア): {values:?}"
    );
    assert_eq!(
        values[0], 121,
        "{label}: 先頭 marker が 121 でない。target-defn 以外の probe に到達している: {values:?}"
    );
    assert_eq!(
        values[1], 59,
        "{label}: probe が見ている decl index が 59 でない: {values:?}"
    );
    values.chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

/// `parse_target_defn_pairs` の結果から marker の値を引く。無ければ落ちる。
fn target_defn_marker(pairs: &[(i64, i64)], marker: i64, label: &str) -> i64 {
    pairs
        .iter()
        .find_map(|&(m, v)| (m == marker).then_some(v))
        .unwrap_or_else(|| panic!("{label}: marker {marker} が見つからない: {pairs:?}"))
}

/// 旧 `let` shape 前提の body 内ナビゲーションが AST の外を読んでいる marker のうち、
/// **値が binary 依存になるもの**。`I-80` / `decisions-target-defn-probe-shape-drift.md`。
/// 範囲外読み出しは 0 を返すとは限らず、実測 2026-08-27 では
/// stage1 が `[4294967296, 72057594054705152]`、stage2 が `[0, 0]` を返した。
const TARGET_DEFN_OUT_OF_RANGE_MARKERS: [i64; 2] = [127, 128];

//! 前方参照された呼び出しが型検査されない問題 (`I-46` 健全性側)
//!
//! callee が caller より後ろに定義されていると、その呼び出しは引数型も arity も検査されず、
//! caller の結果型は束縛されない型変数のまま generalize されて `forall a. () -> a` になる。
//! 宣言順を入れ替えただけの同じ program は `Mismatch` で落ちるので、
//! **順序だけが判定を変える**。
//!
//! ここが守る契約は「宣言順によらず誤用を拒否する」ことだけである。
//! 前方参照した多相関数を複数の型で使えるようにする完全性側は `INFER-FORWARD-POLY-01`。
//!
//! **検出側 5 件は `#[ignore]` である。** 直し方は分かっていて GREEN も取れているが、
//! selfhost のソースが同じ穴に依存しており (`I-48`)、当てると selfhost の 262 defn が
//! 推論に失敗する。`SELFHOST-TUPLE-REC-01` が閉じたら `#[ignore]` を外す。
//! 退行防止側 3 件は現状でも通るので live のまま回す。

use lsharp_types::infer::{Infer, TypeError};

/// 誤用が `Mismatch` で拒否されることを要求する。
fn expect_misuse_rejected(label: &str, source: &str) {
    let program =
        lsharp_syntax::parse(source).unwrap_or_else(|e| panic!("{label}: parse 失敗 {e:?}"));
    let error = Infer::new()
        .infer_program(&program)
        .err()
        .unwrap_or_else(|| panic!("{label}: 型の誤用を通してはならない"));
    assert!(
        matches!(error, TypeError::Mismatch { .. }),
        "{label}: Mismatch を期待したが {error:?}"
    );
}

/// 前方参照した呼び出しの**引数型**が検査されない。
/// `helper` は `String` を取るのに `Int` を渡していて、順序を入れ替えれば `ArgMismatch` になる。
#[test]
#[ignore = "I-46: 修正は用意済みだが I-48 (selfhost の異種 vector) で blocked"]
fn forward_declared_callee_checks_argument_types() {
    expect_misuse_rejected(
        "callsite-forward",
        r#"(defn main [] (helper 1))
           (defn helper [x] (string-length x))"#,
    );
}

/// 前方参照した呼び出しの **arity** が検査されない。
/// 1 引数の関数を 2 引数で呼んでいるのに通り、codegen まで無検査で届く。
#[test]
#[ignore = "I-46: 修正は用意済みだが I-48 (selfhost の異種 vector) で blocked"]
fn forward_declared_callee_checks_arity() {
    expect_misuse_rejected(
        "arity-forward",
        r#"(defn main [] (helper 1 2))
           (defn helper [x] x)"#,
    );
}

/// 呼び出し元の結果型が汎化される。上記の穴の帰結。
/// computation を一切含まない plain な `defn` で再現するので、computation builder は発見経路にすぎない。
#[test]
#[ignore = "I-46: 修正は用意済みだが I-48 (selfhost の異種 vector) で blocked"]
fn forward_declared_callee_does_not_generalize_caller_result() {
    expect_misuse_rejected(
        "plain-forward",
        r#"(defn main [] (helper 1))
           (defn helper [x] x)
           (defn misuse [] (string-length (main)))"#,
    );
}

/// 引数を持たない callee でも同じ。callee 自身は `Fun([], Int)` に解決されるのに
/// caller だけが自由変数のまま取り残される形。
#[test]
#[ignore = "I-46: 修正は用意済みだが I-48 (selfhost の異種 vector) で blocked"]
fn forward_declared_zero_arg_callee_does_not_generalize_caller_result() {
    expect_misuse_rejected(
        "fwd-noargs",
        r#"(defn main [] (helper))
           (defn helper [] 1)
           (defn misuse [] (string-length (main)))"#,
    );
}

/// 発見経路。computation builder の member を前方参照した場合。
#[test]
#[ignore = "I-46: 修正は用意済みだが I-48 (selfhost の異種 vector) で blocked"]
fn forward_declared_computation_builder_members_do_not_generalize_result() {
    expect_misuse_rejected(
        "builder-forward",
        r#"(computation-builder identity identity-bind identity-return)
           (defn main [] (computation identity (return 42)))
           (defn identity-return [x] x)
           (defn identity-bind [m f] (f m))
           (defn misuse [] (string-length (main)))"#,
    );
}

/// 宣言順を入れ替えた側は元から落ちる。修正で壊さないことを見る回帰。
#[test]
fn ordered_declaration_still_rejects_misuse() {
    expect_misuse_rejected(
        "plain-ordered",
        r#"(defn helper [x] x)
           (defn main [] (helper 1))
           (defn misuse [] (string-length (main)))"#,
    );
}

/// 相互再帰は元から正しく落ちる。修正で壊さないことを見る回帰。
#[test]
fn mutual_recursion_still_rejects_misuse() {
    expect_misuse_rejected(
        "mutual",
        r#"(defn is-even [n] (if (= n 0) true (is-odd (- n 1))))
           (defn is-odd [n] (if (= n 0) false (is-even (- n 1))))
           (defn misuse [] (string-length (is-even 3)))"#,
    );
}

/// 誤りでない前方参照は通り続ける。修正が過剰に拒否しないことを見る。
#[test]
fn valid_forward_reference_still_type_checks() {
    const SOURCE: &str = r#"(defn main [] (helper 1))
                            (defn helper [x] x)
                            (defn ok [] (+ (main) 1))"#;
    let program = lsharp_syntax::parse(SOURCE).expect("parse できるべき");
    let inferred = Infer::new()
        .infer_program(&program)
        .expect("正しい前方参照を拒否してはならない");
    assert!(
        inferred.iter().any(|(name, _)| name == "ok"),
        "ok の推論結果が存在するべき"
    );
}

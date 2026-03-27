//! Wasm バイナリ生成のスナップショットテスト (QA-4)
//!
//! codegen/wasi の Wasm バイナリ構成を insta スナップショットで回帰テストする。
//! バイナリそのものではなく、セクション構成・サイズ・関数数などの
//! 人間可読な情報をスナップショットとして保存する。

use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

/// ソースコードから Wasm バイナリを生成し、構造情報を文字列ダンプで返す
fn wasm_structure_dump(source: &str) -> String {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    dump_wasm_sections(&wasm_bytes)
}

/// Wasm バイナリのセクション構成をパースしてテキスト表現を返す
fn dump_wasm_sections(bytes: &[u8]) -> String {
    let mut result = String::new();
    result.push_str(&format!("=== Wasm バイナリ構造 ===\n"));
    result.push_str(&format!("バイナリサイズ: {} bytes\n", bytes.len()));

    // マジックバイトとバージョン
    if bytes.len() >= 8 {
        let magic = &bytes[0..4];
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        result.push_str(&format!(
            "マジック: {:02x} {:02x} {:02x} {:02x}\n",
            magic[0], magic[1], magic[2], magic[3]
        ));
        result.push_str(&format!("バージョン: {}\n", version));
    }

    // セクションをパース
    let mut offset = 8;
    let mut sections = Vec::new();
    while offset < bytes.len() {
        if offset >= bytes.len() {
            break;
        }
        let section_id = bytes[offset];
        offset += 1;

        // LEB128 でセクションサイズを読む
        let (size, consumed) = read_leb128_u32(&bytes[offset..]);
        offset += consumed;

        let section_name = match section_id {
            0 => "Custom",
            1 => "Type",
            2 => "Import",
            3 => "Function",
            4 => "Table",
            5 => "Memory",
            6 => "Global",
            7 => "Export",
            8 => "Start",
            9 => "Element",
            10 => "Code",
            11 => "Data",
            12 => "DataCount",
            _ => "Unknown",
        };

        sections.push((section_id, section_name, size));
        offset += size as usize;
    }

    result.push_str(&format!("\nセクション数: {}\n", sections.len()));
    for (id, name, size) in &sections {
        result.push_str(&format!("  [{:2}] {:<12} {} bytes\n", id, name, size));
    }

    result
}

/// LEB128 unsigned 整数を読み取り、(値, 消費バイト数) を返す
fn read_leb128_u32(bytes: &[u8]) -> (u32, usize) {
    let mut result: u32 = 0;
    let mut shift = 0;
    let mut i = 0;
    loop {
        if i >= bytes.len() {
            break;
        }
        let byte = bytes[i];
        result |= ((byte & 0x7f) as u32) << shift;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (result, i)
}

// === スナップショットテスト ===

#[test]
fn test_snapshot_wasm_simple_return() {
    // 最も単純な関数: 42 を返すだけ
    let dump = wasm_structure_dump("(defn main [] 42)");
    insta::assert_snapshot!("wasm_simple_return", dump);
}

#[test]
fn test_snapshot_wasm_print_output() {
    // print 呼び出しを含む関数
    let dump = wasm_structure_dump("(defn main [] (print 42))");
    insta::assert_snapshot!("wasm_print_output", dump);
}

#[test]
fn test_snapshot_wasm_arithmetic() {
    // 算術演算を含む関数
    let dump = wasm_structure_dump("(defn main [] (+ (* 3 4) 5))");
    insta::assert_snapshot!("wasm_arithmetic", dump);
}

#[test]
fn test_snapshot_wasm_if_expr() {
    // if 式を含む関数
    let dump = wasm_structure_dump("(defn main [] (if (< 1 2) 42 0))");
    insta::assert_snapshot!("wasm_if_expr", dump);
}

#[test]
fn test_snapshot_wasm_let_binding() {
    // let 束縛を含む関数
    let dump = wasm_structure_dump("(defn main [] (let [x 10 y 20] (+ x y)))");
    insta::assert_snapshot!("wasm_let_binding", dump);
}

#[test]
fn test_snapshot_wasm_multiple_functions() {
    // 複数関数の定義
    let dump = wasm_structure_dump(
        "(defn double [x] (* x 2))
         (defn main [] (print (double 21)))",
    );
    insta::assert_snapshot!("wasm_multiple_functions", dump);
}

#[test]
fn test_snapshot_wasm_recursive_function() {
    // 再帰関数 (fibonacci)
    let dump = wasm_structure_dump(
        "(defn fib [n]
           (if (<= n 1) n
             (+ (fib (- n 1)) (fib (- n 2)))))
         (defn main [] (print (fib 10)))",
    );
    insta::assert_snapshot!("wasm_recursive_function", dump);
}

#[test]
fn test_snapshot_wasm_string_operations() {
    // 文字列操作を含む関数
    let dump = wasm_structure_dump(r#"(defn main [] (do (print-string "hello") 0))"#);
    insta::assert_snapshot!("wasm_string_operations", dump);
}

#[test]
fn test_snapshot_wasm_adt_construct() {
    // ADT コンストラクタを含む関数
    let dump = wasm_structure_dump(
        "(type (Maybe a) (Just a) Nothing)
         (defn main [] (do (print (Just 42)) 0))",
    );
    insta::assert_snapshot!("wasm_adt_construct", dump);
}

#[test]
fn test_snapshot_wasm_adt_match() {
    // ADT パターンマッチを含む関数
    let dump = wasm_structure_dump(
        "(type (Maybe a) (Just a) Nothing)
         (defn from-maybe [m d]
           (match m
             [(Just x) x]
             [Nothing d]))
         (defn main [] (print (from-maybe (Just 42) 0)))",
    );
    insta::assert_snapshot!("wasm_adt_match", dump);
}

#[test]
fn test_snapshot_wasm_closure() {
    // クロージャを含む関数
    let dump = wasm_structure_dump(
        "(defn make-adder [n] (fn [x] (+ x n)))
         (defn apply [f x] (f x))
         (defn main [] (print (apply (make-adder 10) 32)))",
    );
    insta::assert_snapshot!("wasm_closure", dump);
}

#[test]
fn test_snapshot_wasm_trait_dispatch() {
    // トレイト + 静的ディスパッチを含む関数
    let dump = wasm_structure_dump(
        "(trait (Describable a)
           (defn describe [self] : Int))
         (impl (Describable Int)
           (defn describe [self] self))
         (defn main [] (do (print (describe 99)) 0))",
    );
    insta::assert_snapshot!("wasm_trait_dispatch", dump);
}

#[test]
fn test_snapshot_wasm_do_block() {
    // do ブロック (複数 print)
    let dump = wasm_structure_dump("(defn main [] (do (print 1) (print 2) (print 3) 0))");
    insta::assert_snapshot!("wasm_do_block", dump);
}

#[test]
fn test_snapshot_wasm_match_literal() {
    // リテラルパターンマッチ
    let dump = wasm_structure_dump(
        "(defn classify [n]
           (match n
             [0 100]
             [1 200]
             [_ 0]))
         (defn main [] (print (classify 1)))",
    );
    insta::assert_snapshot!("wasm_match_literal", dump);
}

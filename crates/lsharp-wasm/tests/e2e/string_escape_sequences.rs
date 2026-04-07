use super::support::*;

/// Test escape sequences in L# string literals
/// This test verifies how the L# compiler processes escape sequences like \n and \\

#[test]
fn test_escape_sequence_newline() {
    // Test literal newline vs escaped newline
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "Test 1: ")
            (print-string "\\n")
            (print-string "\n")
            (print-string "Test 2: ")
            (print-string "\\\\")
            (print-string "\n")
            0))
    "#,
    );

    // Print the output in a detailed way to see exact bytes
    eprintln!("Output length: {} bytes", result.len());
    for (i, byte) in result.as_bytes().iter().enumerate() {
        eprintln!("  Byte {}: 0x{:02X} ({})", i, byte, *byte as char);
    }

    // The output should be:
    // "Test 1: " + <content of "\\n"> + newline + "Test 2: " + <content of "\\\\"> + newline

    // If "\\n" produces a literal backslash-n:
    // Test 1: \n
    // Test 2: \\

    // If "\\n" produces a newline:
    // Test 1:
    // Test 2: \\

    println!("Raw output: {:?}", result);
    assert!(result.contains("Test 1:"));
    assert!(result.contains("Test 2:"));
}

#[test]
fn test_double_backslash() {
    // Test if \\ produces a single backslash
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "before")
            (print-string "\\")
            (print-string "after")
            0))
    "#,
    );

    eprintln!("\nDouble backslash test:");
    eprintln!("Output: {:?}", result);
    eprintln!("Bytes:");
    for (i, byte) in result.as_bytes().iter().enumerate() {
        eprintln!(
            "  Byte {}: 0x{:02X} ({})",
            i,
            byte,
            if *byte >= 32 && *byte < 127 {
                (*byte as char).to_string()
            } else {
                "non-printable".to_string()
            }
        );
    }

    assert!(result.contains("before"));
    assert!(result.contains("after"));
}

#[test]
fn test_escaped_n_sequence() {
    // Test what \n produces
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "line1")
            (print-string "\n")
            (print-string "line2")
            0))
    "#,
    );

    eprintln!("\nEscaped \\n test:");
    eprintln!("Output: {:?}", result);
    eprintln!("Bytes:");
    for (i, byte) in result.as_bytes().iter().enumerate() {
        eprintln!("  Byte {}: 0x{:02X}", i, byte);
    }

    // If \n produces a newline, we should see 0x0A
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
}

#[test]
fn test_verify_hex_dump() {
    // Create a simple test that shows actual bytes
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "A")
            (print-string "\\n")
            (print-string "B")
            0))
    "#,
    );

    eprintln!("\nHex dump of 'A' + '\\\\n' + 'B':");
    let bytes = result.as_bytes();
    println!(
        "\nOutput bytes (hex): {}",
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    println!(
        "\nOutput bytes (decimal): {}",
        bytes
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );

    println!("\nOutput as string: {:?}", result);
}

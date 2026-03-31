use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn write_embedded_component(bytes: &[u8], embedded_output: &Path) {
    fs::write(embedded_output, bytes).unwrap_or_else(|err| {
        panic!(
            "embedded component の書き込みに失敗しました ({}): {err}",
            embedded_output.display()
        )
    });
}

fn copy_embedded_component(source_path: &Path, embedded_output: &Path) {
    let bytes = fs::read(source_path).unwrap_or_else(|err| {
        panic!(
            "LSHARP_EMBED_COMPONENT_PATH の読み込みに失敗しました ({}): {err}",
            source_path.display()
        )
    });
    write_embedded_component(&bytes, embedded_output);
}

fn build_default_embedded_component(project_root: &Path) -> Vec<u8> {
    let entry_path = project_root
        .join("selfhost")
        .join("src")
        .join("App")
        .join("EmbeddedCli.ls");
    let module = lsharp_ir::compile_multi_file(&entry_path).unwrap_or_else(|err| {
        panic!(
            "default embedded component 用 selfhost CLI のコンパイルに失敗しました ({}): {err}",
            entry_path.display()
        )
    });
    lsharp_wasm::wasi::emit_wasm_wasi_p2(&module).unwrap_or_else(|err| {
        panic!(
            "default embedded component の component 化に失敗しました ({}): {err}",
            entry_path.display()
        )
    })
}

fn main() {
    println!("cargo:rerun-if-env-changed=LSHARP_EMBED_COMPONENT_PATH");

    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should exist"));
    let project_root = manifest_dir.join("../..");
    println!(
        "cargo:rerun-if-changed={}",
        project_root.join("selfhost").join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        project_root.join("wit").display()
    );

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should exist"));
    let embedded_output = out_dir.join("embedded-lsharp.component.wasm");

    match env::var_os("LSHARP_EMBED_COMPONENT_PATH") {
        Some(path) if !path.is_empty() => {
            let source_path = PathBuf::from(path);
            println!("cargo:rerun-if-changed={}", source_path.display());
            copy_embedded_component(&source_path, &embedded_output);
            println!("cargo:rustc-env=LSHARP_EMBEDDED_COMPONENT_PRESENT=1");
        }
        _ => {
            let component = build_default_embedded_component(&project_root);
            write_embedded_component(&component, &embedded_output);
            println!("cargo:rustc-env=LSHARP_EMBEDDED_COMPONENT_PRESENT=1");
        }
    }
}

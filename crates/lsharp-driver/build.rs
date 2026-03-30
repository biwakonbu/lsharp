use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=LSHARP_EMBED_COMPONENT_PATH");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should exist"));
    let embedded_output = out_dir.join("embedded-lsharp.component.wasm");

    match env::var_os("LSHARP_EMBED_COMPONENT_PATH") {
        Some(path) if !path.is_empty() => {
            let source_path = PathBuf::from(path);
            println!("cargo:rerun-if-changed={}", source_path.display());

            let bytes = fs::read(&source_path).unwrap_or_else(|err| {
                panic!(
                    "LSHARP_EMBED_COMPONENT_PATH の読み込みに失敗しました ({}): {err}",
                    source_path.display()
                )
            });
            fs::write(&embedded_output, &bytes).unwrap_or_else(|err| {
                panic!(
                    "embedded component のコピーに失敗しました ({}): {err}",
                    embedded_output.display()
                )
            });
            println!("cargo:rustc-env=LSHARP_EMBEDDED_COMPONENT_PRESENT=1");
        }
        _ => {
            fs::write(&embedded_output, []).unwrap_or_else(|err| {
                panic!(
                    "empty embedded component placeholder の生成に失敗しました ({}): {err}",
                    embedded_output.display()
                )
            });
            println!("cargo:rustc-env=LSHARP_EMBEDDED_COMPONENT_PRESENT=0");
        }
    }
}

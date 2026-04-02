use std::{
    env, fs,
    path::{Path, PathBuf},
};

use wasmtime_wit_bindgen::Opts;
use wit_parser::Resolve;

fn main() {
    println!("cargo:rerun-if-changed=../../wit/lsharp-http-handler.wit");
    println!("cargo:rerun-if-changed=../../wit/deps");

    generate_http_handler_bindings().expect("HTTP handler bindings generation should succeed");
}

fn generate_http_handler_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let staged_root = out_dir.join("lsharp-http-handler-wit");
    let world_source = manifest_dir.join("../../wit/lsharp-http-handler.wit");

    if staged_root.exists() {
        fs::remove_dir_all(&staged_root)?;
    }
    fs::create_dir_all(&staged_root)?;
    stage_wit_workspace(&world_source, &staged_root)?;

    let mut resolve = Resolve::default();
    resolve
        .features
        .insert("informational-outbound-responses".to_string());
    let (package_id, _) = resolve.push_dir(&staged_root)?;
    let world = resolve.select_world(package_id, Some("lsharp-http-handler"))?;

    let mut opts = Opts {
        rustfmt: false,
        require_store_data_send: true,
        ..Opts::default()
    };
    opts.with.insert(
        "wasi:cli@0.2.3".to_string(),
        "wasmtime_wasi::bindings::sync::cli".to_string(),
    );
    opts.with.insert(
        "wasi:clocks@0.2.3".to_string(),
        "wasmtime_wasi::bindings::sync::clocks".to_string(),
    );
    opts.with.insert(
        "wasi:io@0.2.3".to_string(),
        "wasmtime_wasi::bindings::sync::io".to_string(),
    );
    opts.with.insert(
        "wasi:random@0.2.3".to_string(),
        "wasmtime_wasi::bindings::sync::random".to_string(),
    );

    let bindings = opts.generate(&resolve, world)?;
    fs::write(out_dir.join("http_handler_bindings.rs"), bindings)?;
    Ok(())
}

fn stage_wit_workspace(
    source: &Path,
    staged_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = source.file_name().ok_or_else(|| {
        format!(
            "HTTP handler WIT source file 名の取得に失敗しました: {}",
            source.display()
        )
    })?;
    fs::copy(source, staged_root.join(file_name))?;

    let deps_dir = source
        .parent()
        .ok_or_else(|| {
            format!(
                "HTTP handler WIT parent の取得に失敗しました: {}",
                source.display()
            )
        })?
        .join("deps");
    if deps_dir.is_dir() {
        copy_dir_all(&deps_dir, &staged_root.join("deps"))?;
    }

    Ok(())
}

fn copy_dir_all(source: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn write_embedded_component(bytes: &[u8], embedded_output: &Path) {
    // 内容が同じなら書かない。`include_bytes!` の dep-info は mtime を見るので、OUT_DIR の
    // artifact は mtime だけでも動かさないほうが安全である。
    //
    // ただしこの skip だけでは driver の再コンパイルは防げない。build script が走った時点で
    // cargo は crate の fingerprint を無効化するためである (計測でも cache hit 時に
    // `Compiling lsharp-driver` が出ている)。害が無く、build script が再実行されない経路では
    // 効くので残すが、これが速さの理由だとは考えないこと。
    if fs::read(embedded_output).is_ok_and(|existing| existing == bytes) {
        return;
    }
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

/// embedded component cache に残す entry の上限。1 entry が 1MB 超あるので無制限にはしない。
const EMBEDDED_COMPONENT_CACHE_ENTRIES: usize = 8;

/// selfhost tree と emitter identity から component bytes を content-addressed で再利用する。
///
/// cache は最適化でしかないので、どの失敗も fresh compile への fallback にする。
/// cache が壊れていることで build が落ちてはいけない。
///
/// key 導出と cache root の逆算は `lsharp-wasm` 側に置いてある (build.rs は直接テストできない)。
fn cached_default_embedded_component(project_root: &Path, out_dir: &Path) -> Vec<u8> {
    use lsharp_wasm::embedded_component_cache::{
        cache_root_from_out_dir, current_executable_fingerprint, embedded_component_key_sources,
        EmbeddedComponentCache, EmbeddedComponentKey,
    };

    // 走査する root は `EMBEDDED_COMPONENT_KEY_ROOTS` が正本で、下の `rerun-if-changed` と
    // 同じ一覧から導かれる (I-16)。ここに root を直接書かないこと。
    let key = match (
        cache_root_from_out_dir(out_dir),
        embedded_component_key_sources(project_root),
        current_executable_fingerprint(),
    ) {
        (Some(root), Ok(sources), Ok(emitter)) => {
            Some((root, EmbeddedComponentKey::from_parts(&sources, &emitter)))
        }
        _ => None,
    };

    if let Some((root, key)) = &key {
        let cache = EmbeddedComponentCache::new(root);
        match cache.load(key) {
            Ok(Some(bytes)) => {
                // hit は正常系なので cargo:warning にしない (毎 build 警告が出るのは邪魔)。
                // `cargo build -vv` で見える plain stdout に留める。
                println!("embedded component cache hit ({key})");
                return bytes;
            }
            Ok(None) => {}
            Err(error) => println!("cargo:warning=embedded component cache の読み込みを諦めます: {error}"),
        }
    }

    let component = build_default_embedded_component(project_root);

    if let Some((root, key)) = &key {
        let cache = EmbeddedComponentCache::new(root);
        if let Err(error) = cache.store(key, &component) {
            println!("cargo:warning=embedded component cache の保存を諦めます: {error}");
        }
        // 1 entry が 1MB 超なので上限を設ける。branch 往復と直近の編集を数世代賄える程度に取る。
        if let Err(error) = cache.trim_to_entries(EMBEDDED_COMPONENT_CACHE_ENTRIES) {
            println!("cargo:warning=embedded component cache の刈り込みを諦めます: {error}");
        }
    }
    component
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
    // cache key の入力と build script の再実行条件を同じ一覧から出す。片方にだけ root が
    // 載っている状態を作らないための措置である (I-16)。
    for root in lsharp_wasm::embedded_component_cache::EMBEDDED_COMPONENT_KEY_ROOTS {
        let mut path = project_root.clone();
        for segment in root.split('/') {
            path.push(segment);
        }
        println!("cargo:rerun-if-changed={}", path.display());
    }

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
            let component = cached_default_embedded_component(&project_root, &out_dir);
            write_embedded_component(&component, &embedded_output);
            println!("cargo:rustc-env=LSHARP_EMBEDDED_COMPONENT_PRESENT=1");
        }
    }
}

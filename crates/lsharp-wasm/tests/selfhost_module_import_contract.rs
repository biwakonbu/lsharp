//! selfhost module の import 契約を、ソース走査だけで検査する。
//!
//! `Types/TypeInfer.ls` は推論の主要関数を stub として持ち、実装は
//! `TypeInferApply` / `TypeInferBlock` / `TypeInferPattern` / `TypeInferRecord` が
//! 同名 `defn` で上書きする。上書きは後勝ちで解決される
//! (`Backend/Wasm/CompilerBase.ls` の `ftable-lookup-loop` が末尾側から走査する)。
//!
//! したがって **`Types.TypeInfer` を import 閉包に含む entry module は、
//! override 4 本も import 閉包に含めなければならない**。含めないと、compile も実行も
//! 成功したまま推論だけが静かに緩む (`I-101` / `I-102`)。
//!
//! この契約は `2b0c54b1` (2026-07-20) が確立したが commit message にしか残らず、
//! その後に足された entry で守られなかった。本 test はそれを機械で守る。
//! 裁定は `docs/adr/decisions-selfhost-typeinfer-stub-override.md` の決定 3。
//!
//! **compile も wasm 実行もしない純ファイル走査なので `#[ignore]` を付けない。**
//! lane 送りにすると保護価値がほぼ消える。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// 上書き実装を持つ module 群。`Types.TypeInfer` と同時に link されねばならない。
const TYPEINFER_OVERRIDE_MODULES: &[&str] = &[
    "Types.TypeInferApply",
    "Types.TypeInferBlock",
    "Types.TypeInferPattern",
    "Types.TypeInferRecord",
];

const TYPEINFER_BASE_MODULE: &str = "Types.TypeInfer";

fn selfhost_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost/src")
}

fn collect_ls_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("selfhost/src の走査に失敗 {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry が読めない").path();
        if path.is_dir() {
            collect_ls_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "ls") {
            out.push(path);
        }
    }
}

struct ModuleInfo {
    imports: BTreeSet<String>,
    has_main: bool,
    path: PathBuf,
}

/// `(module X)` / `(import Y)` / 行頭 `(defn main ` を行単位で拾う。
///
/// selfhost のソースはこの 3 つをいずれも行頭に書く規約なので、
/// S 式を完全にパースせずとも判定できる。
fn scan_modules() -> BTreeMap<String, ModuleInfo> {
    let mut files = Vec::new();
    collect_ls_files(&selfhost_src_root(), &mut files);
    assert!(
        files.len() > 50,
        "selfhost/src の .ls が {} 本しか見つからない。走査経路が壊れている",
        files.len()
    );

    let mut modules = BTreeMap::new();
    for path in files {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("読み込み失敗 {}: {e}", path.display()));
        let mut name = None;
        let mut imports = BTreeSet::new();
        let mut has_main = false;
        for line in source.lines() {
            if let Some(rest) = line.strip_prefix("(module ") {
                if let Some(m) = rest.strip_suffix(')') {
                    name = Some(m.trim().to_string());
                }
            } else if let Some(rest) = line.strip_prefix("(import ") {
                let head = rest
                    .split([' ', ')'])
                    .next()
                    .expect("import 行に module 名がない");
                imports.insert(head.to_string());
            } else if line.starts_with("(defn main ") || line.starts_with("(defn main[") {
                has_main = true;
            }
        }
        if let Some(name) = name {
            modules.insert(
                name,
                ModuleInfo {
                    imports,
                    has_main,
                    path,
                },
            );
        }
    }
    modules
}

fn import_closure(modules: &BTreeMap<String, ModuleInfo>, entry: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![entry.to_string()];
    while let Some(current) = stack.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if let Some(info) = modules.get(&current) {
            for dep in &info.imports {
                if !seen.contains(dep) {
                    stack.push(dep.clone());
                }
            }
        }
    }
    seen
}

/// override 4 本が base を import していること。
///
/// **これが順序契約の土台である。** Rust の topological sort でも selfhost の
/// post-order append でも、この import があるかぎり base が先に来るので
/// 後勝ちで override が勝つ。逆向きに張ると SCC になり、両 backend で順序が
/// 食い違う (ADR の「却下した案」)。
#[test]
fn test_typeinfer_override_modules_import_the_base_module() {
    let modules = scan_modules();
    for name in TYPEINFER_OVERRIDE_MODULES {
        let info = modules
            .get(*name)
            .unwrap_or_else(|| panic!("{name} が selfhost/src に見つからない"));
        assert!(
            info.imports.contains(TYPEINFER_BASE_MODULE),
            "{name} は {TYPEINFER_BASE_MODULE} を import しなければならない \
             (後勝ち解決で override を後ろに置くため)"
        );
    }
}

/// `Types.TypeInfer` を閉包に含む entry は override 4 本も含むこと。
#[test]
fn test_selfhost_entry_modules_coimport_typeinfer_overrides() {
    let modules = scan_modules();
    let mut violations = Vec::new();

    for (name, info) in &modules {
        if !info.has_main {
            continue;
        }
        // base 自身と override 自身は entry ではないので対象外。
        if name == TYPEINFER_BASE_MODULE || TYPEINFER_OVERRIDE_MODULES.contains(&name.as_str()) {
            continue;
        }
        let closure = import_closure(&modules, name);
        if !closure.contains(TYPEINFER_BASE_MODULE) {
            continue;
        }
        let missing: Vec<&str> = TYPEINFER_OVERRIDE_MODULES
            .iter()
            .filter(|m| !closure.contains(**m))
            .copied()
            .collect();
        if !missing.is_empty() {
            violations.push(format!(
                "  {name} ({}) が欠いている: {}",
                info.path
                    .strip_prefix(selfhost_src_root())
                    .unwrap_or(&info.path)
                    .display(),
                missing.join(", ")
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Types.TypeInfer を link する entry module が override を co-import していない。\n\
         module-graph 経路で link すると stub 推論器が無診断で使われる (I-101 / I-102)。\n\
         App/Cli.ls:18-22 の順序を写して import を足すこと。\n{}",
        violations.join("\n")
    );
}

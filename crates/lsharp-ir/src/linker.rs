use super::{GcTypeDef, GcTypeKind, Instruction, IrType, Module};

/// 複数の IR モジュールを単一モジュールにリンク
///
/// 関数インデックスとGC型インデックスをリベースして結合する。
pub fn link_modules(modules: &[Module]) -> Module {
    use std::collections::HashMap;

    let mut linked_functions = Vec::new();
    let mut linked_gc_types = Vec::new();
    let mut linked_imports = Vec::new();

    // import 関数の重複除去
    // (module, name) -> 新 import index
    let mut import_dedup: HashMap<(String, String), u32> = HashMap::new();
    // (モジュールindex, 旧import_index) -> 新import_index
    let mut import_remap: HashMap<(usize, u32), u32> = HashMap::new();
    // linked import が採用した module-local signature の出所。
    let mut import_sources: Vec<(usize, u32)> = Vec::new();

    for (mod_idx, module) in modules.iter().enumerate() {
        for (old_idx, imp) in module.imports.iter().enumerate() {
            let key = (imp.module.clone(), imp.name.clone());
            if let Some(&existing_idx) = import_dedup.get(&key) {
                import_remap.insert((mod_idx, old_idx as u32), existing_idx);
            } else {
                let new_idx = linked_imports.len() as u32;
                import_dedup.insert(key, new_idx);
                import_remap.insert((mod_idx, old_idx as u32), new_idx);
                linked_imports.push(imp.clone());
                import_sources.push((mod_idx, old_idx as u32));
            }
        }
    }

    // GC 型インデックスのリベースマップ
    // (モジュールindex, 旧型index) -> 新型index
    let mut gc_type_remap: HashMap<(usize, u32), u32> = HashMap::new();

    // まず全 GC 型を集約
    for (mod_idx, module) in modules.iter().enumerate() {
        for (old_idx, gc_type) in module.gc_types.iter().enumerate() {
            let new_idx = linked_gc_types.len() as u32;
            gc_type_remap.insert((mod_idx, old_idx as u32), new_idx);
            linked_gc_types.push(gc_type.clone());
        }
    }

    // 関数インデックスのリベースマップ
    // (モジュールindex, 旧関数index) -> 新関数index
    let mut func_remap: HashMap<(usize, u32), u32> = HashMap::new();
    let mut func_idx = 0u32;

    // import 関数数分オフセット（各モジュールの import 数を考慮）
    let total_imports = linked_imports.len() as u32;

    for (mod_idx, module) in modules.iter().enumerate() {
        let module_import_count = module.imports.len() as u32;
        for (old_idx, _func) in module.functions.iter().enumerate() {
            // ユーザー関数は import 数分オフセット
            func_remap.insert(
                (mod_idx, old_idx as u32 + module_import_count),
                func_idx + total_imports,
            );
            func_idx += 1;
        }
    }

    // `CallRef` が参照する function type index のリベースマップ。
    // IR の type section は GC 型 → import 関数型 → user 関数型の順で構成する。
    let mut function_type_remap: HashMap<(usize, u32), u32> = HashMap::new();
    let linked_function_type_start = linked_gc_types.len() as u32 + total_imports;
    let mut linked_function_offset = 0u32;
    for (mod_idx, module) in modules.iter().enumerate() {
        let old_import_type_start = module.gc_types.len() as u32;
        for (old_import_idx, _) in module.imports.iter().enumerate() {
            if let Some(&new_import_idx) = import_remap.get(&(mod_idx, old_import_idx as u32)) {
                function_type_remap.insert(
                    (mod_idx, old_import_type_start + old_import_idx as u32),
                    linked_gc_types.len() as u32 + new_import_idx,
                );
            }
        }

        let old_function_type_start = old_import_type_start + module.imports.len() as u32;
        for old_function_idx in 0..module.functions.len() as u32 {
            function_type_remap.insert(
                (mod_idx, old_function_type_start + old_function_idx),
                linked_function_type_start + linked_function_offset + old_function_idx,
            );
        }
        linked_function_offset += module.functions.len() as u32;
    }

    // 関数シグネチャと GC 型定義にも module-local な型 index が残るため、
    // 命令列と同じ remap を適用する。WasmGC の env struct は field に
    // `Ref(gc_type)` と `TypedFuncRef(function_type)` の両方を持つため、
    // 命令だけを直しても linked module の型境界が壊れる。
    for (mod_idx, module) in modules.iter().enumerate() {
        for (old_idx, gc_type) in module.gc_types.iter().enumerate() {
            let Some(&new_idx) = gc_type_remap.get(&(mod_idx, old_idx as u32)) else {
                continue;
            };
            let mut remapped = gc_type.clone();
            remap_gc_type_definition(&mut remapped, mod_idx, &gc_type_remap, &function_type_remap);
            linked_gc_types[new_idx as usize] = remapped;
        }
    }

    // import の params/result も linked type index の境界に含める。重複 import は最初に
    // 採用した module-local signature を正本として remap する。
    for (linked_idx, (source_mod_idx, source_import_idx)) in import_sources.iter().enumerate() {
        let source_import = &modules[*source_mod_idx].imports[*source_import_idx as usize];
        let linked_import = &mut linked_imports[linked_idx];
        for ty in &mut linked_import.params {
            *ty = remap_ir_type(*ty, *source_mod_idx, &gc_type_remap, &function_type_remap);
        }
        linked_import.result = remap_ir_type(
            source_import.result,
            *source_mod_idx,
            &gc_type_remap,
            &function_type_remap,
        );
    }

    // 全関数を集約（命令のインデックスをリベース）
    for (mod_idx, module) in modules.iter().enumerate() {
        let module_import_count = module.imports.len() as u32;
        for func in &module.functions {
            let mut new_func = func.clone();

            for ty in &mut new_func.params {
                *ty = remap_ir_type(*ty, mod_idx, &gc_type_remap, &function_type_remap);
            }
            new_func.result = remap_ir_type(
                new_func.result,
                mod_idx,
                &gc_type_remap,
                &function_type_remap,
            );
            for ty in &mut new_func.locals {
                *ty = remap_ir_type(*ty, mod_idx, &gc_type_remap, &function_type_remap);
            }

            // 命令内のインデックスをリベース
            for instr in &mut new_func.body {
                remap_instruction_with_imports(
                    instr,
                    mod_idx,
                    module_import_count,
                    &func_remap,
                    &import_remap,
                    &gc_type_remap,
                    &function_type_remap,
                );
            }

            linked_functions.push(new_func);
        }
    }

    Module {
        functions: linked_functions,
        gc_types: linked_gc_types,
        imports: linked_imports,
        globals: Vec::new(),
        string_data: Vec::new(),
    }
}

/// module-local な GC/function type index を linked module の index へ変換する。
fn remap_ir_type(
    ty: IrType,
    mod_idx: usize,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
    function_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) -> IrType {
    match ty {
        IrType::Ref(index) => gc_type_remap
            .get(&(mod_idx, index))
            .copied()
            .map(IrType::Ref)
            .unwrap_or(IrType::Ref(index)),
        IrType::TypedFuncRef(index) => function_type_remap
            .get(&(mod_idx, index))
            .copied()
            .map(IrType::TypedFuncRef)
            .unwrap_or(IrType::TypedFuncRef(index)),
        other => other,
    }
}

/// GC struct/array の field type に linked module の型 index を適用する。
fn remap_gc_type_definition(
    gc_type: &mut GcTypeDef,
    mod_idx: usize,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
    function_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) {
    match &mut gc_type.kind {
        GcTypeKind::Struct(fields) => {
            for field in fields {
                field.ty = remap_ir_type(field.ty, mod_idx, gc_type_remap, function_type_remap);
            }
        }
        GcTypeKind::Array(element_type) => {
            *element_type =
                remap_ir_type(*element_type, mod_idx, gc_type_remap, function_type_remap);
        }
        GcTypeKind::PackedByteArray => {}
    }
}

/// 命令内のインデックスをリベース（import 対応版）
fn remap_instruction_with_imports(
    instr: &mut Instruction,
    mod_idx: usize,
    module_import_count: u32,
    func_remap: &std::collections::HashMap<(usize, u32), u32>,
    import_remap: &std::collections::HashMap<(usize, u32), u32>,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
    function_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) {
    match instr {
        Instruction::Call(idx) | Instruction::RefFunc(idx) => {
            if *idx < module_import_count {
                // import 関数の呼び出し
                if let Some(&new_idx) = import_remap.get(&(mod_idx, *idx)) {
                    *idx = new_idx;
                }
            } else {
                // ユーザー関数の呼び出し
                if let Some(&new_idx) = func_remap.get(&(mod_idx, *idx)) {
                    *idx = new_idx;
                }
            }
        }
        Instruction::StructNew(idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        Instruction::StructGet(type_idx, _) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::StructSet(type_idx, _) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::RefCast(idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        Instruction::RefNull(idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        Instruction::ArrayNewFixed(type_idx, _)
        | Instruction::ArrayNewDefault(type_idx)
        | Instruction::ArrayGet(type_idx)
        | Instruction::ArraySet(type_idx)
        | Instruction::ArrayLen(type_idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::CallRef(type_idx) => {
            if let Some(&new_idx) = function_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::CallIndirect(_) => {
            // CallIndirect の型インデックスはリマップ不要
        }
        Instruction::FuncIdx(idx) => {
            // FuncIdx は Call と同じインデックス空間
            if *idx < module_import_count {
                if let Some(&new_idx) = import_remap.get(&(mod_idx, *idx)) {
                    *idx = new_idx;
                }
            } else if let Some(&new_idx) = func_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        _ => {}
    }
}

//! L# 中間表現 (IR)
//!
//! MVP ではフラット化された命令列を使用。
//! 将来的に SSA 形式の BasicBlock ベースに拡張する。

pub mod lower;
pub mod module_graph;

use std::fmt;

/// IR モジュール（コンパイル単位）
#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
    /// GC 型定義（WasmGC struct 用）
    pub gc_types: Vec<GcTypeDef>,
    /// import 関数定義
    pub imports: Vec<ImportFunc>,
    /// グローバル変数定義（辞書インスタンス等）
    pub globals: Vec<GlobalDef>,
    /// 文字列定数データ（data section 用）
    pub string_data: Vec<(String, Vec<u8>)>,
}

/// import 関数定義
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportFunc {
    /// import 元モジュール名 (例: "wasi_snapshot_preview1")
    pub module: String,
    /// import 関数名 (例: "fd_write")
    pub name: String,
    /// パラメータ型
    pub params: Vec<IrType>,
    /// 戻り値型
    pub result: IrType,
}

/// グローバル変数定義（辞書インスタンス等）
#[derive(Debug, Clone)]
pub struct GlobalDef {
    pub name: String,
    pub ty: IrType,
    pub mutable: bool,
    /// 初期値命令列（const 式）
    pub init: Vec<Instruction>,
}

/// GC 型定義（WasmGC struct/array 用）
#[derive(Debug, Clone)]
pub struct GcTypeDef {
    pub name: String,
    pub kind: GcTypeKind,
}

/// GC 型の種別
#[derive(Debug, Clone)]
pub enum GcTypeKind {
    /// struct 型 (レコード, ADT バリアント)
    Struct(Vec<GcField>),
    /// array 型 (文字列等)
    Array(IrType),
}

/// GC struct のフィールド
#[derive(Debug, Clone)]
pub struct GcField {
    pub name: String,
    pub ty: IrType,
    pub mutable: bool,
}

/// 関数定義
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<IrType>,
    pub result: IrType,
    pub locals: Vec<IrType>,
    pub body: Vec<Instruction>,
    pub is_export: bool,
}

/// IR の型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrType {
    I64,
    F64,
    I32,
    /// GC 参照型 (WasmGC struct/array への参照)
    Ref(u32),
    /// 関数参照型 (funcref)
    FuncRef,
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::I64 => write!(f, "i64"),
            IrType::F64 => write!(f, "f64"),
            IrType::I32 => write!(f, "i32"),
            IrType::Ref(idx) => write!(f, "ref({idx})"),
            IrType::FuncRef => write!(f, "funcref"),
        }
    }
}

/// IR 命令
#[derive(Debug, Clone)]
pub enum Instruction {
    // 定数
    I64Const(i64),
    F64Const(f64),
    I32Const(i32),

    // ローカル変数操作
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),

    // 整数演算
    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
    I64Rem,

    // 浮動小数点演算
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,

    // 比較演算 (結果は i32)
    I64Eq,
    I64Ne,
    I64LtS,
    I64GtS,
    I64LeS,
    I64GeS,

    // 論理演算 (i32)
    I32Eqz,
    I32And,
    I32Or,

    // 型変換
    I64ExtendI32S,
    I32WrapI64,

    // 制御フロー
    Call(u32),        // 関数インデックス
    If(IrType),       // if-then-else 開始（結果型付き）
    Else,
    End,
    Block(IrType),    // ブロック開始
    Loop(IrType),     // ループ開始
    Br(u32),          // 分岐
    BrIf(u32),        // 条件分岐
    Return,
    Unreachable,

    // ホスト関数呼び出し
    CallImport(u32),  // import された関数のインデックス

    // スタック操作
    Drop,

    // GC 命令 (WasmGC)
    StructNew(u32),         // struct.new type_idx
    StructGet(u32, u32),    // struct.get type_idx field_idx
    StructSet(u32, u32),    // struct.set type_idx field_idx
    RefCast(u32),           // ref.cast type_idx (ダウンキャスト)

    // 関数参照 (vtable/辞書パスイング)
    RefFunc(u32),           // ref.func func_idx
    CallRef(u32),           // call_ref type_idx (funcref 経由の間接呼び出し)

    // グローバル変数
    GlobalGet(u32),         // global.get idx
    GlobalSet(u32),         // global.set idx

    // メモリ操作
    I32Load { offset: u32 },
    I32Store { offset: u32 },
    I32Load8U { offset: u32 },
    I32Store8 { offset: u32 },
    I64Load { offset: u32 },
    I64Store { offset: u32 },

    // 型変換（符号なし拡張）
    I64ExtendI32U,

    // i32 算術演算
    I32Add,
    I32Sub,
    I32Mul,

    // i32 比較（符号なし）
    I32GtU,
    I32GeU,

    // ビット操作
    I32Shl,
    I32ShrU,

    // メモリ管理
    MemoryGrow,
    MemorySize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::I64Const(n) => write!(f, "i64.const {n}"),
            Instruction::F64Const(n) => write!(f, "f64.const {n}"),
            Instruction::I32Const(n) => write!(f, "i32.const {n}"),
            Instruction::LocalGet(i) => write!(f, "local.get {i}"),
            Instruction::LocalSet(i) => write!(f, "local.set {i}"),
            Instruction::LocalTee(i) => write!(f, "local.tee {i}"),
            Instruction::I64Add => write!(f, "i64.add"),
            Instruction::I64Sub => write!(f, "i64.sub"),
            Instruction::I64Mul => write!(f, "i64.mul"),
            Instruction::I64Div => write!(f, "i64.div_s"),
            Instruction::I64Rem => write!(f, "i64.rem_s"),
            Instruction::F64Add => write!(f, "f64.add"),
            Instruction::F64Sub => write!(f, "f64.sub"),
            Instruction::F64Mul => write!(f, "f64.mul"),
            Instruction::F64Div => write!(f, "f64.div"),
            Instruction::I64Eq => write!(f, "i64.eq"),
            Instruction::I64Ne => write!(f, "i64.ne"),
            Instruction::I64LtS => write!(f, "i64.lt_s"),
            Instruction::I64GtS => write!(f, "i64.gt_s"),
            Instruction::I64LeS => write!(f, "i64.le_s"),
            Instruction::I64GeS => write!(f, "i64.ge_s"),
            Instruction::I32Eqz => write!(f, "i32.eqz"),
            Instruction::I32And => write!(f, "i32.and"),
            Instruction::I32Or => write!(f, "i32.or"),
            Instruction::I64ExtendI32S => write!(f, "i64.extend_i32_s"),
            Instruction::I32WrapI64 => write!(f, "i32.wrap_i64"),
            Instruction::Call(i) => write!(f, "call {i}"),
            Instruction::If(ty) => write!(f, "if ({ty})"),
            Instruction::Else => write!(f, "else"),
            Instruction::End => write!(f, "end"),
            Instruction::Block(ty) => write!(f, "block ({ty})"),
            Instruction::Loop(ty) => write!(f, "loop ({ty})"),
            Instruction::Br(i) => write!(f, "br {i}"),
            Instruction::BrIf(i) => write!(f, "br_if {i}"),
            Instruction::Return => write!(f, "return"),
            Instruction::Unreachable => write!(f, "unreachable"),
            Instruction::CallImport(i) => write!(f, "call_import {i}"),
            Instruction::Drop => write!(f, "drop"),
            Instruction::StructNew(idx) => write!(f, "struct.new {idx}"),
            Instruction::StructGet(type_idx, field_idx) => {
                write!(f, "struct.get {type_idx} {field_idx}")
            }
            Instruction::StructSet(type_idx, field_idx) => {
                write!(f, "struct.set {type_idx} {field_idx}")
            }
            Instruction::RefCast(idx) => write!(f, "ref.cast {idx}"),
            Instruction::RefFunc(idx) => write!(f, "ref.func {idx}"),
            Instruction::CallRef(idx) => write!(f, "call_ref {idx}"),
            Instruction::GlobalGet(idx) => write!(f, "global.get {idx}"),
            Instruction::GlobalSet(idx) => write!(f, "global.set {idx}"),
            // メモリ操作
            Instruction::I32Load { offset } => write!(f, "i32.load offset={offset}"),
            Instruction::I32Store { offset } => write!(f, "i32.store offset={offset}"),
            Instruction::I32Load8U { offset } => write!(f, "i32.load8_u offset={offset}"),
            Instruction::I32Store8 { offset } => write!(f, "i32.store8 offset={offset}"),
            Instruction::I64Load { offset } => write!(f, "i64.load offset={offset}"),
            Instruction::I64Store { offset } => write!(f, "i64.store offset={offset}"),
            // 型変換
            Instruction::I64ExtendI32U => write!(f, "i64.extend_i32_u"),
            // i32 算術演算
            Instruction::I32Add => write!(f, "i32.add"),
            Instruction::I32Sub => write!(f, "i32.sub"),
            Instruction::I32Mul => write!(f, "i32.mul"),
            // i32 比較
            Instruction::I32GtU => write!(f, "i32.gt_u"),
            Instruction::I32GeU => write!(f, "i32.ge_u"),
            // ビット操作
            Instruction::I32Shl => write!(f, "i32.shl"),
            Instruction::I32ShrU => write!(f, "i32.shr_u"),
            // メモリ管理
            Instruction::MemoryGrow => write!(f, "memory.grow"),
            Instruction::MemorySize => write!(f, "memory.size"),
        }
    }
}

impl Module {
    /// IR のテキスト表示
    pub fn dump(&self) -> String {
        let mut out = String::new();

        // GC 型定義を出力
        for gc_type in &self.gc_types {
            out.push_str(&format!("gc_type {} = ", gc_type.name));
            match &gc_type.kind {
                GcTypeKind::Struct(fields) => {
                    out.push_str("struct {");
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        let mut_str = if field.mutable { "mut " } else { "" };
                        out.push_str(&format!("{}{}: {}", mut_str, field.name, field.ty));
                    }
                    out.push_str("}\n");
                }
                GcTypeKind::Array(elem_ty) => {
                    out.push_str(&format!("array({elem_ty})\n"));
                }
            }
        }

        if !self.gc_types.is_empty() {
            out.push('\n');
        }

        for func in &self.functions {
            out.push_str(&format!("fn {}(", func.name));
            for (i, p) in func.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{p}"));
            }
            out.push_str(&format!(") -> {}:\n", func.result));

            if !func.locals.is_empty() {
                out.push_str("  locals: ");
                for (i, l) in func.locals.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("{l}"));
                }
                out.push('\n');
            }

            for instr in &func.body {
                out.push_str(&format!("  {instr}\n"));
            }
            out.push('\n');
        }
        out
    }
}

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

    // 全関数を集約（命令のインデックスをリベース）
    for (mod_idx, module) in modules.iter().enumerate() {
        let module_import_count = module.imports.len() as u32;
        for func in &module.functions {
            let mut new_func = func.clone();

            // 命令内のインデックスをリベース
            for instr in &mut new_func.body {
                remap_instruction_with_imports(
                    instr,
                    mod_idx,
                    module_import_count,
                    &func_remap,
                    &import_remap,
                    &gc_type_remap,
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

/// 命令内のインデックスをリベース（後方互換用）
#[allow(dead_code)]
fn remap_instruction(
    instr: &mut Instruction,
    mod_idx: usize,
    func_remap: &std::collections::HashMap<(usize, u32), u32>,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) {
    match instr {
        Instruction::Call(idx) => {
            if let Some(&new_idx) = func_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
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
        _ => {}
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
) {
    match instr {
        Instruction::Call(idx) => {
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
        _ => {}
    }
}

#[cfg(test)]
mod linker_tests {
    use super::*;

    #[test]
    fn test_link_empty_modules() {
        let modules: Vec<Module> = vec![];
        let linked = link_modules(&modules);
        assert!(linked.functions.is_empty());
        assert!(linked.gc_types.is_empty());
    }

    #[test]
    fn test_link_single_module() {
        let module = Module {
            functions: vec![Function {
                name: "main".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::I64Const(42)],
                is_export: true,
            }],
            gc_types: vec![],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };
        let linked = link_modules(&[module]);
        assert_eq!(linked.functions.len(), 1);
        assert_eq!(linked.functions[0].name, "main");
    }

    #[test]
    fn test_link_two_modules() {
        let mod_a = Module {
            functions: vec![Function {
                name: "helper".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::LocalGet(0),
                    Instruction::I64Const(1),
                    Instruction::I64Add,
                ],
                is_export: false,
            }],
            gc_types: vec![],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };
        let mod_b = Module {
            functions: vec![Function {
                name: "main".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::I64Const(41),
                    Instruction::Call(0), // mod_b 内の index 0 = helper(mod_a)
                ],
                is_export: true,
            }],
            gc_types: vec![],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[mod_a, mod_b]);
        assert_eq!(linked.functions.len(), 2);
        assert_eq!(linked.functions[0].name, "helper");
        assert_eq!(linked.functions[1].name, "main");
    }

    #[test]
    fn test_link_gc_type_rebase() {
        let mod_a = Module {
            functions: vec![],
            gc_types: vec![GcTypeDef {
                name: "Point".to_string(),
                kind: GcTypeKind::Struct(vec![
                    GcField { name: "x".to_string(), ty: IrType::I64, mutable: false },
                    GcField { name: "y".to_string(), ty: IrType::I64, mutable: false },
                ]),
            }],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };
        let mod_b = Module {
            functions: vec![Function {
                name: "make_point".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::I64Const(1),
                    Instruction::I64Const(2),
                    Instruction::StructNew(0), // mod_b 内の GC type 0
                ],
                is_export: true,
            }],
            gc_types: vec![GcTypeDef {
                name: "Color".to_string(),
                kind: GcTypeKind::Struct(vec![
                    GcField { name: "r".to_string(), ty: IrType::I64, mutable: false },
                ]),
            }],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[mod_a, mod_b]);
        assert_eq!(linked.gc_types.len(), 2);
        assert_eq!(linked.gc_types[0].name, "Point");
        assert_eq!(linked.gc_types[1].name, "Color");

        // mod_b の StructNew(0) は新しいインデックス 1 にリベースされる
        if let Instruction::StructNew(idx) = &linked.functions[0].body[2] {
            assert_eq!(*idx, 1);
        } else {
            panic!("Expected StructNew");
        }
    }
}

#[cfg(test)]
mod import_dedup_tests {
    use super::*;

    #[test]
    fn test_import_deduplication() {
        // 両方のモジュールが同じ import (wasi fd_write) を持つ場合、1つに統合される
        let mod_a = Module {
            functions: vec![Function {
                name: "write_a".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::Call(0)], // import index 0
                is_export: false,
            }],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "wasi_snapshot_preview1".to_string(),
                name: "fd_write".to_string(),
                params: vec![IrType::I32, IrType::I32, IrType::I32, IrType::I32],
                result: IrType::I32,
            }],
            globals: vec![],
            string_data: vec![],
        };
        let mod_b = Module {
            functions: vec![Function {
                name: "write_b".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::Call(0)], // import index 0 (同じ fd_write)
                is_export: false,
            }],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "wasi_snapshot_preview1".to_string(),
                name: "fd_write".to_string(),
                params: vec![IrType::I32, IrType::I32, IrType::I32, IrType::I32],
                result: IrType::I32,
            }],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[mod_a, mod_b]);
        // import は1つに重複除去される
        assert_eq!(linked.imports.len(), 1);
        assert_eq!(linked.imports[0].name, "fd_write");
        // 両方の関数が同じ import index 0 を参照
        if let Instruction::Call(idx) = &linked.functions[0].body[0] {
            assert_eq!(*idx, 0);
        }
        if let Instruction::Call(idx) = &linked.functions[1].body[0] {
            assert_eq!(*idx, 0);
        }
    }

    #[test]
    fn test_different_imports_not_deduplicated() {
        let mod_a = Module {
            functions: vec![],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "print".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
            }],
            globals: vec![],
            string_data: vec![],
        };
        let mod_b = Module {
            functions: vec![],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "read".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
            }],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[mod_a, mod_b]);
        assert_eq!(linked.imports.len(), 2);
        assert_eq!(linked.imports[0].name, "print");
        assert_eq!(linked.imports[1].name, "read");
    }

    #[test]
    fn test_empty_imports() {
        let module = Module {
            functions: vec![],
            gc_types: vec![],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };
        let linked = link_modules(&[module]);
        assert!(linked.imports.is_empty());
    }
}

#[cfg(test)]
mod memory_instruction_tests {
    use super::*;

    #[test]
    fn test_memory_load_store_instructions() {
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I32Load { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I32Const(42),
            Instruction::I32Store { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i64_memory_instructions() {
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I64Load { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I64Const(12345),
            Instruction::I64Store { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_byte_memory_instructions() {
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I32Load8U { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I32Const(65),
            Instruction::I32Store8 { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i32_arithmetic_instructions() {
        let instructions = vec![
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32Add,
            Instruction::I32Sub,
            Instruction::I32Mul,
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i32_comparison_instructions() {
        let instructions = vec![
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32GtU,
            Instruction::I32GeU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_i32_bitwise_instructions() {
        let instructions = vec![
            Instruction::I32Const(0xFF),
            Instruction::I32Const(4),
            Instruction::I32Shl,
            Instruction::I32ShrU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_memory_management_instructions() {
        let instructions = vec![
            Instruction::MemorySize,
            Instruction::I32Const(1),
            Instruction::MemoryGrow,
        ];
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_i64_extend_i32_unsigned() {
        let instructions = vec![
            Instruction::I32Const(42),
            Instruction::I64ExtendI32U,
        ];
        assert_eq!(instructions.len(), 2);
    }

    #[test]
    fn test_instruction_display_memory_ops() {
        assert_eq!(
            format!("{}", Instruction::I32Load { offset: 0 }),
            "i32.load offset=0"
        );
        assert_eq!(
            format!("{}", Instruction::I32Store { offset: 4 }),
            "i32.store offset=4"
        );
        assert_eq!(
            format!("{}", Instruction::MemoryGrow),
            "memory.grow"
        );
        assert_eq!(
            format!("{}", Instruction::MemorySize),
            "memory.size"
        );
        assert_eq!(
            format!("{}", Instruction::I32Add),
            "i32.add"
        );
    }
}

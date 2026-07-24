//! L# 中間表現 (IR)
//!
//! MVP ではフラット化された命令列を使用。
//! 将来的に SSA 形式の BasicBlock ベースに拡張する。

pub mod cache;
pub mod closure;
pub mod lower;
pub mod module_graph;

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use lsharp_types::infer::ExprTypeKey;
use sha2::{Digest, Sha256};

pub use cache::{CompilationCache, ModuleCacheEntry, ModuleIrSegments};

/// SHA-256 ベースのソース fingerprint。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    pub fn from_source(source: &str) -> Self {
        Self::from_bytes(source.as_bytes())
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        Self(digest.into())
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let source = std::fs::read_to_string(path)?;
        Ok(Self::from_source(&source))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for SourceFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

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
    /// packed `i8` array 型 (UTF-8 byte string 等)
    PackedByteArray,
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
    /// 特定の関数型を指す non-nullable ではない concrete funcref。
    ///
    /// 値は IR の型セクション上の関数型インデックスを保持する。WasmGC
    /// emitter は synthetic import の有無に応じて最終的な型インデックスへ
    /// rebasing する。
    TypedFuncRef(u32),
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::I64 => write!(f, "i64"),
            IrType::F64 => write!(f, "f64"),
            IrType::I32 => write!(f, "i32"),
            IrType::Ref(idx) => write!(f, "ref({idx})"),
            IrType::FuncRef => write!(f, "funcref"),
            IrType::TypedFuncRef(idx) => write!(f, "funcref({idx})"),
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
    Call(u32),  // 関数インデックス
    If(IrType), // if-then-else 開始（結果型付き）
    Else,
    End,
    Block(IrType), // ブロック開始 (結果型あり)
    Loop(IrType),  // ループ開始 (結果型あり)
    BlockEmpty,    // ブロック開始 (結果型なし)
    LoopEmpty,     // ループ開始 (結果型なし)
    IfEmpty,       // if-then-else 開始 (結果型なし)
    Br(u32),       // 分岐
    BrIf(u32),     // 条件分岐
    Return,
    Unreachable,

    // ホスト関数呼び出し
    CallImport(u32), // import された関数のインデックス

    // バックエンド専用ランタイム命令
    // 既存の import index を消費せず、Vector の下位 8 bit を raw bytes として書き込む。
    WriteFileBytes,

    // スタック操作
    Drop,

    // GC 命令 (WasmGC)
    StructNew(u32),          // struct.new type_idx
    StructGet(u32, u32),     // struct.get type_idx field_idx
    StructSet(u32, u32),     // struct.set type_idx field_idx
    RefCast(u32),            // ref.cast type_idx (ダウンキャスト)
    RefNull(u32),            // ref.null concrete type_idx
    ArrayNewFixed(u32, u32), // array.new_fixed type_idx length
    ArrayNewDefault(u32),    // array.new_default type_idx (dynamic length)
    ArrayGet(u32),           // array.get type_idx
    ArraySet(u32),           // array.set type_idx
    ArrayLen(u32),           // array.len type_idx (validation metadata)

    // 関数参照 (vtable/辞書パスイング)
    RefFunc(u32), // ref.func func_idx
    CallRef(u32), // call_ref type_idx (funcref 経由の間接呼び出し)

    // グローバル変数
    GlobalGet(u32), // global.get idx
    GlobalSet(u32), // global.set idx

    // メモリ操作
    I32Load {
        offset: u32,
    },
    I32Store {
        offset: u32,
    },
    I32Load8U {
        offset: u32,
    },
    I32Store8 {
        offset: u32,
    },
    I64Load {
        offset: u32,
    },
    I64Store {
        offset: u32,
    },

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
    I64Shl,
    I64ShrU,
    I64And,
    I64Or,
    I64Xor,

    // メモリ管理
    MemoryGrow,
    MemorySize,
    MemoryCopy,
    MemoryFill,

    // 間接呼び出し (クロージャ用)
    /// call_indirect: テーブルインデックスと型インデックスで間接呼び出し
    /// type_idx はリフト関数の型インデックスを指す
    CallIndirect(u32),

    /// 関数インデックスを i32 値としてスタックに積む
    /// Call(idx) と同じインデックス空間。codegen でリマップされる。
    FuncIdx(u32),

    /// 文字列定数: string_data のインデックスを指す
    /// codegen でヒープ上に String オブジェクト (tag=1, len, bytes) を確保し、アドレスを返す
    StringConst(u32),
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
            Instruction::BlockEmpty => write!(f, "block"),
            Instruction::LoopEmpty => write!(f, "loop"),
            Instruction::IfEmpty => write!(f, "if"),
            Instruction::Br(i) => write!(f, "br {i}"),
            Instruction::BrIf(i) => write!(f, "br_if {i}"),
            Instruction::Return => write!(f, "return"),
            Instruction::Unreachable => write!(f, "unreachable"),
            Instruction::CallImport(i) => write!(f, "call_import {i}"),
            Instruction::WriteFileBytes => write!(f, "write_file_bytes"),
            Instruction::Drop => write!(f, "drop"),
            Instruction::StructNew(idx) => write!(f, "struct.new {idx}"),
            Instruction::StructGet(type_idx, field_idx) => {
                write!(f, "struct.get {type_idx} {field_idx}")
            }
            Instruction::StructSet(type_idx, field_idx) => {
                write!(f, "struct.set {type_idx} {field_idx}")
            }
            Instruction::RefCast(idx) => write!(f, "ref.cast {idx}"),
            Instruction::RefNull(idx) => write!(f, "ref.null {idx}"),
            Instruction::ArrayNewFixed(type_idx, length) => {
                write!(f, "array.new_fixed {type_idx} {length}")
            }
            Instruction::ArrayNewDefault(type_idx) => write!(f, "array.new_default {type_idx}"),
            Instruction::ArrayGet(type_idx) => write!(f, "array.get {type_idx}"),
            Instruction::ArraySet(type_idx) => write!(f, "array.set {type_idx}"),
            Instruction::ArrayLen(type_idx) => write!(f, "array.len {type_idx}"),
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
            Instruction::I64Shl => write!(f, "i64.shl"),
            Instruction::I64ShrU => write!(f, "i64.shr_u"),
            Instruction::I64And => write!(f, "i64.and"),
            Instruction::I64Or => write!(f, "i64.or"),
            Instruction::I64Xor => write!(f, "i64.xor"),
            // メモリ管理
            Instruction::MemoryGrow => write!(f, "memory.grow"),
            Instruction::MemorySize => write!(f, "memory.size"),
            Instruction::MemoryCopy => write!(f, "memory.copy"),
            Instruction::MemoryFill => write!(f, "memory.fill"),
            Instruction::CallIndirect(type_idx) => write!(f, "call_indirect {type_idx}"),
            Instruction::FuncIdx(idx) => write!(f, "func_idx {idx}"),
            Instruction::StringConst(idx) => write!(f, "string_const {idx}"),
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
                GcTypeKind::PackedByteArray => {
                    out.push_str("array(i8)\n");
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

/// マルチファイルコンパイルのパイプライン
///
/// エントリファイルから依存関係を解決し、トポロジカルソート順に
/// パース → 型チェックを行い、全モジュールの AST を結合してから
/// IR 変換することで、関数インデックスの一貫性を保つ。
#[derive(Debug, Clone, Default)]
struct ImportVisibilitySpec {
    only: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModuleTypeSurface {
    results: Vec<(String, lsharp_types::types::TypeScheme)>,
    hidden: HashSet<String>,
    expr_types: HashMap<ExprTypeKey, lsharp_types::types::Type>,
}

impl ModuleTypeSurface {
    fn export_surface_eq(&self, other: &Self) -> bool {
        self.results == other.results && self.hidden == other.hidden
    }
}

fn type_surface_key(surface: &ModuleTypeSurface) -> u64 {
    let mut results = surface.results.clone();
    results.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hidden = surface.hidden.iter().cloned().collect::<Vec<_>>();
    hidden.sort();

    let mut hasher = DefaultHasher::new();
    results.hash(&mut hasher);
    hidden.hash(&mut hasher);
    hasher.finish()
}

fn dependency_surface_key(
    direct_imports: &HashMap<String, ImportVisibilitySpec>,
    current_surfaces: &HashMap<String, ModuleTypeSurface>,
    cache: &CompilationCache,
) -> u64 {
    let mut dependencies = direct_imports.keys().cloned().collect::<Vec<_>>();
    dependencies.sort();

    let mut hasher = DefaultHasher::new();
    for dependency in dependencies {
        dependency.hash(&mut hasher);
        if let Some(surface) = current_surfaces.get(&dependency) {
            type_surface_key(surface).hash(&mut hasher);
        } else if let Some(entry) = cache.get(&dependency) {
            type_surface_key(&entry.type_surface_clone()).hash(&mut hasher);
        } else {
            0u8.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn push_defn_origins_infer_order(
    decls: &[lsharp_syntax::ast::Decl],
    file_module: &str,
    module_prefix: Option<&str>,
    out: &mut Vec<String>,
) {
    use lsharp_syntax::ast::Decl;
    for decl in decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        match actual_decl {
            Decl::Defn { .. } => out.push(file_module.to_string()),
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let prefix = if let Some(outer) = module_prefix {
                    format!("{outer}.{name}")
                } else {
                    name.clone()
                };
                push_defn_origins_infer_order(body, file_module, Some(prefix.as_str()), out);
            }
            _ => {}
        }
    }
}

fn collect_import_visibility(
    program: &lsharp_syntax::ast::Program,
) -> HashMap<String, ImportVisibilitySpec> {
    let mut imports = HashMap::new();
    for decl in &program.decls {
        if let lsharp_syntax::ast::Decl::ImportDecl { module, only, .. } = decl {
            let entry = imports
                .entry(module.clone())
                .or_insert_with(ImportVisibilitySpec::default);
            match (&mut entry.only, only.as_ref()) {
                (None, None) => {}
                (slot @ None, Some(next)) => {
                    *slot = Some(next.clone());
                }
                (Some(existing), Some(next)) => {
                    for symbol in next {
                        if !existing.contains(symbol) {
                            existing.push(symbol.clone());
                        }
                    }
                }
                (Some(_), None) => {
                    entry.only = None;
                }
            }
        }
    }
    imports
}

fn collect_import_modules(program: &lsharp_syntax::ast::Program) -> Vec<String> {
    let mut imports = Vec::new();
    let mut seen = HashSet::new();
    for decl in &program.decls {
        if let lsharp_syntax::ast::Decl::ImportDecl { module, .. } = decl
            && seen.insert(module.clone())
        {
            imports.push(module.clone());
        }
    }
    imports
}

fn parse_program_for_incremental(
    source: &str,
) -> Result<lsharp_syntax::ast::Program, lsharp_syntax::ParseAllError> {
    #[cfg(test)]
    {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_PARSE_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
    lsharp_syntax::parse(source)
}

fn cached_program_or_parse(
    mod_name: &str,
    source: &str,
    fingerprint: SourceFingerprint,
    cache: &CompilationCache,
) -> Result<Arc<lsharp_syntax::ast::Program>, lsharp_syntax::ParseAllError> {
    if let Some(entry) = cache.get(mod_name)
        && entry.fingerprint() == fingerprint
    {
        return Ok(entry.ast_arc());
    }
    parse_program_for_incremental(source).map(Arc::new)
}

fn note_incremental_type_infer() {
    #[cfg(test)]
    {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_scc_infer() {
    #[cfg(test)]
    {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_scc_merged_fast_path() {
    #[cfg(test)]
    {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_lower() {
    #[cfg(test)]
    {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LOWER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_module_segment_lower_by(_count: usize) {
    #[cfg(test)]
    {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|slot| {
                    slot.set(slot.get() + _count);
                });
            }
        });
    }
}

fn note_incremental_link_full() {
    #[cfg(test)]
    {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_link_cache_hit() {
    #[cfg(test)]
    {
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn build_module_cache_entry(
    fingerprint: SourceFingerprint,
    deps_key: u64,
    program: &Arc<lsharp_syntax::ast::Program>,
    type_surface: ModuleTypeSurface,
) -> ModuleCacheEntry {
    ModuleCacheEntry::new(
        fingerprint,
        deps_key,
        Arc::clone(program),
        type_surface,
        Module {
            functions: Vec::new(),
            gc_types: Vec::new(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: Vec::new(),
        },
        ModuleIrSegments::empty(),
        collect_import_modules(program.as_ref()),
    )
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_PARSE_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalParseTracker;

#[cfg(test)]
impl IncrementalParseTracker {
    fn new() -> Self {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_PARSE_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_PARSE_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_PARSE_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalParseTracker {
    fn drop(&mut self) {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_PARSE_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_TYPE_INFER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_TYPE_INFER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INCREMENTAL_SCC_INFER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_SCC_INFER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalTypeInferTracker;

#[cfg(test)]
impl IncrementalTypeInferTracker {
    fn new() -> Self {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalTypeInferTracker {
    fn drop(&mut self) {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
struct IncrementalSccInferTracker;

#[cfg(test)]
impl IncrementalSccInferTracker {
    fn new() -> Self {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalSccInferTracker {
    fn drop(&mut self) {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
struct IncrementalSccMergedFastPathTracker;

#[cfg(test)]
impl IncrementalSccMergedFastPathTracker {
    fn new() -> Self {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalSccMergedFastPathTracker {
    fn drop(&mut self) {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_LOWER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_LOWER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalLowerTracker;

#[cfg(test)]
impl IncrementalLowerTracker {
    fn new() -> Self {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_LOWER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_LOWER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_LOWER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalLowerTracker {
    fn drop(&mut self) {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_LOWER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalModuleSegmentLowerTracker;

#[cfg(test)]
impl IncrementalModuleSegmentLowerTracker {
    fn new() -> Self {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalModuleSegmentLowerTracker {
    fn drop(&mut self) {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_LINK_FULL_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_LINK_FULL_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INCREMENTAL_LINK_CACHE_HIT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalLinkTracker;

#[cfg(test)]
impl IncrementalLinkTracker {
    fn new() -> Self {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(0));
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(0));
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(0));
    }

    fn full_count(&self) -> usize {
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.get())
    }

    fn cache_hit_count(&self) -> usize {
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalLinkTracker {
    fn drop(&mut self) {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(0));
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(0));
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum MultiFileLoweringMode {
    Merged,
    Modular,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SegmentRange {
    start: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleLinkRanges {
    defns_functions: SegmentRange,
    accessors_functions: SegmentRange,
    trait_impls_functions: SegmentRange,
    constraints_functions: SegmentRange,
    ctors_functions: SegmentRange,
    defn_lifted_functions: SegmentRange,
    trait_impl_lifted_functions: SegmentRange,
    defns_gc_types: SegmentRange,
    defns_string_data: SegmentRange,
    trait_impls_string_data: SegmentRange,
}

fn module_has_content(module: &Module) -> bool {
    !module.functions.is_empty()
        || !module.gc_types.is_empty()
        || !module.imports.is_empty()
        || !module.globals.is_empty()
        || !module.string_data.is_empty()
}

fn build_segment_module(
    functions: Vec<Function>,
    gc_types: Vec<GcTypeDef>,
    string_data: Vec<(String, Vec<u8>)>,
) -> Module {
    Module {
        functions,
        gc_types,
        imports: Vec::new(),
        globals: Vec::new(),
        string_data,
    }
}

fn link_modules_preserving_indices(modules: &[Module]) -> Module {
    let mut linked = Module {
        functions: Vec::new(),
        gc_types: Vec::new(),
        imports: Vec::new(),
        globals: Vec::new(),
        string_data: Vec::new(),
    };

    for module in modules {
        linked.functions.extend(module.functions.clone());
        linked.gc_types.extend(module.gc_types.clone());
        linked.imports.extend(module.imports.clone());
        linked.globals.extend(module.globals.clone());
        linked.string_data.extend(module.string_data.clone());
    }

    linked
}

fn lower_multi_file_merged(
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
) -> Result<Module, lower::LowerError> {
    let merged_program = lsharp_syntax::ast::Program {
        decls: all_decls.to_vec(),
    };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx.lower_program_with_expr_types(
        &merged_program,
        all_type_results,
        all_expr_type_results,
    )
}

fn prime_cached_string_data(lower_ctx: &mut lower::Lower, string_data: &[(String, Vec<u8>)]) {
    for (label, bytes) in string_data {
        lower_ctx.string_data.push((label.clone(), bytes.clone()));
        lower_ctx.string_offset += bytes.len() as u32;
    }
}

fn prime_cached_lifted(lower_ctx: &mut lower::Lower, module: &Module) {
    lower_ctx.lambda_counter += module.functions.len() as u32;
    lower_ctx.lifted_functions.extend(module.functions.clone());
}

fn link_module_ir_segments(segments: &[ModuleIrSegments]) -> Module {
    let mut modules = Vec::new();

    for segment in segments {
        if module_has_content(segment.defns()) {
            modules.push(segment.defns().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.accessors()) {
            modules.push(segment.accessors().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.trait_impls()) {
            modules.push(segment.trait_impls().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.constraints()) {
            modules.push(segment.constraints().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.ctors()) {
            modules.push(segment.ctors().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.defn_lifted()) {
            modules.push(segment.defn_lifted().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.trait_impl_lifted()) {
            modules.push(segment.trait_impl_lifted().clone());
        }
    }

    link_modules_preserving_indices(&modules)
}

fn next_segment_range(cursor: &mut usize, len: usize) -> SegmentRange {
    let range = SegmentRange {
        start: *cursor,
        len,
    };
    *cursor += len;
    range
}

fn compute_module_link_ranges(segments: &[ModuleIrSegments]) -> Vec<ModuleLinkRanges> {
    let mut ranges = vec![ModuleLinkRanges::default(); segments.len()];

    let mut function_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_functions =
            next_segment_range(&mut function_cursor, segment.defns().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].accessors_functions =
            next_segment_range(&mut function_cursor, segment.accessors().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impls_functions =
            next_segment_range(&mut function_cursor, segment.trait_impls().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].constraints_functions =
            next_segment_range(&mut function_cursor, segment.constraints().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].ctors_functions =
            next_segment_range(&mut function_cursor, segment.ctors().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defn_lifted_functions =
            next_segment_range(&mut function_cursor, segment.defn_lifted().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impl_lifted_functions = next_segment_range(
            &mut function_cursor,
            segment.trait_impl_lifted().functions.len(),
        );
    }

    let mut gc_type_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_gc_types =
            next_segment_range(&mut gc_type_cursor, segment.defns().gc_types.len());
    }

    let mut string_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_string_data =
            next_segment_range(&mut string_cursor, segment.defns().string_data.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impls_string_data =
            next_segment_range(&mut string_cursor, segment.trait_impls().string_data.len());
    }

    ranges
}

fn segment_layout_matches(old: &ModuleIrSegments, new: &ModuleIrSegments) -> bool {
    old.defns().functions.len() == new.defns().functions.len()
        && old.accessors().functions.len() == new.accessors().functions.len()
        && old.trait_impls().functions.len() == new.trait_impls().functions.len()
        && old.constraints().functions.len() == new.constraints().functions.len()
        && old.ctors().functions.len() == new.ctors().functions.len()
        && old.defn_lifted().functions.len() == new.defn_lifted().functions.len()
        && old.trait_impl_lifted().functions.len() == new.trait_impl_lifted().functions.len()
        && old.defns().gc_types.len() == new.defns().gc_types.len()
        && old.defns().string_data.len() == new.defns().string_data.len()
        && old.trait_impls().string_data.len() == new.trait_impls().string_data.len()
}

fn can_patch_linked_module(
    cache: &CompilationCache,
    module_order: &[String],
    old_segments: &[ModuleIrSegments],
    new_segments: &[ModuleIrSegments],
) -> bool {
    cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
        && old_segments.len() == new_segments.len()
        && old_segments
            .iter()
            .zip(new_segments.iter())
            .all(|(old, new)| segment_layout_matches(old, new))
}

fn overwrite_range<T: Clone>(target: &mut [T], range: SegmentRange, replacement: &[T]) {
    debug_assert_eq!(range.len, replacement.len());
    target[range.start..range.start + range.len].clone_from_slice(replacement);
}

fn patch_linked_module(
    base: &Module,
    old_segments: &[ModuleIrSegments],
    new_segments: &[ModuleIrSegments],
) -> Module {
    let ranges = compute_module_link_ranges(old_segments);
    let mut patched = base.clone();

    for (range, segment) in ranges.iter().zip(new_segments.iter()) {
        overwrite_range(
            &mut patched.functions,
            range.defns_functions,
            &segment.defns().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.accessors_functions,
            &segment.accessors().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.trait_impls_functions,
            &segment.trait_impls().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.constraints_functions,
            &segment.constraints().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.ctors_functions,
            &segment.ctors().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.defn_lifted_functions,
            &segment.defn_lifted().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.trait_impl_lifted_functions,
            &segment.trait_impl_lifted().functions,
        );
        overwrite_range(
            &mut patched.gc_types,
            range.defns_gc_types,
            &segment.defns().gc_types,
        );
        overwrite_range(
            &mut patched.string_data,
            range.defns_string_data,
            &segment.defns().string_data,
        );
        overwrite_range(
            &mut patched.string_data,
            range.trait_impls_string_data,
            &segment.trait_impls().string_data,
        );
    }

    patched
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModulePrecomputedShape {
    defn_count: usize,
    accessor_count: usize,
    trait_impl_count: usize,
    constraint_count: usize,
    ctor_count: usize,
    gc_type_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleDefnStateShape {
    string_bytes: usize,
    lifted_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleTraitImplStateShape {
    string_bytes: usize,
    lifted_count: usize,
}

fn module_precomputed_shape_from_program(
    program: &lsharp_syntax::ast::Program,
) -> ModulePrecomputedShape {
    use lsharp_syntax::ast::Decl;

    let mut shape = ModulePrecomputedShape::default();
    for decl in &program.decls {
        match decl {
            Decl::Defn { .. } => shape.defn_count += 1,
            Decl::RecordDef { fields, .. } => {
                shape.accessor_count += fields.len();
                shape.gc_type_count += 1;
            }
            Decl::ImplDef { methods, .. } => {
                shape.trait_impl_count += methods.len();
            }
            Decl::TypeConstrained { .. } => {
                shape.constraint_count += 2;
            }
            Decl::TypeDef { variants, .. } => {
                shape.ctor_count += variants.len();
            }
            Decl::ModuleDecl { .. } | Decl::ImportDecl { .. } => {}
            _ => {}
        }
    }
    shape
}

fn module_precomputed_shape_from_segments(segments: &ModuleIrSegments) -> ModulePrecomputedShape {
    ModulePrecomputedShape {
        defn_count: segments.defns().functions.len(),
        accessor_count: segments.accessors().functions.len(),
        trait_impl_count: segments.trait_impls().functions.len(),
        constraint_count: segments.constraints().functions.len(),
        ctor_count: segments.ctors().functions.len(),
        gc_type_count: segments.defns().gc_types.len(),
    }
}

fn module_defn_state_shape(module: &ModuleIrSegments) -> ModuleDefnStateShape {
    ModuleDefnStateShape {
        string_bytes: module
            .defns()
            .string_data
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        lifted_count: module.defn_lifted().functions.len(),
    }
}

fn module_trait_impl_state_shape(module: &ModuleIrSegments) -> ModuleTraitImplStateShape {
    ModuleTraitImplStateShape {
        string_bytes: module
            .trait_impls()
            .string_data
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        lifted_count: module.trait_impl_lifted().functions.len(),
    }
}

fn defn_state_depends_on_prefix(shape: ModuleDefnStateShape) -> bool {
    shape.string_bytes > 0 || shape.lifted_count > 0
}

struct ModularLoweringResult {
    segments: Vec<ModuleIrSegments>,
    fresh_defn_lower_count: usize,
}

fn lower_multi_file_modular_with_segments(
    module_programs: &[lsharp_syntax::ast::Program],
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
    reusable_segments: &[Option<ModuleIrSegments>],
    segment_reuse_candidates: &[bool],
) -> Result<ModularLoweringResult, lower::LowerError> {
    let merged_program = lsharp_syntax::ast::Program {
        decls: all_decls.to_vec(),
    };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx.prepare_program_state(&merged_program, all_type_results);
    lower_ctx.expr_type_results = all_expr_type_results.clone();

    let mut segments = vec![ModuleIrSegments::empty(); module_programs.len()];
    let cached_precomputed_shapes: Vec<Option<ModulePrecomputedShape>> = reusable_segments
        .iter()
        .map(|segments| {
            segments
                .as_ref()
                .map(module_precomputed_shape_from_segments)
        })
        .collect();
    let cached_defn_shapes: Vec<Option<ModuleDefnStateShape>> = reusable_segments
        .iter()
        .map(|segments| segments.as_ref().map(module_defn_state_shape))
        .collect();
    let cached_trait_shapes: Vec<Option<ModuleTraitImplStateShape>> = reusable_segments
        .iter()
        .map(|segments| segments.as_ref().map(module_trait_impl_state_shape))
        .collect();
    let precomputed_shape_matches: Vec<bool> = module_programs
        .iter()
        .zip(cached_precomputed_shapes.iter())
        .map(|(program, cached)| {
            cached
                .map(|cached_shape| module_precomputed_shape_from_program(program) == cached_shape)
                .unwrap_or(false)
        })
        .collect();
    let mut defn_shape_matches = vec![false; module_programs.len()];
    let mut fresh_defn_lower_count = 0usize;
    let mut precomputed_prefix_stable = true;
    let mut defn_prefix_stable = true;

    for (idx, program) in module_programs.iter().enumerate() {
        let current_defn_needs_prefix_state =
            cached_defn_shapes[idx].is_some_and(defn_state_depends_on_prefix);
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && (!current_defn_needs_prefix_state || defn_prefix_stable)
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            prime_cached_string_data(&mut lower_ctx, &cached.defns().string_data);
            prime_cached_lifted(&mut lower_ctx, cached.defn_lifted());
            segments[idx].set_defns(cached.defns().clone());
            segments[idx].set_defn_lifted(cached.defn_lifted().clone());
            defn_shape_matches[idx] = true;
        } else {
            fresh_defn_lower_count += 1;
            let gc_types = lower_ctx.gc_types_for_program(program);
            let string_start = lower_ctx.string_data.len();
            let lifted_start = lower_ctx.lifted_functions.len();
            let functions = lower_ctx.lower_defn_functions(program)?;
            let string_data = lower_ctx.clone_string_data_from(string_start);
            let lifted = lower_ctx.lifted_functions[lifted_start..].to_vec();
            segments[idx].set_defns(build_segment_module(functions, gc_types, string_data));
            segments[idx].set_defn_lifted(build_segment_module(lifted, Vec::new(), Vec::new()));
            defn_shape_matches[idx] = cached_defn_shapes[idx].is_some_and(|cached_shape| {
                module_defn_state_shape(&segments[idx]) == cached_shape
            });
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        defn_prefix_stable &= defn_shape_matches[idx];
    }

    let defn_global_stable = defn_shape_matches.iter().all(|stable| *stable);

    precomputed_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            segments[idx].set_accessors(cached.accessors().clone());
        } else {
            segments[idx].set_accessors(build_segment_module(
                lower_ctx.lower_field_accessors(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
    }

    precomputed_prefix_stable = true;
    let mut trait_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && defn_global_stable
            && precomputed_prefix_stable
            && trait_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            prime_cached_string_data(&mut lower_ctx, &cached.trait_impls().string_data);
            prime_cached_lifted(&mut lower_ctx, cached.trait_impl_lifted());
            segments[idx].set_trait_impls(cached.trait_impls().clone());
            segments[idx].set_trait_impl_lifted(cached.trait_impl_lifted().clone());
        } else {
            let string_start = lower_ctx.string_data.len();
            let lifted_start = lower_ctx.lifted_functions.len();
            let functions = lower_ctx.lower_trait_impl_functions(program)?;
            let string_data = lower_ctx.clone_string_data_from(string_start);
            let lifted = lower_ctx.lifted_functions[lifted_start..].to_vec();
            segments[idx].set_trait_impls(build_segment_module(functions, Vec::new(), string_data));
            segments[idx].set_trait_impl_lifted(build_segment_module(
                lifted,
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        trait_prefix_stable &= cached_trait_shapes[idx].is_some_and(|cached_shape| {
            module_trait_impl_state_shape(&segments[idx]) == cached_shape
        });
    }

    precomputed_prefix_stable = true;
    let mut constraint_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && constraint_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            lower_ctx.late_func_idx += cached.constraints().functions.len() as u32;
            segments[idx].set_constraints(cached.constraints().clone());
        } else {
            segments[idx].set_constraints(build_segment_module(
                lower_ctx.lower_constraint_functions(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        constraint_prefix_stable &= cached_precomputed_shapes[idx].is_some_and(|cached_shape| {
            module_precomputed_shape_from_segments(&segments[idx]).constraint_count
                == cached_shape.constraint_count
        });
    }

    precomputed_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            segments[idx].set_ctors(cached.ctors().clone());
        } else {
            segments[idx].set_ctors(build_segment_module(
                lower_ctx.lower_adt_constructors(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
    }

    Ok(ModularLoweringResult {
        segments,
        fresh_defn_lower_count,
    })
}

fn lower_multi_file_modular(
    module_programs: &[lsharp_syntax::ast::Program],
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
) -> Result<Module, lower::LowerError> {
    let reusable_segments = vec![None; module_programs.len()];
    let lowering = lower_multi_file_modular_with_segments(
        module_programs,
        all_decls,
        all_type_results,
        all_expr_type_results,
        &reusable_segments,
        &vec![false; module_programs.len()],
    )?;
    Ok(link_module_ir_segments(&lowering.segments))
}

fn collect_private_surface_names(
    decls: &[lsharp_syntax::ast::Decl],
    module_prefix: Option<&str>,
    out: &mut HashSet<String>,
) {
    use lsharp_syntax::ast::Decl;

    for decl in decls {
        match decl {
            Decl::Private { inner, .. } => match inner.as_ref() {
                Decl::Defn { name, .. }
                | Decl::TypeDef { name, .. }
                | Decl::RecordDef { name, .. }
                | Decl::TypeAlias { name, .. }
                | Decl::TypeConstrained { name, .. } => {
                    let qualified = module_prefix
                        .map(|prefix| format!("{prefix}.{name}"))
                        .unwrap_or_else(|| name.clone());
                    out.insert(qualified);
                }
                Decl::ModuleDecl { name, body, .. } => {
                    let qualified = module_prefix
                        .map(|prefix| format!("{prefix}.{name}"))
                        .unwrap_or_else(|| name.clone());
                    collect_private_surface_names(body, Some(&qualified), out);
                }
                _ => {}
            },
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let qualified = module_prefix
                    .map(|prefix| format!("{prefix}.{name}"))
                    .unwrap_or_else(|| name.clone());
                collect_private_surface_names(body, Some(&qualified), out);
            }
            _ => {}
        }
    }
}

fn register_expr_scope_owner(
    owners: &mut HashMap<String, Option<String>>,
    scope: String,
    module_name: &str,
) {
    if let Some(existing) = owners.get_mut(&scope) {
        if existing.as_deref() != Some(module_name) {
            *existing = None;
        }
    } else {
        owners.insert(scope, Some(module_name.to_string()));
    }
}

fn collect_expr_scope_owners(
    decls: &[lsharp_syntax::ast::Decl],
    module_prefix: Option<&str>,
    module_name: &str,
    owners: &mut HashMap<String, Option<String>>,
) {
    use lsharp_syntax::ast::Decl;

    for decl in decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        match actual_decl {
            Decl::Defn { name, .. } => {
                let scope = module_prefix
                    .map(|prefix| format!("{prefix}.{name}"))
                    .unwrap_or_else(|| name.clone());
                register_expr_scope_owner(owners, scope, module_name);
            }
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let prefix = module_prefix
                    .map(|outer| format!("{outer}.{name}"))
                    .unwrap_or_else(|| name.clone());
                collect_expr_scope_owners(body, Some(&prefix), module_name, owners);
            }
            Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } => {
                for method in methods {
                    let method = match method {
                        Decl::Private { inner, .. } => inner.as_ref(),
                        other => other,
                    };
                    if let Decl::Defn { name, .. } = method {
                        register_expr_scope_owner(
                            owners,
                            format!("{}::{}{}{}", trait_name, name, '$', type_name),
                            module_name,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn try_build_unrestricted_merged_scc_surfaces(
    group: &[String],
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
    direct_imports: &HashMap<String, HashMap<String, ImportVisibilitySpec>>,
    inferred_private_names: &[String],
    merged_expr_types: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
    results_by_module: &mut HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>>,
) -> Option<HashMap<String, ModuleTypeSurface>> {
    if !inferred_private_names.is_empty()
        || group.iter().any(|module_name| {
            direct_imports
                .get(module_name)
                .into_iter()
                .flat_map(HashMap::values)
                .any(|import| import.only.is_some())
        })
    {
        return None;
    }

    let mut owners = HashMap::new();
    for module_name in group {
        let program = parsed_modules.get(module_name)?;
        collect_expr_scope_owners(&program.decls, None, module_name, &mut owners);
        results_by_module.get(module_name)?;
    }

    let mut expr_types_by_module: HashMap<String, HashMap<ExprTypeKey, lsharp_types::types::Type>> =
        HashMap::new();
    for (key, ty) in merged_expr_types {
        let Some(Some(module_name)) = owners.get(&key.scope) else {
            return None;
        };
        expr_types_by_module
            .entry(module_name.clone())
            .or_default()
            .insert(key.clone(), ty.clone());
    }

    let mut surfaces = HashMap::new();
    for module_name in group {
        surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results: results_by_module.remove(module_name)?,
                hidden: HashSet::new(),
                expr_types: expr_types_by_module.remove(module_name).unwrap_or_default(),
            },
        );
    }
    Some(surfaces)
}

/// SCC の merged inference 用に宣言を連結する。
///
/// 同じ import 宣言が SCC 内の複数 module に現れても、型環境への注入は一度で足りる。
/// ただし `:only`、alias、`open` が異なる import は意味が異なるため、完全一致する宣言
/// だけを重複除去する。宣言の順序と defn の所属 module は維持する。
type SccImportKey = (String, Option<String>, Option<Vec<String>>, bool);

fn merge_scc_declarations(
    group: &[String],
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
) -> Result<(Vec<lsharp_syntax::ast::Decl>, Vec<String>), String> {
    use lsharp_syntax::ast::Decl;

    let mut merged_decls = Vec::new();
    let mut defn_origins = Vec::new();
    let mut seen_imports: HashSet<SccImportKey> = HashSet::new();

    for module_name in group {
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        for decl in &program.decls {
            match decl {
                Decl::ModuleDecl { body, .. } if body.is_empty() => {}
                Decl::ImportDecl {
                    module,
                    alias,
                    only,
                    open,
                    ..
                } => {
                    let key = (module.clone(), alias.clone(), only.clone(), *open);
                    if seen_imports.insert(key) {
                        merged_decls.push(decl.clone());
                    }
                }
                _ => {
                    push_defn_origins_infer_order(
                        std::slice::from_ref(decl),
                        module_name,
                        None,
                        &mut defn_origins,
                    );
                    merged_decls.push(decl.clone());
                }
            }
        }
    }

    Ok((merged_decls, defn_origins))
}

fn infer_scc_type_surfaces(
    group: &[String],
    graph: &module_graph::ModuleGraph,
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
    module_paths: &HashMap<String, std::path::PathBuf>,
    direct_imports: &HashMap<String, HashMap<String, ImportVisibilitySpec>>,
    known_surfaces: &HashMap<String, ModuleTypeSurface>,
) -> Result<HashMap<String, ModuleTypeSurface>, String> {
    use lsharp_syntax::ast::Program;

    let group_set: HashSet<&str> = group.iter().map(String::as_str).collect();
    if group.len() == 1 {
        let module_name = &group[0];
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        let mut infer = lsharp_types::infer::Infer::new();
        for dependency in graph.dependency_closure(module_name) {
            if group_set.contains(dependency.as_str()) {
                continue;
            }
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = known_surfaces.get(&dependency)
            {
                infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        note_incremental_type_infer();
        let results = infer.infer_program(program).map_err(|error| {
            let path = module_paths
                .get(module_name)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| module_name.clone());
            format!("{path}: {error}")
        })?;
        return Ok(HashMap::from([(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            },
        )]));
    }

    let (merged_decls, defn_origins) = merge_scc_declarations(group, parsed_modules)?;

    let mut infer = lsharp_types::infer::Infer::new();
    for module_name in group {
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        for dependency in graph.dependency_closure(module_name) {
            if group_set.contains(dependency.as_str()) {
                continue;
            }
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = known_surfaces.get(&dependency)
            {
                infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
    }

    let merged = Program {
        decls: merged_decls,
    };
    let type_results = infer.infer_program(&merged).map_err(|error| {
        let path = group
            .first()
            .and_then(|module| module_paths.get(module))
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| group.join(", "));
        format!("{path}: {error}")
    })?;
    if type_results.len() != defn_origins.len() {
        return Err(format!(
            "SCC の型結果数が宣言数と一致しません: modules={}, results={}, origins={}",
            group.join(", "),
            type_results.len(),
            defn_origins.len()
        ));
    }

    let mut results_by_module: HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>> =
        HashMap::new();
    for ((name, scheme), origin) in type_results.into_iter().zip(defn_origins) {
        results_by_module
            .entry(origin)
            .or_default()
            .push((name, scheme));
    }

    let inferred_private_names = infer.module_env.privates.clone();
    let merged_expr_types = infer.expr_type_results_snapshot();
    if let Some(surfaces) = try_build_unrestricted_merged_scc_surfaces(
        group,
        parsed_modules,
        direct_imports,
        &inferred_private_names,
        &merged_expr_types,
        &mut results_by_module,
    ) {
        note_incremental_scc_merged_fast_path();
        return Ok(surfaces);
    }

    let mut provisional_surfaces = HashMap::new();
    for module_name in group {
        let results = results_by_module.remove(module_name).unwrap_or_default();
        let mut private_names = HashSet::new();
        if let Some(program) = parsed_modules.get(module_name) {
            collect_private_surface_names(&program.decls, None, &mut private_names);
        }
        let hidden = inferred_private_names
            .iter()
            .filter(|name| private_names.contains(*name))
            .cloned()
            .collect();
        provisional_surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden,
                expr_types: HashMap::new(),
            },
        );
    }

    // merged prepass で相互再帰の型を確定した後、各 module を元の import visibility
    // で再検証する。これにより SCC 内でも `:only` / private の境界を失わない。
    let mut surfaces = HashMap::new();
    for module_name in group {
        let mut module_infer = lsharp_types::infer::Infer::new();
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        for dependency in graph.dependency_closure(module_name) {
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = provisional_surfaces
                    .get(&dependency)
                    .or_else(|| known_surfaces.get(&dependency))
            {
                module_infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        let results = module_infer.infer_program(program).map_err(|error| {
            let path = module_paths
                .get(module_name)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| module_name.clone());
            format!("{path}: {error}")
        })?;
        surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden: module_infer.module_env.privates.iter().cloned().collect(),
                expr_types: module_infer.expr_type_results_snapshot(),
            },
        );
    }

    Ok(surfaces)
}

fn compile_multi_file_with_mode(
    entry_file: &std::path::Path,
    lowering_mode: MultiFileLoweringMode,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    // 1. モジュールグラフの構築とファイル探索
    let (graph, sorted_files) = ModuleGraph::build_from_entry_with_scc(entry_file)
        .map_err(|e| format!("モジュールグラフ構築エラー: {e}"))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    // 単一ファイルの場合は通常のパイプライン
    if sorted_files.len() == 1 {
        let (_, mod_path) = &sorted_files[0];
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let program =
            lsharp_syntax::parse(&source).map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let mut infer = lsharp_types::infer::Infer::new();
        let type_results = infer
            .infer_program(&program)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let expr_type_results = infer.expr_type_results_snapshot();
        let mut lower_ctx = lower::Lower::new();
        return lower_ctx
            .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    // 2. 全モジュールを依存順にパースし、SCC ごとに型チェックする。
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut all_expr_type_results: HashMap<ExprTypeKey, lsharp_types::types::Type> = HashMap::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut module_programs: Vec<lsharp_syntax::ast::Program> = Vec::new();
    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();

    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;

        let program =
            lsharp_syntax::parse(&source).map_err(|e| format!("{}: {e}", mod_path.display()))?;
        direct_imports.insert(mod_name.clone(), collect_import_visibility(&program));
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program);
    }

    for group in graph.scc_groups() {
        let surfaces = infer_scc_type_surfaces(
            &group,
            &graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    for (mod_name, _) in &sorted_files {
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        all_type_results.extend(surface.results.clone());
        all_expr_type_results.extend(surface.expr_types.clone());

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        // 宣言を収集（module 宣言と import 宣言は除外）
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });
    }

    let lowered = match lowering_mode {
        MultiFileLoweringMode::Merged => {
            lower_multi_file_merged(&all_decls, &all_type_results, &all_expr_type_results)
        }
        MultiFileLoweringMode::Modular => lower_multi_file_modular(
            &module_programs,
            &all_decls,
            &all_type_results,
            &all_expr_type_results,
        ),
    };

    lowered.map_err(|e| format!("IR 変換エラー: {e}"))
}

pub fn compile_multi_file(entry_file: &std::path::Path) -> Result<Module, String> {
    compile_multi_file_with_mode(entry_file, MultiFileLoweringMode::Modular)
}

/// CLI compile で再利用できる解析/IR cache 付きの multi-file compile 入口。
///
/// 既存の `compile_multi_file_incremental` は互換 API として残し、公開 surface には
/// cache の意図が名前に現れるこちらを推奨する。
pub fn compile_multi_file_with_cache(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    cache.prepare_for_entry(entry_file);
    compile_multi_file_incremental(entry_file, cache)
}

/// 循環を含む multi-file compile の incremental cache 更新を行う。
///
/// SCC は module 単位のトポロジカル推論では扱えないため、SCC ごとに一括推論してから
/// modular lowering を行う。型推論は SCC 単位で行うが、lowering は clean module の
/// segment を再利用し、dirty module の segment だけを生成し直す。
fn compile_multi_file_incremental_scc(
    graph: &module_graph::ModuleGraph,
    sorted_files: &[(String, std::path::PathBuf)],
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    let module_order = sorted_files
        .iter()
        .map(|(mod_name, _)| mod_name.clone())
        .collect::<Vec<_>>();
    let mut current_fingerprints = HashMap::new();
    let mut all_clean = true;
    for (mod_name, mod_path) in sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        all_clean &= cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        current_fingerprints.insert(mod_name.clone(), fingerprint);
    }
    if all_clean
        && let Some(linked) = cache
            .linked_module()
            .filter(|linked| linked.module_order() == module_order)
    {
        return Ok(linked.final_module().clone());
    }

    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();
    let mut fingerprints = HashMap::new();

    for (mod_name, mod_path) in sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = current_fingerprints
            .get(mod_name)
            .copied()
            .unwrap_or_else(|| SourceFingerprint::from_source(&source));
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        direct_imports.insert(
            mod_name.clone(),
            collect_import_visibility(program.as_ref()),
        );
        fingerprints.insert(mod_name.clone(), fingerprint);
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program.as_ref().clone());
    }

    let mut per_module_type_results = HashMap::new();
    for group in graph.scc_groups() {
        let group_cache_hit = group.iter().all(|module_name| {
            let Some(fingerprint) = current_fingerprints.get(module_name) else {
                return false;
            };
            let Some(imports) = direct_imports.get(module_name) else {
                return false;
            };
            let deps_key = dependency_surface_key(imports, &per_module_type_results, cache);
            cache.get(module_name).is_some_and(|entry| {
                entry.fingerprint() == *fingerprint && entry.deps_key() == deps_key
            })
        });
        if group_cache_hit {
            for module_name in &group {
                let surface = cache
                    .get(module_name)
                    .map(|entry| entry.type_surface_clone())
                    .ok_or_else(|| format!("型 surface cache がありません: {module_name}"))?;
                per_module_type_results.insert(module_name.clone(), surface);
            }
            continue;
        }

        note_incremental_scc_infer();
        let surfaces = infer_scc_type_surfaces(
            &group,
            graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    let mut all_decls = Vec::new();
    let mut all_type_results = Vec::new();
    let mut all_expr_type_results = HashMap::new();
    let mut module_programs = Vec::new();
    let mut cache_entries = Vec::new();
    let mut surface_changed_modules = HashSet::new();
    let mut segment_reuse_candidates = Vec::new();
    for (mod_name, _) in sorted_files {
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        all_type_results.extend(surface.results.clone());
        all_expr_type_results.extend(surface.expr_types.clone());

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });

        let direct_imports = direct_imports
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの import 情報がありません: {mod_name}"))?;
        let fingerprint = fingerprints
            .get(mod_name)
            .copied()
            .ok_or_else(|| format!("モジュールの fingerprint がありません: {mod_name}"))?;
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let deps_key = dependency_surface_key(direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);
        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let segment_reuse_candidate = clean_hit && deps_hit && !direct_dep_surface_changed;
        let surface_changed = cache
            .get(mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let program = std::sync::Arc::new(program.clone());
        let entry = build_module_cache_entry(fingerprint, deps_key, &program, surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        segment_reuse_candidates.push(segment_reuse_candidate);
    }

    let mut reusable_segments = vec![None; module_programs.len()];
    for (idx, (mod_name, _)) in cache_entries.iter().enumerate() {
        if let Some(cached_entry) = cache.get(mod_name)
            && !cached_entry.ir_segments().is_empty()
        {
            reusable_segments[idx] = Some(cached_entry.ir_segments().clone());
        }
    }

    note_incremental_lower();
    let lowering = lower_multi_file_modular_with_segments(
        &module_programs,
        &all_decls,
        &all_type_results,
        &all_expr_type_results,
        &reusable_segments,
        &segment_reuse_candidates,
    )
    .map_err(|e| format!("IR 変換エラー: {e}"))?;
    note_incremental_module_segment_lower_by(lowering.fresh_defn_lower_count);
    let new_segments = lowering.segments;
    let old_segments: Option<Vec<ModuleIrSegments>> = if cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
    {
        cache_entries
            .iter()
            .map(|(mod_name, _)| cache.get(mod_name).map(|entry| entry.ir_segments().clone()))
            .collect()
    } else {
        None
    };
    let final_module =
        if let (Some(old_segments), Some(linked)) = (old_segments, cache.linked_module()) {
            if can_patch_linked_module(cache, &module_order, &old_segments, &new_segments) {
                note_incremental_link_cache_hit();
                patch_linked_module(linked.final_module(), &old_segments, &new_segments)
            } else {
                note_incremental_link_full();
                link_module_ir_segments(&new_segments)
            }
        } else {
            note_incremental_link_full();
            link_module_ir_segments(&new_segments)
        };

    for ((mod_name, mut entry), segments) in cache_entries.into_iter().zip(new_segments) {
        entry.set_ir(final_module.clone());
        entry.set_ir_segments(segments);
        cache.insert_module(mod_name.clone(), entry);
    }
    cache.set_linked_module(module_order, final_module.clone());

    Ok(final_module)
}

/// source override を含む循環 module の incremental analysis を行う。
///
/// LSP の未保存 source でも compile と同じ SCC 推論境界を使い、解析結果だけを cache に保存する。
/// lowering はこの入口の責務ではないため、IR は空のまま保持する。
fn analyze_multi_file_incremental_scc_with_overrides(
    graph: &module_graph::ModuleGraph,
    sorted_files: &[(String, std::path::PathBuf)],
    source_overrides: &HashMap<std::path::PathBuf, String>,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();
    let mut fingerprints = HashMap::new();

    for (mod_name, mod_path) in sorted_files {
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        direct_imports.insert(
            mod_name.clone(),
            collect_import_visibility(program.as_ref()),
        );
        fingerprints.insert(mod_name.clone(), fingerprint);
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program.as_ref().clone());
    }

    let mut per_module_type_results = HashMap::new();
    for group in graph.scc_groups() {
        let group_cache_hit = group.iter().all(|module_name| {
            let Some(fingerprint) = fingerprints.get(module_name) else {
                return false;
            };
            let Some(imports) = direct_imports.get(module_name) else {
                return false;
            };
            let deps_key = dependency_surface_key(imports, &per_module_type_results, cache);
            cache.get(module_name).is_some_and(|entry| {
                entry.fingerprint() == *fingerprint && entry.deps_key() == deps_key
            })
        });
        if group_cache_hit {
            for module_name in &group {
                let surface = cache
                    .get(module_name)
                    .map(|entry| entry.type_surface_clone())
                    .ok_or_else(|| format!("型 surface cache がありません: {module_name}"))?;
                per_module_type_results.insert(module_name.clone(), surface);
            }
            continue;
        }

        note_incremental_scc_infer();
        let surfaces = infer_scc_type_surfaces(
            &group,
            graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    for (mod_name, _) in sorted_files {
        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        let direct_imports = direct_imports
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの import 情報がありません: {mod_name}"))?;
        let fingerprint = fingerprints
            .get(mod_name)
            .copied()
            .ok_or_else(|| format!("モジュールの fingerprint がありません: {mod_name}"))?;
        let program = std::sync::Arc::new(program.clone());
        let deps_key = dependency_surface_key(direct_imports, &per_module_type_results, cache);
        let entry = build_module_cache_entry(fingerprint, deps_key, &program, surface.clone());
        cache.insert_module(mod_name.clone(), entry);
    }

    Ok(())
}

fn read_source_with_overrides(
    path: &std::path::Path,
    source_overrides: &HashMap<std::path::PathBuf, String>,
) -> Result<String, String> {
    if let Some(source) = source_overrides.get(path) {
        return Ok(source.clone());
    }

    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn analyze_single_file_incremental(
    module_name: &str,
    source: &str,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    let fingerprint = SourceFingerprint::from_source(source);
    let clean_hit = cache
        .get(module_name)
        .is_some_and(|entry| entry.fingerprint() == fingerprint);
    if clean_hit {
        return Ok(());
    }

    let program = cached_program_or_parse(module_name, source, fingerprint, cache)
        .map_err(|e| format!("{e}"))?;
    let mut infer = lsharp_types::infer::Infer::new();
    note_incremental_type_infer();
    let type_results = infer
        .infer_program(program.as_ref())
        .map_err(|e| format!("{e}"))?;
    let type_surface = ModuleTypeSurface {
        results: type_results,
        hidden: infer.module_env.privates.iter().cloned().collect(),
        expr_types: infer.expr_type_results_snapshot(),
    };
    let entry = build_module_cache_entry(fingerprint, 0, &program, type_surface);
    cache.insert_module(module_name.to_string(), entry);
    Ok(())
}

pub fn analyze_multi_file_incremental_with_overrides(
    entry_file: &std::path::Path,
    source_overrides: &HashMap<std::path::PathBuf, String>,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    use module_graph::ModuleGraph;

    cache.prepare_for_entry(entry_file);
    let (graph, sorted_files) =
        ModuleGraph::build_from_entry_with_overrides_scc(entry_file, source_overrides)
            .map_err(|e| format!("モジュールグラフ構築エラー: {e}"))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    if sorted_files.len() == 1 {
        let (mod_name, mod_path) = &sorted_files[0];
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        return analyze_single_file_incremental(mod_name, &source, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    if graph.scc_groups().iter().any(|group| group.len() > 1) {
        return analyze_multi_file_incremental_scc_with_overrides(
            &graph,
            &sorted_files,
            source_overrides,
            cache,
        );
    }

    let mut module_inputs = Vec::new();
    let mut changed_modules = Vec::new();
    for (mod_name, mod_path) in &sorted_files {
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if !clean_hit {
            changed_modules.push(mod_name.clone());
        }
        module_inputs.push((mod_name.clone(), mod_path.clone(), source, fingerprint));
    }

    if changed_modules.is_empty() {
        return Ok(());
    }

    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut cache_entries: Vec<(String, ModuleCacheEntry)> = Vec::new();
    let mut surface_changed_modules: HashSet<String> = HashSet::new();

    for (mod_name, mod_path, source, fingerprint) in module_inputs {
        let clean_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let program = cached_program_or_parse(&mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let direct_imports = collect_import_visibility(program.as_ref());
        let deps_key = dependency_surface_key(&direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);

        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));

        let type_surface = if clean_hit && deps_hit && !direct_dep_surface_changed {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(&mod_name) {
                if let Some(import_spec) = direct_imports.get(&dep_name)
                    && let Some(dep_surface) = per_module_type_results.get(&dep_name)
                {
                    infer.inject_external_types_for_import(
                        &dep_name,
                        import_spec.only.as_deref(),
                        &dep_surface.hidden,
                        &dep_surface.results,
                    );
                }
            }
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: {e}", mod_path.display()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(&type_surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        let entry = build_module_cache_entry(fingerprint, deps_key, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name, type_surface);
    }

    for (mod_name, entry) in cache_entries {
        cache.insert_module(mod_name, entry);
    }

    Ok(())
}

pub fn compile_multi_file_incremental(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    let (graph, sorted_files) = ModuleGraph::build_from_entry_with_scc(entry_file)
        .map_err(|e| format!("モジュールグラフ構築エラー: {e}"))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    if sorted_files.len() == 1 {
        let (mod_name, mod_path) = &sorted_files[0];
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if clean_hit {
            return Ok(cache
                .get(mod_name)
                .expect("clean hit should have cache entry")
                .ir()
                .clone());
        }
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let type_surface = if clean_hit {
            cache
                .get(mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: {e}", mod_path.display()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let mut lower_ctx = lower::Lower::new();
        note_incremental_lower();
        let module = lower_ctx
            .lower_program_with_expr_types(
                program.as_ref(),
                &type_surface.results,
                &type_surface.expr_types,
            )
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let mut entry = build_module_cache_entry(fingerprint, 0, &program, type_surface);
        entry.set_ir(module.clone());
        cache.insert_module(mod_name.clone(), entry);
        return Ok(module);
    }

    if graph.scc_groups().iter().any(|group| group.len() > 1) {
        return compile_multi_file_incremental_scc(&graph, &sorted_files, cache);
    }

    let mut module_inputs = Vec::new();
    let mut changed_modules = Vec::new();
    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if !clean_hit {
            changed_modules.push(mod_name.clone());
        }
        module_inputs.push((mod_name.clone(), mod_path.clone(), source, fingerprint));
    }
    if changed_modules.is_empty() {
        let first_clean_entry = module_inputs
            .first()
            .and_then(|(mod_name, _, _, _)| cache.get(mod_name).map(|entry| entry.ir().clone()))
            .expect("all clean hits should have cache entries");
        return Ok(first_clean_entry);
    }
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut all_expr_type_results: HashMap<ExprTypeKey, lsharp_types::types::Type> = HashMap::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut cache_entries: Vec<(String, ModuleCacheEntry)> = Vec::new();
    let mut surface_changed_modules: HashSet<String> = HashSet::new();
    let mut module_programs: Vec<lsharp_syntax::ast::Program> = Vec::new();
    let mut segment_reuse_candidates: Vec<bool> = Vec::new();

    for (mod_name, mod_path, source, fingerprint) in module_inputs {
        let clean_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let program = cached_program_or_parse(&mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let direct_imports = collect_import_visibility(program.as_ref());
        let deps_key = dependency_surface_key(&direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);

        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let segment_reuse_candidate = clean_hit && deps_hit && !direct_dep_surface_changed;

        let type_surface = if clean_hit && deps_hit && !direct_dep_surface_changed {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(&mod_name) {
                if let Some(import_spec) = direct_imports.get(&dep_name)
                    && let Some(dep_surface) = per_module_type_results.get(&dep_name)
                {
                    infer.inject_external_types_for_import(
                        &dep_name,
                        import_spec.only.as_deref(),
                        &dep_surface.hidden,
                        &dep_surface.results,
                    );
                }
            }
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: {e}", mod_path.display()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(&type_surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        all_type_results.extend(type_surface.results.clone());
        all_expr_type_results.extend(type_surface.expr_types.clone());
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });

        let entry = build_module_cache_entry(fingerprint, deps_key, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name.clone(), type_surface);
        segment_reuse_candidates.push(segment_reuse_candidate);
    }

    let mut reusable_segments = vec![None; module_programs.len()];
    for (idx, (mod_name, _)) in cache_entries.iter().enumerate() {
        if let Some(cached_entry) = cache.get(mod_name)
            && !cached_entry.ir_segments().is_empty()
        {
            reusable_segments[idx] = Some(cached_entry.ir_segments().clone());
        }
    }

    note_incremental_lower();
    let lowering = lower_multi_file_modular_with_segments(
        &module_programs,
        &all_decls,
        &all_type_results,
        &all_expr_type_results,
        &reusable_segments,
        &segment_reuse_candidates,
    )
    .map_err(|e| format!("IR 変換エラー: {e}"))?;
    note_incremental_module_segment_lower_by(lowering.fresh_defn_lower_count);
    let new_segments = lowering.segments;
    let module_order: Vec<String> = cache_entries
        .iter()
        .map(|(mod_name, _)| mod_name.clone())
        .collect();
    let old_segments: Option<Vec<ModuleIrSegments>> = if cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
    {
        cache_entries
            .iter()
            .map(|(mod_name, _)| cache.get(mod_name).map(|entry| entry.ir_segments().clone()))
            .collect()
    } else {
        None
    };
    let final_module =
        if let (Some(old_segments), Some(linked)) = (old_segments, cache.linked_module()) {
            if can_patch_linked_module(cache, &module_order, &old_segments, &new_segments) {
                note_incremental_link_cache_hit();
                patch_linked_module(linked.final_module(), &old_segments, &new_segments)
            } else {
                note_incremental_link_full();
                link_module_ir_segments(&new_segments)
            }
        } else {
            note_incremental_link_full();
            link_module_ir_segments(&new_segments)
        };

    for ((mod_name, mut entry), segments) in cache_entries.into_iter().zip(new_segments) {
        entry.set_ir(final_module.clone());
        entry.set_ir_segments(segments);
        cache.insert_module(mod_name, entry);
    }
    cache.set_linked_module(module_order, final_module.clone());

    Ok(final_module)
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
                    GcField {
                        name: "x".to_string(),
                        ty: IrType::I64,
                        mutable: false,
                    },
                    GcField {
                        name: "y".to_string(),
                        ty: IrType::I64,
                        mutable: false,
                    },
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
                kind: GcTypeKind::Struct(vec![GcField {
                    name: "r".to_string(),
                    ty: IrType::I64,
                    mutable: false,
                }]),
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

    #[test]
    fn test_link_funcref_rebases_function_and_type_indices() {
        let module_a = Module {
            functions: vec![Function {
                name: "a".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::RefFunc(0),
                    Instruction::Drop,
                    Instruction::RefFunc(1),
                    Instruction::Drop,
                    Instruction::CallRef(1),
                    Instruction::Drop,
                    Instruction::CallRef(2),
                ],
                is_export: false,
            }],
            gc_types: vec![GcTypeDef {
                name: "A".to_string(),
                kind: GcTypeKind::Struct(vec![]),
            }],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "a-import".to_string(),
                params: vec![],
                result: IrType::I64,
            }],
            globals: vec![],
            string_data: vec![],
        };
        let module_b = Module {
            functions: vec![Function {
                name: "b".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::RefFunc(0),
                    Instruction::Drop,
                    Instruction::RefFunc(1),
                    Instruction::Drop,
                    Instruction::CallRef(1),
                    Instruction::Drop,
                    Instruction::CallRef(2),
                ],
                is_export: false,
            }],
            gc_types: vec![GcTypeDef {
                name: "B".to_string(),
                kind: GcTypeKind::Struct(vec![]),
            }],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "b-import".to_string(),
                params: vec![],
                result: IrType::I64,
            }],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[module_a, module_b]);
        assert_eq!(linked.imports.len(), 2);
        assert_eq!(linked.functions.len(), 2);

        assert_ref_func(&linked.functions[0].body[0], 0);
        assert_ref_func(&linked.functions[0].body[2], 2);
        assert_call_ref(&linked.functions[0].body[4], 2);
        assert_call_ref(&linked.functions[0].body[6], 4);
        assert_ref_func(&linked.functions[1].body[0], 1);
        assert_ref_func(&linked.functions[1].body[2], 3);
        assert_call_ref(&linked.functions[1].body[4], 3);
        assert_call_ref(&linked.functions[1].body[6], 5);

        fn assert_ref_func(instruction: &Instruction, expected: u32) {
            match instruction {
                Instruction::RefFunc(index) => assert_eq!(*index, expected),
                other => panic!("expected RefFunc, got {other:?}"),
            }
        }

        fn assert_call_ref(instruction: &Instruction, expected: u32) {
            match instruction {
                Instruction::CallRef(index) => assert_eq!(*index, expected),
                other => panic!("expected CallRef, got {other:?}"),
            }
        }
    }

    #[test]
    fn test_link_funcref_rebases_typed_local_and_gc_field_types() {
        let module_a = Module {
            functions: vec![Function {
                name: "a".to_string(),
                params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
                result: IrType::TypedFuncRef(2),
                locals: vec![IrType::Ref(0), IrType::TypedFuncRef(1)],
                body: vec![
                    Instruction::RefFunc(0),
                    Instruction::RefFunc(1),
                    Instruction::CallRef(2),
                ],
                is_export: false,
            }],
            gc_types: vec![GcTypeDef {
                name: "A".to_string(),
                kind: GcTypeKind::Struct(vec![
                    GcField {
                        name: "value".to_string(),
                        ty: IrType::Ref(0),
                        mutable: false,
                    },
                    GcField {
                        name: "call".to_string(),
                        ty: IrType::TypedFuncRef(2),
                        mutable: false,
                    },
                ]),
            }],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "a-import".to_string(),
                params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
                result: IrType::Ref(0),
            }],
            globals: vec![],
            string_data: vec![],
        };
        let module_b = Module {
            functions: vec![Function {
                name: "b".to_string(),
                params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
                result: IrType::TypedFuncRef(2),
                locals: vec![IrType::Ref(0), IrType::TypedFuncRef(1)],
                body: vec![
                    Instruction::RefFunc(0),
                    Instruction::RefFunc(1),
                    Instruction::CallRef(2),
                ],
                is_export: false,
            }],
            gc_types: vec![GcTypeDef {
                name: "B".to_string(),
                kind: GcTypeKind::Struct(vec![
                    GcField {
                        name: "value".to_string(),
                        ty: IrType::Ref(0),
                        mutable: false,
                    },
                    GcField {
                        name: "call".to_string(),
                        ty: IrType::TypedFuncRef(2),
                        mutable: false,
                    },
                ]),
            }],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "b-import".to_string(),
                params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
                result: IrType::Ref(0),
            }],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[module_a, module_b]);
        assert_eq!(
            linked.imports[0].params,
            vec![IrType::Ref(0), IrType::TypedFuncRef(4)]
        );
        assert_eq!(linked.imports[0].result, IrType::Ref(0));
        assert_eq!(
            linked.imports[1].params,
            vec![IrType::Ref(1), IrType::TypedFuncRef(5)]
        );
        assert_eq!(linked.imports[1].result, IrType::Ref(1));
        assert_eq!(
            linked.functions[0].params,
            vec![IrType::Ref(0), IrType::TypedFuncRef(4)]
        );
        assert_eq!(linked.functions[0].result, IrType::TypedFuncRef(4));
        assert_eq!(
            linked.functions[0].locals,
            vec![IrType::Ref(0), IrType::TypedFuncRef(2)]
        );
        assert_eq!(
            linked.functions[1].params,
            vec![IrType::Ref(1), IrType::TypedFuncRef(5)]
        );
        assert_eq!(linked.functions[1].result, IrType::TypedFuncRef(5));
        assert_eq!(
            linked.functions[1].locals,
            vec![IrType::Ref(1), IrType::TypedFuncRef(3)]
        );

        fn assert_gc_field_types(gc_type: &GcTypeDef, expected_ref: u32, expected_func: u32) {
            let GcTypeKind::Struct(fields) = &gc_type.kind else {
                panic!("expected struct GC type: {gc_type:?}");
            };
            assert_eq!(fields[0].ty, IrType::Ref(expected_ref));
            assert_eq!(fields[1].ty, IrType::TypedFuncRef(expected_func));
        }

        assert_gc_field_types(&linked.gc_types[0], 0, 4);
        assert_gc_field_types(&linked.gc_types[1], 1, 5);
    }

    #[test]
    fn test_link_funcref_rebases_array_element_type() {
        fn module(name: &str) -> Module {
            Module {
                functions: vec![Function {
                    name: name.to_string(),
                    params: vec![],
                    result: IrType::I64,
                    locals: vec![],
                    body: vec![Instruction::I64Const(0)],
                    is_export: false,
                }],
                gc_types: vec![GcTypeDef {
                    name: format!("{name}-array"),
                    kind: GcTypeKind::Array(IrType::TypedFuncRef(1)),
                }],
                imports: vec![],
                globals: vec![],
                string_data: vec![],
            }
        }

        let linked = link_modules(&[module("left"), module("right")]);
        assert_eq!(linked.gc_types.len(), 2);
        assert_eq!(linked.functions.len(), 2);

        assert_array_element_type(&linked.gc_types[0], IrType::TypedFuncRef(2));
        assert_array_element_type(&linked.gc_types[1], IrType::TypedFuncRef(3));

        fn assert_array_element_type(gc_type: &GcTypeDef, expected: IrType) {
            let GcTypeKind::Array(element_type) = &gc_type.kind else {
                panic!("expected array GC type: {gc_type:?}");
            };
            assert_eq!(*element_type, expected);
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
mod multifile_compile_tests {
    use super::*;

    #[test]
    fn test_merged_scc_declarations_deduplicate_identical_imports() {
        let module_a =
            lsharp_syntax::parse("(module A) (import Shared) (import Shared) (defn a [] 1)")
                .expect("module A should parse");
        let module_b = lsharp_syntax::parse("(module B) (import Shared) (defn b [] 2)")
            .expect("module B should parse");
        let parsed_modules =
            HashMap::from([("A".to_string(), module_a), ("B".to_string(), module_b)]);
        let group = vec!["A".to_string(), "B".to_string()];

        let (merged_decls, defn_origins) =
            merge_scc_declarations(&group, &parsed_modules).expect("SCC declarations should merge");
        let imports = merged_decls
            .iter()
            .filter_map(|decl| match decl {
                lsharp_syntax::ast::Decl::ImportDecl { module, .. } => Some(module.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(imports, vec!["Shared"]);
        assert_eq!(defn_origins, vec!["A", "B"]);
    }

    #[test]
    fn test_merged_scc_declarations_keep_distinct_import_visibility() {
        let module_a = lsharp_syntax::parse("(module A) (import Shared :only [x]) (defn a [] 1)")
            .expect("module A should parse");
        let module_b = lsharp_syntax::parse("(module B) (import Shared :only [y]) (defn b [] 2)")
            .expect("module B should parse");
        let parsed_modules =
            HashMap::from([("A".to_string(), module_a), ("B".to_string(), module_b)]);
        let group = vec!["A".to_string(), "B".to_string()];

        let (merged_decls, _) =
            merge_scc_declarations(&group, &parsed_modules).expect("SCC declarations should merge");
        let imports = merged_decls
            .iter()
            .filter_map(|decl| match decl {
                lsharp_syntax::ast::Decl::ImportDecl { only, .. } => only.clone(),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(imports, vec![vec!["x".to_string()], vec!["y".to_string()]]);
    }

    fn main_function(module: &Module) -> &Function {
        module
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function should exist")
    }

    fn call_positions(body: &[Instruction], target: u32) -> Vec<usize> {
        body.iter()
            .enumerate()
            .filter_map(|(idx, instr)| match instr {
                Instruction::Call(actual) if *actual == target => Some(idx),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_compile_multi_file_injects_only_dependency_closure() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_dependency_closure");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("A.ls"), "(module A)\n(defn status [x] x)\n").unwrap();
        std::fs::write(
            dir.join("Noise.ls"),
            "(module Noise)\n(defn status [x] true)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("ZConsumer.ls"),
            "(module ZConsumer)\n(import A)\n(defn check [x] (= (status x) 1))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(import Noise)\n(import ZConsumer)\n(defn main [] (if (check 1) 1 0))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_ok(),
            "unrelated sibling module types should not pollute dependency inference: {result:?}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_import_only_blocks_non_selected_symbol() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_import_only_blocks");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(defn helper [] 1)\n(defn secret [] 2)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils :only [helper])\n(defn main [] (secret))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_err(),
            ":only で除外されたシンボルは compile でも参照できないべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_infers_mutual_recursive_scc() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_mutual_recursive_scc_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_ok(),
            "相互再帰 SCC はモジュール単位の循環エラーではなく一括推論へ進めるべき: {result:?}"
        );
        let module = result.unwrap();
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "a-step")
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "b-step")
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "main")
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_unrestricted_scc_uses_merged_surface_fast_path() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_unrestricted_scc_fast_path_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let tracker = IncrementalSccMergedFastPathTracker::new();
        tracker.reset();
        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_ok(),
            "公開 import の SCC は compile できるべき: {result:?}"
        );
        assert_eq!(
            tracker.count(),
            1,
            "可視性制約のない A↔B SCC は merged inference の surface を再検証なしで利用するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_infers_mutual_recursive_scc() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_incremental_mutual_recursive_scc_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        let type_tracker = IncrementalTypeInferTracker::new();
        type_tracker.reset();
        let first = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache);
        assert!(
            first.is_ok(),
            "incremental compile も相互再帰 SCC を受理するべき: {first:?}"
        );
        assert_eq!(
            type_tracker.count(),
            1,
            "singleton SCC は module-local inference を 1 回だけ実行するべき"
        );
        drop(type_tracker);
        let first = first.unwrap();
        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        let second = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(
            tracker.count(),
            0,
            "SCC の clean rebuild は module inference を再実行しないべき"
        );
        assert_eq!(
            first.dump(),
            second.dump(),
            "SCC の clean rebuild は同じ linked IR を返すべき"
        );

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
        )
        .unwrap();
        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert!(
            tracker.count() > 0,
            "SCC の dirty rebuild は型推論を再実行するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_scc_reuses_clean_ir_segments_after_dirty_module() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_incremental_scc_segments_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert!(
            !cache
                .get("Base")
                .expect("Base module should be cached")
                .ir_segments()
                .is_empty(),
            "SCC compile 後も独立した module の IR segment を cache するべき"
        );

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let link_tracker = IncrementalLinkTracker::new();
        link_tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "SCC 内の A だけが dirty なら clean module の segment を再利用し、fresh lower は A のみにするべき"
        );
        assert_eq!(
            link_tracker.cache_hit_count(),
            1,
            "SCC の segment 長が不変なら cached final module を range patch するべき"
        );
        assert_eq!(
            link_tracker.full_count(),
            0,
            "SCC の segment 長が不変なら full relink を再実行しないべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "SCC の dirty segment reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "SCC の dirty segment reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_scc_reuses_clean_type_surfaces_after_impl_change() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_incremental_scc_type_cache_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
        )
        .unwrap();

        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "A の実装だけが dirty な場合は Base/Main の clean SCC を再推論せず、A↔B SCC だけを再推論するべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "SCC type surface reuse 後も final linked IR は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_scc_preserves_import_only_visibility() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_scc_import_only_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B :only [b-step])\n(defn a-step [] (secret))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [] (a-step))\n(defn secret [] 2)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_err(),
            "SCC 内でも import :only の境界を越えてはならない"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("secret"),
            "診断に拒否された symbol を含めるべき: {error}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_private_import_blocks_symbol() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_private_blocks");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(private (defn secret [] 2))\n(defn helper [] 1)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils)\n(defn main [] (secret))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_err(),
            "private なシンボルは compile でも参照できないべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_modular_lowering_matches_merged_reference_with_strings() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_modular_matches_merged");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Lib.ls"),
            "(module Lib)\n(defn helper [] \"lib\")\n(defn helper2 [] \"++\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Suffix.ls"),
            "(module Suffix)\n(defn bang [] \"!\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(import Suffix)\n(defn main [] (string-concat (string-concat (helper) (helper2)) (bang)))\n",
        )
        .unwrap();

        let merged =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Merged)
                .unwrap();
        let modular =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Modular)
                .unwrap();

        assert_eq!(
            merged.dump(),
            modular.dump(),
            "module-local lowering は merged lowering と同じ関数順序・命令列を維持するべき"
        );
        assert_eq!(
            merged.string_data, modular.string_data,
            "module-local lowering は merged lowering と同じ string_data 配列を維持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_closure_call_roots_local_generic_result_argument() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_closure_generic_result_rooting");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Lib.ls"),
            "(module Lib)\n(defn make-show [] (fn [s] (string-length s)))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (let [id (fn [x] x) f (make-show)] (f (id \"hello\"))))\n",
        )
        .unwrap();

        let merged =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Merged)
                .unwrap();
        let modular =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Modular)
                .unwrap();

        assert_eq!(
            call_positions(&main_function(&merged).body, 14).len(),
            4,
            "multi-file merged lowering でも local generic closure result を使う closure call は outer arg 用まで root_push するべき: {:?}",
            main_function(&merged).body
        );
        assert_eq!(
            call_positions(&main_function(&modular).body, 14).len(),
            4,
            "multi-file modular lowering でも local generic closure result を使う closure call は outer arg 用まで root_push するべき: {:?}",
            main_function(&modular).body
        );
        assert_eq!(
            merged.dump(),
            modular.dump(),
            "expr-type table を通した modular lowering も merged lowering と同一 IR を維持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    #[test]
    fn test_source_fingerprint_identical_content() {
        let left = SourceFingerprint::from_source("(defn answer [] 42)\n");
        let right = SourceFingerprint::from_source("(defn answer [] 42)\n");

        assert_eq!(left, right, "同一ソースは同一 fingerprint になるべき");
    }

    #[test]
    fn test_source_fingerprint_one_char_change() {
        let left = SourceFingerprint::from_source("(defn answer [] 42)\n");
        let right = SourceFingerprint::from_source("(defn answer [] 43)\n");

        assert_ne!(
            left, right,
            "1 文字でも変更されたソースは別 fingerprint になるべき"
        );
    }

    #[test]
    fn test_source_fingerprint_empty_source() {
        let empty = SourceFingerprint::from_source("");
        let also_empty = SourceFingerprint::from_source("");
        let whitespace = SourceFingerprint::from_source(" ");

        assert_eq!(
            empty, also_empty,
            "空ソースは決定的に fingerprint できるべき"
        );
        assert_ne!(
            empty, whitespace,
            "空ソースと空白 1 文字は別 fingerprint になるべき"
        );
    }
}

#[cfg(test)]
mod incremental_compile_tests {
    use super::*;

    fn main_function(module: &Module) -> &Function {
        module
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function should exist")
    }

    fn call_positions(body: &[Instruction], target: u32) -> Vec<usize> {
        body.iter()
            .enumerate()
            .filter_map(|(idx, instr)| match instr {
                Instruction::Call(actual) if *actual == target => Some(idx),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_compile_multi_file_incremental_empty_cache_matches_full_compile() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_empty_cache");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let lib_source = "(module Lib)\n(defn helper [] 7)\n";
        let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();
        let mut cache = CompilationCache::new();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let main_entry = cache.get("Main").expect("Main module should be cached");

        assert_eq!(
            full.dump(),
            incremental.dump(),
            "空キャッシュ初回コンパイルは既存のフルコンパイルと同一結果になるべき"
        );
        assert_eq!(
            cache.len(),
            2,
            "初回 incremental compile は通過したモジュールを cache に記録するべき"
        );
        assert!(
            main_entry.type_result_len() > 0,
            "cache entry は型サーフェスも保持するべき"
        );
        assert_eq!(
            main_entry.fingerprint(),
            SourceFingerprint::from_source(main_source),
            "cache entry は読み込んだソースの fingerprint を保持するべき"
        );
        assert_eq!(
            main_entry.imports(),
            ["Lib"],
            "cache entry は direct import module 名を保持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_with_cache_matches_fresh_and_warm_compile() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_with_cache_api_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let lib_source = "(module Lib)\n(defn helper [] 7)\n";
        let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let fresh = compile_multi_file(&dir.join("Main.ls")).unwrap();
        let mut cache = CompilationCache::new();
        let tracker = IncrementalTypeInferTracker::new();
        let cold = compile_multi_file_with_cache(&dir.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(fresh.dump(), cold.dump());
        assert_eq!(cache.len(), 2);

        tracker.reset();
        let warm = compile_multi_file_with_cache(&dir.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(cold.dump(), warm.dump());
        assert_eq!(
            tracker.count(),
            0,
            "warm cache compile は再型推論しないべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_with_cache_isolated_by_entry_root() {
        let base = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_with_cache_scope_{}",
            std::process::id()
        ));
        if base.exists() {
            std::fs::remove_dir_all(&base).unwrap();
        }
        let first = base.join("first");
        let second = base.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();

        std::fs::write(first.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            first.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();
        std::fs::write(second.join("Main.ls"), "(module Main)\n(defn main [] 42)\n").unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_with_cache(&first.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(
            cache.len(),
            2,
            "first project は Main と Lib を cache するべき"
        );

        let second_module = compile_multi_file_with_cache(&second.join("Main.ls"), &mut cache)
            .expect("entry root が変わっても compile できるべき");
        assert!(matches!(
            main_function(&second_module).body.as_slice(),
            [Instruction::I64Const(42)]
        ));
        assert_eq!(
            cache.len(),
            1,
            "別 project の stale module は cache に残さないべき"
        );

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn test_compile_multi_file_with_cache_tracks_dependency_surface_key() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_with_cache_dependency_key_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        let lib_path = dir.join("Lib.ls");
        let main_path = dir.join("Main.ls");
        std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            &main_path,
            "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
        let initial_key = cache.get("Main").unwrap().deps_key();

        std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 8)\n").unwrap();
        compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
        let implementation_only_key = cache.get("Main").unwrap().deps_key();
        assert_eq!(
            initial_key, implementation_only_key,
            "依存 module の実装だけが変わった場合、公開型 key は維持するべき"
        );

        std::fs::write(&lib_path, "(module Lib)\n(defn helper [] true)\n").unwrap();
        compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
        let surface_changed_key = cache.get("Main").unwrap().deps_key();
        assert_ne!(
            implementation_only_key, surface_changed_key,
            "依存 module の公開型が変わった場合、依存 key も変わるべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_roots_local_generic_closure_result_argument() {
        let dir = std::env::temp_dir()
            .join("lsharp_compile_multi_file_incremental_closure_generic_result_rooting");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Lib.ls"),
            "(module Lib)\n(defn make-show [] (fn [s] (string-length s)))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (let [id (fn [x] x) f (make-show)] (f (id \"hello\"))))\n",
        )
        .unwrap();

        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();
        let mut cache = CompilationCache::new();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            call_positions(&main_function(&full).body, 14).len(),
            4,
            "full multi-file compile は local generic closure result を使う closure call で outer arg 用まで root_push するべき: {:?}",
            main_function(&full).body
        );
        assert_eq!(
            call_positions(&main_function(&incremental).body, 14).len(),
            4,
            "incremental multi-file compile も expr-type cache を通して outer arg 用まで root_push するべき: {:?}",
            main_function(&incremental).body
        );
        assert_eq!(
            full.dump(),
            incremental.dump(),
            "incremental multi-file compile も expr-type table を含めて full compile と同一 IR を維持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_parse_on_cache_hit() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let lib_source = "(module Lib)\n(defn helper [] 7)\n";
        let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let mut cache = CompilationCache::new();
        let tracker = IncrementalParseTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        tracker.reset();
        cached_program_or_parse(
            "Lib",
            lib_source,
            SourceFingerprint::from_source(lib_source),
            &cache,
        )
        .unwrap();
        cached_program_or_parse(
            "Main",
            main_source,
            SourceFingerprint::from_source(main_source),
            &cache,
        )
        .unwrap();
        assert_eq!(
            tracker.count(),
            0,
            "事前確認として cache helper 単体では両モジュールとも hit するべき"
        );

        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            0,
            "fingerprint が不変な再コンパイルでは AST cache hit により parse をスキップするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reparses_only_changed_module() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_single_change");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 2))\n",
        )
        .unwrap();

        let tracker = IncrementalParseTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "1 モジュールだけ fingerprint が変わった場合はその AST だけ再パースするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_cached_ast_arc_on_cache_hit() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_ast_arc_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let main_source = "(module Main)\n(defn main [] 1)\n";
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        let cached = cache
            .get("Main")
            .expect("Main module should be cached")
            .ast_arc();
        let reused = cached_program_or_parse(
            "Main",
            main_source,
            SourceFingerprint::from_source(main_source),
            &cache,
        )
        .unwrap();

        assert!(
            std::sync::Arc::ptr_eq(&cached, &reused),
            "AST cache hit では同じ Arc<Program> を再利用するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_type_inference_on_clean_cache_hit() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        let tracker = IncrementalTypeInferTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            0,
            "dirty set が空なら cached ModuleTypeSurface を再利用して型推論をスキップするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_ir_generation_on_clean_cache_hit() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_ir_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        let tracker = IncrementalLowerTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            0,
            "dirty set が空なら cached IR を再利用して lowering をスキップするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_analyze_single_file_incremental_skips_parse_and_infer_on_clean_cache_hit() {
        let mut cache = CompilationCache::new();
        let source = "(module Main)\n(defn main [] 1)\n";

        analyze_single_file_incremental("lsp://Main", source, &mut cache).unwrap();

        let parse_tracker = IncrementalParseTracker::new();
        let infer_tracker = IncrementalTypeInferTracker::new();
        parse_tracker.reset();
        infer_tracker.reset();

        analyze_single_file_incremental("lsp://Main", source, &mut cache).unwrap();

        assert_eq!(
            parse_tracker.count(),
            0,
            "single-file incremental analysis は clean hit で parse を再実行しないべき"
        );
        assert_eq!(
            infer_tracker.count(),
            0,
            "single-file incremental analysis は clean hit で type infer を再実行しないべき"
        );
    }

    #[test]
    fn test_analyze_single_file_incremental_reparses_and_reinfers_on_source_change() {
        let mut cache = CompilationCache::new();
        analyze_single_file_incremental(
            "lsp://Main",
            "(module Main)\n(defn main [] 1)\n",
            &mut cache,
        )
        .unwrap();

        let parse_tracker = IncrementalParseTracker::new();
        let infer_tracker = IncrementalTypeInferTracker::new();
        parse_tracker.reset();
        infer_tracker.reset();

        analyze_single_file_incremental(
            "lsp://Main",
            "(module Main)\n(defn main [] 2)\n",
            &mut cache,
        )
        .unwrap();

        assert_eq!(
            parse_tracker.count(),
            1,
            "single-file incremental analysis は fingerprint が変わった source を再パースするべき"
        );
        assert_eq!(
            infer_tracker.count(),
            1,
            "single-file incremental analysis は fingerprint が変わった source を再推論するべき"
        );
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_reports_unsaved_missing_import() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir()
            .join("lsharp_analyze_multi_file_incremental_overlay_missing_import");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] 1)\n",
        )
        .unwrap();

        let mut overrides = HashMap::new();
        overrides.insert(
            dir.join("Main.ls"),
            "(module Main)\n(import Missing)\n(defn main [] 1)\n".to_string(),
        );
        let mut cache = CompilationCache::new();

        let result = analyze_multi_file_incremental_with_overrides(
            &dir.join("Main.ls"),
            &overrides,
            &mut cache,
        );

        let _ = std::fs::remove_dir_all(&dir);

        let error = result.expect_err("unsaved import override は missing module error を返すべき");
        assert!(
            error.contains("Missing"),
            "error は unsaved source の import 先 Missing を含むべき: {error}"
        );
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_isolated_by_entry_root() {
        use std::collections::HashMap;

        let base = std::env::temp_dir().join(format!(
            "lsharp_analyze_multi_file_incremental_overlay_scope_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let first = base.join("first");
        let second = base.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            first.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
        )
        .unwrap();
        std::fs::write(second.join("Main.ls"), "(module Main)\n(defn main [] 42)\n").unwrap();

        let overrides = HashMap::new();
        let mut cache = CompilationCache::new();
        analyze_multi_file_incremental_with_overrides(
            &first.join("Main.ls"),
            &overrides,
            &mut cache,
        )
        .unwrap();
        assert_eq!(
            cache.len(),
            2,
            "first workspace は Main と Lib を cache するべき"
        );

        analyze_multi_file_incremental_with_overrides(
            &second.join("Main.ls"),
            &overrides,
            &mut cache,
        )
        .unwrap();
        assert_eq!(
            cache.len(),
            1,
            "別 workspace の override analysis は stale module を残さないべき"
        );

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_infers_mutual_recursive_scc() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_analyze_multi_file_incremental_overlay_scc_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut overrides = HashMap::new();
        overrides.insert(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n"
                .to_string(),
        );
        let mut cache = CompilationCache::new();
        let result = analyze_multi_file_incremental_with_overrides(
            &dir.join("Main.ls"),
            &overrides,
            &mut cache,
        );

        assert!(
            result.is_ok(),
            "source override analysis も相互再帰 SCC を受理するべき: {result:?}"
        );
        assert_eq!(cache.len(), 3, "SCC 内外の 3 module を cache するべき");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_reuses_clean_scc_type_surfaces() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_analyze_multi_file_incremental_overlay_type_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut overrides = HashMap::new();
        overrides.insert(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n"
                .to_string(),
        );
        let mut cache = CompilationCache::new();
        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache)
            .unwrap();
        assert_eq!(
            tracker.count(),
            3,
            "override の初回分析は Base / A↔B / Main の3 SCCを型推論するべき"
        );

        overrides.insert(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n"
                .to_string(),
        );
        tracker.reset();
        analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache)
            .unwrap();
        assert_eq!(
            tracker.count(),
            1,
            "override で A の実装だけが dirty な場合は A↔B SCC だけを再推論するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_prefix_module_ir_segments_before_first_dirty_module()
     {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_module_ir_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert!(
            !cache
                .get("Base")
                .expect("Base module should be cached")
                .ir_segments()
                .is_empty(),
            "warm cache 後は prefix module の IR segment が保存されるべき"
        );

        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 2))\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "tail module だけ dirty な場合は clean prefix module の IR segment を再利用し、fresh lower は dirty suffix のみで済むべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "prefix IR segment reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "prefix IR segment reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_clean_suffix_module_when_dirty_middle_layout_is_stable()
     {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_suffix_ir_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 21))\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "dirty middle module が layout 不変なら clean suffix module の IR segment も再利用し、fresh defn lower は dirty module のみで済むべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "clean suffix IR segment reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "clean suffix IR segment reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_clean_suffix_when_dirty_middle_only_changes_string_state()
     {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_suffix_string_state");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n(defn mid-label [] \"a\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n(defn mid-label [] \"alphabet\")\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "dirty middle module の defn string state だけ変わる場合は clean suffix module の defn を再 lower しないべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "suffix defn reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "suffix defn reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_patches_cached_final_link_when_segment_lengths_match() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_link_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 21))\n",
        )
        .unwrap();

        let tracker = IncrementalLinkTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.cache_hit_count(),
            1,
            "module order と segment 長が不変なら cached final module を range patch して full relink を避けるべき"
        );
        assert_eq!(
            tracker.full_count(),
            0,
            "range patch が成立する変更では full relink を再実行しないべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "link cache hit 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "link cache hit 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_invalidates_link_cache_when_segment_lengths_change() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_link_cache_miss");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(defn mid-val [] \"a\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (mid-val))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(defn mid-val [] (string-concat \"a\" \"b\"))\n",
        )
        .unwrap();

        let tracker = IncrementalLinkTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.cache_hit_count(),
            0,
            "string_data segment 長が変わる変更では cached final module patch は使わないべき"
        );
        assert_eq!(
            tracker.full_count(),
            1,
            "segment 長が変わる変更では full relink にフォールバックするべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "link cache miss 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "link cache miss 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_dependent_reinfer_when_surface_unchanged() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_impl_change");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 8)\n").unwrap();

        let tracker = IncrementalTypeInferTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "依存先の実装変更で型サーフェスが不変なら dependency のみ再型推論し、dependent は再利用するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reinfers_on_dependency_signature_change() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_sig_change");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] true)\n").unwrap();

        let tracker = IncrementalTypeInferTracker::new();
        tracker.reset();
        let result = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache);

        assert!(
            result.is_err(),
            "依存先シグネチャ変更で不整合になれば compile は失敗するべき"
        );
        assert_eq!(
            tracker.count(),
            2,
            "依存先シグネチャ変更では dependency + dependent を再型推論するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds() {
        let cli_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../selfhost/src/App/Cli.ls");
        let mut cache = CompilationCache::new();

        compile_multi_file_incremental(&cli_path, &mut cache)
            .expect("first incremental compile of selfhost Cli.ls should succeed");
        let second = compile_multi_file_incremental(&cli_path, &mut cache);

        assert!(
            second.is_ok(),
            "clean rebuild with formatter trio cache should not fail: {second:?}"
        );
    }

    #[test]
    fn test_formatter_modules_declare_cross_module_dispatch_imports() {
        let source_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../selfhost/src/Tools/Text");
        let expr = std::fs::read_to_string(source_root.join("FormatterExpr.ls")).unwrap();
        let decl = std::fs::read_to_string(source_root.join("FormatterDecl.ls")).unwrap();

        assert!(
            expr.lines()
                .any(|line| line.trim() == "(import Tools.Text.Formatter)"),
            "FormatterExpr は dispatch 関数の提供元を明示 import するべき"
        );
        assert!(
            decl.lines()
                .any(|line| line.trim() == "(import Tools.Text.Formatter)"),
            "FormatterDecl は dispatch 関数の提供元を明示 import するべき"
        );
    }
}

#[cfg(test)]
mod memory_instruction_tests {
    use super::*;

    #[test]
    fn test_memory_load_store_instructions() {
        let instructions = [
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
        let instructions = [
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
        let instructions = [
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
        let instructions = [
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
        let instructions = [
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32GtU,
            Instruction::I32GeU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_i32_bitwise_instructions() {
        let instructions = [
            Instruction::I32Const(0xFF),
            Instruction::I32Const(4),
            Instruction::I32Shl,
            Instruction::I32ShrU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_memory_management_instructions() {
        let instructions = [
            Instruction::MemorySize,
            Instruction::I32Const(1),
            Instruction::MemoryGrow,
        ];
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_i64_extend_i32_unsigned() {
        let instructions = [Instruction::I32Const(42), Instruction::I64ExtendI32U];
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
        assert_eq!(format!("{}", Instruction::MemoryGrow), "memory.grow");
        assert_eq!(format!("{}", Instruction::MemorySize), "memory.size");
        assert_eq!(format!("{}", Instruction::I32Add), "i32.add");
    }
}

#[cfg(test)]
mod selfhost_collision_tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    fn selfhost_path(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(rel)
    }

    fn parse_defn_names(rel: &str) -> HashSet<String> {
        let path = selfhost_path(rel);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{} を読めませんでした: {err}", path.display()));
        let program = lsharp_syntax::parse(&source)
            .unwrap_or_else(|err| panic!("{} を parse できませんでした: {err}", path.display()));
        program
            .decls
            .into_iter()
            .filter_map(|decl| match decl {
                lsharp_syntax::ast::Decl::Defn { name, .. } => Some(name),
                lsharp_syntax::ast::Decl::Private { inner, .. } => match *inner {
                    lsharp_syntax::ast::Decl::Defn { name, .. } => Some(name),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_selfhost_ir_control_flow_builders_do_not_reuse_ast_make_if_name() {
        let ast_names = parse_defn_names("selfhost/src/Syntax/AST.ls");
        let ir_names = parse_defn_names("selfhost/src/IR/IR.ls");

        assert!(
            ast_names.contains("make-if"),
            "Syntax.AST 側の make-if 前提が崩れている"
        );
        assert!(
            !ir_names.contains("make-if"),
            "IR.IR に make-if があると multi-file lowering で Syntax.AST.make-if と衝突する"
        );
    }
}

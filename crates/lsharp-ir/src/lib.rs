//! L# 中間表現 (IR)
//!
//! MVP ではフラット化された命令列を使用。
//! 将来的に SSA 形式の BasicBlock ベースに拡張する。

pub mod cache;
pub mod closure;
pub mod lower;
pub mod module_graph;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use module_graph::{FORMATTER_TRIO_DECL, FORMATTER_TRIO_EXPR, FORMATTER_TRIO_MAIN};
use sha2::{Digest, Sha256};

pub use cache::{CompilationCache, ModuleCacheEntry};

/// SHA-256 ベースのソース fingerprint。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    pub fn from_source(source: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
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

    // スタック操作
    Drop,

    // GC 命令 (WasmGC)
    StructNew(u32),      // struct.new type_idx
    StructGet(u32, u32), // struct.get type_idx field_idx
    StructSet(u32, u32), // struct.set type_idx field_idx
    RefCast(u32),        // ref.cast type_idx (ダウンキャスト)

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

fn try_infer_formatter_trio_batch(
    sorted_files: &[(String, std::path::PathBuf)],
) -> Option<HashMap<String, ModuleTypeSurface>> {
    use lsharp_syntax::ast::{Decl, Program};

    let path_map: HashMap<String, std::path::PathBuf> = sorted_files.iter().cloned().collect();
    let p_expr = path_map.get(FORMATTER_TRIO_EXPR)?;
    let p_decl = path_map.get(FORMATTER_TRIO_DECL)?;
    let p_fmt = path_map.get(FORMATTER_TRIO_MAIN)?;

    let mut merged_decls: Vec<Decl> = Vec::new();
    let mut defn_origins: Vec<String> = Vec::new();
    let mut seen_import: HashSet<String> = HashSet::new();

    for (mod_name, path) in [
        (FORMATTER_TRIO_EXPR, p_expr),
        (FORMATTER_TRIO_DECL, p_decl),
        (FORMATTER_TRIO_MAIN, p_fmt),
    ] {
        let source = std::fs::read_to_string(path).ok()?;
        let program = lsharp_syntax::parse(&source).ok()?;
        for decl in program.decls {
            match &decl {
                Decl::ImportDecl { module, .. } => {
                    if seen_import.insert(module.clone()) {
                        merged_decls.push(decl);
                    }
                }
                Decl::ModuleDecl { .. } => {}
                _ => {
                    push_defn_origins_infer_order(
                        std::slice::from_ref(&decl),
                        mod_name,
                        None,
                        &mut defn_origins,
                    );
                    merged_decls.push(decl);
                }
            }
        }
    }

    let merged = Program {
        decls: merged_decls,
    };
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer.infer_program(&merged).ok()?;
    if type_results.len() != defn_origins.len() {
        return None;
    }

    let mut by_mod: HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>> =
        HashMap::new();
    for ((name, scheme), origin) in type_results.into_iter().zip(defn_origins.into_iter()) {
        by_mod.entry(origin).or_default().push((name, scheme));
    }

    let mut out_map: HashMap<String, ModuleTypeSurface> = HashMap::new();
    for (k, v) in by_mod {
        out_map.insert(
            k,
            ModuleTypeSurface {
                results: v,
                hidden: HashSet::new(),
            },
        );
    }
    Some(out_map)
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

fn build_module_cache_entry(
    fingerprint: SourceFingerprint,
    program: &Arc<lsharp_syntax::ast::Program>,
    type_surface: ModuleTypeSurface,
) -> ModuleCacheEntry {
    ModuleCacheEntry::new(
        fingerprint,
        Arc::clone(program),
        type_surface,
        Module {
            functions: Vec::new(),
            gc_types: Vec::new(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: Vec::new(),
        },
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

pub fn compile_multi_file(entry_file: &std::path::Path) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    // 1. モジュールグラフの構築とファイル探索
    let (graph, sorted_files) = ModuleGraph::build_from_entry(entry_file)
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
        let mut lower_ctx = lower::Lower::new();
        return lower_ctx
            .lower_program(&program, &type_results)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    // 2. 全モジュールをトポロジカルソート順にパース → 型チェック
    //    全宣言を結合して、1つの Program として扱う
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();

    let formatter_trio_batch = try_infer_formatter_trio_batch(&sorted_files);

    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;

        let program =
            lsharp_syntax::parse(&source).map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let direct_imports = collect_import_visibility(&program);

        let (type_results, surface_hidden): (
            Vec<(String, lsharp_types::types::TypeScheme)>,
            HashSet<String>,
        ) = if let Some(ref batch) = formatter_trio_batch
            && let Some(surface) = batch.get(mod_name)
        {
            (surface.results.clone(), surface.hidden.clone())
        } else {
            // 型チェック（直接 import されたモジュールの公開シンボルだけを注入）
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(mod_name) {
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
            let type_results = infer
                .infer_program(&program)
                .map_err(|e| format!("{}: {e}", mod_path.display()))?;
            let hidden: HashSet<String> = infer.module_env.privates.iter().cloned().collect();
            (type_results, hidden)
        };

        // 型結果を蓄積
        all_type_results.extend(type_results.clone());
        per_module_type_results.insert(
            mod_name.clone(),
            ModuleTypeSurface {
                results: type_results,
                hidden: surface_hidden,
            },
        );

        // 宣言を収集（module 宣言と import 宣言は除外）
        for decl in program.decls {
            match &decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. } => {}
                lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => all_decls.push(decl),
            }
        }
    }

    // 3. 結合した宣言で 1 つの Program を構成し、IR 変換
    let merged_program = lsharp_syntax::ast::Program { decls: all_decls };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx
        .lower_program(&merged_program, &all_type_results)
        .map_err(|e| format!("IR 変換エラー: {e}"))
}

pub fn compile_multi_file_incremental(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    let (graph, sorted_files) = ModuleGraph::build_from_entry(entry_file)
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
            return Ok(
                cache
                    .get(mod_name)
                    .expect("clean hit should have cache entry")
                    .ir()
                    .clone(),
            );
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
            }
        };
        let mut lower_ctx = lower::Lower::new();
        note_incremental_lower();
        let module = lower_ctx
            .lower_program(program.as_ref(), &type_surface.results)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let mut entry = build_module_cache_entry(fingerprint, &program, type_surface);
        entry.set_ir(module.clone());
        cache.insert_module(mod_name.clone(), entry);
        return Ok(module);
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
    let formatter_trio_dirty = changed_modules.iter().any(|module| {
        matches!(
            module.as_str(),
            FORMATTER_TRIO_EXPR | FORMATTER_TRIO_DECL | FORMATTER_TRIO_MAIN
        )
    });
    if changed_modules.is_empty() {
        let first_clean_entry = module_inputs
            .first()
            .and_then(|(mod_name, _, _, _)| cache.get(mod_name).map(|entry| entry.ir().clone()))
            .expect("all clean hits should have cache entries");
        return Ok(first_clean_entry);
    }
    let mut formatter_trio_batch = if formatter_trio_dirty {
        try_infer_formatter_trio_batch(&sorted_files)
    } else {
        None
    };
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
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

        let is_formatter_module = matches!(
            mod_name.as_str(),
            FORMATTER_TRIO_EXPR | FORMATTER_TRIO_DECL | FORMATTER_TRIO_MAIN
        );
        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let formatter_trio_needs_batch = is_formatter_module
            && (formatter_trio_dirty || direct_dep_surface_changed || !clean_hit);
        if formatter_trio_needs_batch && formatter_trio_batch.is_none() {
            formatter_trio_batch = try_infer_formatter_trio_batch(&sorted_files);
        }

        let type_surface = if clean_hit
            && !direct_dep_surface_changed
            && (!is_formatter_module || !formatter_trio_dirty)
        {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else if formatter_trio_needs_batch
            && let Some(ref batch) = formatter_trio_batch
            && let Some(surface) = batch.get(&mod_name)
        {
            surface.clone()
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
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| entry.type_surface_clone() != type_surface)
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        all_type_results.extend(type_surface.results.clone());
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => all_decls.push(decl.clone()),
            }
        }

        let entry = build_module_cache_entry(fingerprint, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name.clone(), type_surface);
    }

    let merged_program = lsharp_syntax::ast::Program { decls: all_decls };
    let mut lower_ctx = lower::Lower::new();
    note_incremental_lower();
    let final_module = lower_ctx
        .lower_program(&merged_program, &all_type_results)
        .map_err(|e| format!("IR 変換エラー: {e}"))?;

    for (mod_name, mut entry) in cache_entries {
        entry.set_ir(final_module.clone());
        cache.insert_module(mod_name, entry);
    }

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

        assert_eq!(empty, also_empty, "空ソースは決定的に fingerprint できるべき");
        assert_ne!(
            empty, whitespace,
            "空ソースと空白 1 文字は別 fingerprint になるべき"
        );
    }
}

#[cfg(test)]
mod incremental_compile_tests {
    use super::*;

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
    fn test_compile_multi_file_incremental_skips_parse_on_cache_hit() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_cache_hit");
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
        cached_program_or_parse("Lib", lib_source, SourceFingerprint::from_source(lib_source), &cache)
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
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_cache_hit");
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

        assert!(result.is_err(), "依存先シグネチャ変更で不整合になれば compile は失敗するべき");
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
        let instructions = vec![Instruction::I32Const(42), Instruction::I64ExtendI32U];
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

use super::{Instruction, IrType};

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

use std::collections::HashMap;

use crate::IrType;

/// 関数変換コンテキスト
pub(crate) struct FuncCtx {
    pub(crate) function_name: String,
    pub(crate) type_scope_key: String,
    pub(crate) instructions: Vec<crate::Instruction>,
    pub(crate) locals_map: HashMap<String, u32>,
    pub(crate) local_type_names: HashMap<String, String>,
    pub(crate) param_count: u32,
    pub(crate) next_local: u32,
    /// Wasm local index ごとの IR 型（param を除く extra local の型生成にも使う）。
    pub(crate) local_types: Vec<IrType>,
}

impl FuncCtx {
    pub(crate) fn with_type_scope(name: String, type_scope_key: String) -> Self {
        Self {
            function_name: name,
            type_scope_key,
            instructions: Vec::new(),
            locals_map: HashMap::new(),
            local_type_names: HashMap::new(),
            param_count: 0,
            next_local: 0,
            local_types: Vec::new(),
        }
    }

    pub(crate) fn emit(&mut self, instr: crate::Instruction) {
        self.instructions.push(instr);
    }

    pub(crate) fn alloc_local(&mut self, name: String) -> u32 {
        self.alloc_local_typed(name, IrType::I64)
    }

    pub(crate) fn alloc_local_typed(&mut self, name: String, ty: IrType) -> u32 {
        // compiler が使う `_` prefix の一時ローカルは、入れ子の式で同名再利用すると
        // 外側の一時値を内側の lowering が上書きしてしまうため常に fresh にする。
        if name.starts_with('_') {
            let idx = self.next_local;
            self.next_local += 1;
            self.local_types.push(ty);
            return idx;
        }
        if let Some(&idx) = self.locals_map.get(&name) {
            return idx;
        }
        let idx = self.next_local;
        self.locals_map.insert(name, idx);
        self.next_local += 1;
        self.local_types.push(ty);
        idx
    }

    pub(crate) fn alloc_scoped_local_typed(&mut self, name: String, ty: IrType) -> u32 {
        let idx = self.next_local;
        self.locals_map.insert(name, idx);
        self.next_local += 1;
        self.local_types.push(ty);
        idx
    }

    pub(crate) fn restore_local_binding(
        &mut self,
        name: String,
        previous_local: Option<u32>,
        previous_type: Option<String>,
    ) {
        if let Some(idx) = previous_local {
            self.locals_map.insert(name.clone(), idx);
        } else {
            self.locals_map.remove(&name);
        }

        if let Some(type_name) = previous_type {
            self.local_type_names.insert(name, type_name);
        } else {
            self.local_type_names.remove(&name);
        }
    }
}

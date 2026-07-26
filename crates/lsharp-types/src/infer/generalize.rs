use super::Infer;
use crate::types::{Type, TypeEnv, TypeScheme, TypeVarId};

impl Infer {
    /// 型を汎化
    pub(super) fn generalize(&self, env: &TypeEnv, ty: &Type) -> TypeScheme {
        let env_vars = env.free_vars();
        let ty_vars = ty.free_vars();
        let vars: Vec<TypeVarId> = ty_vars
            .into_iter()
            .filter(|v| !env_vars.contains(v))
            .collect();
        TypeScheme {
            vars,
            constraints: Vec::new(),
            ty: ty.clone(),
        }
    }
}

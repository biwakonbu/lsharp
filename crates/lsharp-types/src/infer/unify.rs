use super::{Infer, TypeError, TypeErrorCode};
use crate::types::{Substitution, Type, TypeVarId};
use lsharp_syntax::span::Span;

impl Infer {
    /// 2つの型を統合 (Unification)
    pub(super) fn unify(
        &mut self,
        t1: &Type,
        t2: &Type,
        span: Span,
    ) -> Result<Substitution, TypeError> {
        match (t1, t2) {
            (Type::Con(a), Type::Con(b)) if a == b => Ok(Substitution::new()),

            (left, right) if Self::int_heap_compatible(left, right) => Ok(Substitution::new()),

            (Type::Var(id), ty) | (ty, Type::Var(id)) => self.bind_var(*id, ty, span),

            (Type::Fun(params1, ret1), Type::Fun(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::Mismatch {
                        expected: t1.clone(),
                        found: t2.clone(),
                        span,
                        error_code: TypeErrorCode::General,
                    });
                }
                let mut subst = Substitution::new();
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let s = self.unify(&p1.apply_subst(&subst), &p2.apply_subst(&subst), span)?;
                    subst = subst.compose(&s);
                }
                let s_ret =
                    self.unify(&ret1.apply_subst(&subst), &ret2.apply_subst(&subst), span)?;
                Ok(subst.compose(&s_ret))
            }

            (Type::App(name1, args1), Type::App(name2, args2))
                if name1 == name2 && args1.len() == args2.len() =>
            {
                let mut subst = Substitution::new();
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    let s = self.unify(&a1.apply_subst(&subst), &a2.apply_subst(&subst), span)?;
                    subst = subst.compose(&s);
                }
                Ok(subst)
            }

            (Type::Record(name1, fields1), Type::Record(name2, fields2))
                if name1 == name2 && fields1.len() == fields2.len() =>
            {
                let mut subst = Substitution::new();
                for ((n1, t1), (n2, t2)) in fields1.iter().zip(fields2.iter()) {
                    if n1 != n2 {
                        return Err(TypeError::Mismatch {
                            expected: Type::Record(name1.clone(), fields1.clone()),
                            found: Type::Record(name2.clone(), fields2.clone()),
                            span,
                            error_code: TypeErrorCode::General,
                        });
                    }
                    let s = self.unify(&t1.apply_subst(&subst), &t2.apply_subst(&subst), span)?;
                    subst = subst.compose(&s);
                }
                Ok(subst)
            }

            // レコード型と Con 型の統合（レコード名が一致する場合）
            (Type::Record(name, _), Type::Con(con_name))
            | (Type::Con(con_name), Type::Record(name, _))
                if name == con_name =>
            {
                Ok(Substitution::new())
            }

            _ => Err(TypeError::Mismatch {
                expected: t1.clone(),
                found: t2.clone(),
                span,
                error_code: TypeErrorCode::General,
            }),
        }
    }

    fn int_heap_compatible(left: &Type, right: &Type) -> bool {
        (matches!(left, Type::Con(name) if name == "Int") && right.is_heap_handle())
            || (matches!(right, Type::Con(name) if name == "Int") && left.is_heap_handle())
    }

    /// 型変数を型に束縛（occurs check 付き）
    fn bind_var(
        &mut self,
        var: TypeVarId,
        ty: &Type,
        span: Span,
    ) -> Result<Substitution, TypeError> {
        if let Type::Var(id) = ty
            && *id == var
        {
            return Ok(Substitution::new());
        }

        if ty.free_vars().contains(&var) {
            return Err(TypeError::InfiniteType {
                var,
                ty: ty.clone(),
                span,
            });
        }

        let mut subst = Substitution::new();
        subst.insert(var, ty.clone());
        // グローバル代入に累積（制約チェック用）
        self.global_subst.insert(var, ty.clone());
        Ok(subst)
    }
}

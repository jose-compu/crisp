use crate::result::ErrorSet;

pub fn error_type_name_from_annotation(ty: &crisp_ast::ty::Type) -> Option<String> {
    use crisp_ast::ty::TypeKind;
    match &ty.kind {
        TypeKind::Named(id) => Some(id.name.clone()),
        TypeKind::Never => Some("never".into()),
        _ => None,
    }
}

pub fn declared_set_from_fn(def: &crisp_ast::item::FunctionDef) -> (Option<ErrorSet>, bool) {
    let Some(err) = &def.error_type else {
        return (None, false);
    };
    let mut set = ErrorSet::new();
    let mut asserts_never = false;
    for v in &err.variants {
        if let Some(name) = error_type_name_from_annotation(v) {
            if name == "never" {
                asserts_never = true;
            } else {
                set.insert(name);
            }
        }
    }
    (Some(set), asserts_never)
}

pub fn thrown_error_name(expr: &crisp_ast::expr::Expr) -> Option<String> {
    use crisp_ast::expr::ExprKind;
    match &expr.kind {
        ExprKind::Str(_) => Some("Thrown".into()),
        ExprKind::StructLit { name, .. } => Some(name.name.clone()),
        ExprKind::Call { func, .. } => {
            if let ExprKind::Ident(id) = &func.kind {
                return Some(id.name.clone());
            }
            None
        }
        ExprKind::Ident(id) => Some(id.name.clone()),
        _ => None,
    }
}

pub fn catch_handled_set(arms: &[crisp_ast::expr::CatchArm]) -> ErrorSet {
    use crisp_ast::pat::PatKind;
    let mut set = ErrorSet::new();
    for arm in arms {
        match &arm.pat.kind {
            PatKind::Wildcard => {
                return ErrorSet::from_iter(["*".into()]);
            }
            PatKind::Ident(id) if id.name == "_" => {
                return ErrorSet::from_iter(["*".into()]);
            }
            PatKind::Ident(id) => {
                set.insert(id.name.clone());
            }
            PatKind::Enum { variant, .. } => {
                set.insert(variant.name.clone());
            }
            PatKind::Struct { name, .. } => {
                set.insert(name.name.clone());
            }
            _ => {}
        }
    }
    set
}

pub fn absorbs_all(handled: &ErrorSet) -> bool {
    handled.contains("*")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_and_subtract() {
        let mut a = ErrorSet::new();
        a.insert("IoError");
        let mut b = ErrorSet::new();
        b.insert("ParseError");
        let u = ErrorSet::union(&a, &b);
        assert!(u.contains("IoError"));
        assert!(u.contains("ParseError"));
        let mut h = ErrorSet::new();
        h.insert("IoError");
        let r = ErrorSet::subtract(&u, &h);
        assert!(!r.contains("IoError"));
        assert!(r.contains("ParseError"));
    }
}

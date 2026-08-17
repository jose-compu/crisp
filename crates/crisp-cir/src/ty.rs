use crisp_typeck::Ty;

#[derive(Debug, Clone, PartialEq)]
pub enum CirTy {
    Never,
    Unit,
    Bool,
    Int,
    UInt,
    Float,
    Char,
    Str,
    Named { name: String, args: Vec<CirTy> },
    Option(Box<CirTy>),
    Result { ok: Box<CirTy>, err: String },
    Ref { mutable: bool, inner: Box<CirTy> },
    Boxed(Box<CirTy>),
    Tuple(Vec<CirTy>),
    Fn { params: Vec<CirTy>, ret: Box<CirTy> },
    Var(u32),
    Error,
}

impl CirTy {
    pub fn from_ty(ty: &Ty) -> Self {
        match ty {
            Ty::Never => CirTy::Never,
            Ty::Unit => CirTy::Unit,
            Ty::Bool => CirTy::Bool,
            Ty::Int => CirTy::Int,
            Ty::UInt => CirTy::UInt,
            Ty::Float => CirTy::Float,
            Ty::Char => CirTy::Char,
            Ty::Str | Ty::StrSlice => CirTy::Str,
            Ty::Var(v) => CirTy::Var(*v),
            Ty::Tuple(ts) => CirTy::Tuple(ts.iter().map(CirTy::from_ty).collect()),
            Ty::Option(inner) => CirTy::Option(Box::new(CirTy::from_ty(inner))),
            Ty::Ref { mutable, inner } => CirTy::Ref {
                mutable: *mutable,
                inner: Box::new(CirTy::from_ty(inner)),
            },
            Ty::Named { name, args } => CirTy::Named {
                name: name.clone(),
                args: args.iter().map(CirTy::from_ty).collect(),
            },
            Ty::Fn { params, ret } => CirTy::Fn {
                params: params.iter().map(CirTy::from_ty).collect(),
                ret: Box::new(CirTy::from_ty(ret)),
            },
            Ty::Error => CirTy::Error,
            _ => CirTy::Error,
        }
    }

    pub fn is_stringish(&self) -> bool {
        matches!(self, CirTy::Str)
    }
}

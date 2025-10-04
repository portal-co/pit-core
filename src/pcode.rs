use alloc::{boxed::Box, string::String, vec::Vec};

/// Expression tree for pcode operations.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum PExpr {
    Param(usize),
    Var(String),
    Call {
        rid: [u8; 32],
        method: String,
        obj: Box<PExpr>,
        args: Vec<PExpr>,
        ret: Pat,
    },
    LitI32(u32),
    LitI64(u64),
    LitF32(u32),
    LitF64(u64),
}
/// Pattern for pcode expressions, including parameters and body.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Pat {
    pub params: Vec<String>,
    pub body: Box<PExpr>,
}

use crate::entities::names::NameId;

pub enum Expr {
    Int(i64),
    Ident(NameId),
}

impl Expr {
    pub fn parse() -> Self {
        todo!()
    }
}

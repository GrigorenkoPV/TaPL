use culpa::throws;

use crate::ast::Program;

mod error;

pub(crate) use self::error::Error;

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    todo!()
}

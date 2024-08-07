use std::fmt::Debug;

#[derive(Debug)]
pub struct CompileError(pub proc_macro2::TokenStream);

impl PartialEq for CompileError {
    fn eq(&self, other: &Self) -> bool {
        streams_equal(&self.0, &other.0)
    }
}

impl From<CompileError> for proc_macro::TokenStream {
    fn from(err: CompileError) -> Self {
        err.0.into()
    }
}

impl From<CompileError> for proc_macro2::TokenStream {
    fn from(err: CompileError) -> Self {
        err.0
    }
}

macro_rules! span_compile_error(($span: expr => $msg: expr) => {
    crate::error::CompileError(quote::quote_spanned! { $span => compile_error!($msg) }.into())
});

pub(crate) use span_compile_error;

use crate::common::streams_equal;


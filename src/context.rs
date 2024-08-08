mod cream_context;

pub use cream_context::CreamContext;
pub use cream_derive::FromContext;

pub trait FromContext<C> {
    fn from_context(ctx: &C) -> Self;
}

pub trait ContextExtend<C> {
    fn provide_context(&self) -> &C;
}


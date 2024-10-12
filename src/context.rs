mod cream_context;

pub use cream_context::CreamContext;
pub use cream_derive::ContextProvide;

pub trait ContextProvide<S> {
    fn ctx_provide(&self) -> S;
}

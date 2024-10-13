mod cream_context;

pub use cream_context::CreamContext;
pub use cream_derive::ContextProvide;

pub trait ContextProvide<S> {
    fn ctx_provide(&self) -> S;
}

pub trait ContextCreate<S> {
    type Args;
    type Deps;
    fn ctx_create(&self, args: Self::Args, deps: Self::Deps) -> S;
}

pub trait Context {
    #[inline]
    fn provide<S>(&self) -> S
    where
        Self: ContextProvide<S>,
    {
        self.ctx_provide()
    }

    #[inline]
    fn create<S>(&self, args: Self::Args) -> S
    where
        Self: ContextCreate<S>,
        Self: ContextProvide<Self::Deps>,
    {
        let deps = self.provide();
        self.ctx_create(args, deps)
    }
}

impl<C> ContextProvide<()> for C
where
    C: Context,
{
    fn ctx_provide(&self) {}
}

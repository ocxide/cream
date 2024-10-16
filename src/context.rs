mod cream_context;
pub mod events_context;

mod helpers {
    #[macro_export]
    macro_rules! pub_provide (($ctx: path : $provider: path { $($service: path),* $(,)? }) => {
        $(
        impl $crate::context::ContextProvide<$service> for $ctx {
            fn ctx_provide(&self) -> $service {
                let ctx = <Self as $crate::context::ContextExtend<$provider>>::provide_ctx(self);
                ctx.ctx_provide()
            }
        }
        )*
    });

    pub use pub_provide;
}

pub use cream_context::CreamContext;
pub use cream_derive::ContextProvide;
pub use helpers::*;

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

pub trait ContextExtend<C: Context> {
    fn provide_ctx(&self) -> &C;
}

mod cream_context;
pub mod events_context;

mod helpers {
    #[macro_export]
    macro_rules! pub_provide (($ctx: path : $provider: path { $($service: path),* $(,)? }) => {
        $(
        impl $crate::context::FromContext<$ctx> for $service {
            fn from_context(_ctx: &$ctx) -> Self {
                let ctx: &$provider = <$ctx as $crate::context::ContextExtend<$provider>>::provide_ctx(_ctx);
                <Self as $crate::context::FromContext<$provider>>::from_context(&ctx)
            }
        }
        )*
    });

    pub use pub_provide;
}

pub use cream_context::CreamContext;
pub use cream_derive::*;
pub use helpers::*;

pub trait FromContext<C> {
    fn from_context(ctx: &C) -> Self;
}

pub trait CreateFromContext<C> {
    type Args;
    fn create_from_context(ctx: &C, args: Self::Args) -> Self;
}

pub trait ContextProvide<S>: Context {
    fn ctx_provide(&self) -> S;
}

impl<C: Context, S> ContextProvide<S> for C
where
    S: FromContext<C>,
{
    fn ctx_provide(&self) -> S {
        S::from_context(self)
    }
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
    fn create<S>(&self, args: S::Args) -> S
    where
        Self: Sized,
        S: CreateFromContext<Self>,
    {
        S::create_from_context(self, args)
    }
}

impl<C> FromContext<C> for () {
    fn from_context(_ctx: &C) {}
}

pub trait ContextExtend<C: Context> {
    fn provide_ctx(&self) -> &C;
}

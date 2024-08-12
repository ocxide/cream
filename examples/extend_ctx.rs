// Just check all compiles
#![allow(unused)]

use cream::context::{ContextProvide, CreamContext};


struct AppCtx {
    cream: CreamContext,
}

trait CtxConfig {}

impl<S> ContextProvide<S> for AppCtx
where
    CreamContext: ContextProvide<S>,
{
    fn provide(&self) -> S {
        self.cream.provide()
    }
}

mod foo {
    use super::AppCtx;
    use cream::{context::ContextProvide, event_bus::EventBusPort};

    #[derive(ContextProvide)]
    #[provider_context(AppCtx)]
    pub struct MyHandler {
        bus: EventBusPort,
    }
}

fn main() {}

// Just check all compiles
#![allow(unused)]

use cream::{
    context::{ContextProvide, CreamContext},
    event_bus::EventBusPort,
};

struct OtherCtx {
    port: EventBusPort,
}

impl ContextProvide<EventBusPort> for OtherCtx {
    fn ctx_provide(&self) -> EventBusPort {
        self.port.clone()
    }
}

struct Ctx {
    cream_ctx: CreamContext,
    other: OtherCtx,
    dep: Dep,
}

impl<S> ContextProvide<S> for Ctx
where
    CreamContext: ContextProvide<S>,
{
    fn ctx_provide(&self) -> S {
        self.cream_ctx.ctx_provide()
    }
}

#[derive(Clone)]
struct Dep;

impl ContextProvide<Dep> for Ctx {
    fn ctx_provide(&self) -> Dep {
        self.dep.clone()
    }
}

fn main() {}

use crate::event_bus_port::EventBusPort;

use super::{ContextExtend, FromContext};

#[derive(Clone)]
pub struct CreamContext {
    event_bus_port: EventBusPort,
}

impl CreamContext {
    pub fn new(event_bus_port: EventBusPort) -> Self {
        Self { event_bus_port }
    }
}

impl FromContext<CreamContext> for EventBusPort {
    fn from_context(ctx: &CreamContext) -> Self {
        ctx.event_bus_port.clone()
    }
}

impl<C, S> FromContext<C> for S
where
    C: ContextExtend<CreamContext>,
    S: FromContext<CreamContext>,
{
    fn from_context(ctx: &C) -> Self {
        let cream_ctx = ctx.provide_context();
        <S as FromContext<CreamContext>>::from_context(cream_ctx)
    }
}

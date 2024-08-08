use crate::event_bus::EventBusPort;

use super::FromContext;

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

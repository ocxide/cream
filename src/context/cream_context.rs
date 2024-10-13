use crate::event_bus::EventBusPort;

use super::{Context, ContextProvide};

#[derive(Clone)]
pub struct CreamContext {
    event_bus_port: EventBusPort,
}

impl CreamContext {
    pub fn new(event_bus_port: EventBusPort) -> Self {
        Self { event_bus_port }
    }
}

impl Context for CreamContext {}

impl ContextProvide<EventBusPort> for CreamContext {
    fn ctx_provide(&self) -> EventBusPort {
        self.event_bus_port.clone()
    }
}

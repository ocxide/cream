use crate::{
    event_bus::{EventBusPort, EventBusSocket},
    events::router::Router,
    router_bus::RouterBus,
    tasks::Tasks,
};

use super::{Context, ContextProvide, CreamContext};

#[derive(Clone)]
pub struct EventsContext {
    port: EventBusPort,
}

impl Context for EventsContext {}

impl ContextProvide<EventBusPort> for EventsContext {
    fn ctx_provide(&self) -> EventBusPort {
        self.port.clone()
    }
}

pub struct EventsContextBuilder {
    channel_size: usize,
}

impl Default for EventsContextBuilder {
    fn default() -> Self {
        Self { channel_size: 10 }
    }
}

impl EventsContextBuilder {
    pub fn with_channel_size(mut self, size: usize) -> Self {
        self.channel_size = size;
        self
    }

    pub fn build(self, cream_ctx: &CreamContext) -> (EventsContext, EventsContextSetup) {
        let tasks: Tasks = cream_ctx.provide();
        let (port, socket) = {
            let tasks = cream_ctx.provide();
            crate::event_bus::create(self.channel_size, tasks)
        };

        let ctx = EventsContext { port };
        let setup = EventsContextSetup { socket, tasks };

        (ctx, setup)
    }
}

pub struct EventsContextSetup {
    socket: EventBusSocket,
    tasks: Tasks,
}

impl EventsContextSetup {
    pub fn setup<C: Send + 'static + Sync>(self, router: Router<C>, ctx: C) {
        let mut bus = RouterBus::new(self.socket, ctx, router, self.tasks);
        tokio::spawn(async move { bus.listen().await });
    }
}


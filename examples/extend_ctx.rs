// Just check all compiles
#![allow(unused)]

use cream::{
    context::{
        events_context::EventsContext, ContextExtend, ContextProvide, CreamContext, FromContext,
    },
    event_bus::EventBusPort,
    pub_provide,
};

#[derive(FromContext)]
#[context(MyCtx)]
struct Dep1;

struct MyCtx {
    events: EventsContext,
    dep1: Dep1,
}

impl ContextExtend<EventsContext> for MyCtx {
    fn provide_ctx(&self) -> &EventsContext {
        &self.events
    }
}

pub_provide!(MyCtx : EventsContext {
    EventBusPort
});

#[derive(FromContext)]
#[context(MyCtx)]
struct Service1 {
    dep1: Dep1,
    bus: EventBusPort,
}

fn main() {
}

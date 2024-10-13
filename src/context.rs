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

mod cream_context {
    use crate::tasks::Tasks;

    use super::{Context, ContextProvide};

    #[derive(Clone)]
    pub struct CreamContext {
        tasks: Tasks,
    }

    impl Default for CreamContext {
        fn default() -> Self {
            Self {
                tasks: Tasks::new(),
            }
        }
    }

    impl Context for CreamContext {}

    impl ContextProvide<Tasks> for CreamContext {
        fn ctx_provide(&self) -> Tasks {
            self.tasks.clone()
        }
    }
}

pub mod events_context {
    use crate::{
        event_bus::EventBusPort, events::router::Router, router_bus::RouterBus, tasks::Tasks,
    };

    use super::{Context, ContextProvide, CreamContext};

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

        pub fn build<C: Send + 'static + Sync>(
            self,
            cream_ctx: &CreamContext,
            router: Router<C>,
            ctx: C,
        ) -> EventsContext {
            let tasks: Tasks = cream_ctx.provide();
            let (port, socket) = {
                let tasks = cream_ctx.provide();
                crate::event_bus::create(self.channel_size, tasks)
            };

            let mut bus = RouterBus::new(socket, ctx, router, tasks);
            tokio::spawn(async move { bus.listen().await });

            EventsContext { port }
        }
    }
}

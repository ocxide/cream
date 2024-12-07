use crate::{event_bus::EventBusSocket, events::router::Router, tasks::Tasks};

pub struct RouterBus<C: 'static> {
    recv: EventBusSocket,
    ctx: C,
    router: Router<C>,
    tasks: Tasks,
}

impl<C: 'static> RouterBus<C> {
    pub fn new(socket: EventBusSocket, ctx: C, router: Router<C>, tasks: Tasks) -> Self {
        RouterBus {
            recv: socket,
            ctx,
            router,
            tasks,
        }
    }
}

impl<C: 'static> RouterBus<C> {
    pub async fn listen_once(&mut self) -> Option<()> {
        let event = self.recv.recv().await?;
        let (name, version) = (event.name(), event.version());
        let Some(fut) = self.router.call(&self.ctx, event) else {
            println!("warning: got unhandable event, {}@{}", name, version);
            return Some(());
        };

        self.tasks.spawn(fut);
        Some(())
    }

    pub async fn listen(&mut self) {
        while self.listen_once().await.is_some() {}
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        context::{events_context::EventsContextBuilder, Context, CreamContext, FromContext},
        event_bus::EventBusPort,
        events::{DomainEvent, Error, Handler},
    };

    use super::*;

    #[tokio::test]
    async fn can_send_events_through_threads() {
        static VAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

        struct Ctx;
        impl Context for Ctx {}

        #[derive(Clone)]
        struct MyEvent;
        impl DomainEvent for MyEvent {
            fn name(&self) -> &'static str {
                "MyEvent"
            }

            fn version(&self) -> &'static str {
                "1.0.0"
            }
        }

        #[derive(FromContext)]
        #[context(Ctx)]
        struct MyHandler;

        impl Handler for MyHandler {
            type Event = MyEvent;
            async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                VAL.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
        }

        let ctx = Ctx;
        let cream_ctx = CreamContext::default();

        let mut router = Router::<Ctx>::default();
        router.add::<MyHandler>();

        let (events_ctx, setup) = EventsContextBuilder::default().build(&cream_ctx);
        setup.setup(router, ctx);

        let tasks: Tasks = cream_ctx.provide();
        let port: EventBusPort = events_ctx.provide();

        port.publish(MyEvent);
        tasks.close();
        tasks.wait().await;

        assert!(VAL.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn can_build_ctx_with_cream() {
        #[allow(dead_code)]
        struct Ctx(crate::context::CreamContext);

        let _ctx = Ctx(crate::context::CreamContext::default());
    }
}

use crate::{
    event_bus::{EventBusPort, EventBusSocket},
    events::router::Router,
};

pub struct RouterBus<C: 'static> {
    recv: EventBusSocket,
    ctx: C,
    router: Router<C>,
}

impl<C: 'static> RouterBus<C> {
    pub fn new(socket: EventBusSocket, ctx: C, router: Router<C>) -> Self {
        RouterBus {
            recv: socket,
            ctx,
            router,
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

        tokio::spawn(fut);
        Some(())
    }

    pub async fn listen(&mut self) {
        while self.listen_once().await.is_some() {}
    }
}

/// Recommended channel config for EventBus
pub fn create_channel() -> (EventBusPort, EventBusSocket) {
    crate::event_bus::create(10)
}

#[cfg(test)]
mod tests {

    use crate::{
        events::DomainEvent,
        events::{Error, Handler},
        context::ContextProvide,
    };

    use super::*;

    #[test]
    fn can_send_events_through_threads() {
        static VAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

        struct Ctx;

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

        #[derive(ContextProvide)]
        #[provider_context(Ctx)]
        struct MyHandler;

        impl Handler for MyHandler {
            type Event = MyEvent;
            async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                VAL.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
        }

        let mut router = Router::default();
        router.add::<MyHandler>();
        let (port, socket) = create_channel();

        let ctx = Ctx;

        let mut bus = RouterBus::new(socket, ctx, router);

        tokio::runtime::Builder::new_multi_thread()
            .build()
            .unwrap()
            .block_on(async move {
                let bus_handle = tokio::spawn(async move {
                    bus.listen_once().await;
                });

                port.publish(MyEvent);
                let _ = bus_handle.await;

                assert!(VAL.load(std::sync::atomic::Ordering::Relaxed));
            });
    }

    #[test]
    fn can_build_ctx_with_cream() {
        #[allow(dead_code)]
        struct Ctx(crate::context::CreamContext);

        let router = Router::default();
        let (port, socket) = create_channel();
        let ctx = Ctx(crate::context::CreamContext::new(port));

        let _ = RouterBus::new(socket, ctx, router);
    }
}

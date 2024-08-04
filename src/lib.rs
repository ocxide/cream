pub mod cream_context {
    use crate::{context::Context, event_bus_port::EventBusPort};

    #[derive(Clone)]
    pub struct CreamContext {
        event_bus_port: EventBusPort,
    }

    impl Context for CreamContext {}

    impl CreamContext {
        pub fn new(event_bus_port: EventBusPort) -> Self {
            Self { event_bus_port }
        }

        pub fn event_bus_port(&self) -> EventBusPort {
            self.event_bus_port.clone()
        }
    }
}

pub mod event_bus {
    use crate::{
        context::Context,
        event_bus_port::{EventBusPort, EventBusSocket},
        event_router::EventRouter,
    };

    pub struct EventBus<C: Context + 'static> {
        recv: EventBusSocket,
        ctx: C,
        router: EventRouter<C>,
    }

    impl<C: Context + 'static> EventBus<C> {
        pub fn new(socket: EventBusSocket, ctx: C, router: EventRouter<C>) -> Self {
            EventBus {
                recv: socket,
                ctx,
                router,
            }
        }
    }

    impl<C: Context + 'static> EventBus<C> {
        pub async fn listen_once(&mut self) -> Option<()> {
            let event = self.recv.recv().await?;
            let (name, version) = (event.name(), event.version());
            let Some(fut) = self.router.handle(&self.ctx, event) else {
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
        crate::event_bus_port::create(10)
    }

    #[cfg(test)]
    mod tests {
        use crate::{
            domain_event::DomainEvent,
            event_handler::{Error, EventHandler},
            from_context::FromContext,
        };

        use super::*;

        #[test]
        fn can_send_events_through_threads() {
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

            struct MyHandler;
            impl EventHandler for MyHandler {
                type Event = MyEvent;
                async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                    VAL.store(true, std::sync::atomic::Ordering::Relaxed);
                    Ok(())
                }
            }

            impl FromContext<Ctx> for MyHandler {
                fn from_context(_: &Ctx) -> Self {
                    Self {}
                }
            }

            let mut router = EventRouter::default();
            router.register::<MyHandler>();
            let (port, socket) = create_channel();

            let ctx = Ctx;

            let mut bus = EventBus::new(socket, ctx, router);

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
            struct Ctx(crate::cream_context::CreamContext);
            impl Context for Ctx {}

            let router = EventRouter::default();
            let (port, socket) = create_channel();
            let ctx = Ctx(crate::cream_context::CreamContext::new(port));

            let _ = EventBus::new(socket, ctx, router);
        }
    }
}

pub mod event_bus_port {
    use crate::domain_event::DomainEvent;

    #[derive(Clone)]
    pub struct EventBusPort(tokio::sync::mpsc::Sender<Box<dyn DomainEvent>>);

    impl EventBusPort {
        pub(crate) fn new(tx: tokio::sync::mpsc::Sender<Box<dyn DomainEvent>>) -> Self {
            Self(tx)
        }

        pub fn publish(&self, event: impl DomainEvent + 'static) {
            let event = Box::new(event);

            let Err(err) = self.0.try_send(event) else {
                return;
            };

            match err {
                tokio::sync::mpsc::error::TrySendError::Full(_) => "Channel is full",
                tokio::sync::mpsc::error::TrySendError::Closed(_) => "Channel is closed",
            };

            panic!("Failed to send event: {}", err);
        }
    }

    pub struct EventBusSocket(tokio::sync::mpsc::Receiver<Box<dyn DomainEvent>>);

    impl EventBusSocket {
        pub async fn recv(&mut self) -> Option<Box<dyn DomainEvent>> {
            self.0.recv().await
        }
    }

    pub(crate) fn create(size: usize) -> (EventBusPort, EventBusSocket) {
        let (tx, rx) = tokio::sync::mpsc::channel(size);
        (EventBusPort::new(tx), EventBusSocket(rx))
    }
}

pub mod context {
    pub trait Context {}
}

pub mod domain_event {
    use std::any::Any;

    // This trait is for internal use only
    #[allow(private_bounds)]
    pub trait DomainEvent: DynEvent + 'static + Send + Sync {
        fn name(&self) -> &'static str;
        fn version(&self) -> &'static str;
    }

    pub(crate) trait DynEvent {
        fn as_any(&self) -> &dyn Any;
    }

    impl<E: 'static> DynEvent for E {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }
}

pub mod event_handler {
    use std::future::Future;

    use crate::domain_event::DomainEvent;

    pub trait EventHandler: Send {
        type Event: DomainEvent + Sized + Send + 'static + Clone;
        fn handle(&self, event: Self::Event) -> impl Future<Output = Result<(), Error>> + Send;
    }

    pub enum Error {}
}

pub mod from_context {
    pub trait FromContext<C> {
        fn from_context(context: &C) -> Self;
    }
}

pub mod event_router;

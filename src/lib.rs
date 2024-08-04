pub mod cream_context {
    use crate::{context::Context, event_bus_port::EventBusPort};

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
    use crate::{context::Context, domain_event::DomainEvent, event_bus_port::EventBusPort, event_router::EventRouter};

    pub struct EventBus(tokio::sync::mpsc::Receiver<Box<dyn DomainEvent>>);

    impl EventBus {
        pub async fn listen_app<C: Context + 'static>(&mut self, ctx: &C, router: EventRouter<C>) {
            while let Some(event) = self.0.recv().await {
                let (name, version) = (event.name(), event.version());
                let Some(fut) = router.handle(ctx, event) else {
                    println!("warning: got unhandable event, {}@{}", name, version);
                    continue;
                };

                tokio::spawn(fut);
            }
        }
    }

    pub fn create() -> (EventBus, EventBusPort) {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        (EventBus(rx), EventBusPort::new(tx))
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
}

pub mod context {
    pub trait Context {}
}

pub mod domain_event {
    use std::any::Any;

    // This trait is for internal use only
    #[allow(private_bounds)]
    pub trait DomainEvent : DynEvent {
        fn name(&self) -> &'static str;
        fn version(&self) -> &'static str;
    }

    pub(crate) trait DynEvent {
        fn as_any(&self) -> &dyn Any;
    }

    impl<E: 'static>  DynEvent for E {
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
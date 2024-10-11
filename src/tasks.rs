use std::future::Future;

use tokio_util::task::TaskTracker;

#[derive(Default, Clone)]
pub struct Tasks(TaskTracker);

impl Tasks {
    pub fn new() -> Self {
        Self(TaskTracker::new())
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.spawn(f);
    }

    pub async fn wait(&self) {
        self.0.wait().await;
    }

    pub fn close(&self) {
        self.0.close();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicU8, Arc};

    use crate::{
        context::ContextProvide,
        event_bus::{self, EventBusPort},
        events::{router, DomainEvent, Error, Handler},
        router_bus::{self, create_channel},
    };

    use super::*;

    #[tokio::test]
    async fn test_spawn() {
        let ran = Arc::new(AtomicU8::new(0));

        let tasks = Tasks::new();
        tasks.spawn({
            let ran = ran.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                ran.store(2, std::sync::atomic::Ordering::Relaxed);
            }
        });

        ran.store(1, std::sync::atomic::Ordering::Relaxed);

        tasks.close();
        tasks.wait().await;

        assert_eq!(ran.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn works_with_events_router() {
        use crate::context::CreamContext;

        #[derive(Clone)]
        struct MyCtx {
            cream: CreamContext,
            ran: Arc<AtomicU8>,
        }

        impl ContextProvide<EventBusPort> for MyCtx {
            fn provide(&self) -> EventBusPort {
                self.cream.provide()
            }
        }

        impl ContextProvide<Arc<AtomicU8>> for MyCtx {
            fn provide(&self) -> Arc<AtomicU8> {
                self.ran.clone()
            }
        }

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

        struct MyHandler {
            ran: Arc<AtomicU8>,
        }

        impl ContextProvide<MyHandler> for MyCtx {
            fn provide(&self) -> MyHandler {
                println!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA;A");
                MyHandler {
                    ran: self.ran.clone(),
                }
            }
        }

        impl Handler for MyHandler {
            type Event = MyEvent;
            async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                self.ran.store(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
        }

        let (port, socket) = create_channel();

        let ctx = MyCtx {
            cream: CreamContext::new(port.clone()),
            ran: Arc::new(AtomicU8::new(0)),
        };

        let mut router = router::Router::<MyCtx>::default();
        router.add::<MyHandler>();

        let mut router_bus = router_bus::RouterBus::new(socket, ctx.clone(), router);
        let tasks = Tasks::new();

        tokio::spawn({
            let tasks = tasks.clone();
            async move { router_bus.listen(tasks).await }
        });

        port.publish(MyEvent);

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        tasks.close();
        tasks.wait().await;

        assert_eq!(ctx.ran.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}

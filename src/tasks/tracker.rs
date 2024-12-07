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
        // By wating twice, we ensure that the tasks are completed
        // I think a single wait should be enough, but just works if there are two wait
        self.0.wait().await;
        self.0.wait().await;
    }

    pub fn close(&self) {
        self.0.close();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, AtomicU8},
        Arc,
    };

    use crate::{
        context::{events_context::EventsContextBuilder, Context, FromContext},
        event_bus::EventBusPort,
        events::{router, DomainEvent, Error, Handler},
        tasks::shutdown::Shutdown,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn works_with_events_router() {
        use crate::context::CreamContext;

        #[derive(Clone)]
        struct MyCtx {
            ran: Arc<AtomicBool>,
            created: Arc<AtomicBool>,
        }

        impl Context for MyCtx {}

        impl FromContext<MyCtx> for Arc<AtomicBool> {
            fn from_context(ctx: &MyCtx) -> Arc<AtomicBool> {
                ctx.ran.clone()
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
            ran: Arc<AtomicBool>,
        }

        impl FromContext<MyCtx> for MyHandler {
            fn from_context(ctx: &MyCtx) -> Self {
                ctx.created
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                Self { ran: ctx.provide() }
            }
        }

        impl Handler for MyHandler {
            type Event = MyEvent;
            async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                self.ran.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
        }

        let ctx = MyCtx {
            ran: Arc::new(AtomicBool::new(false)),
            created: Arc::new(AtomicBool::new(false)),
        };

        let mut router = router::Router::<MyCtx>::default();
        router.add::<MyHandler>();

        let cream_ctx = CreamContext::default();
        let (events_ctx, setup) = EventsContextBuilder::default().build(&cream_ctx);

        setup.setup(router, ctx.clone());

        let tasks: Tasks = cream_ctx.provide();
        let port: EventBusPort = events_ctx.provide();

        port.publish(MyEvent);

        let shutdown: Shutdown = cream_ctx.provide();
        shutdown.run().await;

        assert_eq!(tasks.0.len(), 0, "there should be no tasks left");

        assert!(
            ctx.created.load(std::sync::atomic::Ordering::Relaxed),
            "handler should have been created"
        );
        assert!(
            ctx.ran.load(std::sync::atomic::Ordering::Relaxed),
            "handler should have run"
        );
    }
}

/// config for providing repositories, EventBusPort, etc.
pub mod context;
/// ports & sockets for emitting & recieving events
pub mod event_bus;
/// define events, handlers & config the router
pub mod events;
/// listen for events and dispatch to handlers
pub mod router_bus;

pub mod tasks {
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
}

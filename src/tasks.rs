mod tracker;
mod shutdown {
    use super::Tasks;
    use crate::context::{ContextProvide, CreamContext};

    #[derive(ContextProvide)]
    #[provider_context(CreamContext)]
    pub struct Shutdown {
        tasks: Tasks,
    }

    impl Shutdown {
        pub async fn run(self) {
            // Allow other threads to run
            // TODO: Find a better way to do this
            tokio::time::sleep(std::time::Duration::ZERO).await;

            self.tasks.close();
            self.tasks.wait().await;
        }
    }
}

pub use tracker::*;

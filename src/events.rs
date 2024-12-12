pub use cream_events_core::DomainEvent;

use std::future::Future;

pub enum Error {}

pub trait Handler: Send {
    type Event: DomainEvent + Sized + Send + 'static + Clone;
    fn handle(&self, event: Self::Event) -> impl Future<Output = Result<(), Error>> + Send;
}

pub mod router;


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

impl<E: DomainEvent> DynEvent for E {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

use std::future::Future;

pub enum Error {}

pub trait Handler: Send {
    type Event: DomainEvent + Sized + Send + 'static + Clone;
    fn handle(&self, event: Self::Event) -> impl Future<Output = Result<(), Error>> + Send;
}

pub mod router;


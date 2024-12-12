use std::any::Any;

pub trait DomainEvent: DynEvent + 'static + Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
}

// This trait is for internal use only
pub trait DynEvent {
    fn as_any(&self) -> &dyn Any;
}

impl<E: DomainEvent> DynEvent for E {
    fn as_any(&self) -> &dyn Any {
        self
    }
}


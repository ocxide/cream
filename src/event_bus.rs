use crate::events::DomainEvent;

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


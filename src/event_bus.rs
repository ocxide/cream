use crate::{events::DomainEvent, tasks::Tasks};

#[derive(Clone)]
pub struct EventBusPort {
    tx: tokio::sync::mpsc::Sender<Box<dyn DomainEvent>>,
    tasks: Tasks,
}

impl EventBusPort {
    pub fn publish(&self, event: impl DomainEvent + 'static) {
        let event = Box::new(event);

        let tx = self.tx.clone();
        self.tasks.spawn(async move {
            let Err(e) = tx.send(event).await else {
                return;
            };

            eprintln!("Failed to send event: {}", e);
        });
    }
}

pub struct EventBusSocket(tokio::sync::mpsc::Receiver<Box<dyn DomainEvent>>);

impl EventBusSocket {
    pub async fn recv(&mut self) -> Option<Box<dyn DomainEvent>> {
        self.0.recv().await
    }
}

pub(crate) fn create(size: usize, tasks: Tasks) -> (EventBusPort, EventBusSocket) {
    let (tx, rx) = tokio::sync::mpsc::channel(size);
    (EventBusPort { tasks, tx }, EventBusSocket(rx))
}

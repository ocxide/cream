use std::{any::TypeId, collections::HashMap, future::Future, pin::Pin};

use tokio::task::JoinSet;

use crate::{
    context::ContextProvide,
    events::{DomainEvent, Handler},
};

trait Handlers<C>: AsAnyC<C> + Send {
    fn call(&self, ctx: &C, event: Box<dyn DomainEvent>) -> JoinSet<()>;
}

trait AsAnyC<C> {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<C, H> AsAnyC<C> for H
where
    H: Handlers<C> + 'static + Sized,
{
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

type Caller<C, E> = fn(&C, E) -> Pin<Box<dyn Future<Output = ()> + Send>>;
struct EventHandlers<C, E>(Vec<Caller<C, E>>);

impl<C: 'static, E: DomainEvent + Clone> Handlers<C> for EventHandlers<C, E> {
    fn call(&self, ctx: &C, event: Box<dyn DomainEvent>) -> JoinSet<()> {
        let event = event
            .as_any()
            .downcast_ref::<E>()
            .expect("Invalid event type");

        self.0
            .iter()
            .map(|handler| (handler)(ctx, event.clone()))
            .collect()
    }
}

impl<C, E: DomainEvent> EventHandlers<C, E> {
    fn add<H>(&mut self)
    where
        H: Handler<Event = E> + Send + 'static,
        C: ContextProvide<H>,
    {
        let caller: Caller<C, H::Event> = |ctx, event| {
            let handler = ctx.ctx_provide();
            Box::pin(async move {
                let _ = handler.handle(event).await;
            })
        };

        self.0.push(caller);
    }
}

impl<C, E> Default for EventHandlers<C, E> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct Router<C>(HashMap<TypeId, Box<dyn Handlers<C>>>);

impl<C> Default for Router<C> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<C: 'static> Router<C> {
    pub fn call(&self, ctx: &C, event: Box<dyn DomainEvent>) -> Option<impl Future<Output = ()>> {
        let id = event.as_any().type_id();

        let handlers = self.0.get(&id)?;
        let mut join = handlers.call(ctx, event);
        Some(async move { while join.join_next().await.is_some() {} })
    }

    pub fn add<H>(&mut self)
    where
        H: Handler + 'static,
        C: ContextProvide<H>,
    {
        let id = TypeId::of::<H::Event>();
        match self.0.get_mut(&id) {
            None => {
                let mut handlers = EventHandlers::<C, H::Event>::default();
                handlers.add::<H>();
                self.0.insert(id, Box::new(handlers));
            }

            Some(handlers) => {
                handlers
                    .as_any_mut()
                    .downcast_mut::<EventHandlers<C, H::Event>>()
                    .expect("Invalid handler type")
                    .add::<H>();
            }
        };
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Mutex;

    use crate::{
        context::FromContext,
        events::{DomainEvent, Error, Handler},
    };

    #[test]
    fn router_calls() {
        struct MockContext {
            val: Mutex<bool>,
        }

        impl MockContext {
            fn my_service(&self) {
                *self.val.lock().unwrap() = true;
            }
        }

        #[derive(Clone)]
        struct TestEvent;

        impl DomainEvent for TestEvent {
            fn name(&self) -> &'static str {
                "TestEvent"
            }

            fn version(&self) -> &'static str {
                "1.0.0"
            }
        }

        struct TestHandler;
        impl FromContext<MockContext> for TestHandler {
            fn from_context(ctx: &MockContext) -> Self {
                ctx.my_service();
                TestHandler
            }
        }

        impl Handler for TestHandler {
            type Event = TestEvent;
            async fn handle(&self, _event: Self::Event) -> Result<(), Error> {
                println!("handle");
                Ok(())
            }
        }

        let mut router = super::Router::<MockContext>::default();
        router.add::<TestHandler>();

        let context = MockContext {
            val: Mutex::new(false),
        };

        let val = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async move {
                router.call(&context, Box::new(TestEvent)).unwrap().await;
                *context.val.lock().unwrap()
            });

        assert!(val);
    }
}

use std::{any::TypeId, collections::HashMap, future::Future, pin::Pin};

use tokio::task::JoinSet;

use crate::{domain_event::DomainEvent, event_handler::EventHandler, from_context::FromContext};

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
        H: EventHandler<Event = E> + FromContext<C> + Send + 'static,
    {
        let caller: Caller<C, H::Event> = |ctx, event| {
            let handler = H::from_context(ctx);
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

pub struct EventRouter<C>(HashMap<TypeId, Box<dyn Handlers<C>>>);

impl<C> Default for EventRouter<C> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<C: 'static> EventRouter<C> {
    pub fn handle(&self, ctx: &C, event: Box<dyn DomainEvent>) -> Option<impl Future<Output = ()>> {
        let id = event.as_any().type_id();

        let handlers = self.0.get(&id)?;
        let mut join = handlers.call(ctx, event);
        Some(async move { while join.join_next().await.is_some() {} })
    }

    pub fn register<H>(&mut self)
    where
        H: EventHandler + FromContext<C> + Send + 'static,
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
    use std::sync::{Arc, Mutex};

    use crate::{
        context::Context,
        domain_event::DomainEvent,
        event_handler::{Error, EventHandler},
        from_context::FromContext,
    };

    #[test]
    fn router_calls() {
        struct MockContext {
            val: Mutex<bool>,
        }
        impl Context for MockContext {}

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
                Self {}
            }
        }

        impl EventHandler for TestHandler {
            type Event = TestEvent;
            async fn handle(&self, _event: Self::Event) -> Result<(), Error> {
                println!("handle");
                Ok(())
            }
        }

        let mut router = super::EventRouter::<MockContext>::default();
        router.register::<TestHandler>();

        let context = MockContext {
            val: Mutex::new(false),
        };

        let val = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async move {
                router.handle(&context, Box::new(TestEvent)).unwrap().await;
                *context.val.lock().unwrap()
            });

        assert!(val);
    }

    #[test]
    fn multiple_contexts() {
        type TestPoint = Arc<Mutex<usize>>;
        trait TestContext: Context {
            fn specific_impl(&self) -> usize;
            fn provide_data(&self) -> Arc<Mutex<usize>>;
        }
        trait TestFromContext {
            fn from_context(ctx: &impl TestContext) -> Self;
        }

        impl<C, S> FromContext<C> for S
        where
            C: TestContext,
            S: TestFromContext,
        {
            fn from_context(ctx: &C) -> Self {
                <Self as TestFromContext>::from_context(ctx)
            }
        }

        struct ContextOne(TestPoint);
        struct ContextTwo(TestPoint);

        impl TestContext for ContextOne {
            fn specific_impl(&self) -> usize {
                1
            }

            fn provide_data(&self) -> TestPoint {
                self.0.clone()
            }
        }

        impl Context for ContextOne {}

        impl TestContext for ContextTwo {
            fn specific_impl(&self) -> usize {
                2
            }

            fn provide_data(&self) -> TestPoint {
                self.0.clone()
            }
        }

        impl Context for ContextTwo {}

        #[derive(Clone)]
        struct MyEvent;
        impl DomainEvent for MyEvent {
            fn name(&self) -> &'static str {
                "foo.bar.bes"
            }

            fn version(&self) -> &'static str {
                "1.0.0"
            }
        }

        struct MyHandler {
            point: TestPoint,
            n: usize,
        }

        impl EventHandler for MyHandler {
            type Event = MyEvent;
            async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                *self.point.lock().unwrap() = self.n;
                Ok(())
            }
        }

        impl TestFromContext for MyHandler {
            fn from_context(ctx: &impl TestContext) -> Self {
                Self {
                    point: ctx.provide_data(),
                    n: ctx.specific_impl(),
                }
            }
        }

        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                let data = Arc::new(Mutex::new(0));
                {
                    let context_one = ContextOne(data.clone());

                    let mut router = super::EventRouter::<ContextOne>::default();
                    router.register::<MyHandler>();

                    router
                        .handle(&context_one, Box::new(MyEvent))
                        .unwrap()
                        .await;

                    assert_eq!(*data.lock().unwrap(), 1);
                }

                {
                    let context_two = ContextTwo(data.clone());
                    let mut router = super::EventRouter::<ContextTwo>::default();
                    router.register::<MyHandler>();
                    router
                        .handle(&context_two, Box::new(MyEvent))
                        .unwrap()
                        .await;
                    assert_eq!(*data.lock().unwrap(), 2);
                }
            });
    }
}

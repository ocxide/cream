use std::{any::TypeId, collections::HashMap, future::Future, pin::Pin};

use tokio::task::JoinSet;

use crate::{context::FromContext, events::DomainEvent, events::Handler};

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
        H: Handler<Event = E> + FromContext<C> + Send + 'static,
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
        H: Handler + FromContext<C> + 'static,
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
        context::FromContext,
        events::DomainEvent,
        events::{Error, Handler},
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
                Self {}
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

    #[test]
    fn multiple_contexts() {
        type TestPoint = Arc<Mutex<usize>>;
        trait TestContext {
            fn specific_impl(&self) -> usize;
            fn provide_data(&self) -> Arc<Mutex<usize>>;
        }

        struct ContextOne(TestPoint);
        struct ContextTwo(TestPoint);

        impl<C: TestContext> FromContext<C> for TestPoint {
            fn from_context(ctx: &C) -> Self {
                ctx.provide_data()
            }
        }

        impl<C: TestContext> FromContext<C> for usize {
            fn from_context(ctx: &C) -> Self {
                ctx.specific_impl()
            }
        }

        impl TestContext for ContextOne {
            fn specific_impl(&self) -> usize {
                1
            }

            fn provide_data(&self) -> TestPoint {
                self.0.clone()
            }
        }

        impl TestContext for ContextTwo {
            fn specific_impl(&self) -> usize {
                2
            }

            fn provide_data(&self) -> TestPoint {
                self.0.clone()
            }
        }

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

        #[derive(FromContext)]
        #[from_context(C: TestContext)]
        struct MyHandler {
            point: TestPoint,
            n: usize,
        }

        impl Handler for MyHandler {
            type Event = MyEvent;
            async fn handle(&self, _: Self::Event) -> Result<(), Error> {
                *self.point.lock().unwrap() = self.n;
                Ok(())
            }
        }

        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                let data = Arc::new(Mutex::new(0));
                {
                    let context_one = ContextOne(data.clone());

                    let mut router = super::Router::<ContextOne>::default();
                    router.add::<MyHandler>();

                    router.call(&context_one, Box::new(MyEvent)).unwrap().await;

                    assert_eq!(*data.lock().unwrap(), 1);
                }

                {
                    let context_two = ContextTwo(data.clone());
                    let mut router = super::Router::<ContextTwo>::default();
                    router.add::<MyHandler>();
                    router.call(&context_two, Box::new(MyEvent)).unwrap().await;
                    assert_eq!(*data.lock().unwrap(), 2);
                }
            });
    }
}

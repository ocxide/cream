use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    pin::Pin,
};

use tokio::task::JoinSet;

use crate::{
    context::Context, domain_event::DomainEvent, event_handler::EventHandler,
    from_context::FromContext,
};

type EventHandlerFut = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
type Runner<C, Event> = fn(&C, Event) -> EventHandlerFut;
type DynEventHandler<C> = fn(&C, Box<dyn DomainEvent>, &Runners) -> JoinSet<()>;

#[derive(Default)]
struct Runners(Vec<Box<dyn Any>>);

impl Runners {
    fn iter<C: 'static, E: 'static>(&self) -> impl Iterator<Item = &Runner<C, E>> {
        self.0.iter().map(|a| {
            a.as_ref()
                .downcast_ref::<Runner<C, E>>()
                .expect("downcast runner")
        })
    }

    fn add<H: EventHandler + FromContext<C> + 'static, C: Context + 'static>(&mut self) {
        let runner: Runner<C, H::Event> = |ctx, event| {
            let handler = H::from_context(ctx);
            Box::pin(async move {
                let _ = handler.handle(event).await;
            })
        };

        self.0.push(Box::new(runner));
    }
}

pub struct EventRouter<C: Context> {
    routes: HashMap<TypeId, (DynEventHandler<C>, Runners)>,
}

impl<C: Context> Default for EventRouter<C> {
    fn default() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }
}

impl<C: Context + Sized + 'static> EventRouter<C> {
    pub fn handle(&self, ctx: &C, event: Box<dyn DomainEvent>) -> Option<impl Future<Output = ()>> {
        let event_type = event.as_ref().as_any().type_id();
        let (dyn_handler, runners) = self.routes.get(&event_type)?;

        let mut handle = (dyn_handler)(ctx, event, runners);
        Some(async move { while (handle.join_next().await).is_some() {} })
    }

    pub fn register<H: EventHandler + FromContext<C> + 'static>(&mut self) {
        let entry = self
            .routes
            .entry(TypeId::of::<H::Event>())
            .or_insert_with(|| {
                let dyn_event_handler: DynEventHandler<C> = |ctx, event, runners| {
                    let event = event.as_ref().as_any().downcast_ref::<H::Event>().unwrap();

                    runners
                        .iter::<C, H::Event>()
                        .map(|runner| (runner)(ctx, event.clone()))
                        .collect()
                };

                (dyn_event_handler, Runners::default())
            });

        entry.1.add::<H, C>();
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

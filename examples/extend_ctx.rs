// Just check all compiles
#![allow(unused)]

use cream::{
    context::{ContextProvide, CreamContext, FromContext},
    event_bus::EventBusPort,
};

trait Ctx: ContextProvide<Dep> + ContextProvide<Dep2> {}

struct MyCtx {
    dep: Dep,
}

impl Ctx for MyCtx {}
impl FromContext<MyCtx> for Dep {
    fn from_context(ctx: &MyCtx) -> Self {
        ctx.dep.clone()
    }
}

impl FromContext<MyCtx> for Dep2 {
    fn from_context(ctx: &MyCtx) -> Self {
        Dep2
    }
}

#[derive(Clone)]
struct Dep;

#[derive(Clone)]
struct Dep2;

#[derive(Clone, FromContext)]
#[context(MyCtx)]
struct Service1 {
    dep: Dep,
    dep2: Dep2,
}

#[derive(Clone, FromContext)]
#[context(MyCtx)]
struct Service2 {
    dep: Dep,
    service1: Service1,
}

fn main() {
    let a = Service2::from_context(&MyCtx { dep: Dep });
}

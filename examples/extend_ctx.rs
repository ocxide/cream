// Just check all compiles
#![allow(unused)]

use cream::{
    context::{ContextProvide, ContextProvide2, CreamContext, FromContext2},
    event_bus::EventBusPort,
};

trait Ctx: ContextProvide2<Dep> + ContextProvide2<Dep2> {}

struct MyCtx {
    dep: Dep,
}

impl Ctx for MyCtx {}
impl FromContext2<MyCtx> for Dep {
    fn from_ctx(ctx: &MyCtx) -> Self {
        ctx.dep.clone()
    }
}

impl FromContext2<MyCtx> for Dep2 {
    fn from_ctx(ctx: &MyCtx) -> Self {
        Dep2
    }
}

#[derive(Clone)]
struct Dep;

#[derive(Clone)]
struct Dep2;

#[derive(Clone)]
struct Service1 {
    dep: Dep,
    dep2: Dep2,
}

impl<C: Ctx> FromContext2<C> for Service1 {
    fn from_ctx(ctx: &C) -> Self {
        Self {
            dep: ctx.ctx_provide(),
            dep2: ctx.ctx_provide(),
        }
    }
}

#[derive(Clone)]
struct Service2 {
    dep: Dep,
    service1: Service1,
}

impl<C: Ctx> FromContext2<C> for Service2 {
    fn from_ctx(ctx: &C) -> Self {
        Self {
            dep: ctx.ctx_provide(),
            service1: ctx.ctx_provide(),
        }
    }
}

fn main() {
    let a = Service2::from_ctx(&MyCtx { dep: Dep });
}

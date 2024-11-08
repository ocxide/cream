// Just check all compiles
#[allow(unused, dead_code)]
use cream::context::{Context, ContextCreate, FromContext};

struct Ctx;

impl Context for Ctx {}

impl FromContext<Ctx> for Dep {
    fn from_context(_: &Ctx) -> Self {
        Dep
    }
}

struct Dep;

#[derive(FromContext)]
#[context(Ctx)]
struct Deps {
    dep: Dep,
}

struct ArgdService1 {
    name: String,
    deps: Deps,
}

impl ContextCreate<ArgdService1> for Ctx {
    type Args = String;
    type Deps = Deps;
    fn ctx_create(&self, name: Self::Args, deps: Self::Deps) -> ArgdService1 {
        ArgdService1 { name, deps }
    }
}

struct ArgdService2 {
    nval: u32,
    deps: Deps,
}

impl ContextCreate<ArgdService2> for Ctx {
    type Args = u32;
    type Deps = Deps;
    fn ctx_create(&self, nval: Self::Args, deps: Self::Deps) -> ArgdService2 {
        ArgdService2 { nval, deps }
    }
}

struct ArgdService3 {
    deps: Deps,
}

impl ContextCreate<ArgdService3> for Ctx {
    type Args = ();
    type Deps = Deps;
    fn ctx_create(&self, _: Self::Args, deps: Self::Deps) -> ArgdService3 {
        ArgdService3 { deps }
    }
}

fn main() {
    let ctx = Ctx;
    let service1: ArgdService1 = ctx.create("name".to_string());
    let service2: ArgdService2 = ctx.create(42);
    let service3: ArgdService3 = ctx.create(());

    assert_eq!(service1.name, "name");
    assert_eq!(service2.nval, 42);
}

use cream::context::CreateFromContext;
// Just check all compiles
#[allow(unused, dead_code)]
use cream::context::{Context, FromContext};

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

impl CreateFromContext<Ctx> for ArgdService1 {
    type Args = String;
    fn create_from_context(ctx: &Ctx, name: Self::Args) -> Self {
        ArgdService1 {
            name,
            deps: ctx.provide(),
        }
    }
}

struct ArgdService2 {
    nval: u32,
    deps: Deps,
}

impl CreateFromContext<Ctx> for ArgdService2 {
    type Args = u32;
    fn create_from_context(ctx: &Ctx, nval: Self::Args) -> Self {
        ArgdService2 {
            nval,
            deps: ctx.provide(),
        }
    }
}

struct ArgdService3 {
    deps: Deps,
}

impl CreateFromContext<Ctx> for ArgdService3 {
    type Args = ();
    fn create_from_context(ctx: &Ctx, _: Self::Args) -> Self {
        ArgdService3 {
            deps: ctx.provide(),
        }
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

use crate::tasks::Tasks;

use super::{Context, FromContext};

#[derive(Clone)]
pub struct CreamContext {
    tasks: Tasks,
}

impl Default for CreamContext {
    fn default() -> Self {
        Self {
            tasks: Tasks::new(),
        }
    }
}

impl Context for CreamContext {}

impl FromContext<CreamContext> for Tasks {
    fn from_context(ctx: &CreamContext) -> Self {
        ctx.tasks.clone()
    }
}

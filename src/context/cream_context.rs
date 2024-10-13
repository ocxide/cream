use crate::tasks::Tasks;

use super::{Context, ContextProvide};

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

impl ContextProvide<Tasks> for CreamContext {
    fn ctx_provide(&self) -> Tasks {
        self.tasks.clone()
    }
}


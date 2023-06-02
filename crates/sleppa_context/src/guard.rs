//! Context guard module.

use crate::context::Context;

use std::marker::PhantomData;

/// Context guard data structure.
///
/// A guard aims at properly synchronizing the [Context] when switching from one thread to another
/// one. When dropped, this context guard resets the current context to its prior state.
/// A [Context] is associated (or bound) to a caller's current execution unit using the
/// [Context::bind] method.
#[allow(missing_debug_implementations)]
pub struct ContextGuard {
    pub previous_context: Option<Context>,

    // ensure this type is !Send as it relies on thread locals
    pub _marker: PhantomData<*const ()>,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        if let Some(previous_context) = self.previous_context.take() {
            let _ = crate::context::CURRENT_CONTEXT
                .try_with(|current| current.replace(previous_context));
        }
    }
}

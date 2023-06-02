use crate::guard::ContextGuard;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

thread_local! {
    pub static CURRENT_CONTEXT: RefCell<Context> = RefCell::new(Context::default());
    pub static DEFAULT_CONTEXT: Context = Context::default();
}

/// Execution context structure.
///
/// An execution context is an immutable data structure that contains a bunch of properties. It is
/// a thread-safe propagation mechanism used for sharing values (or properties) between logically
/// associated execution units.
/// When writing data to an execution [Context], the latter is cloned and the new property is
/// appended to it (i.e. a kind of copy-on-write pattern).
#[derive(Clone, Default)]
pub struct Context {
    properties: HashMap<TypeId, Arc<dyn Any + Sync + Send>, BuildHasherDefault<TypeIdHasher>>,
}

/// Executes a closure with a reference to this thread's current context.
///
/// Note: This function will panic if you attempt to attach another context
/// while the context is still borrowed.
fn get_current_context<F: FnMut(&Context) -> T, T>(mut f: F) -> T {
    CURRENT_CONTEXT
        .try_with(|context| f(&context.borrow()))
        .unwrap_or_else(|_| DEFAULT_CONTEXT.with(|cx| f(cx)))
}

impl Context {
    /// Creates an empty context.
    ///
    /// For building a new context populated with seminal properties, the [`with_property`] method
    /// should be used instead.
    pub fn new() -> Self {
        Context::default()
    }

    /// Returns an immutable clone of the current thread's context.
    ///
    /// # Examples
    ///
    /// ```
    /// use sleppa_context::Context;
    ///
    /// static REPOSITORY_URL: &str = "https://github.com/SofairOfficial/largo";
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct RepositoryUrl(&'static str);
    ///
    /// // Access current context and assert the repository URL property value
    /// fn access_current_context() {
    ///     assert_eq!(
    ///         Context::current().get(),
    ///         Some(&RepositoryUrl(REPOSITORY_URL))
    ///     );
    /// }
    ///
    /// // Create a new context with the repository URL property set
    /// let new_context = Context::new()
    ///     .with_property(RepositoryUrl(REPOSITORY_URL));
    ///
    /// // attach the new context to the current thread
    /// let _guard = new_context.bind();
    ///
    /// // Do some work on the context
    /// access_current_context()
    /// ```
    pub fn current() -> Self {
        get_current_context(|context| context.clone())
    }

    /// Binds the context to the current thread.
    ///
    /// Concretely, the current context on the a thread is replaced by this new thread. When later
    /// switching back to the previous context, by dropping the context guard, the current context
    /// is reset to its previous value. A [ContextGuard] do not need to be explicitely dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sleppa_context::Context;
    ///
    /// static USER: &str = "admin";
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct UserProperty(&'static str);
    ///
    /// // Create a new context with the user property set
    /// let new_context = Context::new().with_property(UserProperty(USER));
    ///
    /// // Bind the new context to the current thread
    /// let _guard = new_context.bind();
    ///
    /// // Get the user property from the current context
    /// assert_eq!(
    ///     Context::current().get::<UserProperty>(),
    ///     Some(&UserProperty(USER))
    /// );
    ///
    /// // Drop the context guard so that to restore the previous context
    /// drop(_guard);
    ///
    /// // Get the user property from the restored context
    /// assert_eq!(
    ///     Context::current().get::<UserProperty>(),
    ///     None
    /// );
    /// ```
    pub fn bind(self) -> ContextGuard {
        let previous_context = CURRENT_CONTEXT
            .try_with(|current| current.replace(self))
            .ok();

        ContextGuard {
            previous_context,
            _marker: PhantomData,
        }
    }

    /// Returns a copy of the context with the given property appended to it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sleppa_context::Context;
    /// ```
    pub fn with_property<T: 'static + Send + Sync>(&self, property: T) -> Self {
        let mut context = self.clone();

        context
            .properties
            .insert(TypeId::of::<T>(), Arc::new(property));

        context
    }

    /// Returns a reference to the property with corresponding type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sleppa_context::Context;
    ///
    /// static REPOSITORY_URL: &str = "https://github.com/SofairOfficial/largo";
    ///
    /// // Repository URL property
    /// #[derive(Debug, PartialEq)]
    /// struct RepositoryUrlProperty(&'static str);
    ///
    /// // Repository user property
    /// #[derive(Debug, PartialEq)]
    /// struct RepositoryUserProperty(&'static str);
    ///
    /// // Create a new context with only the repository url property set
    /// let context = Context::new()
    ///     .with_property(RepositoryUrlProperty(REPOSITORY_URL));
    ///
    /// // Get the repository url property from the current context
    /// assert_eq!(
    ///     context.get::<RepositoryUrlProperty>(),
    ///     Some(&RepositoryUrlProperty(REPOSITORY_URL)));
    ///
    /// // Try to get unset repository user property from the current context
    /// assert_eq!(
    ///     context.get::<RepositoryUserProperty>(),
    ///     None);
    /// ```
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.properties
            .get(&TypeId::of::<T>())
            .and_then(|rc| rc.downcast_ref())
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("properties", &self.properties.len())
            .finish()
    }
}

/// Type identifier hasher.
///
/// The nice thing when using `TypeId` as the hash map key is that it is already hashed by the Rust
/// compiler. Consequently, there's no need for complex bit fiddling or so to build a unique hashed
/// key and prevent collisions.
#[derive(Clone, Default, Debug)]
struct TypeIdHasher(u64);

impl Hasher for TypeIdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_context_with_an_unset_property() {
        static REPOSITORY_URL: &str = "https://github.com/SofairOfficial/largo";

        // Define repository URL property
        #[derive(Debug, PartialEq)]
        struct RepositoryUrl(&'static str);

        // Define repository user property
        #[derive(Debug, PartialEq)]
        struct RepositoryUser(&'static str);

        // Create a new context with only the repository URL property set
        let context = Context::new().with_property(RepositoryUrl(REPOSITORY_URL));

        // Query repository URL property
        assert_eq!(
            context.get::<RepositoryUrl>(),
            Some(&RepositoryUrl(REPOSITORY_URL))
        );

        // Query yet unset repository user property (where None should be returned)
        assert_eq!(context.get::<RepositoryUser>(), None);
    }
}

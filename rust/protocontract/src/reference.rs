//! What the code generated for a reference message uses.
//!
//! A message marked `(protocontract.v1.service)` becomes a type that holds
//! an implementation of that service. Its methods are generated per
//! service; the pieces they share live here. Nothing here is meant to be
//! named by hand.

use std::fmt;
use std::sync::Arc;

use crate::{Error, Failure, RuntimeCode};

/// An implementation of a service in the form it was given: `S` is the
/// blocking trait object (`dyn <Service>Sync`), `A` the async handle
/// (`Dyn<Service>Async`).
pub enum Form<S: ?Sized, A> {
    /// A blocking implementation.
    Sync(Arc<S>),
    /// An async implementation.
    Async(A),
}

impl<S: ?Sized, A: Clone> Clone for Form<S, A> {
    fn clone(&self) -> Self {
        match self {
            Form::Sync(service) => Form::Sync(Arc::clone(service)),
            Form::Async(service) => Form::Async(service.clone()),
        }
    }
}

impl<S: ?Sized, A> fmt::Debug for Form<S, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Form::Sync(_) => "sync",
            Form::Async(_) => "async",
        })
    }
}

/// The failure of a reference to `service` that holds no implementation.
#[must_use]
pub fn unbound(service: &str) -> Error {
    Error::from(Failure::new(
        RuntimeCode::Unbound,
        format!(
            "the reference to {service} holds no implementation: it was never given one, or \
             it was decoded, and a reference does not survive encoding"
        ),
    ))
}

/// The failure of asking for an async implementation of `service` in the
/// blocking form where no tokio runtime can run it.
#[must_use]
pub fn no_runtime(service: &str) -> Error {
    Error::from(Failure::new(
        RuntimeCode::NoRuntime,
        format!(
            "the implementation of {service} is async, and the blocking form needs a tokio \
             runtime to run it: ask for it inside a runtime, in a build with the `tokio` \
             feature"
        ),
    ))
}

/// The runtime the calling code runs in, for a bridge to `service`.
///
/// # Errors
///
/// [`RuntimeCode::NoRuntime`] outside a tokio runtime.
#[cfg(feature = "tokio")]
pub fn current_handle(service: &str) -> Result<tokio::runtime::Handle, Error> {
    tokio::runtime::Handle::try_current().map_err(|_| no_runtime(service))
}

#[cfg(test)]
mod tests {
    use crate::RuntimeCode;

    use super::{no_runtime, unbound};

    #[test]
    fn the_failures_carry_their_codes_and_name_the_service() {
        let error = unbound("example.v1.Clock");
        assert!(error.is(RuntimeCode::Unbound));
        assert!(error.to_string().contains("example.v1.Clock"));
        let error = no_runtime("example.v1.Clock");
        assert!(error.is(RuntimeCode::NoRuntime));
        assert!(error.to_string().contains("example.v1.Clock"));
    }

    #[cfg(feature = "tokio")]
    #[test]
    fn there_is_no_handle_outside_a_runtime() {
        let error = super::current_handle("example.v1.Clock").unwrap_err();
        assert!(error.is(RuntimeCode::NoRuntime));
    }
}

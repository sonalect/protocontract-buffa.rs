//! Runtime for the code that `protoc-gen-protocontract-buffa` generates.
//!
//! For each protobuf `service`, the plugin emits:
//!
//! - `<Service>Sync`: each method returns `Result<Reply, Error>` when the
//!   call is done. Streams are [`BoxIter`]s. The trait is dyn-compatible,
//!   so an implementation chosen at run time is held as
//!   `Arc<dyn GreeterSync>`.
//! - `<Service>Async`: each method returns `impl Future` of the same
//!   result, so an implementation writes plain `async fn`. Streams are
//!   [`Stream`]s. Generic callers (`impl GreeterAsync`) pay no
//!   allocation.
//! - `Dyn<Service>Async`: a cloneable handle that holds any `<Service>Async`
//!   implementation behind dynamic dispatch and implements the trait
//!   itself. Each call through it boxes the future ([`BoxFuture`]) and the
//!   reply stream ([`BoxStream`]).
//!
//! Every method takes its request by value and returns its reply owned; a
//! call in the same process is a plain method call, with no encoding and
//! no transport.
//!
//! Streaming methods, per form:
//!
//! | Method | `<Service>Sync` | `<Service>Async` |
//! | - | - | - |
//! | server streaming | `Result<BoxIter<'static, Result<Reply, Error>>, Error>` | `Result<impl Stream<Item = Result<Reply, Error>>, Error>` |
//! | client streaming | takes `BoxIter<'static, Result<Request, Error>>` | takes `impl Stream<Item = Result<Request, Error>>` |
//! | bidirectional | both | both |
//!
//! An `Err` before the first item fails the whole call; an `Err` item ends
//! a stream with that failure. Inbound items are `Result`s because a stream
//! that crosses a process boundary can break, and the implementation must
//! see that rather than a stream that merely ends early. No stream borrows
//! the service, so a stream can outlive the call and move between threads.
//!
//! # Bridges
//!
//! The plugin also implements each form for wrappers around the other:
//!
//! | Have | Need | Wrapper | How |
//! | - | - | - | - |
//! | sync impl | async form | [`Inline`] | runs the sync call inside `poll`; for calls of microseconds |
//! | sync impl | async form | `Offload` (feature `tokio`) | runs the sync call on tokio's blocking pool |
//! | async impl | sync form | `Blocking` (feature `tokio`) | blocks the calling thread on a stored runtime handle |
//!
//! Each wrapper implements the form itself, so it fits in a
//! `Dyn<Service>Async` handle or an `Arc<dyn <Service>Sync>`.
//!
//! # Errors
//!
//! Every method fails with one type, [`Error`], whatever the service: an
//! implementation chosen at run time, a bridge, and a broken stream all
//! produce it, and `?` carries it across contracts. An `Error` holds a
//! [`Fault`] and the stack of operations it passed ([`Context`]).
//!
//! - **Code.** The class of the failure, which a caller branches on. Codes
//!   are open: any [`ErrorCode`], which every protobuf enum is. A contract
//!   declares the codes its callers may rely on as an `enum` next to its
//!   `service`; [`RuntimeCode`] holds the runtime's own.
//! - **Retryable.** Whether the same call may succeed later; `false`
//!   unless the fault says otherwise.
//! - **Message, source, backtrace.** `Display`, `std::error::Error::source`,
//!   and [`Error::backtrace`].
//!
//! [`Failure`] is the ready-made fault; an implementation with more to say
//! implements [`Fault`] on a type of its own.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sonalect/protocontract-buffa.rs/main/assets/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/sonalect/protocontract-buffa.rs/main/assets/logo.svg"
)]

#[cfg(feature = "tokio")]
mod blocking;
mod code;
mod error;
mod inline;
#[cfg(feature = "tokio")]
mod offload;
#[doc(hidden)]
pub mod reference;
mod stream;

use std::future::Future;
use std::pin::Pin;

#[cfg(feature = "tokio")]
pub use blocking::Blocking;
pub use code::{Code, ErrorCode, RuntimeCode};
pub use error::{Context, Error, Failure, Fault};
pub use futures_core::Stream;
pub use inline::Inline;
#[cfg(feature = "tokio")]
pub use offload::Offload;
#[cfg(feature = "tokio")]
pub use stream::ChannelStream;
pub use stream::IterStream;

/// The future a call through a `Dyn<Service>Async` handle returns: boxed,
/// `Send`, and borrowing for at most `'a`.
///
/// ```
/// use protocontract::{BoxFuture, Error};
///
/// fn double(value: i64) -> BoxFuture<'static, Result<i64, Error>> {
///     Box::pin(async move { Ok(value * 2) })
/// }
/// # let _ = double(2);
/// ```
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stream of a sync streaming method: a boxed `Send` iterator that
/// borrows for at most `'a`.
///
/// ```
/// use protocontract::{BoxIter, Error};
///
/// let replies: BoxIter<'static, Result<u32, Error>> = Box::new((1..=3).map(Ok));
/// assert_eq!(replies.count(), 3);
/// ```
pub type BoxIter<'a, T> = Box<dyn Iterator<Item = T> + Send + 'a>;

/// A stream of an async streaming method called through a
/// `Dyn<Service>Async` handle: boxed, pinned, and `Send`.
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

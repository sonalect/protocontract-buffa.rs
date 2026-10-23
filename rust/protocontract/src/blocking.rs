//! `Blocking`: the sync form of an async implementation.

use std::future::Future;
use std::pin::Pin;

use futures_core::Stream;
use tokio::runtime::{Handle, RuntimeFlavor};
use tokio::sync::mpsc;

use crate::offload::{STREAM_BUFFER, pump};
use crate::stream::next;
use crate::{BoxIter, ChannelStream, Error, RuntimeCode};

/// The sync form of an async implementation: each call blocks the calling
/// thread until the async method, run on the runtime behind the stored
/// handle, is done.
///
/// A call may come from any thread outside a runtime, from a
/// `spawn_blocking` closure, or from inside a multi-thread runtime, where
/// it moves the worker's other tasks away first (`block_in_place`) and
/// suspends any other future of the same task meanwhile. From inside a
/// current-thread runtime a call fails with [`RuntimeCode::CannotBlock`] instead:
/// blocking that runtime's only thread would stop the call it waits for.
/// The check cannot tell that runtime's `spawn_blocking` threads apart, so
/// they fail the same way; call from a `std::thread` there.
///
/// The handle should belong to a multi-thread runtime, or to a
/// current-thread runtime that another thread drives: a current-thread
/// runtime does not run its I/O and timers for a call made from elsewhere.
///
/// - A reply stream is an iterator whose `next` blocks for the next item;
///   each `next` is checked like a call.
/// - An inbound iterator is pulled on the runtime's blocking pool and handed
///   to the async method as a stream that holds up to 16 items ahead; a
///   panic of that iterator ends the stream with [`RuntimeCode::Panicked`].
///
/// A panic of the async method itself reaches the caller as a panic, as it
/// would from a direct call.
///
/// The plugin implements each `<Service>Sync` for `Blocking<T>` where `T`
/// implements `<Service>Async`. The helpers below are associated functions
/// (`Blocking::block_on(&blocking, …)`), so they never hide a trait method
/// of the same name.
#[derive(Clone, Debug)]
pub struct Blocking<T> {
    service: T,
    handle: Handle,
}

impl<T> Blocking<T> {
    /// The sync form of `service`, run on the runtime behind `handle`.
    pub fn new(service: T, handle: Handle) -> Blocking<T> {
        Blocking { service, handle }
    }

    /// The async implementation.
    pub fn get_ref(this: &Blocking<T>) -> &T {
        &this.service
    }

    /// The async implementation, moved out.
    pub fn into_inner(this: Blocking<T>) -> T {
        this.service
    }

    /// Block until `call` is done and return its result.
    ///
    /// # Errors
    ///
    /// `FAILED_PRECONDITION` when called inside a current-thread runtime;
    /// otherwise whatever `call` returns.
    pub fn block_on<R>(
        this: &Blocking<T>,
        call: impl Future<Output = Result<R, Error>>,
    ) -> Result<R, Error> {
        block_on(&this.handle, call)
    }

    /// Block until `call` returns its reply stream, and return the stream
    /// as an iterator that blocks for each item.
    ///
    /// # Errors
    ///
    /// As [`Blocking::block_on`].
    pub fn block_on_stream<X, S>(
        this: &Blocking<T>,
        call: impl Future<Output = Result<S, Error>>,
    ) -> Result<BoxIter<'static, Result<X, Error>>, Error>
    where
        S: Stream<Item = Result<X, Error>> + Send + 'static,
        X: Send + 'static,
    {
        let stream = Blocking::block_on(this, call)?;
        Ok(Box::new(BlockingIter {
            handle: this.handle.clone(),
            stream: Some(Box::pin(stream)),
        }))
    }

    /// Pull `requests` on the runtime's blocking pool and return them as a
    /// stream for an async method. Dropping the stream stops the pull: no
    /// `next` of `requests` starts after that. An `Err` item ends it.
    pub fn feed<X: Send + 'static>(
        this: &Blocking<T>,
        requests: BoxIter<'static, Result<X, Error>>,
    ) -> ChannelStream<Result<X, Error>> {
        let (sender, receiver) = mpsc::channel(STREAM_BUFFER);
        drop(this.handle.spawn_blocking(move || pump(requests, &sender)));
        ChannelStream::new(receiver)
    }
}

/// Block the current thread on `future`, run on `handle`'s runtime.
fn block_on<R>(
    handle: &Handle,
    future: impl Future<Output = Result<R, Error>>,
) -> Result<R, Error> {
    check_can_block()?;
    tokio::task::block_in_place(|| handle.block_on(future))
}

/// [`RuntimeCode::CannotBlock`] inside a current-thread runtime, where blocking
/// would stop the runtime the call needs.
fn check_can_block() -> Result<(), Error> {
    match Handle::try_current() {
        Ok(current) if current.runtime_flavor() == RuntimeFlavor::CurrentThread => Err(Error::new(
            RuntimeCode::CannotBlock,
            "a sync call through Blocking cannot wait inside a current-thread tokio \
                 runtime; call it from a std::thread or use a multi-thread runtime",
        )),
        _ => Ok(()),
    }
}

/// An async reply stream read as a blocking iterator.
struct BlockingIter<S> {
    handle: Handle,
    /// `None` once the stream has ended or failed.
    stream: Option<Pin<Box<S>>>,
}

impl<X, S: Stream<Item = Result<X, Error>>> Iterator for BlockingIter<S> {
    type Item = Result<X, Error>;

    fn next(&mut self) -> Option<Result<X, Error>> {
        let stream = self.stream.as_mut()?;
        // A panic of the async stream runs on this thread and reaches the
        // caller as it would from a direct call.
        let item = match block_on(&self.handle, async { Ok(next(stream).await) }) {
            Ok(item) => item,
            Err(error) => Some(Err(error)),
        };
        if !matches!(item, Some(Ok(_))) {
            self.stream = None;
        }
        item
    }
}

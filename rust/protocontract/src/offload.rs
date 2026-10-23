//! `Offload`: the async form of a sync implementation, on tokio's blocking
//! pool.

use std::any::Any;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use futures_core::Stream;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};

use crate::stream::next;
use crate::{BoxIter, ChannelStream, Error, Failure, RuntimeCode};

/// How many items a stream moved between threads holds ahead of its
/// reader.
pub(crate) const STREAM_BUFFER: usize = 16;

/// The async form of a sync implementation: each call runs the sync method
/// on tokio's blocking pool, so the async side never blocks.
///
/// - A unary call costs one hop to the blocking pool and back.
/// - A reply stream is pulled on the blocking pool and handed over through
///   a channel that holds up to 16 items; dropping the stream stops the
///   pull: no `next` of the sync iterator starts after that, and one
///   already running finishes.
/// - An inbound stream reaches the sync method as an iterator whose `next`
///   waits on the blocking thread, so a bidirectional call can answer each
///   request as it arrives.
///
/// A panic in the sync method becomes [`RuntimeCode::Panicked`]: the call fails,
/// or the stream ends with that item. The work starts when the future is
/// first polled; once started, dropping the future does not stop a call
/// that is already running on the blocking pool.
///
/// The plugin implements each `<Service>Async` for `Offload<T>` where `T`
/// implements `<Service>Sync`. The helpers below are associated functions
/// (`Offload::call(&offload, …)`), so they never hide a trait method of the
/// same name.
#[derive(Debug)]
pub struct Offload<T> {
    service: Arc<T>,
    handle: Handle,
}

impl<T> Clone for Offload<T> {
    fn clone(&self) -> Offload<T> {
        Offload {
            service: Arc::clone(&self.service),
            handle: self.handle.clone(),
        }
    }
}

impl<T> Offload<T> {
    /// The async form of `service`, run on the blocking pool of the runtime
    /// behind `handle`.
    pub fn new(service: T, handle: Handle) -> Offload<T> {
        Offload::from_arc(Arc::new(service), handle)
    }

    /// The async form of a shared `service`.
    pub fn from_arc(service: Arc<T>, handle: Handle) -> Offload<T> {
        Offload { service, handle }
    }

    /// The sync implementation.
    pub fn get_ref(this: &Offload<T>) -> &T {
        &this.service
    }
}

impl<T: Send + Sync + 'static> Offload<T> {
    /// Run `call` on the blocking pool and return its result.
    pub fn call<R, F>(
        this: &Offload<T>,
        call: F,
    ) -> impl Future<Output = Result<R, Error>> + Send + 'static
    where
        F: FnOnce(&T) -> Result<R, Error> + Send + 'static,
        R: Send + 'static,
    {
        let (service, handle) = (Arc::clone(&this.service), this.handle.clone());
        async move {
            let task = handle.spawn_blocking(move || {
                catch_unwind(AssertUnwindSafe(|| call(&service)))
                    .unwrap_or_else(|panic| Err(panicked(&panic)))
            });
            task.await.unwrap_or_else(|error| Err(join_failed(error)))
        }
    }

    /// Run `call`, which returns a reply stream, on the blocking pool, and
    /// pull that stream there.
    pub fn server_streaming<X, F>(
        this: &Offload<T>,
        call: F,
    ) -> impl Future<Output = Result<ChannelStream<Result<X, Error>>, Error>> + Send + 'static
    where
        F: FnOnce(&T) -> Result<BoxIter<'static, Result<X, Error>>, Error> + Send + 'static,
        X: Send + 'static,
    {
        let (service, handle) = (Arc::clone(&this.service), this.handle.clone());
        streaming(handle, move || call(&service))
    }

    /// Run `call` on the blocking pool with `requests` as an iterator and
    /// return its result.
    pub fn client_streaming<S, X, R, F>(
        this: &Offload<T>,
        requests: S,
        call: F,
    ) -> impl Future<Output = Result<R, Error>> + Send + 'static
    where
        S: Stream<Item = Result<X, Error>> + Send + 'static,
        X: Send + 'static,
        F: FnOnce(&T, BoxIter<'static, Result<X, Error>>) -> Result<R, Error> + Send + 'static,
        R: Send + 'static,
    {
        let inbound = this.handle.clone();
        Offload::call(this, move |service| {
            call(service, Box::new(StreamIter::new(inbound, requests)))
        })
    }

    /// Run `call` on the blocking pool with `requests` as an iterator, and
    /// pull the reply stream it returns there.
    pub fn bidirectional<S, X, Y, F>(
        this: &Offload<T>,
        requests: S,
        call: F,
    ) -> impl Future<Output = Result<ChannelStream<Result<Y, Error>>, Error>> + Send + 'static
    where
        S: Stream<Item = Result<X, Error>> + Send + 'static,
        X: Send + 'static,
        Y: Send + 'static,
        F: FnOnce(
                &T,
                BoxIter<'static, Result<X, Error>>,
            ) -> Result<BoxIter<'static, Result<Y, Error>>, Error>
            + Send
            + 'static,
    {
        let (service, handle) = (Arc::clone(&this.service), this.handle.clone());
        let inbound = handle.clone();
        streaming(handle, move || {
            call(&service, Box::new(StreamIter::new(inbound, requests)))
        })
    }
}

/// Run `start` on the blocking pool; if it returns a stream, pull it there
/// into a channel and return the channel's reading end.
async fn streaming<X, F>(handle: Handle, start: F) -> Result<ChannelStream<Result<X, Error>>, Error>
where
    F: FnOnce() -> Result<BoxIter<'static, Result<X, Error>>, Error> + Send + 'static,
    X: Send + 'static,
{
    let (sender, receiver) = mpsc::channel(STREAM_BUFFER);
    let (started, start_result) = oneshot::channel();
    // The task runs until the stream ends or its reader goes away; its
    // outcome reaches the reader through the two channels.
    drop(handle.spawn_blocking(move || {
        let items = match catch_unwind(AssertUnwindSafe(start)) {
            Ok(Ok(items)) => items,
            Ok(Err(error)) => {
                let _ = started.send(Err(error));
                return;
            }
            Err(panic) => {
                let _ = started.send(Err(panicked(&panic)));
                return;
            }
        };
        if started.send(Ok(())).is_ok() {
            pump(items, &sender);
        }
    }));
    match start_result.await {
        Ok(Ok(())) => Ok(ChannelStream::new(receiver)),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(Error::new(
            RuntimeCode::Cancelled,
            "the blocking call ended without a result",
        )),
    }
}

/// Send each item of `items` until it ends, an `Err` item has been sent
/// (an `Err` ends a stream), the reader goes away, or `next` panics (sent
/// as a last [`RuntimeCode::Panicked`] item).
///
/// No `next` starts once the reader has gone; one already running then
/// finishes, and its send fails.
pub(crate) fn pump<X>(
    mut items: BoxIter<'static, Result<X, Error>>,
    sender: &mpsc::Sender<Result<X, Error>>,
) {
    loop {
        if sender.is_closed() {
            return;
        }
        let item = match catch_unwind(AssertUnwindSafe(|| items.next())) {
            Ok(Some(item)) => item,
            Ok(None) => return,
            Err(panic) => {
                let _ = sender.blocking_send(Err(panicked(&panic)));
                return;
            }
        };
        let ends = item.is_err();
        if sender.blocking_send(item).is_err() || ends {
            return;
        }
    }
}

/// A failed blocking task: [`RuntimeCode::Panicked`] or
/// [`RuntimeCode::Cancelled`], with tokio's error as the source.
fn join_failed(error: tokio::task::JoinError) -> Error {
    let code = if error.is_panic() {
        RuntimeCode::Panicked
    } else {
        RuntimeCode::Cancelled
    };
    Failure::new(code, "the blocking call failed")
        .with_source(error)
        .into()
}

/// [`RuntimeCode::Panicked`] for a caught panic, with its message when it
/// has one.
pub(crate) fn panicked(panic: &Box<dyn Any + Send>) -> Error {
    let message = panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message");
    Error::new(
        RuntimeCode::Panicked,
        format!("the implementation panicked: {message}"),
    )
}

/// An async stream read as an iterator on a blocking-pool thread: each
/// `next` waits for the stream's next item.
struct StreamIter<S> {
    handle: Handle,
    stream: std::pin::Pin<Box<S>>,
}

impl<S> StreamIter<S> {
    fn new(handle: Handle, stream: S) -> StreamIter<S> {
        StreamIter {
            handle,
            stream: Box::pin(stream),
        }
    }
}

impl<S: Stream> Iterator for StreamIter<S> {
    type Item = S::Item;

    fn next(&mut self) -> Option<S::Item> {
        self.handle.block_on(next(&mut self.stream))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use tokio::runtime::{Builder, Runtime};

    use super::Offload;
    use crate::stream::next;
    use crate::{Blocking, Error};

    /// The longest any wait in these tests may take.
    const DEADLINE: Duration = Duration::from_secs(10);

    /// An iterator that counts its `next` calls, holds one of them until
    /// the test drops the gate's sender, and reports its own drop: the end
    /// of the pump that owns it.
    struct Probe {
        calls: Arc<AtomicUsize>,
        /// The call to hold, the sender that reports it running, and the
        /// gate it waits on.
        hold: Option<(usize, mpsc::Sender<()>, mpsc::Receiver<()>)>,
        dropped: mpsc::Sender<()>,
    }

    impl Iterator for Probe {
        type Item = Result<usize, Error>;

        fn next(&mut self) -> Option<Result<usize, Error>> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some((held, running, gate)) = &self.hold
                && call == *held
            {
                let _ = running.send(());
                let _ = gate.recv_timeout(DEADLINE);
            }
            Some(Ok(call))
        }
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            let _ = self.dropped.send(());
        }
    }

    /// A runtime with one blocking thread.
    fn runtime() -> Runtime {
        Builder::new_current_thread()
            .max_blocking_threads(1)
            .build()
            .unwrap()
    }

    #[test]
    fn feed_starts_no_item_after_its_reader_is_gone() {
        let runtime = runtime();
        // Hold the only blocking thread, so the pump waits in the queue
        // until its reader is gone. Dropping `gate` lets it go.
        let (gate, gated) = mpsc::channel::<()>();
        let (busy, held) = mpsc::channel();
        drop(runtime.spawn_blocking(move || {
            let _ = busy.send(());
            let _ = gated.recv_timeout(DEADLINE);
        }));
        held.recv_timeout(DEADLINE).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let (dropped, ended) = mpsc::channel();
        let requests = Probe {
            calls: Arc::clone(&calls),
            hold: None,
            dropped,
        };
        let blocking = Blocking::new((), runtime.handle().clone());
        drop(Blocking::feed(&blocking, Box::new(requests)));
        drop(gate);
        ended.recv_timeout(DEADLINE).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn reply_stream_finishes_the_running_item_and_starts_no_more() {
        let runtime = runtime();
        let calls = Arc::new(AtomicUsize::new(0));
        let (running, is_running) = mpsc::channel();
        let (gate, gated) = mpsc::channel::<()>();
        let (dropped, ended) = mpsc::channel();
        let replies = Probe {
            calls: Arc::clone(&calls),
            hold: Some((1, running, gated)),
            dropped,
        };
        let offload = Offload::new((), runtime.handle().clone());
        let mut stream = runtime
            .block_on(Offload::server_streaming(&offload, move |()| {
                Ok(Box::new(replies))
            }))
            .unwrap();
        let first = runtime.block_on(next(&mut stream)).map(Result::unwrap);
        assert_eq!(first, Some(0));
        is_running.recv_timeout(DEADLINE).unwrap();
        drop(stream);
        drop(gate);
        ended.recv_timeout(DEADLINE).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}

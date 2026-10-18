//! A reference to a service in a message field, a request, and a reply,
//! through both forms, and what encoding does to it.

use std::sync::{Arc, Mutex};

use buffa::{Message, MessageView};
use buffa_types::google::protobuf::Empty;
use protocontract::{Error, Inline, RuntimeCode};
use refs_api::proto::refs::v1::{Alarm, AlarmView, Tick};
use refs_api::traits::clock_ref::ClockRef;
use refs_api::traits::{AlarmsAsync, AlarmsSync, ClockAsync, ClockSync};

struct Fixed(i64);

impl ClockSync for Fixed {
    fn now(&self, _empty: Empty) -> Result<Tick, Error> {
        Ok(Tick::default().with_unix_ms(self.0))
    }
}

struct Ticking(i64);

impl ClockAsync for Ticking {
    async fn now(&self, _empty: Empty) -> Result<Tick, Error> {
        tokio::task::yield_now().await;
        Ok(Tick::default().with_unix_ms(self.0))
    }
}

/// Keeps one clock; `set` reads the alarm's clock and remembers the time.
#[derive(Default)]
struct Keeper {
    clock: Mutex<ClockRef>,
    seen: Mutex<Vec<i64>>,
}

impl AlarmsSync for Keeper {
    fn set(&self, alarm: Alarm) -> Result<Empty, Error> {
        let clock = alarm.clock.as_option().cloned().unwrap_or_default();
        let clock = clock.to_sync()?;
        self.seen
            .lock()
            .unwrap()
            .push(clock.now(Empty::default())?.unix_ms.unwrap_or_default());
        for backup in &alarm.backups {
            let backup = backup.to_sync()?;
            self.seen
                .lock()
                .unwrap()
                .push(backup.now(Empty::default())?.unix_ms.unwrap_or_default());
        }
        Ok(Empty::default())
    }

    fn use_clock(&self, clock: ClockRef) -> Result<Empty, Error> {
        *self.clock.lock().unwrap() = clock;
        Ok(Empty::default())
    }

    fn current_clock(&self, _empty: Empty) -> Result<ClockRef, Error> {
        Ok(self.clock.lock().unwrap().clone())
    }
}

fn alarm(clock: ClockRef, backups: Vec<ClockRef>) -> Alarm {
    let mut alarm = Alarm::default().with_label("wake");
    alarm.clock = buffa::MessageField::some(clock);
    alarm.backups = backups;
    alarm
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_carries_data_and_references_of_both_forms() {
    let keeper = Arc::new(Keeper::default());
    let sync = ClockRef::from_sync(Fixed(1));
    let async_ = ClockRef::from_async(Ticking(2));
    let alarm = alarm(sync, vec![async_, ClockRef::from_sync(Fixed(3))]);
    let keeper2 = Arc::clone(&keeper);
    // The blocking implementation blocks on the async clock: off the runtime's
    // worker threads.
    tokio::task::spawn_blocking(move || keeper2.set(alarm))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(*keeper.seen.lock().unwrap(), vec![1, 2, 3]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_request_and_a_reply_are_references_through_the_async_form() {
    // A blocking implementation in the async form: the bridge passes the
    // reference through unchanged.
    let alarms = Inline::new(Keeper::default());
    let clock = ClockRef::from_async(Ticking(7));
    alarms.use_clock(clock.clone()).await.unwrap();
    let back = alarms.current_clock(Empty::default()).await.unwrap();
    assert_eq!(back, clock, "the same implementation comes back");
    let tick = back
        .to_async_inline()
        .unwrap()
        .now(Empty::default())
        .await
        .unwrap();
    assert_eq!(tick.unix_ms, Some(7));
}

#[test]
fn a_reference_does_not_survive_encoding() {
    let bound = alarm(ClockRef::from_sync(Fixed(1)), vec![]);
    let bytes = bound.encode_to_vec();

    let owned = Alarm::decode_from_slice(&bytes).unwrap();
    assert_eq!(owned.label.as_deref(), Some("wake"));
    let clock = owned.clock.as_option().expect("the field stays present");
    assert!(!clock.is_bound());
    let error = clock.to_async_inline().unwrap_err();
    assert!(error.is(RuntimeCode::Unbound), "{error}");

    let view = AlarmView::decode_view(&bytes).unwrap();
    let owned = view.to_owned_message().unwrap();
    assert!(!owned.clock.as_option().unwrap().is_bound());
}

#[test]
fn a_reference_has_no_json_form() {
    let bound = alarm(ClockRef::from_sync(Fixed(1)), vec![]);
    let error = serde_json::to_string(&bound).unwrap_err();
    assert!(error.to_string().contains("no JSON form"), "{error}");

    let none = Alarm::default().with_label("wake");
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"label":"wake"}"#);
    let read: Alarm = serde_json::from_str(r#"{"label":"wake"}"#).unwrap();
    assert!(read.clock.as_option().is_none());

    let error = serde_json::from_str::<Alarm>(r#"{"clock":{}}"#).unwrap_err();
    assert!(
        error.to_string().contains("cannot be read from JSON"),
        "{error}"
    );
}

#[test]
fn references_are_equal_when_they_hold_the_same_implementation() {
    let shared: Arc<dyn ClockSync> = Arc::new(Fixed(1));
    let one = ClockRef::from_shared(Arc::clone(&shared));
    assert_eq!(one, ClockRef::from_shared(shared));
    assert_eq!(one, one.clone());
    assert_ne!(one, ClockRef::from_sync(Fixed(1)));
    let async_ = ClockRef::from_async(Ticking(2));
    assert_eq!(async_, async_.clone());
    assert_ne!(async_, ClockRef::from_async(Ticking(2)));
    assert_eq!(ClockRef::default(), ClockRef::default());
    assert_ne!(one, ClockRef::default());
}

#[test]
fn the_default_reference_is_unbound_in_every_form() {
    let clock = ClockRef::default();
    assert!(!clock.is_bound());
    assert!(clock.to_sync().err().unwrap().is(RuntimeCode::Unbound));
    assert!(
        clock
            .to_async_inline()
            .unwrap_err()
            .is(RuntimeCode::Unbound)
    );
    assert!(
        clock
            .to_async_offload()
            .unwrap_err()
            .is(RuntimeCode::Unbound)
    );
}

#[test]
fn an_async_implementation_has_no_blocking_form_outside_a_runtime() {
    let clock = ClockRef::from_async(Ticking(2));
    let error = clock.to_sync().err().unwrap();
    assert!(error.is(RuntimeCode::NoRuntime), "{error}");
    assert!(error.to_string().contains("refs.v1.Clock"), "{error}");
    // A blocking implementation needs no runtime for its own form.
    assert_eq!(
        ClockRef::from_sync(Fixed(1))
            .to_sync()
            .unwrap()
            .now(Empty::default())
            .unwrap()
            .unix_ms,
        Some(1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blocking_implementation_runs_on_the_blocking_pool_through_offload() {
    let clock = ClockRef::from_sync(Fixed(5)).to_async_offload().unwrap();
    assert_eq!(clock.now(Empty::default()).await.unwrap().unix_ms, Some(5));
    let error = ClockRef::from_sync(Fixed(5))
        .to_async_offload()
        .map(|_| ())
        .err();
    assert!(error.is_none(), "inside a runtime there is a handle");
}

#[test]
fn offload_needs_a_runtime() {
    let error = ClockRef::from_sync(Fixed(5))
        .to_async_offload()
        .unwrap_err();
    assert!(error.is(RuntimeCode::NoRuntime), "{error}");
}

#[test]
fn debug_shows_the_form() {
    assert_eq!(
        format!("{:?}", ClockRef::default()),
        "refs.v1.ClockRef(None)"
    );
    assert_eq!(
        format!("{:?}", ClockRef::from_sync(Fixed(1))),
        "refs.v1.ClockRef(Some(sync))"
    );
}

#[test]
fn a_message_that_holds_references_validates() {
    use protovalidate_buffa::Validate;

    let bound = alarm(
        ClockRef::from_sync(Fixed(1)),
        vec![ClockRef::from_async(Ticking(2))],
    );
    bound.validate().unwrap();
    ClockRef::default().validate().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_form_given_comes_back_as_it_is_with_no_bridge() {
    // Blocking in, blocking out: the same implementation, not a wrapper.
    let shared: Arc<dyn ClockSync> = Arc::new(Fixed(1));
    let sync = ClockRef::from_shared(Arc::clone(&shared))
        .to_sync()
        .unwrap();
    assert!(Arc::ptr_eq(&sync, &shared));

    // Async in, async out, through either method: the same handle.
    let handle = refs_api::traits::DynClockAsync::new(Ticking(2));
    let reference = ClockRef::from_dyn(handle.clone());
    assert!(reference.to_async_inline().unwrap().ptr_eq(&handle));
    assert!(reference.to_async_offload().unwrap().ptr_eq(&handle));
}

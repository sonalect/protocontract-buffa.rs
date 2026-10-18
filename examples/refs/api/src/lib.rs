//! A message that stands for a reference to a service.
//!
//! `refs.v1.ClockRef` is marked `(protocontract.v1.service) =
//! "refs.v1.Clock"`. protocontract-buffa emits its type,
//! [`traits::clock_ref::ClockRef`], and buffa maps the message to it, so
//! every message and every trait that names it holds an implementation of
//! `Clock` in either form.

/// The messages, as `protoc-gen-buffa` writes them.
#[rustfmt::skip]
#[path = "generated/buffa/mod.rs"]
pub mod proto;

/// The `Validate` impls of the messages, as `protoc-gen-protovalidate-buffa`
/// writes them (`packaging=false`).
#[rustfmt::skip]
#[allow(unused_imports)]
mod validators {
    use crate::proto::refs::v1::*;

    include!("generated/buffa/refs.v1.refs.validate.rs");
}

/// The traits, as `protoc-gen-protocontract-buffa` writes them.
#[rustfmt::skip]
pub mod traits {
    include!("generated/contract/refs.v1.rs");
}

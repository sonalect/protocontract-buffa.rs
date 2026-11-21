//! The glossary contract: what a provider and a consumer agree on, and all
//! either of them depends on.
//!
//! Everything here is generated from `proto/glossary/v1/glossary.proto`
//! (`mise run generate`):
//!
//! - the messages, by `protoc-gen-buffa`;
//! - [`GlossarySync`] and [`GlossariesSync`] (and their async forms), by
//!   `protoc-gen-protocontract-buffa`;
//! - [`GlossaryRef`], a reference to an implementation of `Glossary`, also
//!   by `protoc-gen-protocontract-buffa`: the message `GlossaryRef` is
//!   marked `(protocontract.v1.service) = "glossary.v1.Glossary"`, and both
//!   plugins map it to this type, so [`GlossariesSync::open`] returns one
//!   and [`Shelf`] holds one;
//! - [`GlossaryErrorCode`] and [`GlossariesErrorCode`], the failures a
//!   caller may branch on, declared in the proto next to each service. A
//!   name `<Service>ErrorCode` lets the plugin find it and name it in the
//!   traits' rustdoc: from the trait a caller finds its codes.
//!
//! The crate also re-exports the runtime types a provider or a consumer
//! names, so neither needs a dependency on `protocontract` of its own.

/// The messages, as `protoc-gen-buffa` writes them
/// (`buffa_module=crate::proto`).
#[rustfmt::skip]
#[path = "generated/buffa/mod.rs"]
mod proto;

/// The traits of `glossary.v1.Glossary` and `glossary.v1.Glossaries`, and
/// the type of `GlossaryRef`, as `protoc-gen-protocontract-buffa` writes
/// them.
#[rustfmt::skip]
mod traits {
    include!("generated/contract/glossary.v1.rs");
}

pub use buffa::MessageField;
pub use buffa_types::google::protobuf::Empty;
pub use proto::glossary::v1::{
    GlossariesErrorCode, GlossaryErrorCode, Prefix, Shelf, Term, Topic, Word,
};
pub use protocontract::{BoxIter, Code, Context, Error, ErrorCode, Failure, Fault, RuntimeCode};
pub use traits::glossary_ref::GlossaryRef;
pub use traits::{
    DynGlossariesAsync, DynGlossaryAsync, GlossariesAsync, GlossariesSync, GlossaryAsync,
    GlossarySync,
};

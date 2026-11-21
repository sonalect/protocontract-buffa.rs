//! The glossary contract: what a provider and a consumer agree on, and all
//! either of them depends on.
//!
//! Everything here is generated from `proto/glossary/v1/glossary.proto`
//! (`mise run generate`):
//!
//! - the messages, by `protoc-gen-buffa`;
//! - [`GlossarySync`] (and its async form), by `protoc-gen-protocontract-buffa`;
//! - [`GlossaryErrorCode`], the failures a caller may branch on, declared
//!   in the proto next to the service. Its name is `<Service>ErrorCode`, so
//!   the plugin finds it and names it in the traits' rustdoc: from the
//!   trait a caller finds its codes.
//!
//! The crate also re-exports the runtime types a provider or a consumer
//! names, so neither needs a dependency on `protocontract` of its own.

/// The messages, as `protoc-gen-buffa` writes them
/// (`buffa_module=crate::proto`).
#[rustfmt::skip]
#[path = "generated/buffa/mod.rs"]
mod proto;

/// The traits of `glossary.v1.Glossary`, as
/// `protoc-gen-protocontract-buffa` writes them.
#[rustfmt::skip]
mod traits {
    include!("generated/contract/glossary.v1.rs");
}

pub use proto::glossary::v1::{GlossaryErrorCode, Prefix, Term, Word};
pub use protocontract::{BoxIter, Code, Context, Error, ErrorCode, Failure, Fault};
pub use traits::{DynGlossaryAsync, GlossaryAsync, GlossarySync};

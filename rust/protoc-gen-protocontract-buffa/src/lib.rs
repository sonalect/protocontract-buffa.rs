//! `protoc-gen-protocontract-buffa`: a protoc plugin that turns each protobuf
//! `service` into plain Rust traits over the message structs
//! `protoc-gen-buffa` generates.
//!
//! For `service Greeter { rpc Greet(Person) returns (Greeting); }`
//! it emits:
//!
//! ```rust,ignore
//! pub trait GreeterSync: Send + Sync {
//!     fn greet(&self, person: Person) -> Result<Greeting, protocontract::Error>;
//! }
//!
//! pub trait GreeterAsync: Send + Sync {
//!     fn greet(&self, person: Person)
//!         -> impl Future<Output = Result<Greeting, protocontract::Error>> + Send;
//! }
//!
//! /// Any `GreeterAsync` behind dynamic dispatch; implements it too.
//! #[derive(Clone)]
//! pub struct DynGreeterAsync { /* … */ }
//! ```
//!
//! When the package has an enum named after the service
//! (`GreeterErrorCode` or `GreeterServiceErrorCode`), it also emits the
//! alias `GreeterServiceErrorCode` and names it in the traits' rustdoc: the
//! codes the service fails with.
//!
//! An async implementation writes plain `async fn`. Streaming methods take
//! and return `protocontract::BoxIter` in the sync form and `impl Stream` in
//! the async form; see the `protocontract` crate for each kind.
//!
//! The request is taken by value and the reply is owned. Message paths are
//! absolute (`buffa_module=` / `extern_path=`), so the output compiles
//! wherever it is mounted, next to buffa's and connect-rust's output.
//!
//! # Parameters
//!
//! - `buffa_module=<rust_path>`: where the buffa output is mounted;
//!   shorthand for `extern_path=.=<rust_path>`.
//! - `extern_path=<proto>=<rust_path>`: maps a proto package prefix to a
//!   Rust module; repeatable, the longest prefix wins. Every message a
//!   method names must be covered; well-known types map to `buffa_types`
//!   on their own.
//! - `file_per_package`: one `<dotted.pkg>.rs` per package instead of a
//!   `<stem>.__contract.rs` per proto plus a `<pkg>.mod.rs` stitcher.
//! - `element_memory_limit=<bytes|unlimited>`: the decode bound of the
//!   request, as for `protoc-gen-buffa`.
//! - `runtime=<rust_path>`: path of the runtime crate; default
//!   `::protocontract`.
//! - `gate_tokio_feature[=<name>]`: put the `Offload` and `Blocking` impls
//!   under `#[cfg(feature = "<name>")]`; default name `tokio`.
//! - `json=<true|false>`: buffa runs with `json=true`, so the reference
//!   types implement serde too.
//! - `validate=<true|false>`: `protoc-gen-protovalidate-buffa` runs too, so
//!   the reference types implement `Validate`.
//!
//! # References to a service
//!
//! An empty message marked `option (protocontract.v1.service) =
//! "pkg.Service";` (`protocontract/v1/options.proto`, shipped in the
//! `protocontract` crate) gets a type of its own in the module named after
//! it (`ClockRef` gives `clock_ref::ClockRef`), which holds an
//! implementation of the service in either form. Map the message there
//! with `extern_path=.pkg.ClockRef=<mount>::clock_ref::ClockRef`, in this
//! plugin's parameters and in `protoc-gen-buffa`'s.
//!
//! Every path must be absolute: `::some_crate::…`, `crate`, or `crate::…`. Any other
//! parameter fails the run. So does a generated name that another item of
//! the package's module already has; the message names both proto
//! elements.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sonalect/protocontract-buffa.rs/main/assets/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/sonalect/protocontract-buffa.rs/main/assets/logo.svg"
)]

mod docs;
mod emit;
mod error;
mod methods;
mod names;
mod options;
mod paths;
mod references;
mod resolve;
mod sanitize;

#[cfg(test)]
mod tests;

use buffa_codegen::CodeGenConfig;
use buffa_codegen::generated::compiler::code_generator_response::File;
use buffa_codegen::generated::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use buffa_codegen::generated::descriptor::Edition;

pub use error::Error;

use emit::{file_code, layout};
use names::{check_clashes, check_reference_clashes};
use options::Options;
use resolve::TypeResolver;

/// `CodeGeneratorResponse.Feature`: `FEATURE_PROTO3_OPTIONAL` and
/// `FEATURE_SUPPORTS_EDITIONS`.
const SUPPORTED_FEATURES: u64 = 1 | 2;

/// Answer one protoc request, given as the bytes protoc writes to the
/// plugin's stdin. A request the plugin refuses yields a response with
/// `error` set, which protoc reports; the plugin itself still succeeds.
#[must_use]
pub fn run(input: &[u8]) -> CodeGeneratorResponse {
    let result = buffa_codegen::decode_request(input)
        .map_err(Error::new)
        .and_then(|request| generate(&request));
    result.unwrap_or_else(|error| CodeGeneratorResponse {
        error: Some(error.message().to_string()),
        supported_features: Some(SUPPORTED_FEATURES),
        ..Default::default()
    })
}

/// Generate the traits for every service of the requested files.
///
/// # Errors
///
/// Fails on an unknown or malformed parameter, a message type no mapping
/// covers or the request lacks, two methods of a service with one Rust
/// name, and a generated name another item of the package's Rust module
/// already has.
pub fn generate(request: &CodeGeneratorRequest) -> Result<CodeGeneratorResponse, Error> {
    let mut options = Options::parse(request.parameter.as_deref())?;
    let references = references::find(&request.proto_file, &request.file_to_generate)?;
    references::check_paths(&references, &options.extern_paths)?;
    // buffa's own paths, without the mappings of the references: where
    // buffa's view of a reference message lives.
    let mut buffa_config = CodeGenConfig::default();
    buffa_config.extern_paths = options
        .extern_paths
        .iter()
        .filter(|(proto, _)| !references.iter().any(|reference| reference.fqn == *proto))
        .cloned()
        .collect();
    let mut config = CodeGenConfig::default();
    config.extern_paths = std::mem::take(&mut options.extern_paths);
    let resolver = TypeResolver::new(&request.proto_file, &request.file_to_generate, &config);
    let buffa_resolver = TypeResolver::new(
        &request.proto_file,
        &request.file_to_generate,
        &buffa_config,
    );

    let mut code = Vec::new();
    let mut checked_packages = Vec::new();
    for name in &request.file_to_generate {
        let Some(file) = request
            .proto_file
            .iter()
            .find(|file| file.name.as_deref() == Some(name.as_str()))
        else {
            return Err(Error::new(format!(
                "{name}: listed in file_to_generate but missing from proto_file"
            )));
        };
        let in_file: Vec<&references::Reference> = references
            .iter()
            .filter(|reference| reference.proto == *name)
            .collect();
        if file.service.is_empty() && in_file.is_empty() {
            continue;
        }
        let package = file.package.clone().unwrap_or_default();
        if !checked_packages.contains(&package) {
            check_clashes(&request.proto_file, &package)?;
            check_reference_clashes(&request.proto_file, &package, &references)?;
            checked_packages.push(package.clone());
        }
        code.push((
            name.clone(),
            package,
            file_code(
                &request.proto_file,
                file,
                &in_file,
                &resolver,
                &buffa_resolver,
                &options,
            )?,
        ));
    }

    let file = layout(code, options.file_per_package)
        .into_iter()
        .map(|output| File {
            name: Some(output.name),
            content: Some(output.content),
            ..Default::default()
        })
        .collect();
    Ok(CodeGeneratorResponse {
        supported_features: Some(SUPPORTED_FEATURES),
        minimum_edition: Some(Edition::EDITION_2023 as i32),
        maximum_edition: Some(Edition::EDITION_2024 as i32),
        file,
        ..Default::default()
    })
}

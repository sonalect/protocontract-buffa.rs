# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
While the major version is 0, compatible additions bump the patch; breaking
changes to the generated code or the runtime bump the minor.

## [Unreleased]

## [0.4.2] - 2026-10-23

### Fixed

- A stream that `Offload` or `Blocking` pulls from a sync iterator on the
  blocking pool (an `Offload` reply stream, a `Blocking::feed` request
  stream) no longer calls the iterator's `next` once its reader is gone.
  Before, after each item was handed over the pull called `next` again
  without checking, and only the send after it found the stream dropped:
  one more unit of the implementation's work ran for nobody, possibly
  after the caller had dropped the stream and moved on. A `next` already
  running when the stream is dropped still finishes. No code change is
  needed; update `protocontract`.

### Changed

- Build pin: `bazel_utils` 0.4.2. It adds plugin versions to its catalog
  and changes nothing this build uses; the generated code is the same.

## [0.4.1] - 2026-10-19

### Fixed

- The doc comment on the module of a reference type links the type through
  that module (`[`ClockRef`](clock_ref::ClockRef)`): the comment sits
  outside the module, where the type is not in scope, so rustdoc with
  warnings denied failed with `unresolved link`. Regenerate to get it.

## [0.4.0] - 2026-10-18

### Added

- References to a service. An empty message marked
  `option (protocontract.v1.service) = "pkg.Service";` stands for an
  implementation of that service: the plugin emits a type for it
  (`<module>::<Name>`, e.g. `clock_ref::ClockRef`) that holds the
  implementation in the form it was given, and buffa maps the message to it
  with an `extern_path`, so a field, a request, or a reply can carry an
  implementation. The receiver takes the form it needs with `to_sync`,
  `to_async_inline`, or `to_async_offload`. A reference does not survive
  encoding: decoded it is `RuntimeCode::Unbound` when used, and it has no
  JSON form. The option is declared in `protocontract/v1/options.proto`,
  shipped in the `protocontract` crate under `proto/`.
- Plugin parameters `json=<true|false>` and `validate=<true|false>`: with
  buffa's `json=true`, reference types get serde impls too; next to
  `protoc-gen-protovalidate-buffa`, they implement `Validate` (always
  valid), which it calls on every message field.
- `RuntimeCode::Unbound` and `RuntimeCode::NoRuntime`.
- Every `Dyn<Service>Async` handle has `ptr_eq`: whether two handles hold
  the same implementation.
- `examples/refs`: a reference in a field, a list, a request, and a reply.

### Changed

- `RuntimeCode` has two more variants, so an exhaustive `match` on it no
  longer compiles: add an arm for `Unbound` and `NoRuntime`, or a
  wildcard.
- Build pins: `bazel_utils` 0.4.0 and Buf CLI v1.74.0. The generated code is
  unchanged.
- buildifier, markdownlint, buf lint, rustfmt, and cargo-audit now run on
  Windows too, and the golden checks of the generated code on Windows
  ARM64: the reasons they were off are gone (`bazel_utils` runs its test
  scripts on Windows since 0.3.0, and catalogs every plugin there). Only
  the rustdoc tests stay off on Windows, where rules_rust cannot link them;
  `cargo test` runs them. The release workflow names each of these tests,
  so a platform that cannot run one fails.

## [0.3.1] - 2026-10-16

### Added

- `protocontract` and `protoc-gen-protocontract-buffa` are ready for
  crates.io: keywords, categories, the license text in each package, and
  docs.rs builds `protocontract` with every feature, so it shows `Offload`
  and `Blocking`. A logo for the README and the rustdoc pages.
- `bazel test //...` checks the formatting of the hand-written Rust
  (`//rust:fmt`, rustfmt of the pinned toolchain).

### Changed

- The workspace takes its dependencies as caret ranges instead of exact
  pins (`buffa = "0.9.2"`, was `"=0.9.2"`), so a consumer of
  `protocontract` is not held to one version of buffa, futures-core, or
  tokio. `Cargo.lock` still fixes the versions this repository builds and
  tests.
- The README takes `protocontract` from crates.io
  (`protocontract = "0.3"`) and the plugin with `cargo install`, links to
  GitHub by absolute URLs, and corrects the names of the runtime's codes
  (`RuntimeCode::Panicked`, not `PANICKED`), the `strategy: all` of the
  example `buf.gen.yaml`, and how the static release binaries are built.

## [0.3.0] - 2026-10-16

### Changed

- **Breaking:** the project is renamed to protocontract-buffa. The
  repository is `github.com/sonalect/protocontract-buffa.rs` (GitHub
  redirects `sonalect/proto-contract.rs`), the plugin and its binary are
  `protoc-gen-protocontract-buffa` (was `protoc-gen-contract-rust`), and
  the Bazel module is `protocontract_buffa` (was `contract`). A consumer
  renames the plugin in `buf.gen.yaml` (`local:`) and in its Bazel
  `protoc.plugin` tag, points the `protocontract` git dependency at the new
  repository, and downloads the release binaries under their new names.
  The generated code is the same except for its header,
  `// @generated by protoc-gen-protocontract-buffa. DO NOT EDIT.`; the
  runtime crate keeps its name, `protocontract`, and the output files keep
  theirs (`<stem>.__contract.rs`, `<pkg>.mod.rs`).
- The minimum Rust version of `protocontract` and of the generated code is
  1.92 (it was 1.99.1 in 0.2.2): 1.92 is the oldest rustc that compiles the
  generated handles of bidi-streaming methods. The release workflow builds
  and tests both with Rust 1.92. The plugin, built from source with
  `cargo install`, needs Rust 1.99.1.

## [0.2.2] - 2026-10-16

### Changed

- Build pins refreshed: `bazel_utils` 0.3.4 (a prebuilt `protoc` on
  Windows ARM64 instead of one compiled from source), Rust toolchain 1.99.1,
  `syn` 3.0.9, and `tokio` 1.53.3. The generated code is unchanged.
- The minimum Rust version (`rust-version`) is 1.99.1.

## [0.2.1] - 2026-10-12

### Changed

- The plugin's release binaries are built and checked on Ubuntu 26.04 for
  Linux (amd64 and arm64). They stay static musl binaries.
- Build pins refreshed: Bazel 9.3.0, `bazel_utils` 0.3.2,
  `protoc-gen-connect-rust` v0.9.1 for the golden generate, and `syn` 3.0.7.
  The generated code is unchanged.

## [0.2.0] - 2026-10-07

### Added

- `examples/glossary`: a contract in use. A proto with a service and its
  error codes, the crate generated from it, a provider that implements the
  traits, a consumer that calls them without knowing the provider, and the
  application that wires the two (`cargo run -p glossary-app`).
- Open error codes: `protocontract::ErrorCode`, implemented by every protobuf
  enum that `protoc-gen-buffa` generates and by hand for a Rust enum, and
  `protocontract::Code`, a code with its type erased. A contract declares the
  codes its callers may branch on as an `enum` next to its `service`.
- `protocontract::Fault`, the trait of a failure (`code`, `retryable`,
  `backtrace`), and `protocontract::Failure`, the ready-made fault (code,
  message, `with_retryable`, `with_source`, a captured backtrace). Any
  `Fault` converts into `protocontract::Error` with `?`.
- `Error::is`, `code_as`, `retryable`, `stack`, `backtrace`,
  `downcast_ref`, and `context`; the `protocontract::Context` extension adds a
  stack frame to the error of a `Result` (`.context("load")`,
  `.with_context(|| …)`).
- The plugin finds a service's error-code enum by name
  (`GreeterErrorCode`, then `GreeterServiceErrorCode`, in the package of
  `GreeterService`), emits the alias `GreeterServiceErrorCode` next to the
  traits, and names it in their rustdoc. A service without such an enum is
  generated as before; a message or enum already named like the alias
  fails the run.
- `protocontract::RuntimeCode`: `PANICKED` (`Offload`), `CANNOT_BLOCK`
  (`Blocking` inside a current-thread runtime), `CANCELLED` (a blocking
  task dropped by its runtime).

### Changed

- **Breaking.** The runtime crate `contract` is now `protocontract`
  (directory `rust/protocontract`, Bazel target `//rust/protocontract`).
  Depend on `protocontract` and replace `contract::` paths; the plugin's
  `runtime=` default is `::protocontract`. A crate that mounts the traits
  may now name their module `contract`.
- **Breaking.** `contract::Status` is now `protocontract::Error`: every
  generated method returns `Result<_, protocontract::Error>`. Build one with
  `Error::new(code, message)`, where `code` is a value of the contract's
  error enum, or from a `Failure` or a `Fault` of your own.
- **Breaking.** The bridges report `RuntimeCode` values instead of
  `INTERNAL` and `FAILED_PRECONDITION`.
- The runtime depends on `buffa` (for `ErrorCode` on its enums) instead of
  `buffa-types`.
- Each method's parameter is named after its message in snake case
  (`word: Word`, `greet_request: GreetRequest`), plural for an inbound
  stream (`items`), instead of `request` and `requests`. Implementations
  are unaffected: Rust does not bind parameter names.

### Removed

- **Breaking.** The fixed `google.rpc.Code` set (`contract::Code` as an
  enum) and the per-code constructors (`Status::invalid_argument`, …):
  codes are open (see Added).
- **Breaking.** `Status::with_details`, `details`, and `into_parts`: the
  calls stay in one process, so the wire payloads of `google.rpc.Status`
  have no use.
- **Breaking.** `PartialEq` on the error: a fault cannot be compared.
  Compare codes (`error.is(code)`) instead.

## [0.1.0] - 2026-10-06

First release.

### Added

- `protoc-gen-contract-rust`, a protoc plugin that emits, per protobuf
  `service`:
  - `<Service>Sync`, whose methods return `Result<Reply, contract::Status>`.
    It is dyn-compatible: `Arc<dyn <Service>Sync>`.
  - `<Service>Async`, whose methods return
    `impl Future<Output = Result<Reply, Status>> + Send`, so implementations
    write plain `async fn` and generic callers pay no allocation.
  - `Dyn<Service>Async`, a cloneable handle (`new`, `from_arc`) that holds
    any `<Service>Async` implementation behind dynamic dispatch, implements
    the trait itself, and implements `Debug`.

  Requests are taken by value and replies returned owned. Comments in the
  `.proto` become rustdoc.
- Streaming methods of every kind: server, client, and bidirectional. The
  sync form takes and returns `contract::BoxIter<'static, _>`, the async
  form `impl contract::Stream`; items are `Result<_, Status>`, a reply
  stream sits inside the call's `Result`, and no stream borrows the
  service.
- Bridges between the forms, for every kind of method: `contract::Inline`
  (a sync implementation's async form, run inside `poll`),
  `contract::Offload` (the same on tokio's blocking pool), and
  `contract::Blocking` (an async implementation's sync form). `Offload`
  and `Blocking` come with the runtime's `tokio` feature. `Blocking` inside
  a current-thread runtime fails with `FAILED_PRECONDITION` instead of
  panicking, and every bridge ends a stream after its first `Err`.
- Plugin parameters `buffa_module=` (`crate` included), `extern_path=`,
  `file_per_package`, and `element_memory_limit=`, as
  `protoc-gen-connect-rust` reads them, plus `runtime=` for the path of the
  runtime crate and `gate_tokio_feature[=<name>]` to put the `Offload` and
  `Blocking` impls behind a Cargo feature. Message paths are absolute and
  match `protoc-gen-buffa` 0.9.2. Output is a `<stem>.__contract.rs` per
  proto and a `<pkg>.mod.rs` stitcher per package for
  `protoc-gen-buffa-packaging` (`filter=services`), or one
  `<dotted.pkg>.rs` per package with `file_per_package`. The output can sit
  next to `protoc-gen-connect-rust`'s, in the same modules.
- The plugin refuses, with a message that names the proto element: a
  message type no mapping covers, two methods with one Rust name, a
  generated name that another item of the package's module (buffa,
  connect-rust, or Contract) already has, and any unknown parameter.
- Runtime crate `contract`: `Status` (code, message, `Any` details; a
  mirror of `google.rpc.Status`), `Code` (the 17 `google.rpc.Code` values),
  `BoxFuture`, `BoxIter`, `BoxStream`, `Stream` (re-exported from
  `futures-core`), `IterStream`, `Inline`, and with `tokio` `ChannelStream`,
  `Offload`, and `Blocking`.
- Release binaries of the plugin for Linux, macOS, and Windows on amd64
  and arm64, static where the platform allows (macOS links only system
  libraries), with their sha256 in `checksums-sha256.txt`.

## Links

- [Unreleased]
- [0.4.2]
- [0.4.1]
- [0.4.0]
- [0.3.1]
- [0.3.0]
- [0.2.2]
- [0.2.1]
- [0.2.0]
- [0.1.0]

[Unreleased]: https://github.com/sonalect/protocontract-buffa.rs/compare/v0.4.2...HEAD
[0.4.2]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.4.2
[0.4.1]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.4.1
[0.4.0]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.4.0
[0.3.1]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.3.1
[0.3.0]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.3.0
[0.2.2]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.2.2
[0.2.1]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.2.1
[0.2.0]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.2.0
[0.1.0]: https://github.com/sonalect/protocontract-buffa.rs/releases/tag/v0.1.0

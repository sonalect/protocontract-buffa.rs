# protocontract-buffa

<img src="https://raw.githubusercontent.com/sonalect/protocontract-buffa.rs/main/assets/logo.svg" alt="protocontract-buffa logo" width="96">

[![crates.io](https://img.shields.io/crates/v/protocontract.svg)](https://crates.io/crates/protocontract)
[![docs.rs](https://docs.rs/protocontract/badge.svg)](https://docs.rs/protocontract)
[![License](https://img.shields.io/crates/l/protocontract.svg)](https://github.com/sonalect/protocontract-buffa.rs/blob/main/LICENSE)

Plain Rust traits for protobuf services.

`protoc-gen-protocontract-buffa` turns each protobuf `service` into Rust traits
over the message structs [buffa](https://github.com/anthropics/buffa)
generates. A component in the same process is called by a method call: no
encoding, no transport, no view types. A method can also take or give an
implementation of another service, in a field, a request, or a reply
([References to a service](#references-to-a-service)). The runtime crate
`protocontract` holds the types the generated code names. The runtime and the generated code need
Rust 1.92 or newer; building the plugin from source needs Rust 1.99.1 (the
release binaries need no Rust at all).

## What is generated

```protobuf
service Greeter {
  // Greet a person by name.
  rpc Greet(Person) returns (Greeting);
}
```

A contract that stays in one process reads best named the way the Rust
code reads it: the service as the trait (`Greeter`, not `GreeterService`),
the methods over the domain's own messages (`Person`, `Greeting`), not
`…Request` and `…Response` wrappers. buf's `STANDARD` lint asks for the RPC
style; relax `SERVICE_SUFFIX`, `RPC_REQUEST_STANDARD_NAME`,
`RPC_RESPONSE_STANDARD_NAME`, and `RPC_REQUEST_RESPONSE_UNIQUE` for such
protos. The plugin accepts either style.

For each service, three items:

```rust
/// The blocking form. Dyn-compatible: `Arc<dyn GreeterSync>`.
pub trait GreeterSync: Send + Sync {
    /// Greet a person by name.
    fn greet(&self, person: Person) -> Result<Greeting, protocontract::Error>;
}

/// The async form. Implementations write plain `async fn`; a generic
/// caller (`impl GreeterAsync`) pays no allocation.
pub trait GreeterAsync: Send + Sync {
    /// Greet a person by name.
    fn greet(&self, person: Person)
        -> impl Future<Output = Result<Greeting, protocontract::Error>> + Send;
}

/// Any `GreeterAsync` behind dynamic dispatch, for an implementation
/// chosen at run time. It implements `GreeterAsync` itself; each call
/// boxes its future. Cloning shares the implementation; it also
/// implements `Debug`.
#[derive(Clone)]
pub struct DynGreeterAsync { /* … */ }

impl DynGreeterAsync {
    pub fn new<T: GreeterAsync + 'static>(service: T) -> Self;
    pub fn from_arc<T: GreeterAsync + 'static>(service: Arc<T>) -> Self;
    /// Whether both handles hold the same implementation.
    pub fn ptr_eq(&self, other: &Self) -> bool;
}
```

Every method takes its request by value and returns its reply owned; the
error is `protocontract::Error` (see [Errors](#errors)). The parameter is
named after its message in snake case (`Person` gives
`person`), plural for an inbound stream (`Item` gives `items`); an
implementation may name it as it likes. Comments in the `.proto` become
rustdoc.

Implement the form natural to the work: sync for computation, async for
work that waits on I/O.

```rust
struct Greeter;

impl GreeterAsync for Greeter {
    async fn greet(&self, request: Person) -> Result<Greeting, Error> {
        if request.name.is_empty() {
            return Err(Error::new(GreeterErrorCode::EmptyName, "name is empty"));
        }
        Ok(Greeting { text: format!("Hello, {}!", request.name), ..Default::default() })
    }
}

let greeter = DynGreeterAsync::new(Greeter);
let reply = greeter.greet(request).await?;
```

### Streaming

Methods of every kind are generated. The sync form takes and returns
`protocontract::BoxIter`, the async form `protocontract::Stream`; the `Dyn…Async`
handle boxes them as `protocontract::BoxStream`. Stream items are
`Result<_, Error>`.

```protobuf
service Feed {
  rpc Watch(Limit) returns (stream Item);  // server streaming
  rpc Collect(stream Item) returns (Summary);     // client streaming
  rpc Echo(stream Item) returns (stream Item);    // bidirectional
}
```

```rust
pub trait FeedSync: Send + Sync {
    fn watch(&self, limit: Limit)
        -> Result<BoxIter<'static, Result<Item, Error>>, Error>;
    fn collect(&self, items: BoxIter<'static, Result<Item, Error>>) -> Result<Summary, Error>;
    fn echo(&self, items: BoxIter<'static, Result<Item, Error>>)
        -> Result<BoxIter<'static, Result<Item, Error>>, Error>;
}

pub trait FeedAsync: Send + Sync {
    fn watch(&self, limit: Limit) -> impl Future<
        Output = Result<impl Stream<Item = Result<Item, Error>> + Send + use<Self>, Error>,
    > + Send;
    fn collect<R>(&self, items: R) -> impl Future<Output = Result<Summary, Error>> + Send
    where
        R: Stream<Item = Result<Item, Error>> + Send + 'static;
    fn echo<R>(&self, items: R) -> impl Future<
        Output = Result<impl Stream<Item = Result<Item, Error>> + Send + use<Self, R>, Error>,
    > + Send
    where
        R: Stream<Item = Result<Item, Error>> + Send + 'static;
}
```

An `Err` before the first item fails the call; an `Err` item ends the
stream. No stream borrows the service: an implementation clones what its
stream needs, and a stream can outlive the call and move between threads.

## Errors

Every method of every service fails with one type, `protocontract::Error`. One
type, not one per service, because:

- an `Arc<dyn …Sync>` or a `Dyn…Async` handle holds implementations chosen
  at run time, which must agree on the error type;
- the bridges fail on their own (a panic, a runtime that cannot block) and
  must build an error whatever the service;
- `?` carries a failure from one contract into another without a
  conversion.

What a caller needs from a failure is said by the error itself, not by its
type:

| Part | What it says | Read with |
| - | - | - |
| code | the class of the failure, to branch on | `error.is(code)`, `error.code_as::<C>()`, `error.code()` |
| retryable | whether the same call may succeed later | `error.retryable()` |
| message | what happened, for a developer | `Display` |
| source | the error that caused it | `std::error::Error::source` |
| stack | the operations it passed on its way out, outermost first | `error.stack()` |
| backtrace | where it was made (with `RUST_BACKTRACE` set) | `error.backtrace()` |

### 1. The contract declares its codes

A contract's failure codes are part of the contract: put them in the
proto, as an `enum` next to the `service`, named `<Service>ErrorCode`.
For `service Greeter` the enum is `GreeterErrorCode`:

```protobuf
// Greets people by name.
service Greeter {
  rpc Greet(Person) returns (Greeting);
}

// The failures of Greeter a caller may branch on.
enum GreeterErrorCode {
  GREETER_ERROR_CODE_UNSPECIFIED = 0;
  // The name to greet is empty.
  GREETER_ERROR_CODE_EMPTY_NAME = 1;
  // The greeter cannot answer now; the same call may succeed later.
  GREETER_ERROR_CODE_BUSY = 2;
  // The greeter knows no such person.
  GREETER_ERROR_CODE_NO_SUCH_PERSON = 3;
}
```

Every implementation of the service fails with these codes, and every
caller branches on them. A code that is not in the enum means nothing to a
caller who knows only the contract.

### 2. The plugin links the service to its codes

The types cannot tie a service to its enum (the error type is shared by
all services), so the plugin ties them by name. It looks in the service's
package for a top-level enum named after the service, and names it in both
traits' rustdoc:

```rust
/// Blocking form of `example.v1.Greeter`: … It fails with the codes of
/// `GreeterErrorCode` (`example.v1.GreeterErrorCode`): branch on them with
/// `Error::is` or `Error::code_as`.
pub trait GreeterSync: Send + Sync { … }
```

So whoever holds a `GreeterSync` finds its codes in the trait's
documentation. A service without such an enum gets no mention, and is
generated as before.

A service named in the RPC style, `GreeterService`, finds its codes too:
the plugin looks for `GreeterErrorCode` first, then
`GreeterServiceErrorCode`. When the enum is `GreeterErrorCode`, it also
emits an alias named after the service, so the name still follows from the
trait (`GreeterServiceSync` → `GreeterServiceErrorCode`):

```rust
pub type GreeterServiceErrorCode = crate::proto::example::v1::GreeterErrorCode;
```

The alias and the enum are one type; use either. A message or enum that
already has the alias' name fails the run.

buffa gives each enum value a short constant beside its proto name:
`GreeterErrorCode::EmptyName` is `GREETER_ERROR_CODE_EMPTY_NAME`.

### 3. An implementation fails with a code

The common case is a code and a message:

```rust
use protocontract::{Error, Failure};

fn greet(&self, request: Person) -> Result<Greeting, Error> {
    if request.name.is_empty() {
        return Err(Error::new(GreeterErrorCode::EmptyName, "name is empty"));
    }
    …
}
```

`Failure` adds the rest, when there is more to say:

```rust
let backend = self.backend.lookup(&request.name).map_err(|cause| {
    Failure::new(GreeterErrorCode::Busy, "the directory did not answer")
        .with_retryable(true)   // the caller may try again later
        .with_source(cause)     // the I/O error, kept as the source
})?;
```

A failure with fields of its own (a path, a limit, the closest match)
implements `protocontract::Fault` on its own type; `?` turns it into an
`Error`:

```rust
#[derive(Debug)]
struct NoSuchPerson { name: String, closest: Option<String> }

impl std::fmt::Display for NoSuchPerson { … }
impl std::error::Error for NoSuchPerson {}

impl protocontract::Fault for NoSuchPerson {
    fn code(&self) -> protocontract::Code {
        GreeterErrorCode::NoSuchPerson.into()
    }
    // retryable() is false and backtrace() is None unless overridden.
}
```

### 4. A caller branches on the code

```rust
use protocontract::Context;

match greeter.greet(request) {
    Ok(reply) => …,
    Err(error) if error.is(GreeterErrorCode::EmptyName) => …,
    Err(error) if error.retryable() => …,   // try again later
    Err(error) => return Err(error.context("welcome")),
}
```

or, for every code of the contract at once:

```rust
match error.code_as::<GreeterErrorCode>() {
    Some(GreeterErrorCode::EmptyName) => …,
    Some(_) => …,   // another code of this contract
    None => …,      // not a code of this contract: a runtime code, or another contract's
}
```

A code is compared together with its type. `GreeterErrorCode` value 1 and
`FeedErrorCode` value 1 are different codes: `is` is `false` and
`code_as` returns `None` for a code of another enum. A failure whose code
the caller does not know falls through to its last branch; it still
carries its message, source, and stack.

On the way out, `context` adds an operation to the error's stack, and
`Display` prints the stack before the code:

```rust
let reply = greeter.greet(request).context("welcome")?;
// welcome: GREETER_ERROR_CODE_EMPTY_NAME: name is empty
```

`Context` works on any `Result` whose error converts into `Error`
(`.context("…")`, or `.with_context(|| format!(…))` to build the frame only
on failure). `error.downcast_ref::<NoSuchPerson>()` reaches an
implementation's own fault type; use it for diagnostics only, since a
caller that branches on it is bound to that implementation.

### Codes outside the contract

- `protocontract::RuntimeCode` holds the runtime's own codes:
  `RuntimeCode::Panicked` (a panic under `Offload`),
  `RuntimeCode::CannotBlock` (`Blocking` inside a current-thread runtime),
  `RuntimeCode::Cancelled` (a blocking call dropped by its runtime),
  `RuntimeCode::Unbound` (a reference to a service that holds no
  implementation), and `RuntimeCode::NoRuntime` (an async implementation
  asked for in the blocking form where no tokio runtime can run it).
  `Display` prints them as `PANICKED`, `CANNOT_BLOCK`, `CANCELLED`,
  `UNBOUND`, and `NO_RUNTIME`.
- A component may have codes of its own, outside any contract: a Rust enum
  that implements `protocontract::ErrorCode` (`to_i32`, `from_i32`, `name`)
  works like a protobuf enum.

### What the compiler does not check

That an implementation fails only with its contract's codes. Any enum is a
valid code, so a provider can return a code its callers do not know; a
caller then sees an unknown code and takes its fallback branch. Keep each
implementation to its contract's enum, and let its tests check the codes
it returns.

## Bridges

Each form is also implemented for wrappers around the other, so a caller
of one form can use an implementation of the other:

| Wrapper | Gives | How |
| - | - | - |
| `protocontract::Inline<T>` | a sync impl the async form | runs the call inside `poll`; for calls of microseconds |
| `protocontract::Offload<T>` | a sync impl the async form | runs the call on tokio's blocking pool, about 20 µs a call |
| `protocontract::Blocking<T>` | an async impl the sync form | blocks the caller on a stored tokio runtime handle |

```rust
// A sync implementation, called through the async form.
let service = DynGreeterAsync::new(Offload::new(MySyncGreeter, handle.clone()));
// The same, back in the sync form.
let sync = Blocking::new(service.clone(), handle);
```

- Every wrapper covers every kind of method, and each fits in a
  `Dyn…Async` handle or an `Arc<dyn …Sync>`.
- `Inline` reads an inbound stream to its end before the sync call, so a
  bidirectional call through it answers only after the last request.
  `Offload` and `Blocking` answer each request as it arrives.
- `Blocking` works from any thread outside a runtime and inside a
  multi-thread runtime; inside a current-thread runtime it fails with
  `RuntimeCode::CannotBlock` rather than block that runtime's only thread.
- `Offload` turns a panic of the sync implementation into
  `RuntimeCode::Panicked`.

## References to a service

Protobuf has data types only: a message cannot hold an interface. So a
method that takes or gives an implementation (a port a host hands a
component, an object a factory returns) is written with a *reference
message*: an empty message that names the service it stands for.

```protobuf
import "protocontract/v1/options.proto";

// Tells the time.
service Clock {
  rpc Now(google.protobuf.Empty) returns (Tick);
}

// A clock to read the time from.
message ClockRef {
  option (protocontract.v1.service) = "example.v1.Clock";
}

message Alarm {
  string label = 1;
  ClockRef clock = 2;            // data and an implementation in one message
  repeated ClockRef backups = 3;
}

service Alarms {
  rpc UseClock(ClockRef) returns (google.protobuf.Empty);
  rpc CurrentClock(google.protobuf.Empty) returns (ClockRef);
}
```

For each reference message the plugin emits a type in a module named
after it, next to the service's traits:

```rust
pub mod clock_ref {
    pub struct ClockRef { /* the implementation, if any */ }

    impl ClockRef {
        pub fn from_sync<T: ClockSync + 'static>(service: T) -> Self;
        pub fn from_shared(service: Arc<dyn ClockSync>) -> Self;
        pub fn from_async<T: ClockAsync + 'static>(service: T) -> Self;
        pub fn from_dyn(service: DynClockAsync) -> Self;
        pub fn is_bound(&self) -> bool;
        pub fn to_sync(&self) -> Result<Arc<dyn ClockSync>, Error>;
        pub fn to_async_inline(&self) -> Result<DynClockAsync, Error>;
        pub fn to_async_offload(&self) -> Result<DynClockAsync, Error>;  // tokio
    }
}
```

Map the message to that type with the same `extern_path` in both plugins;
every field, request, and reply of `ClockRef` then holds it:

```yaml
- local: protoc-gen-buffa
  opt: [extern_path=.example.v1.ClockRef=crate::traits::clock_ref::ClockRef]
- local: protoc-gen-protocontract-buffa
  opt: [buffa_module=crate::proto, extern_path=.example.v1.ClockRef=crate::traits::clock_ref::ClockRef]
```

```rust
// The caller hands over an implementation in its own form.
alarms.use_clock(ClockRef::from_sync(SystemClock))?;
alarms.use_clock(ClockRef::from_async(NetworkClock::new()))?;

// The receiver takes the form it needs.
let clock = clock_ref.to_sync()?;
let tick = clock.now(Empty::default())?;
```

- **Native when the forms match.** A reference holds the implementation as
  it was given. Asked for that same form, it gives back the same
  implementation, with no wrapper. A bridge runs only when the forms
  differ, and only the one the receiver names: `to_sync` calls an async
  implementation through `Blocking` on the tokio runtime current at the
  call; `to_async_inline` and `to_async_offload` call a blocking one
  through `Inline` (inside `poll`, for calls of microseconds) or `Offload`
  (on tokio's blocking pool). See [Bridges](#bridges).
- **One process.** A reference encodes as an empty message. Decoded, and as
  the default, it holds no implementation, and taking a form fails with
  `RuntimeCode::Unbound`. It has no JSON form: serializing one fails, and so
  does reading one; a message without it reads and writes as before.
- **Equality** is identity: two references are equal when they hold the
  same implementation.
- **No data.** A reference message declares no field, oneof, nested type,
  or extension; the run fails if it does. It is a top-level message, and
  it names a service of its own package. Its type sits in its own module
  (`clock_ref`), which the run checks is free.
- **With buffa's `json=true`**, pass `json=true` to this plugin too, and
  depend on `serde`. **Next to `protoc-gen-protovalidate-buffa`**, which
  calls `validate()` on every message field, pass `validate=true`: the
  reference type implements `Validate`, always valid.

`options.proto` ships inside the `protocontract` crate, under `proto/`
(`proto/protocontract/v1/options.proto`), so whatever has the crate has
the file. Add that directory to the `buf` module that compiles your
contract, next to your own protos; a Bazel build exposes it from the
crate's repository as a `filegroup`.

[`examples/refs`](https://github.com/sonalect/protocontract-buffa.rs/tree/main/examples/refs)
shows all of it in use, protovalidate included.

## Example

[`examples/glossary`](https://github.com/sonalect/protocontract-buffa.rs/tree/main/examples/glossary) is a whole contract in use: the
proto, the generated crate, a provider that implements it, a consumer that
calls it without knowing the provider, and the application that wires the
two. `cargo run -p glossary-app` runs it.

[`examples/refs`](https://github.com/sonalect/protocontract-buffa.rs/tree/main/examples/refs) is a contract whose messages and methods carry
references to a service; `cargo test -p refs-api` runs it.

## Use

Get the plugin from the
[Releases](https://github.com/sonalect/protocontract-buffa.rs/releases):
static binaries for Linux, macOS, and Windows on amd64 and arm64, named
`protoc-gen-protocontract-buffa-<tag>-<os>-<arch>`, with their sha256 in
`checksums-sha256.txt`. Releases up to v0.2.2 name the plugin
`protoc-gen-contract-rust`. Or build it from crates.io:

```bash
cargo install --locked protoc-gen-protocontract-buffa
```

Run it next to `protoc-gen-buffa`, each into its own `out` directory:

```yaml
version: v2
plugins:
  - local: protoc-gen-buffa
    out: src/generated/buffa
    strategy: all
  - local: protoc-gen-buffa-packaging
    out: src/generated/buffa
    strategy: all
  - local: protoc-gen-protocontract-buffa
    out: src/generated/contract
    strategy: all
    opt: [buffa_module=crate::proto]
  - local: protoc-gen-buffa-packaging
    out: src/generated/contract
    strategy: all
    opt: [filter=services]
```

Mount both trees. Name the traits' module anything but `protocontract`:
next to the crate `protocontract` the path `protocontract::…` would be
ambiguous.

```rust
#[path = "generated/buffa/mod.rs"]
pub mod proto;
#[path = "generated/contract/mod.rs"]
pub mod traits;
```

The crate depends on the runtime with its `tokio` feature, since the
generated code implements the traits for `Offload` and `Blocking`, and on
what buffa's output needs (`buffa`, and `buffa-types` for well-known
types):

```toml
[dependencies]
protocontract = { version = "0.4", features = ["tokio"] }
```

To keep tokio optional, pass `gate_tokio_feature` to the plugin: the
`Offload` and `Blocking` impls then sit behind a `tokio` feature of your
crate, which turns on the runtime's:

```toml
[dependencies]
protocontract = "0.4"

[features]
tokio = ["protocontract/tokio"]
```

`protoc-gen-connect-rust` may run on the same protos into the same crate,
even into the same modules: the plugin takes no name and no file that
connect-rust takes, and the run fails if a generated name meets an
existing one, naming both proto elements.

### Parameters

| Parameter | Meaning |
| - | - |
| `buffa_module=<path>` | where the buffa output is mounted; shorthand for `extern_path=.=<path>` |
| `extern_path=<proto>=<path>` | maps a proto package prefix to a Rust module; repeatable, longest prefix wins |
| `file_per_package` | one `<dotted.pkg>.rs` per package instead of per-proto files and a stitcher; drop the packaging step and mount each file yourself |
| `element_memory_limit=<bytes\|unlimited>` | decode bound of the request, as for `protoc-gen-buffa` |
| `runtime=<path>` | path of the runtime crate; default `::protocontract` |
| `gate_tokio_feature[=<name>]` | `Offload` and `Blocking` impls under `#[cfg(feature = "<name>")]`; default `tokio` |
| `json=<true\|false>` | buffa runs with `json=true`: reference types get serde impls too |
| `validate=<true\|false>` | `protoc-gen-protovalidate-buffa` runs too: reference types implement `Validate` |

Paths are absolute (`::…`, `crate`, or `crate::…`). Well-known types map
to `::buffa_types` on their own. Any other parameter fails the run.

## Status

Before 1.0, a minor version may break the generated code or the runtime;
[CHANGELOG.md](https://github.com/sonalect/protocontract-buffa.rs/blob/main/CHANGELOG.md) says what changed and what a
consumer changes. The design, its stages, and its decisions are in
[DESIGN.md](https://github.com/sonalect/protocontract-buffa.rs/blob/main/DESIGN.md).

The traits are for calls inside one process. Across processes, use
connect-rust's stubs directly. Validation is not part of the plugin: run
`protoc-gen-protovalidate-buffa` next to it, and call `validate()` on the
owned messages where you decide to.

## Build

```bash
cargo test --workspace
bazel test //...                        # the gate: golden files, clippy, rustfmt, lint, markdown
bazel run //proto:generate              # refresh the golden files after a plugin change
bazel run //examples/glossary:generate  # and the example's generated code
```

The release binary of the plugin, as the release workflow builds it:

```bash
env -u RUSTFLAGS cargo build --locked --profile dist --target <triple> -p protoc-gen-protocontract-buffa
```

The `dist` profile sets the optimisation (fat LTO, stripped symbols); the
target makes the binary static, with the flags in `.cargo/config.toml`
(`RUSTFLAGS` would replace them, so it is unset):

| Platform | Target | Linking |
| - | - | - |
| Linux | `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` | fully static: musl libc linked in (a static glibc is not supported) |
| Windows | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | C runtime linked in (`+crt-static`): no Visual C++ redistributable |
| macOS | `x86_64-apple-darwin`, `aarch64-apple-darwin` | system libraries only (Apple supports no static binary) |

An annotated version tag `vX.Y.Z` on `main` runs the release workflow:
Bazel build and tests and the plugin's static binary on Linux, macOS, and
Windows, each on amd64 and arm64; the runtime and the generated code built
and tested with the minimum Rust version; then a GitHub Release with the
six binaries and `checksums-sha256.txt`. The tag's message is the release
notes.

## License

Apache-2.0.

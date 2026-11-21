# Example: a glossary

Two components that talk through a Contract, in one process: a
**provider** that holds terms and a **consumer** that annotates text with
them. Neither knows the other; both know only the contract.

```text
proto/glossary/v1/glossary.proto     the contract: service, messages, error codes
        │  mise run generate   (from the repository root)
        ▼
api/        glossary-api       generated messages and traits, runtime re-exports
provider/   glossary-provider  MemoryGlossary implements GlossarySync
consumer/   glossary-consumer  Annotator calls an Arc<dyn GlossarySync>
app/        glossary-app       hands the provider to the consumer; prints the result
```

Run it:

```bash
cargo run -p glossary-app
```

```text
A Provider[1] implements the Contract[2]; a Consumer[3] calls it. Neither needs a prototype of the other.
  [1] Provider: A component that implements the traits generated from a contract.
  [2] Contract: A protobuf service: its messages are the data, its methods the operations.
  [3] Consumer: A component that calls a contract without knowing who provides it.
  not in the glossary: prototype
Terms on "pro": Protobuf, Provider
error: annotate ` `: GLOSSARY_ERROR_CODE_EMPTY_TERM: the word is empty
  code GLOSSARY_ERROR_CODE_EMPTY_TERM (1), retryable false, stack ["annotate ` `"]
```

## The contract

[`glossary.proto`](proto/glossary/v1/glossary.proto) declares the service
and, next to it, the codes a caller may branch on:

```protobuf
service Glossary {
  rpc Define(Word) returns (Term);
  rpc Terms(Prefix) returns (stream Term);
}

message Word { string text = 1; }
message Prefix { string text = 1; }
message Term { string name = 1; string definition = 2; }

enum GlossaryErrorCode {
  GLOSSARY_ERROR_CODE_UNSPECIFIED = 0;
  GLOSSARY_ERROR_CODE_EMPTY_TERM = 1;
  GLOSSARY_ERROR_CODE_UNKNOWN_TERM = 2;
  GLOSSARY_ERROR_CODE_BUSY = 3;
}
```

The contract never leaves the process, so it is named the way the Rust
code reads it, not the way an RPC API is. The service is `Glossary`, not
`GlossaryService`, and the methods take and return the domain's own
messages, not `…Request` and `…Response` wrappers. What the plugin
generates from it reads as plain Rust:

```rust
pub trait GlossarySync: Send + Sync {
    fn define(&self, word: Word) -> Result<Term, protocontract::Error>;
    fn terms(&self, prefix: Prefix)
        -> Result<BoxIter<'static, Result<Term, protocontract::Error>>, protocontract::Error>;
}
// and GlossaryAsync, DynGlossaryAsync
```

The workspace's `buf.yaml` lets a service go without the `Service` suffix
(`SERVICE_SUFFIX`) and messages without the `Request` / `Response` names.
A message still wraps a scalar (`Word`, `Prefix`): protobuf methods take a
message.

The enum is named `<Service>ErrorCode`, so the plugin finds it and names it
in both traits' rustdoc: a consumer reading `GlossarySync` learns there
which codes the service fails with.

[`buf.gen.yaml`](buf.gen.yaml) runs `protoc-gen-buffa` for the messages
and `protoc-gen-protocontract-buffa` for the traits; both write into
`api/src/generated`, checked in. [`api/src/lib.rs`](api/src/lib.rs) mounts
them and re-exports the runtime types (`Error`, `Failure`, `Context`, …),
so the other crates depend on `glossary-api` alone.

## The provider

[`provider/src/lib.rs`](provider/src/lib.rs) implements the generated
trait. It takes the message by value and returns an owned one. The trait
names each parameter after its message (`word: Word`); an implementation
may name it as it likes:

```rust
impl GlossarySync for MemoryGlossary {
    fn define(&self, word: Word) -> Result<Term, Error> {
        let key = word.text.trim().to_lowercase();
        if key.is_empty() {
            return Err(Error::new(GlossaryErrorCode::EmptyTerm, "the word is empty"));
        }
        let Some(term) = self.terms.get(&key) else {
            return Err(UnknownTerm { term: word.text, closest: self.closest(&key) }.into());
        };
        Ok(term.clone())
    }
    // terms returns a stream: see below
}
```

`Terms` is a server stream, and the provider streams for real. The sync
form returns `BoxIter`, an iterator: each `next()` the consumer calls finds
one term, and a consumer that stops early leaves the rest unread. A stream
must not borrow the service (it may outlive the call or move to another
thread), so `MemoryGlossary` keeps its terms in an `Arc`; the stream holds
its own clone of that `Arc` and a cursor, not a copy of the terms:

```rust
fn terms(&self, prefix: Prefix) -> Result<BoxIter<'static, Result<Term, Error>>, Error> {
    Ok(Box::new(TermStream::new(Arc::clone(&self.terms), prefix.text.to_lowercase())))
}

impl Iterator for TermStream {
    type Item = Result<Term, Error>;
    fn next(&mut self) -> Option<Self::Item> { /* the next term after the cursor */ }
}
```

Two ways to fail:

- `Error::new(code, message)`: the common case.
- A type of its own (`UnknownTerm`) that implements `Fault`, when the
  failure has more to say (here, the closest known term). `?` or `.into()`
  turns it into an `Error`.

## The consumer

[`consumer/src/lib.rs`](consumer/src/lib.rs) holds an
`Arc<dyn GlossarySync>`; it does not depend on the provider crate.
It reads a failure through the contract, never through the provider:

```rust
match self.define(marked) {
    Ok(term) => { /* a numbered note */ }
    Err(error) if error.is(GlossaryErrorCode::UnknownTerm) => { /* keep the word */ }
    Err(error) => return Err(error.context(format!("annotate `{marked}`"))),
}
```

- `error.is(code)` / `error.code_as::<GlossaryErrorCode>()` branch on
  the contract's codes, the same for every provider. Each check compares
  the code's type as well as its number: a code of another enum never
  matches, and `code_as` returns `None` for it.
- `error.retryable()` drives the retry loop: a busy glossary is asked
  again, up to three times.
- `error.context(…)` and `Context::with_context` put the operation on the
  error's stack on its way out.
- `AnnotatorCode` is the consumer's own code, a Rust enum that implements
  `ErrorCode`, for a failure the contract does not name: a `[[` that is
  never closed.
- `Annotator::index` calls `terms` (RPC `Terms`; method names are snake
  case) and reads the stream term by term.

Its tests use stub providers (busy, broken streams) in place of
`MemoryGlossary`: the consumer is tested against the
contract alone.

## The application

[`app/src/main.rs`](app/src/main.rs) is the only crate that names both
sides:

```rust
let glossary: Arc<dyn GlossarySync> = Arc::new(MemoryGlossary::new(TERMS));
let annotator = Annotator::new(glossary);
```

Another provider goes in the same place. An async consumer would take a
`DynGlossaryAsync` instead, and the application would hand it
`Offload::new(MemoryGlossary::new(TERMS), handle)`; the bridges come with
`glossary-api`'s `tokio` feature.

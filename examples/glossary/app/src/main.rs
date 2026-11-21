//! The glossary application: the one place that knows both the provider
//! and the consumer, and hands the one to the other: it keeps each glossary
//! under its topic in `Glossaries`, and the consumer opens the topic it
//! needs.
//!
//! ```text
//! cargo run -p glossary-app
//! ```

use glossary_api::{GlossariesSync, GlossaryRef, MessageField, Shelf};
use glossary_consumer::Annotator;
use glossary_provider::{MemoryGlossaries, MemoryGlossary};
use std::io::Write;
use std::process::ExitCode;

const TERMS: [(&str, &str); 4] = [
    (
        "Contract",
        "A protobuf service: its messages are the data, its methods the operations.",
    ),
    (
        "Provider",
        "A component that implements the traits generated from a contract.",
    ),
    (
        "Consumer",
        "A component that calls a contract without knowing who provides it.",
    ),
    (
        "Protobuf",
        "A language for describing data and services, with code generators for many languages.",
    ),
];

const RUST_TERMS: [(&str, &str); 2] = [
    (
        "Crate",
        "A unit of Rust code that Cargo builds and publishes.",
    ),
    ("Trait", "A set of methods a type implements."),
];

const TEXT: &str = "A [[provider]] implements the [[contract]]; a [[consumer]] calls it. \
                    Neither needs a [[prototype]] of the other.";

fn main() -> ExitCode {
    let mut out = std::io::stdout().lock();
    match run(&mut out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Wire the providers into the consumer, and show what the consumer gets.
fn run(out: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    // Each glossary goes to `Glossaries` as a reference, `GlossaryRef`:
    // from here on nobody sees its type. Another provider would go in the
    // same place.
    let glossaries = MemoryGlossaries::new();
    glossaries.add(shelf("Contracts", MemoryGlossary::new(TERMS)))?;
    glossaries.add(shelf("Rust", MemoryGlossary::new(RUST_TERMS)))?;

    // The consumer opens its topic and gets a reference back.
    let annotator = Annotator::open(&glossaries, "contracts")?;

    let annotated = annotator.annotate(TEXT)?;
    writeln!(out, "{}", annotated.text)?;
    for (index, term) in annotated.notes.iter().enumerate() {
        writeln!(out, "  [{}] {}: {}", index + 1, term.name, term.definition)?;
    }
    for term in &annotated.unknown {
        writeln!(out, "  not in the glossary: {term}")?;
    }

    writeln!(
        out,
        "Terms on \"pro\": {}",
        annotator.index("pro")?.join(", ")
    )?;
    writeln!(
        out,
        "Terms in Rust: {}",
        Annotator::open(&glossaries, "Rust")?.index("")?.join(", ")
    )?;

    // A failure the contract names: the consumer passes it on with the
    // term on its stack, and the code says what happened.
    match annotator.annotate("An empty marker: [[ ]].") {
        Ok(annotated) => writeln!(out, "unexpected: {}", annotated.text)?,
        Err(error) => {
            writeln!(out, "error: {error}")?;
            writeln!(
                out,
                "  code {} ({}), retryable {}, stack {:?}",
                error.code(),
                error.code().value(),
                error.retryable(),
                error.stack()
            )?;
        }
    }

    // A topic no glossary is kept on: `Glossaries` names the failure, and
    // the consumer adds the topic to the stack.
    if let Err(error) = Annotator::open(&glossaries, "Chemistry") {
        writeln!(out, "error: {error}")?;
    }
    Ok(())
}

/// A shelf with a reference to `glossary` on `topic`.
fn shelf(topic: &str, glossary: MemoryGlossary) -> Shelf {
    Shelf {
        topic: topic.to_owned(),
        glossary: MessageField::some(GlossaryRef::from_sync(glossary)),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn prints_the_annotated_text_and_the_failure() {
        let mut out = Vec::new();
        run(&mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        let expected = "\
A Provider[1] implements the Contract[2]; a Consumer[3] calls it. Neither needs a prototype of the other.
  [1] Provider: A component that implements the traits generated from a contract.
  [2] Contract: A protobuf service: its messages are the data, its methods the operations.
  [3] Consumer: A component that calls a contract without knowing who provides it.
  not in the glossary: prototype
Terms on \"pro\": Protobuf, Provider
Terms in Rust: Crate, Trait
error: annotate ` `: GLOSSARY_ERROR_CODE_EMPTY_TERM: the word is empty
  code GLOSSARY_ERROR_CODE_EMPTY_TERM (1), retryable false, stack [\"annotate ` `\"]
error: open `Chemistry`: GLOSSARIES_ERROR_CODE_UNKNOWN_TOPIC: no glossary on `Chemistry`
";
        assert_eq!(out, expected);
    }
}

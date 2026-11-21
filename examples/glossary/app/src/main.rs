//! The glossary application: the one place that knows both the provider
//! and the consumer, and hands the one to the other.
//!
//! ```text
//! cargo run -p glossary-app
//! ```

use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;

use glossary_api::GlossarySync;
use glossary_consumer::Annotator;
use glossary_provider::MemoryGlossary;

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

/// Wire the provider into the consumer, and show what the consumer gets.
fn run(out: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    // The provider, behind the contract's trait: from here on nobody sees
    // its type. Another provider would go in the same place.
    let glossary: Arc<dyn GlossarySync> = Arc::new(MemoryGlossary::new(TERMS));
    let annotator = Annotator::new(glossary);

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
    Ok(())
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
error: annotate ` `: GLOSSARY_ERROR_CODE_EMPTY_TERM: the word is empty
  code GLOSSARY_ERROR_CODE_EMPTY_TERM (1), retryable false, stack [\"annotate ` `\"]
";
        assert_eq!(out, expected);
    }
}

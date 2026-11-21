//! A consumer of the glossary contract: annotates a text with the
//! definitions of the terms it marks.
//!
//! It depends on the contract (`glossary-api`) only. The application hands
//! it some implementation of [`GlossarySync`], or [`GlossariesSync`] to
//! find one by topic; which one, this crate never learns. Its tests use
//! stubs in place of a real provider.

use std::sync::Arc;
use std::time::Duration;

use glossary_api::{
    Context, Error, ErrorCode, GlossariesSync, GlossaryErrorCode, GlossarySync, Prefix, Term,
    Topic, Word,
};

/// The consumer's own failures, outside the contract: a Rust enum is an
/// [`ErrorCode`] as well as a protobuf enum is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotatorCode {
    /// The text opens a marker with `[[` and never closes it.
    UnclosedMarker = 1,
}

impl ErrorCode for AnnotatorCode {
    fn to_i32(self) -> i32 {
        self as i32
    }

    fn from_i32(value: i32) -> Option<AnnotatorCode> {
        (value == 1).then_some(AnnotatorCode::UnclosedMarker)
    }

    fn name(self) -> &'static str {
        "ANNOTATOR_UNCLOSED_MARKER"
    }
}

/// A text with its marked terms resolved.
#[derive(Debug, Default, PartialEq)]
pub struct Annotated {
    /// The text with each `[[term]]` replaced by the term and the number of
    /// its note, or by the bare term when the glossary does not know it.
    pub text: String,
    /// The definitions, in the order of their numbers (the first is `[1]`).
    pub notes: Vec<Term>,
    /// The marked terms the glossary does not know, in text order.
    pub unknown: Vec<String>,
}

/// Annotates texts through a glossary, whoever provides it.
pub struct Annotator {
    glossary: Arc<dyn GlossarySync>,
    attempts: u32,
    pause: Duration,
}

impl Annotator {
    /// An annotator over `glossary` that tries a retryable call three
    /// times, 50 ms apart.
    pub fn new(glossary: Arc<dyn GlossarySync>) -> Annotator {
        Annotator {
            glossary,
            attempts: 3,
            pause: Duration::from_millis(50),
        }
    }

    /// An annotator over the glossary `glossaries` keeps on `topic`.
    ///
    /// `Glossaries` replies with a reference, `GlossaryRef`, to whichever
    /// implementation it keeps there; the annotator takes its blocking
    /// form and holds that from then on.
    ///
    /// # Errors
    ///
    /// `GLOSSARIES_ERROR_CODE_UNKNOWN_TOPIC` when no glossary is kept on
    /// `topic`, `UNBOUND` for a reference that holds no implementation, or
    /// `NO_RUNTIME` for an async implementation the blocking form cannot
    /// run; each with the topic on its stack.
    pub fn open(glossaries: &dyn GlossariesSync, topic: &str) -> Result<Annotator, Error> {
        let request = Topic {
            name: topic.to_owned(),
            ..Default::default()
        };
        let glossary = glossaries
            .open(request)
            .and_then(|glossary| glossary.to_sync())
            .with_context(|| format!("open `{topic}`"))?;
        Ok(Annotator::new(glossary))
    }

    /// This annotator, trying a retryable call `attempts` times (at least
    /// once), `pause` apart.
    #[must_use]
    pub fn with_retry(mut self, attempts: u32, pause: Duration) -> Annotator {
        self.attempts = attempts.max(1);
        self.pause = pause;
        self
    }

    /// `text` with each `[[term]]` resolved.
    ///
    /// A term the glossary does not know stays in the text as it is and is
    /// listed in [`Annotated::unknown`].
    ///
    /// # Errors
    ///
    /// [`AnnotatorCode::UnclosedMarker`] for a `[[` without its `]]`. Any
    /// other failure of the glossary, with the term it was about on its
    /// stack: `GLOSSARY_ERROR_CODE_EMPTY_TERM` for an empty marker, or
    /// `GLOSSARY_ERROR_CODE_BUSY` when the glossary stayed busy for every
    /// attempt.
    pub fn annotate(&self, text: &str) -> Result<Annotated, Error> {
        let mut annotated = Annotated::default();
        let mut rest = text;
        while let Some(open) = rest.find("[[") {
            let Some(close) = rest[open + 2..].find("]]") else {
                // The consumer's own failure, with its own code.
                return Err(Error::new(
                    AnnotatorCode::UnclosedMarker,
                    format!(
                        "`[[` at byte {} is never closed",
                        text.len() - rest.len() + open
                    ),
                ));
            };
            let marked = &rest[open + 2..open + 2 + close];
            annotated.text.push_str(&rest[..open]);
            match self.define(marked) {
                Ok(term) => {
                    annotated.text.push_str(&term.name);
                    annotated.notes.push(term);
                    let number = annotated.notes.len();
                    annotated.text.push_str(&format!("[{number}]"));
                }
                // The contract says what this code means, for every
                // provider: the text keeps the term.
                Err(error) if error.is(GlossaryErrorCode::UnknownTerm) => {
                    annotated.text.push_str(marked);
                    annotated.unknown.push(marked.to_owned());
                }
                Err(error) => return Err(error.context(format!("annotate `{marked}`"))),
            }
            rest = &rest[open + 2 + close + 2..];
        }
        annotated.text.push_str(rest);
        Ok(annotated)
    }

    /// The names of the terms that start with `prefix`, in the glossary's
    /// order.
    ///
    /// # Errors
    ///
    /// A failure of the glossary, before the list or within it.
    pub fn index(&self, prefix: &str) -> Result<Vec<String>, Error> {
        let request = Prefix {
            text: prefix.to_owned(),
            ..Default::default()
        };
        // RPC `Terms` is the method `terms`; its stream is read term by term.
        let terms = self
            .glossary
            .terms(request)
            .with_context(|| format!("index `{prefix}`"))?;
        let mut names = Vec::new();
        for term in terms {
            names.push(term.with_context(|| format!("index `{prefix}`"))?.name);
        }
        Ok(names)
    }

    /// The definition of `term`, retried while the glossary says the call
    /// may succeed later.
    fn define(&self, term: &str) -> Result<Term, Error> {
        let mut attempt = 1;
        loop {
            let word = Word {
                text: term.to_owned(),
                ..Default::default()
            };
            match self.glossary.define(word) {
                Ok(term) => return Ok(term),
                Err(error) if error.retryable() && attempt < self.attempts => {
                    attempt += 1;
                    std::thread::sleep(self.pause);
                }
                Err(error) => return Err(error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use glossary_api::{
        BoxIter, Empty, Error, Failure, GlossariesErrorCode, GlossariesSync, GlossaryErrorCode,
        GlossaryRef, GlossarySync, Prefix, RuntimeCode, Shelf, Term, Topic, Word,
    };

    use super::{Annotator, AnnotatorCode};

    fn term(name: &str) -> Term {
        Term {
            name: name.to_owned(),
            definition: format!("What {name} means."),
            ..Default::default()
        }
    }

    /// A stub provider: busy for the first `busy` calls, then it knows
    /// every term but `unknown`.
    struct Stub {
        busy: u32,
        calls: AtomicU32,
    }

    impl Stub {
        fn new(busy: u32) -> Arc<Stub> {
            Arc::new(Stub {
                busy,
                calls: AtomicU32::new(0),
            })
        }
    }

    impl GlossarySync for Stub {
        fn define(&self, word: Word) -> Result<Term, Error> {
            if self.calls.fetch_add(1, Ordering::Relaxed) < self.busy {
                return Err(
                    Failure::new(GlossaryErrorCode::Busy, "the glossary is loading")
                        .with_retryable(true)
                        .into(),
                );
            }
            match word.text.as_str() {
                "" => Err(Error::new(GlossaryErrorCode::EmptyTerm, "no term")),
                "unknown" => Err(Error::new(GlossaryErrorCode::UnknownTerm, "no such term")),
                name => Ok(term(name)),
            }
        }

        fn terms(&self, prefix: Prefix) -> Result<BoxIter<'static, Result<Term, Error>>, Error> {
            let items = vec![
                Ok(term(&format!("{}1", prefix.text))),
                Err(Failure::new(GlossaryErrorCode::Busy, "the list broke off")
                    .with_retryable(true)
                    .into()),
            ];
            Ok(Box::new(items.into_iter()))
        }
    }

    fn annotator(stub: Arc<Stub>) -> Annotator {
        Annotator::new(stub).with_retry(3, Duration::ZERO)
    }

    #[test]
    fn resolves_known_terms_and_keeps_unknown_ones() {
        let annotated = annotator(Stub::new(0))
            .annotate("A [[provider]] and an [[unknown]] for a [[consumer]].")
            .unwrap();
        assert_eq!(
            annotated.text,
            "A provider[1] and an unknown for a consumer[2]."
        );
        let names: Vec<&str> = annotated.notes.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["provider", "consumer"]);
        assert_eq!(annotated.unknown, ["unknown"]);
    }

    #[test]
    fn retries_while_the_glossary_is_busy() {
        let stub = Stub::new(2);
        let annotated = annotator(Arc::clone(&stub)).annotate("[[x]]").unwrap();
        assert_eq!(annotated.text, "x[1]");
        assert_eq!(stub.calls.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn gives_up_after_the_last_attempt() {
        let stub = Stub::new(5);
        let error = annotator(Arc::clone(&stub)).annotate("[[x]]").unwrap_err();
        assert!(error.is(GlossaryErrorCode::Busy), "{error}");
        assert!(error.retryable());
        assert_eq!(error.stack(), ["annotate `x`"]);
        assert_eq!(stub.calls.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn other_failures_carry_the_term() {
        let error = annotator(Stub::new(0)).annotate("[[]]").unwrap_err();
        assert!(error.is(GlossaryErrorCode::EmptyTerm), "{error}");
        assert_eq!(
            error.to_string(),
            "annotate ``: GLOSSARY_ERROR_CODE_EMPTY_TERM: no term"
        );
    }

    #[test]
    fn an_unclosed_marker_is_the_consumers_own_failure() {
        let stub = Stub::new(0);
        let error = annotator(Arc::clone(&stub))
            .annotate("[[x]] and [[open")
            .unwrap_err();
        assert_eq!(
            error.code_as::<AnnotatorCode>(),
            Some(AnnotatorCode::UnclosedMarker)
        );
        assert_eq!(
            error.to_string(),
            "ANNOTATOR_UNCLOSED_MARKER: `[[` at byte 10 is never closed"
        );
        assert_eq!(error.code_as::<GlossaryErrorCode>(), None);
    }

    #[test]
    fn a_broken_list_fails_the_index() {
        let error = annotator(Stub::new(0)).index("p").unwrap_err();
        assert!(error.is(GlossaryErrorCode::Busy), "{error}");
        assert_eq!(error.stack(), ["index `p`"]);
    }

    /// A stub `Glossaries`: a stub glossary on `stub`, an unbound
    /// reference on `unbound`, no glossary on any other topic.
    struct Shelves;

    impl GlossariesSync for Shelves {
        fn open(&self, topic: Topic) -> Result<GlossaryRef, Error> {
            match topic.name.as_str() {
                "stub" => Ok(GlossaryRef::from_shared(Stub::new(0))),
                "unbound" => Ok(GlossaryRef::default()),
                _ => Err(Error::new(
                    GlossariesErrorCode::UnknownTopic,
                    "no such topic",
                )),
            }
        }

        fn add(&self, _shelf: Shelf) -> Result<Empty, Error> {
            Ok(Empty::default())
        }
    }

    #[test]
    fn opens_the_glossary_of_a_topic() {
        let annotated = Annotator::open(&Shelves, "stub")
            .unwrap()
            .annotate("[[provider]]")
            .unwrap();
        assert_eq!(annotated.text, "provider[1]");
    }

    #[test]
    fn an_unknown_topic_carries_the_topic() {
        let Err(error) = Annotator::open(&Shelves, "chemistry") else {
            panic!("a topic with no glossary opened");
        };
        assert!(error.is(GlossariesErrorCode::UnknownTopic), "{error}");
        assert_eq!(error.stack(), ["open `chemistry`"]);
    }

    #[test]
    fn an_unbound_reference_fails_to_open() {
        let Err(error) = Annotator::open(&Shelves, "unbound") else {
            panic!("an unbound reference opened");
        };
        assert!(error.is(RuntimeCode::Unbound), "{error}");
        assert_eq!(error.stack(), ["open `unbound`"]);
    }
}

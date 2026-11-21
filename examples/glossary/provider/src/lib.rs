//! A provider of the glossary contract: terms held in memory, and
//! glossaries kept by topic.
//!
//! [`MemoryGlossary`] implements [`GlossarySync`], [`MemoryGlossaries`]
//! implements [`GlossariesSync`], and neither knows anything of who calls
//! it. A consumer receives them through the contract's traits and
//! references, and never names this crate.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Bound;
use std::sync::{Arc, Mutex, PoisonError};

use glossary_api::{
    BoxIter, Code, Empty, Error, Fault, GlossariesErrorCode, GlossariesSync, GlossaryErrorCode,
    GlossaryRef, GlossarySync, Prefix, Shelf, Term, Topic, Word,
};

/// A glossary over a fixed set of terms, looked up without regard to case.
#[derive(Debug, Default)]
pub struct MemoryGlossary {
    /// Terms by their lowercase name, shared with the streams that list
    /// them.
    terms: Arc<BTreeMap<String, Term>>,
}

impl MemoryGlossary {
    /// A glossary of `(name, definition)` pairs. A later pair replaces an
    /// earlier one with the same name in another case.
    pub fn new<'a>(entries: impl IntoIterator<Item = (&'a str, &'a str)>) -> MemoryGlossary {
        let terms = entries
            .into_iter()
            .map(|(name, definition)| {
                let term = Term {
                    name: name.to_owned(),
                    definition: definition.to_owned(),
                    ..Default::default()
                };
                (name.to_lowercase(), term)
            })
            .collect();
        MemoryGlossary {
            terms: Arc::new(terms),
        }
    }

    /// The known term that shares the longest prefix (two characters at
    /// least) with `key`.
    fn closest(&self, key: &str) -> Option<String> {
        self.terms
            .iter()
            .map(|(known, term)| (common_prefix(known, key), term))
            .filter(|(shared, _)| *shared >= 2)
            .max_by_key(|(shared, _)| *shared)
            .map(|(_, term)| term.name.clone())
    }
}

/// The number of leading characters `a` and `b` share.
fn common_prefix(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

impl GlossarySync for MemoryGlossary {
    fn define(&self, word: Word) -> Result<Term, Error> {
        let key = word.text.trim().to_lowercase();
        if key.is_empty() {
            // The common case: a contract code and a message.
            return Err(Error::new(
                GlossaryErrorCode::EmptyTerm,
                "the word is empty",
            ));
        }
        let Some(term) = self.terms.get(&key) else {
            // A fault of the provider's own type, with more to say.
            return Err(UnknownTerm {
                term: word.text,
                closest: self.closest(&key),
            }
            .into());
        };
        Ok(term.clone())
    }

    fn terms(&self, prefix: Prefix) -> Result<BoxIter<'static, Result<Term, Error>>, Error> {
        // The stream must not borrow `self`, so it holds its own handle on
        // the terms (an `Arc` clone, not a copy) and finds each term when
        // the consumer asks for it.
        Ok(Box::new(TermStream::new(
            Arc::clone(&self.terms),
            prefix.text.to_lowercase(),
        )))
    }
}

/// The stream of `Terms`: one term per `next`, found only
/// when the consumer asks. A consumer that stops early leaves the rest
/// unread and uncopied.
struct TermStream {
    terms: Arc<BTreeMap<String, Term>>,
    prefix: String,
    /// Where the next term is searched from: the prefix itself at first,
    /// then just after the last term sent.
    cursor: Bound<String>,
}

impl TermStream {
    fn new(terms: Arc<BTreeMap<String, Term>>, prefix: String) -> TermStream {
        TermStream {
            terms,
            cursor: Bound::Included(prefix.clone()),
            prefix,
        }
    }
}

impl Iterator for TermStream {
    type Item = Result<Term, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let from = self.cursor.as_ref().map(String::as_str);
        let (key, term) = self
            .terms
            .range::<str, _>((from, Bound::Unbounded))
            .next()?;
        if !key.starts_with(&self.prefix) {
            return None;
        }
        self.cursor = Bound::Excluded(key.clone());
        Some(Ok(term.clone()))
    }
}

/// The glossary has no such term: `GLOSSARY_ERROR_CODE_UNKNOWN_TERM`, with
/// the closest term it does have.
///
/// A consumer branches on the code, which every provider of the contract
/// reports. The `closest` field is this provider's own: a consumer reads it
/// in the message, or downcasts to this type only for diagnostics.
#[derive(Debug)]
pub struct UnknownTerm {
    /// The word as the caller spelled it.
    pub term: String,
    /// The closest term the glossary has, if any.
    pub closest: Option<String>,
}

impl fmt::Display for UnknownTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no term `{}`", self.term)?;
        if let Some(closest) = &self.closest {
            write!(f, "; did you mean `{closest}`?")?;
        }
        Ok(())
    }
}

impl std::error::Error for UnknownTerm {}

impl Fault for UnknownTerm {
    fn code(&self) -> Code {
        GlossaryErrorCode::UnknownTerm.into()
    }
}

/// Glossaries kept by topic, in memory: whichever implementation each
/// reference holds, this one never learns.
///
/// [`GlossariesSync::open`] hands out a clone of the reference kept under
/// the topic: the caller and the shelf then share one implementation.
#[derive(Debug, Default)]
pub struct MemoryGlossaries {
    /// The references by lowercase topic.
    shelves: Mutex<BTreeMap<String, GlossaryRef>>,
}

impl MemoryGlossaries {
    /// No glossary on any topic.
    pub fn new() -> MemoryGlossaries {
        MemoryGlossaries::default()
    }

    /// The references by topic. A panic of another caller while it held
    /// the lock leaves the map whole, so the lock is taken all the same.
    fn shelves(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, GlossaryRef>> {
        self.shelves.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl GlossariesSync for MemoryGlossaries {
    fn open(&self, topic: Topic) -> Result<GlossaryRef, Error> {
        let key = topic.name.trim().to_lowercase();
        self.shelves().get(&key).cloned().ok_or_else(|| {
            Error::new(
                GlossariesErrorCode::UnknownTopic,
                format!("no glossary on `{}`", topic.name),
            )
        })
    }

    fn add(&self, shelf: Shelf) -> Result<Empty, Error> {
        // A reference that holds no implementation would only fail later,
        // in the hands of whoever opens the topic: refuse it here.
        let Some(glossary) = shelf.glossary.into_option().filter(GlossaryRef::is_bound) else {
            return Err(Error::new(
                GlossariesErrorCode::NoGlossary,
                format!("the shelf for `{}` holds no glossary", shelf.topic),
            ));
        };
        self.shelves()
            .insert(shelf.topic.trim().to_lowercase(), glossary);
        Ok(Empty::default())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;
    use std::sync::Arc;

    use glossary_api::{
        GlossariesErrorCode, GlossariesSync, GlossaryErrorCode, GlossaryRef, GlossarySync,
        MessageField, Prefix, Shelf, Topic, Word,
    };

    use super::{MemoryGlossaries, MemoryGlossary, TermStream, UnknownTerm};

    fn glossary() -> MemoryGlossary {
        MemoryGlossary::new([
            ("Provider", "A component that implements a contract."),
            ("Protobuf", "A language for describing data and services."),
            ("Consumer", "A component that calls a contract."),
        ])
    }

    fn define(glossary: &MemoryGlossary, text: &str) -> Result<String, glossary_api::Error> {
        let word = Word {
            text: text.to_owned(),
            ..Default::default()
        };
        Ok(glossary.define(word)?.name)
    }

    fn prefix(text: &str) -> Prefix {
        Prefix {
            text: text.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn defines_a_term_without_regard_to_case() {
        assert_eq!(define(&glossary(), " provider ").unwrap(), "Provider");
    }

    #[test]
    fn an_empty_term_is_refused() {
        let error = define(&glossary(), "  ").unwrap_err();
        assert!(error.is(GlossaryErrorCode::EmptyTerm), "{error}");
        assert!(!error.retryable());
    }

    #[test]
    fn an_unknown_term_names_the_closest() {
        let error = define(&glossary(), "Prototype").unwrap_err();
        assert!(error.is(GlossaryErrorCode::UnknownTerm), "{error}");
        assert_eq!(
            error.to_string(),
            "GLOSSARY_ERROR_CODE_UNKNOWN_TERM: no term `Prototype`; did you mean `Protobuf`?"
        );
        let fault = error.downcast_ref::<UnknownTerm>().unwrap();
        assert_eq!(fault.closest.as_deref(), Some("Protobuf"));
    }

    #[test]
    fn the_stream_finds_one_term_per_next() {
        let glossary = glossary();
        let mut stream = TermStream::new(Arc::clone(&glossary.terms), "pro".to_owned());
        assert!(stream.next().is_some());
        // Only the first term was looked up; the cursor waits after it.
        assert_eq!(stream.cursor, Bound::Excluded("protobuf".to_owned()));
        assert!(stream.next().is_some());
        assert!(stream.next().is_none());
    }

    #[test]
    fn the_stream_outlives_the_glossary() {
        let glossary = glossary();
        let mut stream = glossary.terms(prefix("pro")).unwrap();
        drop(glossary);
        let first = stream.next().unwrap().unwrap();
        assert_eq!(first.name, "Protobuf");
    }

    #[test]
    fn lists_terms_by_prefix_in_order() {
        let terms: Vec<String> = glossary()
            .terms(prefix("PRO"))
            .unwrap()
            .map(|term| term.unwrap().name)
            .collect();
        assert_eq!(terms, ["Protobuf", "Provider"]);
    }

    fn shelf(topic: &str, glossary: GlossaryRef) -> Shelf {
        Shelf {
            topic: topic.to_owned(),
            glossary: MessageField::some(glossary),
            ..Default::default()
        }
    }

    fn topic(name: &str) -> Topic {
        Topic {
            name: name.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn opens_the_glossary_kept_under_a_topic() {
        let glossaries = MemoryGlossaries::new();
        let shared: Arc<dyn GlossarySync> = Arc::new(glossary());
        glossaries
            .add(shelf(
                "Contracts",
                GlossaryRef::from_shared(Arc::clone(&shared)),
            ))
            .unwrap();
        let opened = glossaries.open(topic(" contracts ")).unwrap();
        // The same implementation, not a copy of it.
        assert_eq!(opened, GlossaryRef::from_shared(shared));
        let term = opened
            .to_sync()
            .unwrap()
            .define(Word {
                text: "consumer".to_owned(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(term.name, "Consumer");
    }

    #[test]
    fn a_later_shelf_replaces_the_glossary() {
        let glossaries = MemoryGlossaries::new();
        glossaries
            .add(shelf("rust", GlossaryRef::from_sync(glossary())))
            .unwrap();
        let other =
            GlossaryRef::from_sync(MemoryGlossary::new([("Crate", "A unit of Rust code.")]));
        glossaries.add(shelf("Rust", other.clone())).unwrap();
        assert_eq!(glossaries.open(topic("rust")).unwrap(), other);
    }

    #[test]
    fn an_unknown_topic_is_refused() {
        let error = MemoryGlossaries::new()
            .open(topic("Chemistry"))
            .unwrap_err();
        assert!(error.is(GlossariesErrorCode::UnknownTopic), "{error}");
        assert_eq!(
            error.to_string(),
            "GLOSSARIES_ERROR_CODE_UNKNOWN_TOPIC: no glossary on `Chemistry`"
        );
    }

    #[test]
    fn a_shelf_without_a_glossary_is_refused() {
        let glossaries = MemoryGlossaries::new();
        for empty in [
            Shelf {
                topic: "none".to_owned(),
                ..Default::default()
            },
            shelf("unbound", GlossaryRef::default()),
        ] {
            let error = glossaries.add(empty).unwrap_err();
            assert!(error.is(GlossariesErrorCode::NoGlossary), "{error}");
        }
        assert!(glossaries.open(topic("unbound")).is_err());
    }
}

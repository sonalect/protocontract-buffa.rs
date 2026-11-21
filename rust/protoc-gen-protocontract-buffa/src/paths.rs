//! Rust paths: checks on the ones the user gives, and tokens for emission.

use proc_macro2::TokenStream;

use crate::Error;

/// Whether `path` starts at a crate root (`::…`, `crate`, or `crate::…`),
/// so it names the same item from any module the output is mounted in.
pub(crate) fn is_absolute(path: &str) -> bool {
    path.starts_with("::") || path == "crate" || path.starts_with("crate::")
}

/// Fail unless `path` is an absolute Rust path made of plain identifiers:
/// `::some_crate::…`, `crate`, or `crate::…`. `what` names the plugin
/// parameter for the message.
pub(crate) fn check_absolute_path(path: &str, what: &str) -> Result<(), Error> {
    let rest = path
        .strip_prefix("::")
        .or_else(|| path.strip_prefix("crate::"));
    let valid = path == "crate" || rest.is_some_and(|rest| rest.split("::").all(is_identifier));
    if valid {
        Ok(())
    } else {
        Err(Error::new(format!(
            "plugin parameter `{what}`: {path:?} is not an absolute Rust path; \
             write `::some_crate::module`, `crate`, or `crate::module`"
        )))
    }
}

/// Tokens of a Rust path. Keyword segments become raw identifiers, as
/// `protoc-gen-buffa` emits them.
pub(crate) fn path_tokens(path: &str) -> Result<TokenStream, Error> {
    let rest = path.trim_prefix("::");
    if rest.split("::").all(is_identifier) {
        Ok(buffa_codegen::idents::rust_path_to_tokens(path))
    } else {
        Err(Error::new(format!(
            "{path:?} is not a Rust path; check `buffa_module`, `extern_path`, and `runtime`"
        )))
    }
}

/// An ASCII identifier: a letter or `_` followed by letters, digits, or
/// `_`, and not `_` alone. Keywords pass; they are escaped on emission.
fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let starts_well = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    starts_well && segment != "_" && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::{check_absolute_path, is_absolute, path_tokens};

    #[test]
    fn absolute_paths() {
        assert!(is_absolute("::buffa_types::google::protobuf::Empty"));
        assert!(is_absolute("crate::proto::example::v1::Name"));
        assert!(!is_absolute("super::v1::Name"));
        assert!(!is_absolute("Name"));
        assert!(check_absolute_path("::protocontract", "runtime").is_ok());
        assert!(check_absolute_path("crate::a::b_c::D1", "runtime").is_ok());
        assert!(is_absolute("crate"));
        assert!(!is_absolute("crates::a"));
        assert!(check_absolute_path("crate", "buffa_module").is_ok());
        assert!(check_absolute_path("crates", "runtime").is_err());
        assert!(check_absolute_path("::a::", "runtime").is_err());
        assert!(check_absolute_path("::a::1b", "runtime").is_err());
        assert!(check_absolute_path("::a::_", "runtime").is_err());
    }

    #[test]
    fn keyword_segments_become_raw() {
        let tokens = path_tokens("crate::proto::example::type::v1::Name").unwrap();
        assert_eq!(
            tokens.to_string().replace(' ', ""),
            "crate::proto::example::r#type::v1::Name"
        );
        assert!(path_tokens("crate::a b").is_err());
        assert!(path_tokens("").is_err());
    }
}

//! Rustdoc from `.proto` comments.

use buffa_codegen::generated::descriptor::FileDescriptorProto;
use proc_macro2::TokenStream;
use quote::quote;

use crate::sanitize::sanitize_comment;

/// `FileDescriptorProto.message_type` field number.
const FILE_MESSAGE_TYPE: i32 = 4;
/// `FileDescriptorProto.service` field number.
const FILE_SERVICE: i32 = 6;
/// `ServiceDescriptorProto.method` field number.
const SERVICE_METHOD: i32 = 2;

/// The sanitized comment above (or after) top-level message `message` of
/// `file`.
pub(crate) fn message_comment(file: &FileDescriptorProto, message: usize) -> Option<String> {
    comment_at(file, &[FILE_MESSAGE_TYPE, index(message)?])
}

/// The sanitized comment above (or after) service `service` of `file`.
pub(crate) fn service_comment(file: &FileDescriptorProto, service: usize) -> Option<String> {
    comment_at(file, &[FILE_SERVICE, index(service)?])
}

/// The sanitized comment above (or after) method `method` of service
/// `service` of `file`.
pub(crate) fn method_comment(
    file: &FileDescriptorProto,
    service: usize,
    method: usize,
) -> Option<String> {
    comment_at(
        file,
        &[
            FILE_SERVICE,
            index(service)?,
            SERVICE_METHOD,
            index(method)?,
        ],
    )
}

/// `#[doc = " line"]` attributes, one per line of `text`.
///
/// prettyplease prints `#[doc = "X"]` as `///X`, so each non-empty line
/// gets a leading space; blank lines stay blank to keep paragraph breaks.
pub(crate) fn doc_attrs(text: &str) -> TokenStream {
    let lines = text.lines().map(|line| {
        if line.is_empty() {
            String::new()
        } else {
            format!(" {line}")
        }
    });
    quote! { #(#[doc = #lines])* }
}

fn index(i: usize) -> Option<i32> {
    i32::try_from(i).ok()
}

fn comment_at(file: &FileDescriptorProto, path: &[i32]) -> Option<String> {
    let location = file
        .source_code_info
        .location
        .iter()
        .find(|location| location.path == path)?;
    let comment = location
        .leading_comments
        .as_ref()
        .or(location.trailing_comments.as_ref())?;
    // protoc keeps the space after `//`; drop one per line, keep deeper
    // indentation (code blocks) and blank lines (paragraph breaks).
    let normalized = comment
        .lines()
        .map(|line| line.strip_prefix(' ').unwrap_or(line).trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    let cleaned = sanitize_comment(normalized.trim_matches('\n'));
    (!cleaned.is_empty()).then_some(cleaned)
}

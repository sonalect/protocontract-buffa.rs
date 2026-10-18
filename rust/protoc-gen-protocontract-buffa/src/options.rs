//! Plugin parameters, as `buf.gen.yaml` passes them in `opt:`.

use crate::Error;
use crate::paths::check_absolute_path;

/// Every parameter the plugin accepts, for the message that rejects an
/// unknown one.
const SUPPORTED: &str = "buffa_module=<rust_path>, extern_path=<proto>=<rust_path>, \
     file_per_package, element_memory_limit=<bytes|unlimited>, runtime=<rust_path>, \
     gate_tokio_feature, gate_tokio_feature=<name>, json=<true|false>, validate=<true|false>";

/// The Cargo feature `gate_tokio_feature` names when it has no value.
const DEFAULT_TOKIO_FEATURE: &str = "tokio";

/// The runtime crate path when `runtime=` is not given.
const DEFAULT_RUNTIME: &str = "::protocontract";

/// Parsed plugin parameters.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    /// Proto package prefix (with a leading dot) to Rust module path; the
    /// longest prefix wins. `buffa_module=<p>` is `(".", p)`.
    pub(crate) extern_paths: Vec<(String, String)>,
    /// One `<dotted.pkg>.rs` per package instead of per-proto files and a
    /// stitcher.
    pub(crate) file_per_package: bool,
    /// Absolute path of the runtime crate.
    pub(crate) runtime: String,
    /// The Cargo feature that gates the `Offload` and `Blocking` impls, if
    /// any.
    pub(crate) tokio_gate: Option<String>,
    /// buffa runs with `json=true`: the messages derive serde, so a
    /// reference type implements it too.
    pub(crate) json: bool,
    /// `protoc-gen-protovalidate-buffa` runs too: it calls `validate()` on
    /// every message field, so a reference type implements `Validate`.
    pub(crate) validate: bool,
}

impl Options {
    /// Parse the comma-separated `CodeGeneratorRequest.parameter`.
    pub(crate) fn parse(parameter: Option<&str>) -> Result<Options, Error> {
        let mut options = Options {
            extern_paths: Vec::new(),
            file_per_package: false,
            runtime: DEFAULT_RUNTIME.to_string(),
            tokio_gate: None,
            json: false,
            validate: false,
        };
        let mut runtime_given = false;
        let parameter = parameter.unwrap_or_default();
        for opt in parameter
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let (key, value) = match opt.split_once('=') {
                Some((key, value)) => (key.trim(), Some(value.trim())),
                None => (opt, None),
            };
            match (key, value) {
                ("buffa_module", Some(rust)) => {
                    check_absolute_path(rust, "buffa_module")?;
                    options.add_extern_path(".", rust)?;
                }
                ("extern_path", Some(mapping)) => {
                    let (proto, rust) = parse_extern_path(mapping)?;
                    options.add_extern_path(&proto, rust)?;
                }
                ("runtime", Some(rust)) => {
                    if runtime_given {
                        return Err(Error::new("plugin parameter `runtime` is given twice"));
                    }
                    check_absolute_path(rust, "runtime")?;
                    options.runtime = rust.to_string();
                    runtime_given = true;
                }
                ("file_per_package", None) => options.file_per_package = true,
                ("gate_tokio_feature", feature) => {
                    let feature = feature.unwrap_or(DEFAULT_TOKIO_FEATURE);
                    if !buffa_codegen::FeatureGateNames::is_valid_name(feature) {
                        return Err(Error::new(format!(
                            "plugin parameter `gate_tokio_feature`: {feature:?} is not a valid \
                             Cargo feature name"
                        )));
                    }
                    options.tokio_gate = Some(feature.to_string());
                }
                ("json", Some("true")) => options.json = true,
                ("json", Some("false")) => options.json = false,
                ("validate", Some("true")) => options.validate = true,
                ("validate", Some("false")) => options.validate = false,
                // `buffa_codegen::decode_request` read and applied it while
                // decoding the request; nothing is left to do here.
                (buffa_codegen::ELEMENT_MEMORY_LIMIT_OPT, Some(_)) => {}
                _ => {
                    return Err(Error::new(format!(
                        "unknown plugin parameter {opt:?}. Supported: {SUPPORTED}"
                    )));
                }
            }
        }
        Ok(options)
    }

    fn add_extern_path(&mut self, proto: &str, rust: &str) -> Result<(), Error> {
        if self.extern_paths.iter().any(|(p, _)| p == proto) {
            return Err(Error::new(format!(
                "proto prefix {proto} is mapped twice (by `buffa_module` or \
                 `extern_path`); keep one mapping"
            )));
        }
        self.extern_paths
            .push((proto.to_string(), rust.to_string()));
        Ok(())
    }
}

/// Split `<proto>=<rust>` and normalize the proto prefix to start with a dot.
fn parse_extern_path(mapping: &str) -> Result<(String, &str), Error> {
    let format_error = || {
        Error::new(format!(
            "invalid extern_path {mapping:?}, expected extern_path=.proto.pkg=::rust::path \
             (both sides non-empty)"
        ))
    };
    let (proto, rust) = mapping.split_once('=').ok_or_else(format_error)?;
    let (proto, rust) = (proto.trim(), rust.trim());
    if proto.is_empty() || rust.is_empty() {
        return Err(format_error());
    }
    check_absolute_path(rust, "extern_path")?;
    let proto = if proto.starts_with('.') {
        proto.to_string()
    } else {
        format!(".{proto}")
    };
    Ok((proto, rust))
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn defaults() {
        let options = Options::parse(None).unwrap();
        assert!(options.extern_paths.is_empty());
        assert!(!options.file_per_package);
        assert_eq!(options.runtime, "::protocontract");
        assert_eq!(options.tokio_gate, None);
    }

    #[test]
    fn every_parameter() {
        let options = Options::parse(Some(
            " buffa_module=crate::proto , extern_path=example.shared=::shared,\
             file_per_package,element_memory_limit=unlimited,runtime=crate::rt,",
        ))
        .unwrap();
        assert_eq!(
            options.extern_paths,
            [
                (".".to_string(), "crate::proto".to_string()),
                (".example.shared".to_string(), "::shared".to_string()),
            ]
        );
        assert!(options.file_per_package);
        assert_eq!(options.runtime, "crate::rt");
        assert_eq!(options.tokio_gate, None);
    }

    #[test]
    fn tokio_gate() {
        let options = Options::parse(Some("gate_tokio_feature")).unwrap();
        assert_eq!(options.tokio_gate.as_deref(), Some("tokio"));
        let options = Options::parse(Some("gate_tokio_feature=bridges")).unwrap();
        assert_eq!(options.tokio_gate.as_deref(), Some("bridges"));
        let error = Options::parse(Some("gate_tokio_feature=no good")).unwrap_err();
        assert!(
            error.message().contains("not a valid Cargo feature name"),
            "{error}"
        );
    }

    #[test]
    fn unknown_parameter_lists_the_supported_ones() {
        for opt in [
            "views=true",
            "no_json",
            "gate_client_feature",
            "file_per_package=true",
            "buffa_module",
        ] {
            let error = Options::parse(Some(opt)).unwrap_err();
            assert!(
                error.message().contains("unknown plugin parameter"),
                "{error}"
            );
            assert!(
                error.message().contains("buffa_module=<rust_path>"),
                "{error}"
            );
        }
    }

    #[test]
    fn paths_must_be_absolute() {
        for opt in [
            "buffa_module=proto",
            "buffa_module=super::proto",
            "extern_path=.a=a::b",
            "runtime=contract",
            "runtime=::",
            "buffa_module=crate::",
            "buffa_module=crate::not a path",
            "buffa_module=crate::r#type",
        ] {
            let error = Options::parse(Some(opt)).unwrap_err();
            assert!(
                error.message().contains("absolute Rust path"),
                "{opt}: {error}"
            );
        }
    }

    #[test]
    fn malformed_extern_path() {
        for opt in ["extern_path=.a", "extern_path==::a", "extern_path=.a="] {
            let error = Options::parse(Some(opt)).unwrap_err();
            assert!(
                error.message().contains("invalid extern_path"),
                "{opt}: {error}"
            );
        }
    }

    #[test]
    fn a_prefix_is_mapped_once() {
        let error =
            Options::parse(Some("buffa_module=crate::a,extern_path=.=crate::b")).unwrap_err();
        assert!(error.message().contains("mapped twice"), "{error}");
        let error = Options::parse(Some("runtime=::a,runtime=::b")).unwrap_err();
        assert!(error.message().contains("given twice"), "{error}");
    }
}

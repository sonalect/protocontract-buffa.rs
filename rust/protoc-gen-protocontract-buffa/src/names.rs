//! Names of the generated items, and the check that they are free.
//!
//! protocontract-buffa's output is often mounted into the module that already holds
//! a package's buffa messages and connect-rust's service items. Two items
//! with one name in one module do not compile (E0428), so the run fails
//! first and names both proto elements.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use buffa_codegen::generated::descriptor::{FileDescriptorProto, ServiceDescriptorProto};
use heck::{ToSnakeCase, ToUpperCamelCase};

use crate::Error;
use crate::references::Reference;

/// The names protocontract-buffa gives the items of one service.
pub(crate) struct ServiceNames {
    /// The blocking trait, `<Service>Sync`.
    pub(crate) sync: String,
    /// The async trait, `<Service>Async`.
    pub(crate) asynchronous: String,
    /// The dynamic-dispatch handle, `Dyn<Service>Async`.
    pub(crate) dynamic: String,
    /// The private module of the handle's erased trait,
    /// `__dyn_<service>_async`.
    pub(crate) erased_module: String,
}

impl ServiceNames {
    pub(crate) fn of(service: &ServiceDescriptorProto) -> ServiceNames {
        let base = service
            .name
            .as_deref()
            .unwrap_or_default()
            .to_upper_camel_case();
        let asynchronous = format!("{base}Async");
        ServiceNames {
            sync: format!("{base}Sync"),
            dynamic: format!("Dyn{asynchronous}"),
            erased_module: format!("__dyn_{}", asynchronous.to_snake_case()),
            asynchronous,
        }
    }

    fn all(self) -> [String; 4] {
        [
            self.sync,
            self.asynchronous,
            self.dynamic,
            self.erased_module,
        ]
    }
}

/// The enum of a service's error codes, found by its name.
///
/// For `service GlossaryService` it is the top-level enum
/// `GlossaryErrorCode` (the service name without its `Service` suffix) or
/// `GlossaryServiceErrorCode` (the whole name) in the same package, in that
/// order. protocontract-buffa names it in the traits' rustdoc and emits the alias
/// `<Service>ErrorCode`, so a caller finds the codes from the trait.
pub(crate) struct ErrorCodes {
    /// The proto name of the enum, `.pkg.Name`.
    pub(crate) fqn: String,
    /// The enum's name in the package, `pkg.Name`.
    pub(crate) qualified: String,
    /// The alias `<Service>ErrorCode`; `None` when the enum has that name
    /// already.
    pub(crate) alias: Option<String>,
}

impl ErrorCodes {
    /// The name a caller writes: the alias, or the enum's own name.
    pub(crate) fn rust_name(&self) -> &str {
        match &self.alias {
            Some(alias) => alias,
            None => self.qualified.rsplit('.').next().unwrap_or(&self.qualified),
        }
    }
}

/// The error-code enum of `service` in `package`, if the package has one.
pub(crate) fn error_codes(
    files: &[FileDescriptorProto],
    package: &str,
    service: &ServiceDescriptorProto,
) -> Option<ErrorCodes> {
    let name = service.name.as_deref().unwrap_or_default();
    let base = name
        .strip_suffix("Service")
        .filter(|base| !base.is_empty())
        .unwrap_or(name);
    let alias = format!("{}ErrorCode", name.to_upper_camel_case());
    [format!("{base}ErrorCode"), format!("{name}ErrorCode")]
        .into_iter()
        .find(|wanted| {
            files
                .iter()
                .filter(|file| file.package.as_deref().unwrap_or_default() == package)
                .flat_map(|file| &file.enum_type)
                .any(|enumeration| enumeration.name.as_deref() == Some(wanted.as_str()))
        })
        .map(|found| {
            let qualified = qualified(package, &found);
            ErrorCodes {
                fqn: format!(".{qualified}"),
                alias: (found != alias).then_some(alias),
                qualified,
            }
        })
}

/// The Rust method name of an RPC, as `protoc-gen-connect-rust` makes it.
pub(crate) fn method_name(rpc: &str) -> String {
    rpc.to_snake_case()
}

/// The name of a method's parameter: the request message's name in snake
/// case (`Word` gives `word`), plural for an inbound stream (`words`). A
/// name the generated bodies use for a local of their own (`service`,
/// `stream`) gets a trailing `_`.
pub(crate) fn parameter_name(input_type: &str, streams_in: bool) -> String {
    let short = input_type.rsplit('.').next().unwrap_or(input_type);
    let mut name = short.to_snake_case();
    if streams_in {
        name = plural(&name);
    }
    if matches!(name.as_str(), "service" | "stream") {
        name.push('_');
    }
    name
}

/// `word` in the plural, by the regular English rules: `item` gives
/// `items`, `match` gives `matches`, `reply` gives `replies`. Irregular
/// plurals are not known; the name stays readable either way.
fn plural(word: &str) -> String {
    let consonant_y = word.ends_with('y')
        && !word
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'));
    if consonant_y {
        format!("{}ies", &word[..word.len() - 1])
    } else if ["s", "x", "z", "ch", "sh"]
        .iter()
        .any(|end| word.ends_with(end))
    {
        format!("{word}es")
    } else if word.is_empty() {
        "requests".to_string()
    } else {
        format!("{word}s")
    }
}

/// `package.Name`, or `Name` in the empty package.
pub(crate) fn qualified(package: &str, name: &str) -> String {
    if package.is_empty() {
        name.to_string()
    } else {
        format!("{package}.{name}")
    }
}

/// Fail if a name protocontract-buffa gives an item of a service in `package` is
/// also the name of another item of the package's Rust module.
///
/// Items checked: messages and enums (buffa), the server trait and the `Ext`, `RegisterMarker`, `Server`, and `Client`
/// items of every service (connect-rust), and the items of every other
/// service (protocontract-buffa), the `<Service>ErrorCode` alias included. connect-rust's constants are upper snake case and
/// its `Owned…View` aliases end in `View`, so neither can match a name that
/// ends in `Sync` or `Async`.
pub(crate) fn check_clashes(files: &[FileDescriptorProto], package: &str) -> Result<(), Error> {
    let in_package = || {
        files
            .iter()
            .filter(move |file| file.package.as_deref().unwrap_or_default() == package)
    };

    let mut taken: HashMap<String, String> = HashMap::new();
    for file in in_package() {
        for message in &file.message_type {
            let name = message.name.as_deref().unwrap_or_default();
            taken.insert(
                name.to_string(),
                format!("message {}", qualified(package, name)),
            );
        }
        for enumeration in &file.enum_type {
            let name = enumeration.name.as_deref().unwrap_or_default();
            taken.insert(
                name.to_string(),
                format!("enum {}", qualified(package, name)),
            );
        }
        for service in &file.service {
            let name = service.name.as_deref().unwrap_or_default();
            let base = name.to_upper_camel_case();
            let origin = format!("service {} (connect-rust)", qualified(package, name));
            let server = if base == "Self" {
                "Self_".to_string()
            } else {
                base.clone()
            };
            for item in [
                server,
                format!("{base}Ext"),
                format!("{base}RegisterMarker"),
                format!("{base}Server"),
                format!("{base}Client"),
            ] {
                taken.insert(item, origin.clone());
            }
        }
    }

    for file in in_package() {
        for service in &file.service {
            let name = service.name.as_deref().unwrap_or_default();
            let origin = format!("service {}", qualified(package, name));
            let alias = error_codes(files, package, service).and_then(|codes| codes.alias);
            for item in ServiceNames::of(service).all().into_iter().chain(alias) {
                match taken.entry(item) {
                    Entry::Vacant(entry) => {
                        entry.insert(origin.clone());
                    }
                    Entry::Occupied(entry) => {
                        return Err(Error::new(format!(
                            "{origin}: generated item `{}` has the name of {} in the \
                             same Rust module; rename one of them in the proto",
                            entry.key(),
                            entry.get()
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Fail if the module of a reference type in `package` has the name of
/// another module there: another reference's, or the one buffa gives a
/// message for its nested types.
pub(crate) fn check_reference_clashes(
    files: &[FileDescriptorProto],
    package: &str,
    references: &[Reference],
) -> Result<(), Error> {
    let mut taken: HashMap<String, String> = HashMap::new();
    for file in files
        .iter()
        .filter(|file| file.package.as_deref().unwrap_or_default() == package)
    {
        for message in &file.message_type {
            if message.nested_type.is_empty() && message.enum_type.is_empty() {
                continue;
            }
            let name = message.name.as_deref().unwrap_or_default();
            taken.insert(
                buffa_codegen::idents::escape_mod_ident(&name.to_snake_case()),
                format!("the module of message {}", qualified(package, name)),
            );
        }
    }
    let in_package = references
        .iter()
        .filter(|reference| reference.fqn[1..].rsplit_once('.').map_or("", |(p, _)| p) == package);
    for reference in in_package {
        let origin = format!("the module of reference {}", &reference.fqn[1..]);
        match taken.entry(reference.module.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(origin);
            }
            Entry::Occupied(entry) => {
                return Err(Error::new(format!(
                    "{origin}: `{}` is also {} in the same Rust module; rename one of them in \
                     the proto",
                    entry.key(),
                    entry.get()
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use buffa_codegen::generated::descriptor::{
        DescriptorProto, EnumDescriptorProto, FileDescriptorProto, ServiceDescriptorProto,
    };

    use super::{ServiceNames, check_clashes, error_codes, method_name, parameter_name};

    fn service(name: &str) -> ServiceDescriptorProto {
        ServiceDescriptorProto {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    fn file(package: &str, messages: &[&str], services: &[&str]) -> FileDescriptorProto {
        FileDescriptorProto {
            name: Some(format!("{package}/f.proto")),
            package: Some(package.into()),
            message_type: messages
                .iter()
                .map(|name| DescriptorProto {
                    name: Some((*name).into()),
                    ..Default::default()
                })
                .collect(),
            service: services.iter().map(|name| service(name)).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn names_follow_connect_rust() {
        assert_eq!(
            ServiceNames::of(&service("GreeterService")).all(),
            [
                "GreeterServiceSync",
                "GreeterServiceAsync",
                "DynGreeterServiceAsync",
                "__dyn_greeter_service_async",
            ]
        );
        assert_eq!(
            ServiceNames::of(&service("greeter")).all(),
            [
                "GreeterSync",
                "GreeterAsync",
                "DynGreeterAsync",
                "__dyn_greeter_async"
            ]
        );
        assert_eq!(method_name("GetOptions"), "get_options");
        assert_eq!(method_name("HTTPGet"), "http_get");
    }

    #[test]
    fn parameters_are_named_after_their_message() {
        assert_eq!(parameter_name(".glossary.v1.Word", false), "word");
        assert_eq!(parameter_name(".a.v1.GreetRequest", false), "greet_request");
        assert_eq!(
            parameter_name(".a.v1.GreetRequest.Options", false),
            "options"
        );
        assert_eq!(parameter_name(".google.protobuf.Empty", false), "empty");
        assert_eq!(parameter_name(".a.v1.Item", true), "items");
        assert_eq!(parameter_name(".a.v1.Match", true), "matches");
        assert_eq!(parameter_name(".a.v1.Reply", true), "replies");
        assert_eq!(parameter_name(".a.v1.Key", true), "keys");
        assert_eq!(parameter_name(".a.v1.Status", true), "statuses");
        assert_eq!(parameter_name(".a.v1.Service", false), "service_");
        assert_eq!(parameter_name(".a.v1.Stream", false), "stream_");
        assert_eq!(parameter_name(".a.v1.Type", false), "type");
    }

    #[test]
    fn distinct_names_pass() {
        let files = [file(
            "a.v1",
            &["FooRequest", "FooReply"],
            &["FooService", "BarService"],
        )];
        assert!(check_clashes(&files, "a.v1").is_ok());
    }

    #[test]
    fn clash_with_a_message_names_both() {
        let files = [file("a.v1", &[], &["Foo"]), file("a.v1", &["FooSync"], &[])];
        let error = check_clashes(&files, "a.v1").unwrap_err();
        assert!(error.message().contains("service a.v1.Foo"), "{error}");
        assert!(error.message().contains("`FooSync`"), "{error}");
        assert!(error.message().contains("message a.v1.FooSync"), "{error}");
    }

    #[test]
    fn clash_with_an_enum() {
        let mut files = [file("a.v1", &[], &["Foo"])];
        files[0].enum_type.push(EnumDescriptorProto {
            name: Some("FooAsync".into()),
            ..Default::default()
        });
        let error = check_clashes(&files, "a.v1").unwrap_err();
        assert!(error.message().contains("enum a.v1.FooAsync"), "{error}");
    }

    #[test]
    fn clash_with_connect_rust_server_trait() {
        let files = [file("a.v1", &[], &["Foo", "FooAsync"])];
        let error = check_clashes(&files, "a.v1").unwrap_err();
        assert!(error.message().contains("service a.v1.Foo:"), "{error}");
        assert!(
            error
                .message()
                .contains("service a.v1.FooAsync (connect-rust)"),
            "{error}"
        );
    }

    #[test]
    fn clash_between_two_services_of_contract() {
        let files = [file("a.v1", &[], &["foo", "Foo"])];
        let error = check_clashes(&files, "a.v1").unwrap_err();
        assert!(error.message().contains("`FooSync`"), "{error}");
    }

    #[test]
    fn clash_with_the_dyn_handle() {
        let files = [file("a.v1", &["DynFooAsync"], &["Foo"])];
        let error = check_clashes(&files, "a.v1").unwrap_err();
        assert!(error.message().contains("`DynFooAsync`"), "{error}");
    }

    fn with_enum(mut file: FileDescriptorProto, name: &str) -> FileDescriptorProto {
        file.enum_type.push(EnumDescriptorProto {
            name: Some(name.into()),
            ..Default::default()
        });
        file
    }

    #[test]
    fn error_codes_are_found_by_name() {
        let files = [with_enum(
            file("a.v1", &[], &["FooService"]),
            "FooErrorCode",
        )];
        let codes = error_codes(&files, "a.v1", &service("FooService")).unwrap();
        assert_eq!(codes.fqn, ".a.v1.FooErrorCode");
        assert_eq!(codes.alias.as_deref(), Some("FooServiceErrorCode"));
        assert_eq!(codes.rust_name(), "FooServiceErrorCode");
    }

    #[test]
    fn error_codes_named_after_the_whole_service_need_no_alias() {
        let files = [with_enum(file("a.v1", &[], &["Foo"]), "FooErrorCode")];
        let codes = error_codes(&files, "a.v1", &service("Foo")).unwrap();
        assert_eq!(codes.alias, None);
        assert_eq!(codes.rust_name(), "FooErrorCode");
        assert!(check_clashes(&files, "a.v1").is_ok());

        let files = [with_enum(
            file("a.v1", &[], &["FooService"]),
            "FooServiceErrorCode",
        )];
        let codes = error_codes(&files, "a.v1", &service("FooService")).unwrap();
        assert_eq!(codes.qualified, "a.v1.FooServiceErrorCode");
        assert_eq!(codes.alias, None);
    }

    #[test]
    fn error_codes_stay_in_their_package() {
        let files = [
            file("a.v1", &[], &["FooService"]),
            with_enum(file("b.v1", &[], &[]), "FooErrorCode"),
        ];
        assert!(error_codes(&files, "a.v1", &service("FooService")).is_none());
        let files = [with_enum(
            file("a.v1", &[], &["FooService"]),
            "BarErrorCode",
        )];
        assert!(error_codes(&files, "a.v1", &service("FooService")).is_none());
    }

    #[test]
    fn clash_with_the_error_code_alias() {
        let files = [with_enum(
            file("a.v1", &["FooServiceErrorCode"], &["FooService"]),
            "FooErrorCode",
        )];
        let error = check_clashes(&files, "a.v1").unwrap_err();
        assert!(error.message().contains("`FooServiceErrorCode`"), "{error}");
        assert!(
            error.message().contains("message a.v1.FooServiceErrorCode"),
            "{error}"
        );
    }

    #[test]
    fn other_packages_do_not_count() {
        let files = [file("a.v1", &[], &["Foo"]), file("b.v1", &["FooSync"], &[])];
        assert!(check_clashes(&files, "a.v1").is_ok());
    }
}

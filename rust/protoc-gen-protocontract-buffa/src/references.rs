//! Messages that stand for a reference to a service.
//!
//! A message marked `(protocontract.v1.service) = "pkg.Clock"` becomes a
//! Rust type that holds an implementation of `pkg.Clock`, in the form it
//! was given. buffa maps the message to that type
//! (`extern_path=.pkg.ClockRef=<mount>::clock_ref::ClockRef`), so every
//! field, request, and reply of the message holds it. The type lives in a
//! module of its own, `clock_ref`, next to the service's traits: it carries
//! the `__buffa::view` path where buffa looks for the view of a mapped type,
//! and a module per reference keeps that path from meeting buffa's own
//! `__buffa` or another reference's.

use buffa::UnknownFieldData;
use buffa_codegen::generated::descriptor::{
    DescriptorProto, FileDescriptorProto, ServiceDescriptorProto,
};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::Error;
use crate::docs::{doc_attrs, message_comment};
use crate::emit::service_methods;
use crate::methods::Method;
use crate::names::{ServiceNames, qualified};
use crate::options::Options;
use crate::paths::path_tokens;
use crate::resolve::TypeResolver;

/// The field number of `protocontract.v1.service` on `MessageOptions`
/// (`rust/protocontract/proto/protocontract/v1/options.proto`).
pub(crate) const SERVICE_OPTION: u32 = 57391;

/// One reference message of a file to generate, checked.
pub(crate) struct Reference {
    /// `.pkg.ClockRef`.
    pub(crate) fqn: String,
    /// `ClockRef`.
    pub(crate) name: String,
    /// The module of the generated type: `clock_ref`.
    pub(crate) module: String,
    /// The proto file that declares the message.
    pub(crate) proto: String,
    /// The index of the message in the file's `message_type`.
    pub(crate) index: usize,
    /// The service referred to: `pkg.Clock`.
    pub(crate) service: String,
    /// The proto file that declares the service, and its index there.
    service_file: String,
    service_index: usize,
}

/// Every reference message of the files to generate.
///
/// # Errors
///
/// Fails on a malformed option, a reference message with fields, nested
/// types, or oneofs, a nested reference message, and a service that is
/// not in the message's package.
pub(crate) fn find(
    files: &[FileDescriptorProto],
    to_generate: &[String],
) -> Result<Vec<Reference>, Error> {
    let mut found = Vec::new();
    for name in to_generate {
        let Some(file) = files
            .iter()
            .find(|file| file.name.as_deref() == Some(name.as_str()))
        else {
            continue;
        };
        let package = file.package.as_deref().unwrap_or_default();
        for (index, message) in file.message_type.iter().enumerate() {
            let message_name = message.name.as_deref().unwrap_or_default();
            let full = qualified(package, message_name);
            refuse_nested(message, &full)?;
            let Some(service) = service_option(message, &full)? else {
                continue;
            };
            refuse_content(message, &full)?;
            let (service, service_file, service_index) =
                find_service(files, package, &full, &service)?;
            found.push(Reference {
                fqn: format!(".{full}"),
                name: message_name.to_string(),
                module: buffa_codegen::idents::escape_mod_ident(&message_name.to_snake_case()),
                proto: name.clone(),
                index,
                service,
                service_file,
                service_index,
            });
        }
    }
    Ok(found)
}

/// Fail unless every reference is mapped, by an `extern_path` of its own,
/// to the type this plugin emits for it: `<mount>::<module>::<Name>`.
pub(crate) fn check_paths(
    references: &[Reference],
    extern_paths: &[(String, String)],
) -> Result<(), Error> {
    for reference in references {
        let wanted = format!("::{}::{}", reference.module, reference.name);
        let advice = format!(
            "add extern_path={}=<where this plugin's output for the package is \
             mounted>{wanted} to the parameters of this plugin and of protoc-gen-buffa",
            reference.fqn
        );
        match extern_paths
            .iter()
            .find(|(proto, _)| *proto == reference.fqn)
        {
            None => {
                return Err(Error::new(format!(
                    "{}: the reference message is not mapped to its type; {advice}",
                    &reference.fqn[1..]
                )));
            }
            Some((_, rust)) if !rust.ends_with(&wanted) => {
                return Err(Error::new(format!(
                    "{}: extern_path maps it to {rust}, but this plugin emits its type as \
                     <mount>{wanted}; {advice}",
                    &reference.fqn[1..]
                )));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// The value of `(protocontract.v1.service)` on `message`, if set.
fn service_option(message: &DescriptorProto, full: &str) -> Result<Option<String>, Error> {
    let Some(options) = message.options.as_option() else {
        return Ok(None);
    };
    let mut value = None;
    for field in options.__buffa_unknown_fields.iter() {
        if field.number != SERVICE_OPTION {
            continue;
        }
        let UnknownFieldData::LengthDelimited(bytes) = &field.data else {
            return Err(Error::new(format!(
                "{full}: option (protocontract.v1.service) is not a string; import \
                 protocontract/v1/options.proto from the protocontract crate"
            )));
        };
        let text = String::from_utf8(bytes.clone()).map_err(|_| {
            Error::new(format!(
                "{full}: option (protocontract.v1.service) is not UTF-8"
            ))
        })?;
        // Protobuf keeps the last value of a repeated singular field.
        value = Some(text);
    }
    Ok(value)
}

/// Fail if a message nested in `message` is a reference.
fn refuse_nested(message: &DescriptorProto, full: &str) -> Result<(), Error> {
    for nested in &message.nested_type {
        let nested_full = format!("{full}.{}", nested.name.as_deref().unwrap_or_default());
        if service_option(nested, &nested_full)?.is_some() {
            return Err(Error::new(format!(
                "{nested_full}: a reference message must be a top-level message"
            )));
        }
        refuse_nested(nested, &nested_full)?;
    }
    Ok(())
}

/// Fail unless the reference message is empty.
fn refuse_content(message: &DescriptorProto, full: &str) -> Result<(), Error> {
    let what = if !message.field.is_empty() {
        Some("fields")
    } else if !message.oneof_decl.is_empty() {
        Some("oneofs")
    } else if !message.nested_type.is_empty() || !message.enum_type.is_empty() {
        Some("nested types")
    } else if !message.extension.is_empty() || !message.extension_range.is_empty() {
        Some("extensions")
    } else {
        None
    };
    match what {
        Some(what) => Err(Error::new(format!(
            "{full}: a reference message holds an implementation, not data, so it has \
             no {what}; remove them"
        ))),
        None => Ok(()),
    }
}

/// The service `name` names: `pkg.Service` in the package of the message.
/// Gives its full name, its file, and its index there.
fn find_service(
    files: &[FileDescriptorProto],
    package: &str,
    full: &str,
    name: &str,
) -> Result<(String, String, usize), Error> {
    let name = name.strip_prefix('.').unwrap_or(name);
    let short = match name.rsplit_once('.') {
        Some((service_package, short)) if service_package == package => short,
        None if package.is_empty() => name,
        _ => {
            return Err(Error::new(format!(
                "{full}: option (protocontract.v1.service) names {name:?}; a reference \
                 names a service of its own package, as `{}`",
                qualified(package, "Service")
            )));
        }
    };
    for file in files
        .iter()
        .filter(|file| file.package.as_deref().unwrap_or_default() == package)
    {
        if let Some(index) = file
            .service
            .iter()
            .position(|service| service.name.as_deref() == Some(short))
        {
            return Ok((
                name.to_string(),
                file.name.clone().unwrap_or_default(),
                index,
            ));
        }
    }
    Err(Error::new(format!(
        "{full}: option (protocontract.v1.service) names {name}, which is not a service \
         of the request (missing proto import?)"
    )))
}

/// The module that holds the type of `reference`.
///
/// `resolver` gives the Rust paths of the messages; `buffa_resolver` the
/// paths buffa itself gives them, without the reference's own mapping,
/// which is where buffa's view of the message lives.
pub(crate) fn reference_tokens(
    files: &[FileDescriptorProto],
    file: &FileDescriptorProto,
    reference: &Reference,
    resolver: &TypeResolver<'_>,
    buffa_resolver: &TypeResolver<'_>,
    options: &Options,
) -> Result<TokenStream, Error> {
    let rt = path_tokens(&options.runtime)?;
    let Some(service_file) = files
        .iter()
        .find(|file| file.name.as_deref() == Some(reference.service_file.as_str()))
    else {
        return Err(Error::new(format!(
            "{}: missing from proto_file",
            reference.service_file
        )));
    };
    let service: &ServiceDescriptorProto = &service_file.service[reference.service_index];
    let names = ServiceNames::of(service);
    let sync_trait = format_ident!("{}", names.sync);
    let async_trait = format_ident!("{}", names.asynchronous);
    let dyn_handle = format_ident!("{}", names.dynamic);
    let methods = service_methods(
        service_file,
        reference.service_index,
        service,
        resolver,
        &rt,
        &sync_trait,
        &async_trait,
    )?;
    let shared_impls = methods.iter().map(Method::shared_impl);

    let module = format_ident!("{}", reference.module.trim_start_matches("r#"));
    let module = if reference.module.starts_with("r#") {
        proc_macro2::Ident::new_raw(&module.to_string(), proc_macro2::Span::call_site())
    } else {
        module
    };
    let ty = format_ident!("{}", reference.name);
    let ty_name = &reference.name;
    let full_name = &reference.fqn[1..];
    let package = file.package.as_deref().unwrap_or_default();
    let type_url = format!("type.googleapis.com/{full_name}");
    let service_name = &reference.service;
    let buffa_path = buffa_resolver.rust_path(&reference.fqn)?;
    let view_path = match buffa_path.rsplit_once("::") {
        Some((module, _)) => format!("{module}::{}View", reference.name),
        None => format!("{}View", reference.name),
    };
    let view_path = path_tokens(&view_path)?;

    let comment = message_comment(file, reference.index)
        .map(|comment| format!("{comment}\n\n"))
        .unwrap_or_default();
    let type_doc = doc_attrs(&format!(
        "{comment}A reference to an implementation of `{service_name}`, in the form it was \
         given: blocking (`{sync}`) or async (`{asynchronous}`). Every field, request, and \
         reply of `{full_name}` holds one. Take the form the caller needs with `to_sync`, \
         `to_async_inline`, or `to_async_offload`.\n\nA reference lives in one process. \
         Encoded, it becomes an empty message; decoded, and as the default, it holds no \
         implementation, and taking a form fails with `UNBOUND`. It has no JSON form.\n\n\
         Two references are equal when they hold the same implementation.",
        sync = names.sync,
        asynchronous = names.asynchronous,
    ));
    // The link names the type through its module: the comment sits on the
    // module, outside it, where the type is not in scope.
    let module_doc = doc_attrs(&format!(
        "The reference type of `{full_name}`: [`{ty_name}`]({}::{ty_name}).",
        reference.module
    ));

    let form = quote!(#rt::reference::Form<dyn super::#sync_trait, super::#dyn_handle>);
    let arc = quote!(::std::sync::Arc);
    let async_to_sync = quote! {
        ::core::result::Result::Ok(#arc::new(#rt::Blocking::new(
            ::core::clone::Clone::clone(service),
            #rt::reference::current_handle(#service_name)?,
        )))
    };
    let async_to_sync = match &options.tokio_gate {
        None => async_to_sync,
        Some(feature) => quote! {{
            #[cfg(feature = #feature)]
            { #async_to_sync }
            #[cfg(not(feature = #feature))]
            {
                let _ = service;
                ::core::result::Result::Err(#rt::reference::no_runtime(#service_name))
            }
        }},
    };
    let tokio_gate = match &options.tokio_gate {
        Some(feature) => quote!(#[cfg(feature = #feature)]),
        None => TokenStream::new(),
    };
    let json = if options.json {
        quote! {
            impl ::serde::Serialize for #ty {
                fn serialize<S: ::serde::Serializer>(
                    &self,
                    _serializer: S,
                ) -> ::core::result::Result<S::Ok, S::Error> {
                    ::core::result::Result::Err(::serde::ser::Error::custom(
                        ::core::concat!("a reference to ", #service_name, " has no JSON form"),
                    ))
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #ty {
                fn deserialize<D: ::serde::Deserializer<'de>>(
                    _deserializer: D,
                ) -> ::core::result::Result<Self, D::Error> {
                    ::core::result::Result::Err(::serde::de::Error::custom(
                        ::core::concat!("a reference to ", #service_name, " cannot be read from JSON"),
                    ))
                }
            }

            impl ::buffa::json_helpers::ProtoElemJson for #ty {
                fn serialize_proto_json<S: ::serde::Serializer>(
                    v: &Self,
                    s: S,
                ) -> ::core::result::Result<S::Ok, S::Error> {
                    ::serde::Serialize::serialize(v, s)
                }

                fn deserialize_proto_json<'de, D: ::serde::Deserializer<'de>>(
                    d: D,
                ) -> ::core::result::Result<Self, D::Error> {
                    <Self as ::serde::Deserialize>::deserialize(d)
                }
            }
        }
    } else {
        TokenStream::new()
    };
    let validate = if options.validate {
        quote! {
            /// A reference carries no data, so it has nothing to break.
            impl ::protovalidate_buffa::Validate for #ty {
                fn validate(&self) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
                    ::core::result::Result::Ok(())
                }
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        #module_doc
        pub mod #module {
            #type_doc
            #[derive(Clone, Default)]
            pub struct #ty {
                // Public, as buffa's view builds the struct with
                // `..Default::default()`; not part of the API.
                #[doc(hidden)]
                pub __form: ::core::option::Option<#form>,
                #[doc(hidden)]
                pub __buffa_unknown_fields: ::buffa::UnknownFields,
            }

            impl #ty {
                /// A reference to the blocking implementation `service`.
                pub fn from_sync<T: super::#sync_trait + 'static>(service: T) -> Self {
                    Self::from_shared(#arc::new(service))
                }

                /// A reference to a shared blocking implementation.
                pub fn from_shared(service: #arc<dyn super::#sync_trait>) -> Self {
                    Self {
                        __form: ::core::option::Option::Some(#rt::reference::Form::Sync(service)),
                        __buffa_unknown_fields: ::core::default::Default::default(),
                    }
                }

                /// A reference to the async implementation `service`.
                pub fn from_async<T: super::#async_trait + 'static>(service: T) -> Self {
                    Self::from_dyn(super::#dyn_handle::new(service))
                }

                /// A reference to an async implementation behind dynamic
                /// dispatch.
                pub fn from_dyn(service: super::#dyn_handle) -> Self {
                    Self {
                        __form: ::core::option::Option::Some(#rt::reference::Form::Async(service)),
                        __buffa_unknown_fields: ::core::default::Default::default(),
                    }
                }

                /// Whether the reference holds an implementation. The default
                /// and a decoded reference do not.
                pub fn is_bound(&self) -> bool {
                    self.__form.is_some()
                }

                /// The implementation in the blocking form. An async one is
                /// called through `Blocking`, on the tokio runtime current
                /// now.
                ///
                /// # Errors
                ///
                /// `UNBOUND` when the reference holds no implementation;
                /// `NO_RUNTIME` for an async one outside a tokio runtime, or
                /// in a build without tokio.
                pub fn to_sync(&self) -> ::core::result::Result<#arc<dyn super::#sync_trait>, #rt::Error> {
                    match self.form()? {
                        #rt::reference::Form::Sync(service) => ::core::result::Result::Ok(#arc::clone(service)),
                        #rt::reference::Form::Async(service) => #async_to_sync,
                    }
                }

                /// The implementation in the async form. A blocking one is
                /// called through `Inline`, inside the future's poll: for
                /// calls of microseconds.
                ///
                /// # Errors
                ///
                /// `UNBOUND` when the reference holds no implementation.
                pub fn to_async_inline(&self) -> ::core::result::Result<super::#dyn_handle, #rt::Error> {
                    match self.form()? {
                        #rt::reference::Form::Sync(service) => ::core::result::Result::Ok(
                            super::#dyn_handle::new(#rt::Inline::new(Shared(#arc::clone(service)))),
                        ),
                        #rt::reference::Form::Async(service) => ::core::result::Result::Ok(::core::clone::Clone::clone(service)),
                    }
                }

                /// The implementation in the async form. A blocking one is
                /// called through `Offload`, on the blocking pool of the tokio
                /// runtime current now.
                ///
                /// # Errors
                ///
                /// `UNBOUND` when the reference holds no implementation;
                /// `NO_RUNTIME` for a blocking one outside a tokio runtime.
                #tokio_gate
                pub fn to_async_offload(&self) -> ::core::result::Result<super::#dyn_handle, #rt::Error> {
                    match self.form()? {
                        #rt::reference::Form::Sync(service) => ::core::result::Result::Ok(
                            super::#dyn_handle::new(#rt::Offload::new(
                                Shared(#arc::clone(service)),
                                #rt::reference::current_handle(#service_name)?,
                            )),
                        ),
                        #rt::reference::Form::Async(service) => ::core::result::Result::Ok(::core::clone::Clone::clone(service)),
                    }
                }

                fn form(&self) -> ::core::result::Result<&#form, #rt::Error> {
                    self.__form
                        .as_ref()
                        .ok_or_else(|| #rt::reference::unbound(#service_name))
                }
            }

            /// A shared blocking implementation, as a blocking implementation
            /// the bridges take by value.
            struct Shared(#arc<dyn super::#sync_trait>);

            impl super::#sync_trait for Shared {
                #(#shared_impls)*
            }

            impl ::core::fmt::Debug for #ty {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_tuple(#full_name).field(&self.__form).finish()
                }
            }

            impl ::core::cmp::PartialEq for #ty {
                fn eq(&self, other: &Self) -> bool {
                    match (&self.__form, &other.__form) {
                        (::core::option::Option::None, ::core::option::Option::None) => true,
                        (
                            ::core::option::Option::Some(#rt::reference::Form::Sync(a)),
                            ::core::option::Option::Some(#rt::reference::Form::Sync(b)),
                        ) => #arc::ptr_eq(a, b),
                        (
                            ::core::option::Option::Some(#rt::reference::Form::Async(a)),
                            ::core::option::Option::Some(#rt::reference::Form::Async(b)),
                        ) => a.ptr_eq(b),
                        _ => false,
                    }
                }
            }

            ::buffa::impl_default_instance!(#ty);

            impl ::buffa::MessageName for #ty {
                const PACKAGE: &'static str = #package;
                const NAME: &'static str = #ty_name;
                const FULL_NAME: &'static str = #full_name;
                const TYPE_URL: &'static str = #type_url;
            }

            /// A reference encodes as an empty message: nothing of the
            /// implementation can travel. Decoding keeps the fields it reads
            /// as unknown fields and binds nothing.
            impl ::buffa::Message for #ty {
                fn compute_size(&self, _cache: &mut ::buffa::SizeCache) -> u32 {
                    ::buffa::saturate_size(self.__buffa_unknown_fields.encoded_len() as u64)
                }

                fn write_to(&self, _cache: &mut ::buffa::SizeCache, buf: &mut impl ::buffa::EncodeSink) {
                    self.__buffa_unknown_fields.write_to(buf);
                }

                fn merge_field(
                    &mut self,
                    tag: ::buffa::encoding::Tag,
                    buf: &mut impl ::buffa::bytes::Buf,
                    ctx: ::buffa::DecodeContext<'_>,
                ) -> ::core::result::Result<(), ::buffa::DecodeError> {
                    self.__buffa_unknown_fields
                        .push(::buffa::encoding::decode_unknown_field(tag, buf, ctx)?);
                    ::core::result::Result::Ok(())
                }

                fn clear(&mut self) {
                    self.__form = ::core::option::Option::None;
                    self.__buffa_unknown_fields.clear();
                }
            }

            #json

            #validate

            #[doc(hidden)]
            pub mod __buffa {
                // buffa names the view of a mapped type at
                // `<mapped module>::__buffa::view`; its own view of the message
                // already gives this type as the owned one.
                pub mod view {
                    pub use #view_path;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use buffa::{MessageField, UnknownField, UnknownFieldData, UnknownFields};
    use buffa_codegen::generated::compiler::CodeGeneratorRequest;
    use buffa_codegen::generated::descriptor::{
        DescriptorProto, FieldDescriptorProto, FileDescriptorProto, MessageOptions,
        MethodDescriptorProto, ServiceDescriptorProto,
    };

    use super::SERVICE_OPTION;
    use crate::generate;

    /// A message whose `(protocontract.v1.service)` is `data`.
    fn marked(name: &str, data: UnknownFieldData) -> DescriptorProto {
        let mut unknown = UnknownFields::default();
        unknown.push(UnknownField {
            number: SERVICE_OPTION,
            data,
        });
        DescriptorProto {
            name: Some(name.into()),
            options: MessageField::some(MessageOptions {
                __buffa_unknown_fields: unknown.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn reference(name: &str, service: &str) -> DescriptorProto {
        marked(
            name,
            UnknownFieldData::LengthDelimited(service.as_bytes().to_vec()),
        )
    }

    fn plain(name: &str) -> DescriptorProto {
        DescriptorProto {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    /// `example/v1/clock.proto` (service `Clock`, message `Tick`) and
    /// `example/v1/refs.proto` with `messages`, of which only the second is
    /// generated unless `both`.
    fn request(messages: Vec<DescriptorProto>, parameter: &str) -> CodeGeneratorRequest {
        let empty = FileDescriptorProto {
            name: Some("google/protobuf/empty.proto".into()),
            package: Some("google.protobuf".into()),
            message_type: vec![plain("Empty")],
            ..Default::default()
        };
        let clock = FileDescriptorProto {
            name: Some("example/v1/clock.proto".into()),
            package: Some("example.v1".into()),
            message_type: vec![plain("Tick")],
            service: vec![ServiceDescriptorProto {
                name: Some("Clock".into()),
                method: vec![MethodDescriptorProto {
                    name: Some("Now".into()),
                    input_type: Some(".google.protobuf.Empty".into()),
                    output_type: Some(".example.v1.Tick".into()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let refs = FileDescriptorProto {
            name: Some("example/v1/refs.proto".into()),
            package: Some("example.v1".into()),
            message_type: messages,
            ..Default::default()
        };
        CodeGeneratorRequest {
            file_to_generate: vec!["example/v1/refs.proto".into()],
            parameter: Some(parameter.into()),
            proto_file: vec![empty, clock, refs],
            ..Default::default()
        }
    }

    const MAPPED: &str = "buffa_module=crate::proto,extern_path=.example.v1.ClockRef=crate::traits::clock_ref::ClockRef";

    fn error(request: &CodeGeneratorRequest) -> String {
        match generate(request) {
            Ok(_) => panic!("the request should fail"),
            Err(error) => error.message().to_string(),
        }
    }

    fn code(request: &CodeGeneratorRequest) -> String {
        let response = generate(request).unwrap();
        assert_eq!(response.error, None);
        assert_eq!(response.file.len(), 2, "a file and its stitcher");
        response.file[0]
            .content
            .as_deref()
            .unwrap()
            .split_whitespace()
            .collect::<String>()
            .replace(",)", ")")
            .replace(",>", ">")
    }

    #[test]
    fn a_reference_gets_a_module_with_its_type_in_a_file_without_services() {
        let code = code(&request(
            vec![reference("ClockRef", "example.v1.Clock")],
            MAPPED,
        ));
        assert!(code.contains("pubmodclock_ref{"), "{code}");
        assert!(
            code.contains("[`ClockRef`](clock_ref::ClockRef)"),
            "the module doc links the type through the module: {code}"
        );
        assert!(code.contains("pubstructClockRef{"), "{code}");
        assert!(code.contains(
            "pub__form:::core::option::Option<::protocontract::reference::Form<dynsuper::ClockSync,super::DynClockAsync>>"
        ));
        assert!(code.contains("implsuper::ClockSyncforShared{"), "{code}");
        assert!(code.contains("pubusecrate::proto::example::v1::ClockRefView;"));
        assert!(code.contains("constFULL_NAME:&'staticstr=\"example.v1.ClockRef\";"));
        assert!(code.contains("pubfnto_async_offload(&self)"));
        assert!(!code.contains("::serde::"), "no serde without json=true");
        assert!(
            !code.contains("protovalidate_buffa"),
            "no Validate without validate=true"
        );
    }

    #[test]
    fn json_and_the_tokio_gate_reach_the_type() {
        let parameter = format!("{MAPPED},json=true,validate=true,gate_tokio_feature");
        let code = code(&request(
            vec![reference("ClockRef", ".example.v1.Clock")],
            &parameter,
        ));
        assert!(code.contains("impl::serde::SerializeforClockRef{"));
        assert!(code.contains("impl<'de>::serde::Deserialize<'de>forClockRef{"));
        assert!(code.contains("#[cfg(feature=\"tokio\")]pubfnto_async_offload"));
        assert!(code.contains("#[cfg(not(feature=\"tokio\"))]"));
        assert!(code.contains("impl::protovalidate_buffa::ValidateforClockRef{"));
    }

    #[test]
    fn a_reference_must_be_mapped_to_its_type() {
        let message = error(&request(
            vec![reference("ClockRef", "example.v1.Clock")],
            "buffa_module=crate::proto",
        ));
        assert!(message.contains("example.v1.ClockRef: the reference message is not mapped"));
        assert!(message.contains(
            "extern_path=.example.v1.ClockRef=<where this plugin's output for the package is \
             mounted>::clock_ref::ClockRef"
        ));
        let message = error(&request(
            vec![reference("ClockRef", "example.v1.Clock")],
            "buffa_module=crate::proto,extern_path=.example.v1.ClockRef=crate::traits::ClockRef",
        ));
        assert!(
            message.contains("maps it to crate::traits::ClockRef"),
            "{message}"
        );
    }

    #[test]
    fn a_reference_holds_no_data() {
        let mut with_field = reference("ClockRef", "example.v1.Clock");
        with_field.field = vec![FieldDescriptorProto {
            name: Some("at".into()),
            ..Default::default()
        }];
        let message = error(&request(vec![with_field], MAPPED));
        assert!(
            message.contains("example.v1.ClockRef: a reference message"),
            "{message}"
        );
        assert!(message.contains("no fields"), "{message}");

        let mut with_nested = reference("ClockRef", "example.v1.Clock");
        with_nested.nested_type = vec![plain("Inner")];
        assert!(error(&request(vec![with_nested], MAPPED)).contains("no nested types"));
    }

    #[test]
    fn a_reference_names_a_service_of_its_package_that_exists() {
        let message = error(&request(
            vec![reference("ClockRef", "other.v1.Clock")],
            MAPPED,
        ));
        assert!(
            message.contains("names a service of its own package"),
            "{message}"
        );
        let message = error(&request(
            vec![reference("ClockRef", "example.v1.Watch")],
            MAPPED,
        ));
        assert!(
            message.contains("example.v1.Watch, which is not a service"),
            "{message}"
        );
    }

    #[test]
    fn a_reference_is_a_top_level_string_option() {
        let mut outer = plain("Outer");
        outer.nested_type = vec![reference("ClockRef", "example.v1.Clock")];
        let message = error(&request(vec![outer], MAPPED));
        assert!(
            message.contains("example.v1.Outer.ClockRef: a reference message must be a top-level")
        );
        let message = error(&request(
            vec![marked("ClockRef", UnknownFieldData::Varint(1))],
            MAPPED,
        ));
        assert!(message.contains("is not a string"), "{message}");
    }

    #[test]
    fn the_module_of_a_reference_is_free() {
        let mut other = plain("Clock_ref");
        other.nested_type = vec![plain("Inner")];
        let message = error(&request(
            vec![reference("ClockRef", "example.v1.Clock"), other],
            MAPPED,
        ));
        assert!(
            message.contains("the module of reference example.v1.ClockRef"),
            "{message}"
        );
        assert!(
            message.contains("the module of message example.v1.Clock_ref"),
            "{message}"
        );
    }
}
